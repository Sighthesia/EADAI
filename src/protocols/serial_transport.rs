//! Serial port transport implementation.
//!
//! Wraps `serialport::SerialPort` into the `Transport` trait so the framing
//! layer can consume serial byte streams identically to Crazyradio or USB.

use crate::serial;
use super::{ByteTransport, TransportError, TransportKind, TransportResult};
use std::io::Read;
use std::time::Duration;

const DEFAULT_READ_BUFFER: usize = 1024;

/// Serial port transport.
pub struct SerialTransport {
    kind: TransportKind,
    port: Box<dyn serialport::SerialPort>,
    read_buffer: Vec<u8>,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl SerialTransport {
    /// Opens a serial port and returns a transport wrapper.
    pub fn open(port_name: &str, baud_rate: u32, read_timeout_ms: u64) -> TransportResult<Self> {
        Self::open_with_timeouts(
            port_name,
            baud_rate,
            Duration::from_millis(read_timeout_ms),
            serial::serial_write_timeout(Duration::from_millis(read_timeout_ms)),
        )
    }

    /// Opens a serial port from runtime config and returns a transport wrapper.
    pub fn from_config(config: &crate::cli::RunConfig) -> TransportResult<Self> {
        Self::open_with_timeouts(
            &config.port,
            config.baud_rate,
            config.read_timeout,
            serial::serial_write_timeout(config.read_timeout),
        )
    }

    /// Opens a serial port with explicit read and write timeouts.
    pub fn open_with_timeouts(
        port_name: &str,
        baud_rate: u32,
        read_timeout: Duration,
        write_timeout: Duration,
    ) -> TransportResult<Self> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(write_timeout)
            .open()
            .map_err(|e| TransportError::ConnectionFailed(format!("{port_name}: {e}")))?;

        Ok(Self {
            kind: TransportKind::Serial {
                port: port_name.to_string(),
                baud_rate,
            },
            port,
            read_buffer: vec![0u8; DEFAULT_READ_BUFFER],
            read_timeout,
            write_timeout,
        })
    }

    /// Wraps an already-opened serial port into a transport.
    pub fn from_port(
        port: Box<dyn serialport::SerialPort>,
        config: &crate::cli::RunConfig,
    ) -> Self {
        Self {
            kind: TransportKind::Serial {
                port: config.port.clone(),
                baud_rate: config.baud_rate,
            },
            port,
            read_buffer: vec![0u8; DEFAULT_READ_BUFFER],
            read_timeout: config.read_timeout,
            write_timeout: serial::serial_write_timeout(config.read_timeout),
        }
    }
}

impl ByteTransport for SerialTransport {
    fn kind(&self) -> &TransportKind {
        &self.kind
    }

    fn read_chunk(&mut self) -> TransportResult<Option<Vec<u8>>> {
        let _ = self.port.set_timeout(self.read_timeout);
        match self.port.read(&mut self.read_buffer) {
            Ok(0) => Ok(None),
            Ok(count) => Ok(Some(self.read_buffer[..count].to_vec())),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(None),
            Err(e) => Err(TransportError::ReadFailed(e.to_string())),
        }
    }

    fn write_all(&mut self, data: &[u8]) -> TransportResult<()> {
        let _ = self.port.set_timeout(self.write_timeout);
        std::io::Write::write_all(&mut self.port, data)
            .map_err(|e| TransportError::WriteFailed(e.to_string()))
    }

    fn flush(&mut self) -> TransportResult<()> {
        std::io::Write::flush(&mut self.port)
            .map_err(|e| TransportError::WriteFailed(e.to_string()))
    }

    fn is_connected(&self) -> bool {
        true
    }
}
