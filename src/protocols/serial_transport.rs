//! Serial port transport implementation.
//!
//! Wraps `serialport::SerialPort` into the `Transport` trait so the framing
//! layer can consume serial byte streams identically to Crazyradio or USB.

use super::{ByteTransport, TransportError, TransportKind, TransportResult};
use std::io::Read;

const DEFAULT_READ_BUFFER: usize = 1024;

/// Serial port transport.
pub struct SerialTransport {
    kind: TransportKind,
    port: Box<dyn serialport::SerialPort>,
    read_buffer: Vec<u8>,
}

impl SerialTransport {
    /// Opens a serial port and returns a transport wrapper.
    pub fn open(
        port_name: &str,
        baud_rate: u32,
        read_timeout_ms: u64,
    ) -> TransportResult<Self> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(std::time::Duration::from_millis(read_timeout_ms))
            .open()
            .map_err(|e| TransportError::ConnectionFailed(format!("{port_name}: {e}")))?;

        Ok(Self {
            kind: TransportKind::Serial {
                port: port_name.to_string(),
                baud_rate,
            },
            port,
            read_buffer: vec![0u8; DEFAULT_READ_BUFFER],
        })
    }

    /// Wraps an already-opened serial port into a transport.
    pub fn from_port(port: Box<dyn serialport::SerialPort>, config: &crate::cli::RunConfig) -> Self {
        Self {
            kind: TransportKind::Serial {
                port: config.port.clone(),
                baud_rate: config.baud_rate,
            },
            port,
            read_buffer: vec![0u8; DEFAULT_READ_BUFFER],
        }
    }
}

impl ByteTransport for SerialTransport {
    fn kind(&self) -> &TransportKind {
        &self.kind
    }

    fn read_chunk(&mut self) -> TransportResult<Option<Vec<u8>>> {
        match self.port.read(&mut self.read_buffer) {
            Ok(0) => Ok(None),
            Ok(count) => Ok(Some(self.read_buffer[..count].to_vec())),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(None),
            Err(e) => Err(TransportError::ReadFailed(e.to_string())),
        }
    }

    fn write_all(&mut self, data: &[u8]) -> TransportResult<()> {
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
