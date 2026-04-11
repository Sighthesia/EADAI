use crate::cli::{InteractiveConfig, RunConfig, SendConfig};
use crate::error::AppError;
use crate::message::LinePayload;
use std::io::{ErrorKind, Read, Write};
use std::time::{Duration, Instant};

const READ_BUFFER_SIZE: usize = 1024;

/// Framing status for one emitted line payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameStatus {
    Complete,
    Overflow,
}

/// One framed line produced by the serial framer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FramedLine {
    pub payload: LinePayload,
    pub status: FrameStatus,
}

/// Enumerates available serial ports.
pub fn list_ports() -> Result<Vec<String>, AppError> {
    let ports = serialport::available_ports()?;
    Ok(ports.into_iter().map(|port| port.port_name).collect())
}

/// Opens a serial port with the configured timeout.
///
/// - `config`: runtime serial configuration.
pub fn open_port(config: &RunConfig) -> Result<Box<dyn serialport::SerialPort>, AppError> {
    let port = serialport::new(&config.port, config.baud_rate)
        .timeout(config.read_timeout)
        .open()?;
    Ok(port)
}

/// Opens a serial port for sending or loopback verification.
pub fn open_send_port(config: &SendConfig) -> Result<Box<dyn serialport::SerialPort>, AppError> {
    let port = serialport::new(&config.port, config.baud_rate)
        .timeout(config.read_timeout)
        .open()?;
    Ok(port)
}

/// Opens a serial port for interactive terminal mode.
pub fn open_interactive_port(
    config: &InteractiveConfig,
) -> Result<Box<dyn serialport::SerialPort>, AppError> {
    let port = serialport::new(&config.port, config.baud_rate)
        .timeout(config.read_timeout)
        .open()?;
    Ok(port)
}

/// Converts send config into bytes to write.
pub fn payload_bytes(config: &SendConfig) -> Vec<u8> {
    payload_bytes_for_text(&config.payload, config.append_newline)
}

/// Converts free-form terminal text into bytes to write.
pub fn payload_bytes_for_text(payload: &str, append_newline: bool) -> Vec<u8> {
    let mut bytes = payload.as_bytes().to_vec();
    if append_newline {
        bytes.push(b'\n');
    }
    bytes
}

/// Writes payload bytes to the serial port and flushes the stream.
pub fn write_payload<T>(port: &mut T, payload: &[u8]) -> Result<(), AppError>
where
    T: Write + ?Sized,
{
    port.write_all(payload)?;
    port.flush()?;
    Ok(())
}

/// Reads until the expected echoed line is received or timeout expires.
pub fn read_expected_line<T>(
    port: &mut T,
    framer: &mut LineFramer,
    expected: &str,
    timeout: Duration,
) -> Result<LinePayload, AppError>
where
    T: Read + ?Sized,
{
    let deadline = Instant::now() + timeout;

    loop {
        let mut matched = None;
        pump_port(port, framer, |line| {
            if matched.is_none()
                && line.status == FrameStatus::Complete
                && line.payload.text == expected
            {
                matched = Some(line.payload);
            }
        })?;

        if let Some(line) = matched {
            return Ok(line);
        }

        if Instant::now() >= deadline {
            return Err(AppError::LoopbackTimeout(format!(
                "did not receive expected echo '{expected}' within {} ms",
                timeout.as_millis()
            )));
        }
    }
}

/// Reads one chunk from the serial port and emits completed lines.
///
/// - `port`: opened serial port instance.
/// - `framer`: line framer preserving partial data across reads.
/// - `on_line`: callback invoked for every completed line.
pub fn pump_port<T, F>(
    port: &mut T,
    framer: &mut LineFramer,
    mut on_line: F,
) -> Result<(), AppError>
where
    T: Read + ?Sized,
    F: FnMut(FramedLine),
{
    let mut buffer = [0_u8; READ_BUFFER_SIZE];

    match port.read(&mut buffer) {
        Ok(0) => Ok(()),
        Ok(count) => {
            for line in framer.push(&buffer[..count]) {
                on_line(line);
            }
            Ok(())
        }
        Err(error) if error.kind() == ErrorKind::TimedOut => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Stateful line framer for line-oriented protocols.
#[derive(Clone, Debug)]
pub struct LineFramer {
    buffer: Vec<u8>,
    max_buffer_bytes: usize,
}

impl Default for LineFramer {
    fn default() -> Self {
        Self::with_max_buffer(crate::cli::DEFAULT_MAX_FRAME_BYTES)
    }
}

impl LineFramer {
    /// Creates an empty line framer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty line framer with a bounded internal buffer.
    ///
    /// - `max_buffer_bytes`: maximum buffered bytes allowed without a newline.
    pub fn with_max_buffer(max_buffer_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_buffer_bytes: max_buffer_bytes.max(1),
        }
    }

    /// Pushes bytes into the framer and returns completed lines.
    ///
    /// - `chunk`: raw bytes read from serial.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<FramedLine> {
        self.buffer.extend_from_slice(chunk);
        let mut lines = Vec::new();

        while let Some(position) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let mut raw = self.buffer.drain(..=position).collect::<Vec<u8>>();

            if raw.last() == Some(&b'\n') {
                raw.pop();
            }
            if raw.last() == Some(&b'\r') {
                raw.pop();
            }

            lines.push(FramedLine {
                payload: build_payload(raw),
                status: FrameStatus::Complete,
            });
        }

        if self.buffer.len() > self.max_buffer_bytes {
            let raw = std::mem::take(&mut self.buffer);
            lines.push(FramedLine {
                payload: build_payload(raw),
                status: FrameStatus::Overflow,
            });
        }

        lines
    }

    /// Returns the number of buffered bytes that still wait for a newline.
    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
    }
}

fn build_payload(raw: Vec<u8>) -> LinePayload {
    let text = String::from_utf8_lossy(&raw).into_owned();
    LinePayload { text, raw }
}
