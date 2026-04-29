use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerCaptureRequest {
    pub device_ref: String,
    pub sample_count: u32,
    pub samplerate_hz: Option<u64>,
    #[serde(default)]
    pub channels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerDevice {
    pub reference: String,
    pub name: String,
    pub driver: Option<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    pub note: Option<String>,
    pub raw_line: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerCaptureState {
    pub pid: u32,
    pub started_at_ms: u64,
    pub command: String,
    pub output_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerWaveformChannel {
    pub label: String,
    pub samples: Vec<Option<bool>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerCaptureResult {
    pub output_path: String,
    pub sample_rate_hz: Option<u64>,
    pub sample_count: usize,
    pub channels: Vec<LogicAnalyzerWaveformChannel>,
    pub captured_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LogicAnalyzerSessionState {
    Unavailable,
    Idle,
    Scanning,
    Ready,
    Capturing,
    Stopping,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerStatus {
    pub available: bool,
    pub executable: Option<String>,
    pub session_state: LogicAnalyzerSessionState,
    pub devices: Vec<LogicAnalyzerDevice>,
    pub selected_device_ref: Option<String>,
    pub active_capture: Option<LogicAnalyzerCaptureState>,
    pub last_capture: Option<LogicAnalyzerCaptureResult>,
    pub last_scan_at_ms: Option<u64>,
    pub scan_output: Option<String>,
    pub last_error: Option<String>,
    pub capture_plan: Option<String>,
    pub linux_first_note: String,
}

impl Default for LogicAnalyzerStatus {
    fn default() -> Self {
        Self {
            available: false,
            executable: None,
            session_state: LogicAnalyzerSessionState::Idle,
            devices: Vec::new(),
            selected_device_ref: None,
            active_capture: None,
            last_capture: None,
            last_scan_at_ms: None,
            scan_output: None,
            last_error: None,
            capture_plan: None,
            linux_first_note:
                "Linux-first sigrok path; install `sigrok-cli` or set `EADAI_SIGROK_CLI`."
                    .to_string(),
        }
    }
}
