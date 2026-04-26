pub mod protocol;

use crate::error::AppError;
use crate::serial;
use protocol::{
    DiagnosticDecoder, Frame, HostCommand, NewlineMode, Packet, SampleFrame, SchemaFrame,
    ascii_command_bytes, command_label, encode_host_command,
};
use serialport::{DataBits, FlowControl, Parity, StopBits};
use std::cmp;
use std::time::{Duration, Instant};

pub const DEFAULT_TIMEOUT_MS: u64 = 50;
pub const DEFAULT_SCHEMA_WAIT_MS: u64 = 700;
pub const DEFAULT_STEP_WAIT_MS: u64 = 700;
pub const DEFAULT_OVERALL_TIMEOUT_MS: u64 = 4_500;
pub const DEFAULT_MAX_FRAME_BYTES: usize = 4_096;
const DEFAULT_HEX_PREVIEW_BYTES: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandPath {
    Ascii,
    Binary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagConfig {
    pub port: String,
    pub baud_rate: u32,
    pub read_timeout: Duration,
    pub schema_wait: Duration,
    pub step_wait: Duration,
    pub overall_timeout: Duration,
    pub max_frame_bytes: usize,
    pub listen_only: bool,
    pub skip_ascii: bool,
    pub ascii_newline: NewlineMode,
}

#[derive(Clone, Debug, Default)]
pub struct DiagStats {
    pub raw_chunk_count: usize,
    pub raw_bytes_total: usize,
    pub largest_chunk: usize,
    pub text_count: usize,
    pub identity_count: usize,
    pub schema_count: usize,
    pub sample_count: usize,
    pub unknown_frame_count: usize,
    pub invalid_crc_count: usize,
    pub malformed_frame_count: usize,
    pub desync_drop_bytes: usize,
    pub first_sample_path: Option<CommandPath>,
}

#[derive(Clone, Debug)]
pub struct Verdict {
    pub title: &'static str,
    pub detail: String,
}

#[derive(Clone, Debug)]
struct RunState {
    decoder: DiagnosticDecoder,
    stats: DiagStats,
    last_command_path: Option<CommandPath>,
    schema: Option<SchemaFrame>,
    tx_seq: u8,
}

impl RunState {
    fn new(max_frame_bytes: usize) -> Self {
        Self {
            decoder: DiagnosticDecoder::new(max_frame_bytes),
            stats: DiagStats::default(),
            last_command_path: None,
            schema: None,
            tx_seq: 0,
        }
    }
}

/// Parses CLI arguments for the standalone BMI088 diagnostic binary.
pub fn parse_args<I>(args: I) -> Result<DiagConfig, AppError>
where
    I: IntoIterator<Item = String>,
{
    let collected: Vec<String> = args.into_iter().collect();
    let mut port = None;
    let mut baud_rate = 115_200;
    let mut timeout_ms = DEFAULT_TIMEOUT_MS;
    let mut schema_wait_ms = DEFAULT_SCHEMA_WAIT_MS;
    let mut step_wait_ms = DEFAULT_STEP_WAIT_MS;
    let mut overall_timeout_ms = DEFAULT_OVERALL_TIMEOUT_MS;
    let mut max_frame_bytes = DEFAULT_MAX_FRAME_BYTES;
    let mut listen_only = false;
    let mut skip_ascii = false;
    let mut ascii_newline = NewlineMode::Lf;
    let mut index = 0;

    while index < collected.len() {
        match collected[index].as_str() {
            "--port" => port = Some(next_value(&collected, &mut index, "--port")?),
            "--baud" => baud_rate = parse_number(&next_value(&collected, &mut index, "--baud")?, "--baud")?,
            "--read-timeout-ms" => {
                timeout_ms = parse_number(
                    &next_value(&collected, &mut index, "--read-timeout-ms")?,
                    "--read-timeout-ms",
                )?
            }
            "--schema-wait-ms" => {
                schema_wait_ms = parse_number(
                    &next_value(&collected, &mut index, "--schema-wait-ms")?,
                    "--schema-wait-ms",
                )?
            }
            "--step-wait-ms" => {
                step_wait_ms = parse_number(
                    &next_value(&collected, &mut index, "--step-wait-ms")?,
                    "--step-wait-ms",
                )?
            }
            "--overall-timeout-ms" => {
                overall_timeout_ms = parse_number(
                    &next_value(&collected, &mut index, "--overall-timeout-ms")?,
                    "--overall-timeout-ms",
                )?
            }
            "--max-frame-bytes" => {
                max_frame_bytes = parse_number(
                    &next_value(&collected, &mut index, "--max-frame-bytes")?,
                    "--max-frame-bytes",
                )?
            }
            "--ascii-newline" => {
                ascii_newline = parse_newline_mode(&next_value(
                    &collected,
                    &mut index,
                    "--ascii-newline",
                )?)?
            }
            "--listen-only" => listen_only = true,
            "--skip-ascii" => skip_ascii = true,
            "--help" | "-h" => return Err(AppError::Usage(usage())),
            _ => {
                return Err(AppError::Usage(format!(
                    "Unknown flag: {}\n\n{}",
                    collected[index],
                    usage()
                )))
            }
        }

        index += 1;
    }

    let Some(port) = port else {
        return Err(AppError::Usage(format!(
            "Missing required flag: --port\n\n{}",
            usage()
        )));
    };

    Ok(DiagConfig {
        port,
        baud_rate,
        read_timeout: Duration::from_millis(timeout_ms),
        schema_wait: Duration::from_millis(schema_wait_ms),
        step_wait: Duration::from_millis(step_wait_ms),
        overall_timeout: Duration::from_millis(overall_timeout_ms),
        max_frame_bytes,
        listen_only,
        skip_ascii,
        ascii_newline,
    })
}

/// Runs the host-side BMI088 handshake probe against one serial port.
pub fn run(config: DiagConfig) -> Result<(), AppError> {
    let mut port = serialport::new(&config.port, config.baud_rate)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .flow_control(FlowControl::None)
        .timeout(config.read_timeout)
        .open()?;
    let started_at = Instant::now();
    let mut state = RunState::new(config.max_frame_bytes);

    println!(
        "[open] port={} baud={} timeout_ms={} schema_wait_ms={} step_wait_ms={} total_ms={}",
        config.port,
        config.baud_rate,
        config.read_timeout.as_millis(),
        config.schema_wait.as_millis(),
        config.step_wait.as_millis(),
        config.overall_timeout.as_millis(),
    );
    println!(
        "[note] This probe uses the firmware skill contract: 115200 8N1, type=0x03 EVENT, seq+1-byte len, CRC over header+payload."
    );

    observe_budgeted(&mut *port, &mut state, started_at, config.overall_timeout, config.schema_wait)?;

    if !config.listen_only && state.stats.sample_count == 0 && !config.skip_ascii {
        send_ascii(&mut *port, &mut state, HostCommand::Ack, config.ascii_newline)?;
        observe_budgeted(&mut *port, &mut state, started_at, config.overall_timeout, config.step_wait / 3)?;
        if state.stats.sample_count == 0 {
            send_ascii(&mut *port, &mut state, HostCommand::Start, config.ascii_newline)?;
            observe_budgeted(&mut *port, &mut state, started_at, config.overall_timeout, config.step_wait)?;
        }
    }

    if !config.listen_only && state.stats.sample_count == 0 {
        if state.schema.is_none() {
            send_binary(&mut *port, &mut state, HostCommand::ReqSchema)?;
            observe_budgeted(&mut *port, &mut state, started_at, config.overall_timeout, config.step_wait)?;
        }

        if state.schema.is_some() && state.stats.sample_count == 0 {
            send_binary(&mut *port, &mut state, HostCommand::Ack)?;
            observe_budgeted(&mut *port, &mut state, started_at, config.overall_timeout, config.step_wait / 3)?;
        }

        if state.schema.is_some() && state.stats.sample_count == 0 {
            send_binary(&mut *port, &mut state, HostCommand::Start)?;
            observe_budgeted(&mut *port, &mut state, started_at, config.overall_timeout, config.step_wait)?;
        }
    }

    let remaining = config
        .overall_timeout
        .saturating_sub(started_at.elapsed())
        .min(config.step_wait / 2);
    if !remaining.is_zero() {
        observe_budgeted(&mut *port, &mut state, started_at, config.overall_timeout, remaining)?;
    }

    state.stats.invalid_crc_count = state.decoder.invalid_crc_count();
    state.stats.malformed_frame_count = state.decoder.malformed_frame_count();
    state.stats.desync_drop_bytes = state.decoder.desync_drop_bytes();
    print_summary(&state.stats, build_verdict(&state.stats));
    Ok(())
}

/// Builds a human-readable verdict from the final diagnostic counters.
pub fn build_verdict(stats: &DiagStats) -> Verdict {
    if stats.sample_count > 0 {
        let detail = match stats.first_sample_path {
            Some(CommandPath::Ascii) => "ASCII ACK/START unlocked samples. MCU RX path is alive, so your binary host framing or CRC is the likely failure.".to_string(),
            Some(CommandPath::Binary) => "Spec-correct binary ACK/START unlocked samples. The current host implementation is likely using the wrong BMI088 frame layout or CRC range.".to_string(),
            None => "Samples are flowing. The link is alive and the device is already streaming.".to_string(),
        };
        return Verdict {
            title: "samples-flowing",
            detail,
        };
    }

    if stats.schema_count >= 2 {
        return Verdict {
            title: "schema-repeating-no-samples",
            detail: "MCU is alive and keeps resending SCHEMA, but the handshake never completed. Check host TX routing, ACK -> START order, and CRC correctness.".to_string(),
        };
    }

    if stats.schema_count == 1 {
        return Verdict {
            title: "schema-once-no-samples",
            detail: "A valid SCHEMA arrived, but no samples followed. This usually means ACK/START never landed or were rejected.".to_string(),
        };
    }

    if stats.text_count > 0 {
        return Verdict {
            title: "text-only-traffic",
            detail: "Only text traffic was observed. You may be on the wrong port, wrong baud, or a plain-text console path instead of BMI088 binary telemetry.".to_string(),
        };
    }

    if stats.raw_bytes_total == 0 {
        return Verdict {
            title: "no-data-seen",
            detail: "No data arrived at all. Check the selected serial device, 115200 8N1, board power, and P19 shared UART wiring.".to_string(),
        };
    }

    Verdict {
        title: "raw-no-valid-frames",
        detail: "Bytes arrived, but no valid SCHEMA/SAMPLE frames decoded. This usually points to wrong baud, wrong protocol layout, or CRC/header mismatch.".to_string(),
    }
}

fn observe_budgeted<T: serialport::SerialPort + ?Sized>(
    port: &mut T,
    state: &mut RunState,
    started_at: Instant,
    overall_timeout: Duration,
    requested: Duration,
) -> Result<(), AppError> {
    let remaining = overall_timeout.saturating_sub(started_at.elapsed());
    let window = cmp::min(remaining, requested);
    if window.is_zero() {
        return Ok(());
    }

    let deadline = Instant::now() + window;
    while Instant::now() < deadline {
        if let Some(chunk) = serial::read_bytes(port)? {
            state.stats.raw_chunk_count += 1;
            state.stats.raw_bytes_total += chunk.len();
            state.stats.largest_chunk = state.stats.largest_chunk.max(chunk.len());

            if should_log_raw(&state.stats) {
                println!(
                    "[rx/raw] chunk={} bytes={} total={} preview={}",
                    state.stats.raw_chunk_count,
                    chunk.len(),
                    state.stats.raw_bytes_total,
                    hex_preview(&chunk)
                );
            }

            for packet in state.decoder.push(&chunk) {
                handle_packet(state, packet);
            }
        }
    }

    Ok(())
}

fn handle_packet(state: &mut RunState, packet: Packet) {
    match packet {
        Packet::Text(line) => {
            state.stats.text_count += 1;
            if state.stats.text_count <= 5 {
                println!(
                    "[rx/text] count={} bytes={} text={}",
                    state.stats.text_count,
                    line.payload.raw.len(),
                    line.payload.text.trim()
                );
            }
        }
        Packet::Frame(Frame::Identity(identity)) => {
            state.stats.identity_count += 1;
            println!(
                "[rx/identity] count={} seq={} device={} board={} firmware={} protocol={} transport={}",
                state.stats.identity_count,
                identity.seq,
                identity.device_name,
                identity.board_name,
                identity.firmware_version,
                identity.protocol_version,
                identity.transport_name
            );
        }
        Packet::Frame(Frame::Schema(schema)) => {
            state.stats.schema_count += 1;
            let names = schema
                .fields
                .iter()
                .take(3)
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>()
                .join(",");
            println!(
                "[rx/schema] count={} seq={} rate_hz={} sample_len={} fields={} names={}",
                state.stats.schema_count,
                schema.seq,
                schema.rate_hz,
                schema.sample_len,
                schema.fields.len(),
                names
            );
            state.schema = Some(schema);
        }
        Packet::Frame(Frame::Sample(sample)) => {
            state.stats.sample_count += 1;
            if state.stats.first_sample_path.is_none() {
                state.stats.first_sample_path = state.last_command_path;
            }
            if state.stats.sample_count <= 5 {
                println!(
                    "[rx/sample] count={} seq={} preview={}",
                    state.stats.sample_count,
                    sample.seq,
                    sample_preview(state.schema.as_ref(), &sample)
                );
            }
        }
        Packet::Frame(Frame::Unknown {
            frame_type,
            command,
            seq,
            payload_len,
        }) => {
            state.stats.unknown_frame_count += 1;
            println!(
                "[rx/frame] count={} type=0x{frame_type:02X} cmd=0x{command:02X} seq={} len={}",
                state.stats.unknown_frame_count,
                seq,
                payload_len
            );
        }
    }

    state.stats.invalid_crc_count = state.decoder.invalid_crc_count();
    state.stats.malformed_frame_count = state.decoder.malformed_frame_count();
    state.stats.desync_drop_bytes = state.decoder.desync_drop_bytes();
}

fn send_ascii<T: serialport::SerialPort + ?Sized>(
    port: &mut T,
    state: &mut RunState,
    command: HostCommand,
    newline_mode: NewlineMode,
) -> Result<(), AppError> {
    let payload = ascii_command_bytes(command, newline_mode);
    serial::write_payload(port, &payload)?;
    state.last_command_path = Some(CommandPath::Ascii);
    println!(
        "[tx/ascii] cmd={} newline={} bytes={} payload={}",
        command_label(command),
        newline_label(newline_mode),
        payload.len(),
        String::from_utf8_lossy(&payload).escape_debug()
    );
    Ok(())
}

fn send_binary<T: serialport::SerialPort + ?Sized>(
    port: &mut T,
    state: &mut RunState,
    command: HostCommand,
) -> Result<(), AppError> {
    let seq = state.tx_seq;
    state.tx_seq = state.tx_seq.wrapping_add(1);
    let payload = encode_host_command(command, seq);
    serial::write_payload(port, &payload)?;
    state.last_command_path = Some(CommandPath::Binary);
    println!(
        "[tx/bin] cmd={} seq={} bytes={} frame={}",
        command_label(command),
        seq,
        payload.len(),
        hex_preview(&payload)
    );
    Ok(())
}

fn usage() -> String {
    format!(
        "Usage:\n  cargo run --bin bmi088-handshake-diag -- --port <name> [--baud <rate>] [--read-timeout-ms <ms>] [--schema-wait-ms <ms>] [--step-wait-ms <ms>] [--overall-timeout-ms <ms>] [--max-frame-bytes <bytes>] [--ascii-newline <none|lf|crlf>] [--listen-only] [--skip-ascii]\n\nDefaults:\n  baud = 115200\n  read-timeout-ms = {DEFAULT_TIMEOUT_MS}\n  schema-wait-ms = {DEFAULT_SCHEMA_WAIT_MS}\n  step-wait-ms = {DEFAULT_STEP_WAIT_MS}\n  overall-timeout-ms = {DEFAULT_OVERALL_TIMEOUT_MS}\n  max-frame-bytes = {DEFAULT_MAX_FRAME_BYTES}\n  ascii-newline = lf"
    )
}

fn next_value(values: &[String], index: &mut usize, flag: &str) -> Result<String, AppError> {
    *index += 1;
    values.get(*index).cloned().ok_or_else(|| {
        AppError::Usage(format!("Missing value for {flag}\n\n{}", usage()))
    })
}

fn parse_number<T>(value: &str, flag: &str) -> Result<T, AppError>
where
    T: std::str::FromStr,
{
    value.parse::<T>().map_err(|_| {
        AppError::Usage(format!(
            "Invalid numeric value for {flag}: {value}\n\n{}",
            usage()
        ))
    })
}

fn parse_newline_mode(value: &str) -> Result<NewlineMode, AppError> {
    match value {
        "none" => Ok(NewlineMode::None),
        "lf" => Ok(NewlineMode::Lf),
        "crlf" => Ok(NewlineMode::Crlf),
        _ => Err(AppError::Usage(format!(
            "Invalid value for --ascii-newline: {value}\n\n{}",
            usage()
        ))),
    }
}

fn newline_label(mode: NewlineMode) -> &'static str {
    match mode {
        NewlineMode::None => "none",
        NewlineMode::Lf => "lf",
        NewlineMode::Crlf => "crlf",
    }
}

fn should_log_raw(stats: &DiagStats) -> bool {
    stats.raw_chunk_count <= 8 || (stats.sample_count == 0 && stats.raw_chunk_count <= 20)
}

fn hex_preview(bytes: &[u8]) -> String {
    bytes.iter()
        .take(DEFAULT_HEX_PREVIEW_BYTES)
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn sample_preview(schema: Option<&SchemaFrame>, sample: &SampleFrame) -> String {
    match schema {
        Some(schema) => sample
            .raw_values
            .iter()
            .zip(schema.fields.iter())
            .take(3)
            .map(|(raw, field)| format!("{}={}", field.name, raw))
            .collect::<Vec<_>>()
            .join(", "),
        None => sample
            .raw_values
            .iter()
            .take(3)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn print_summary(stats: &DiagStats, verdict: Verdict) {
    println!(
        "[summary] raw_chunks={} raw_bytes={} largest_chunk={} text={} identity={} schema={} samples={} unknown={} invalid_crc={} malformed={} desync_drop_bytes={}",
        stats.raw_chunk_count,
        stats.raw_bytes_total,
        stats.largest_chunk,
        stats.text_count,
        stats.identity_count,
        stats.schema_count,
        stats.sample_count,
        stats.unknown_frame_count,
        stats.invalid_crc_count,
        stats.malformed_frame_count,
        stats.desync_drop_bytes,
    );
    println!("[verdict] {}: {}", verdict.title, verdict.detail);
}
