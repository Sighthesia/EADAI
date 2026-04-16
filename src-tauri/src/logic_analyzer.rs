use crate::model::{
    LogicAnalyzerCaptureRequest, LogicAnalyzerCaptureResult, LogicAnalyzerCaptureState,
    LogicAnalyzerDevice, LogicAnalyzerSessionState, LogicAnalyzerStatus,
    LogicAnalyzerWaveformChannel,
};
use std::fs;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

const SIGROK_ENV: &str = "EADAI_SIGROK_CLI";
const DEFAULT_SIGROK_BIN: &str = "sigrok-cli";

pub struct LogicAnalyzerService {
    inner: Arc<Mutex<LogicAnalyzerState>>,
}

struct LogicAnalyzerState {
    status: LogicAnalyzerStatus,
    capture: Option<ManagedCapture>,
}

struct ManagedCapture {
    child: Child,
    command: String,
    output_path: String,
    sample_count: u32,
    samplerate_hz: Option<u64>,
}

impl Default for LogicAnalyzerService {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicAnalyzerService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LogicAnalyzerState {
                status: LogicAnalyzerStatus::default(),
                capture: None,
            })),
        }
    }

    pub fn status(&self) -> LogicAnalyzerStatus {
        let mut state = lock_state(&self.inner);
        state.reconcile_capture();
        state.status.clone()
    }

    pub fn refresh_devices(&self) -> Result<LogicAnalyzerStatus, String> {
        let mut state = lock_state(&self.inner);
        state.status.session_state = LogicAnalyzerSessionState::Scanning;
        state.status.last_error = None;

        let Some(executable) = resolve_sigrok_cli() else {
            state.status.available = false;
            state.status.executable = None;
            state.status.devices.clear();
            state.status.selected_device_ref = None;
            state.status.scan_output = None;
            state.status.session_state = LogicAnalyzerSessionState::Unavailable;
            state.status.last_error =
                Some("sigrok-cli not found on PATH or via EADAI_SIGROK_CLI".to_string());
            return Ok(state.status.clone());
        };

        state.status.executable = Some(executable.clone());

        match Command::new(&executable).arg("--scan").output() {
            Ok(output) => {
                let scan_output = combine_output(&output.stdout, &output.stderr);
                let devices = parse_scan_output(&scan_output);
                state.status.available = true;
                state.status.devices = devices;
                state.status.selected_device_ref = state
                    .status
                    .selected_device_ref
                    .clone()
                    .filter(|reference| {
                        state
                            .status
                            .devices
                            .iter()
                            .any(|device| device.reference == *reference)
                    })
                    .or_else(|| {
                        state
                            .status
                            .devices
                            .first()
                            .map(|device| device.reference.clone())
                    });
                state.status.scan_output = Some(scan_output.clone());
                state.status.last_scan_at_ms = Some(now_ms());
                state.status.last_error = if output.status.success() {
                    None
                } else {
                    Some(scan_output.clone())
                };
                state.status.session_state = if state.status.devices.is_empty() {
                    LogicAnalyzerSessionState::Ready
                } else {
                    LogicAnalyzerSessionState::Ready
                };
                Ok(state.status.clone())
            }
            Err(error) => {
                state.status.available = false;
                state.status.devices.clear();
                state.status.selected_device_ref = None;
                state.status.scan_output = None;
                state.status.session_state = LogicAnalyzerSessionState::Error;
                state.status.last_error = Some(format!("failed to execute sigrok-cli: {error}"));
                Ok(state.status.clone())
            }
        }
    }

    pub fn start_capture(
        &self,
        request: LogicAnalyzerCaptureRequest,
    ) -> Result<LogicAnalyzerStatus, String> {
        let mut state = lock_state(&self.inner);
        state.reconcile_capture();

        let executable = match state.status.executable.clone().or_else(resolve_sigrok_cli) {
            Some(value) => value,
            None => {
                state.status.available = false;
                state.status.session_state = LogicAnalyzerSessionState::Unavailable;
                state.status.last_error = Some("sigrok-cli not available".to_string());
                return Ok(state.status.clone());
            }
        };

        if request.device_ref.trim().is_empty() {
            state.status.session_state = LogicAnalyzerSessionState::Error;
            state.status.last_error = Some("device_ref is required".to_string());
            return Ok(state.status.clone());
        }

        if !state
            .status
            .devices
            .iter()
            .any(|device| device.reference == request.device_ref)
        {
            state.status.session_state = LogicAnalyzerSessionState::Error;
            state.status.last_error = Some(format!(
                "device '{}' is not present in the current sigrok scan",
                request.device_ref
            ));
            return Ok(state.status.clone());
        }

        let output_path = capture_output_path();
        let args = build_capture_args(&request, &output_path);
        let command = format_capture_command(&executable, &args);

        let child = Command::new(&executable)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("failed to start sigrok capture: {error}"))?;

        let pid = child.id();
        state.status.available = true;
        state.status.selected_device_ref = Some(request.device_ref.clone());
        state.status.active_capture = Some(LogicAnalyzerCaptureState {
            pid,
            started_at_ms: now_ms(),
            command: command.clone(),
            output_path: output_path.clone(),
        });
        state.status.capture_plan = Some(command);
        state.status.session_state = LogicAnalyzerSessionState::Capturing;
        state.status.last_error = None;
        state.capture = Some(ManagedCapture {
            child,
            command: state.status.capture_plan.clone().unwrap_or_default(),
            output_path,
            sample_count: request.sample_count,
            samplerate_hz: request.samplerate_hz,
        });

        Ok(state.status.clone())
    }

    pub fn stop_capture(&self) -> Result<LogicAnalyzerStatus, String> {
        let mut state = lock_state(&self.inner);
        state.reconcile_capture();

        let Some(mut capture) = state.capture.take() else {
            state.status.session_state = LogicAnalyzerSessionState::Idle;
            state.status.active_capture = None;
            return Ok(state.status.clone());
        };

        state.status.session_state = LogicAnalyzerSessionState::Stopping;
        let _ = capture.child.kill();
        let _ = capture.child.wait();
        state.status.active_capture = None;
        state.status.capture_plan = Some(capture.command);
        state.status.last_error = None;
        state.status.last_capture = match read_capture_result(
            &capture.output_path,
            capture.sample_count,
            capture.samplerate_hz,
        ) {
            Ok(result) => Some(result),
            Err(error) => {
                state.status.last_error = Some(format_capture_warning(&error));
                None
            }
        };
        state.status.session_state = LogicAnalyzerSessionState::Idle;
        Ok(state.status.clone())
    }
}

impl LogicAnalyzerState {
    fn reconcile_capture(&mut self) {
        let finished = match self.capture.as_mut() {
            Some(capture) => match capture.child.try_wait() {
                Ok(Some(exit_status)) => Some(exit_status.success()),
                Ok(None) => None,
                Err(error) => {
                    self.status.last_error =
                        Some(format!("failed to inspect sigrok capture: {error}"));
                    Some(false)
                }
            },
            None => None,
        };

        if let Some(success) = finished {
            if success {
                if let Some(capture) = self.capture.as_ref() {
                    self.status.last_capture = match read_capture_result(
                        &capture.output_path,
                        capture.sample_count,
                        capture.samplerate_hz,
                    ) {
                        Ok(result) => Some(result),
                        Err(error) => {
                            self.status.last_error = Some(format_capture_warning(&error));
                            None
                        }
                    };
                }
            }
            self.capture = None;
            self.status.active_capture = None;
            self.status.session_state = if success {
                LogicAnalyzerSessionState::Ready
            } else {
                LogicAnalyzerSessionState::Error
            };
        }
    }
}

fn read_capture_result(
    output_path: &str,
    sample_count: u32,
    sample_rate_hz: Option<u64>,
) -> Result<LogicAnalyzerCaptureResult, String> {
    let text = fs::read_to_string(output_path)
        .map_err(|error| format!("failed to read capture output '{output_path}': {error}"))?;
    let waveform = parse_waveform_csv(&text, sample_count as usize);

    Ok(LogicAnalyzerCaptureResult {
        output_path: output_path.to_string(),
        sample_rate_hz,
        sample_count: waveform.sample_count,
        channels: waveform.channels,
        captured_at_ms: now_ms(),
    })
}

pub(crate) struct ParsedWaveform {
    pub(crate) sample_count: usize,
    pub(crate) channels: Vec<LogicAnalyzerWaveformChannel>,
}

fn parse_waveform_csv(text: &str, fallback_sample_count: usize) -> ParsedWaveform {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(header_line) = lines.next() else {
        return ParsedWaveform {
            sample_count: 0,
            channels: Vec::new(),
        };
    };

    let header = split_csv_line(header_line);
    let mut channel_columns: Vec<(usize, String)> = header
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            let label = normalize_channel_label(value)?;
            Some((index, label))
        })
        .collect();

    if channel_columns.is_empty() && header.len() > 1 {
        channel_columns = header
            .iter()
            .enumerate()
            .skip(1)
            .map(|(index, value)| (index, normalize_fallback_label(value, index)))
            .collect();
    }

    let mut samples: Vec<Vec<Option<bool>>> = vec![Vec::new(); channel_columns.len()];
    let mut sample_count = 0usize;

    for line in lines {
        let fields = split_csv_line(line);
        if fields.is_empty() || fields.iter().all(|field| field.trim().is_empty()) {
            continue;
        }

        for (sample_index, (column_index, _)) in channel_columns.iter().enumerate() {
            let value = fields
                .get(*column_index)
                .and_then(|field| parse_logic_level(field));
            if let Some(channel_samples) = samples.get_mut(sample_index) {
                channel_samples.push(value);
            }
        }

        sample_count += 1;
    }

    if sample_count == 0 {
        return ParsedWaveform {
            sample_count: fallback_sample_count,
            channels: channel_columns
                .into_iter()
                .map(|(_, label)| LogicAnalyzerWaveformChannel {
                    label,
                    samples: Vec::new(),
                })
                .collect(),
        };
    }

    let channels = channel_columns
        .into_iter()
        .enumerate()
        .map(|(index, (_, label))| LogicAnalyzerWaveformChannel {
            label,
            samples: samples.get(index).cloned().unwrap_or_default(),
        })
        .collect();

    ParsedWaveform {
        sample_count,
        channels,
    }
}

#[allow(dead_code)]
pub(crate) fn parse_capture_csv_for_test(
    text: &str,
    fallback_sample_count: usize,
) -> ParsedWaveform {
    parse_waveform_csv(text, fallback_sample_count)
}

fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(character) = chars.next() {
        match character {
            '"' => {
                if in_quotes && matches!(chars.peek(), Some('"')) {
                    current.push('"');
                    let _ = chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(character),
        }
    }

    fields.push(current.trim().to_string());
    fields
}

fn normalize_channel_label(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    if matches!(lower.as_str(), "time" | "timestamp" | "sample" | "index") {
        return None;
    }

    Some(normalize_fallback_label(trimmed, 0))
}

fn normalize_fallback_label(value: &str, index: usize) -> String {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        format!("D{index}")
    } else {
        trimmed.to_string()
    }
}

fn parse_logic_level(value: &str) -> Option<bool> {
    match value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase()
        .as_str()
    {
        "1" | "true" | "high" => Some(true),
        "0" | "false" | "low" => Some(false),
        "" | "x" | "z" | "-" | "nan" => None,
        other => other.parse::<f64>().ok().map(|parsed| parsed >= 0.5),
    }
}

fn format_capture_warning(error: &str) -> String {
    format!("capture completed, but waveform parsing failed: {error}")
}

pub(crate) fn parse_scan_output(output: &str) -> Vec<LogicAnalyzerDevice> {
    let mut devices = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("Sigrok") || trimmed.starts_with("Found ") {
            continue;
        }

        if let Some(device) = parse_device_line(trimmed) {
            devices.push(device);
        }
    }

    devices
}

#[allow(dead_code)]
pub(crate) fn build_capture_command(
    executable: &str,
    request: &LogicAnalyzerCaptureRequest,
    output_path: &str,
) -> String {
    format_capture_command(executable, &build_capture_args(request, output_path))
}

pub(crate) fn build_capture_args(
    request: &LogicAnalyzerCaptureRequest,
    output_path: &str,
) -> Vec<String> {
    let mut parts = vec![
        "-d".to_string(),
        request.device_ref.clone(),
        "--samples".to_string(),
        request.sample_count.to_string(),
        "-O".to_string(),
        "csv".to_string(),
        "-o".to_string(),
        output_path.to_string(),
    ];

    if let Some(samplerate_hz) = request.samplerate_hz {
        parts.push("-c".to_string());
        parts.push(format!("samplerate={samplerate_hz}"));
    }

    if !request.channels.is_empty() {
        parts.push("-C".to_string());
        parts.push(request.channels.join(","));
    }

    parts
}

fn format_capture_command(executable: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(shell_escape(executable));
    parts.extend(args.iter().map(|arg| shell_escape(arg)));
    parts.join(" ")
}

fn parse_device_line(line: &str) -> Option<LogicAnalyzerDevice> {
    let open = line.rfind('[')?;
    let close = line[open..].find(']')? + open;
    let reference = line[open + 1..close].trim();
    let name = line[..open]
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string();

    Some(LogicAnalyzerDevice {
        reference: reference.to_string(),
        name: if name.is_empty() {
            reference.to_string()
        } else {
            name
        },
        driver: Some(reference.to_string()),
        channels: Vec::new(),
        note: None,
        raw_line: Some(line.to_string()),
    })
}

fn resolve_sigrok_cli() -> Option<String> {
    std::env::var(SIGROK_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| Some(DEFAULT_SIGROK_BIN.to_string()))
}

fn capture_output_path() -> String {
    let filename = format!("eadai-sigrok-{}.csv", now_ms());
    std::env::temp_dir()
        .join(filename)
        .to_string_lossy()
        .into_owned()
}

fn combine_output(stdout: &[u8], stderr: &[u8]) -> String {
    let mut output = String::from_utf8_lossy(stdout).to_string();
    let stderr_text = String::from_utf8_lossy(stderr);
    if !stderr_text.trim().is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&stderr_text);
    }
    output
}

fn shell_escape(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "_./:-".contains(character))
    {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn lock_state(state: &Arc<Mutex<LogicAnalyzerState>>) -> MutexGuard<'_, LogicAnalyzerState> {
    state.lock().unwrap_or_else(|error| error.into_inner())
}
