//! Transport abstraction for protocol byte streams.
//!
//! A `Transport` provides a bidirectional byte stream. The framing layer
//! sits on top and extracts protocol frames from the raw bytes.
//! This separation allows the same framing/semantic/capability layers
//! to work over serial, Crazyradio, or USB native transports.

use std::fmt;

/// Identifies the physical transport backing a connection.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TransportKind {
    /// Standard serial port (USB-UART bridge, CP2102, CH340, etc.)
    Serial { port: String, baud_rate: u32 },
    /// Crazyradio dongle (nRF24-based 2.4 GHz radio)
    Crazyradio {
        uri: String,
        datarate: CrazyradioDatarate,
    },
    /// Crazyflie USB direct connection (Crazyflie 2.x USB port)
    CrazyflieUsb,
    /// Fake/simulated transport for testing
    Fake,
}

/// Radio datarate for Crazyradio connections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CrazyradioDatarate {
    /// 250 kbit/s
    Dr250K,
    /// 1 Mbit/s
    Dr1M,
    /// 2 Mbit/s (default for Crazyflie-link)
    Dr2M,
}

impl fmt::Display for CrazyradioDatarate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dr250K => write!(f, "250K"),
            Self::Dr1M => write!(f, "1M"),
            Self::Dr2M => write!(f, "2M"),
        }
    }
}

impl TransportKind {
    /// Human-readable label for logging and UI display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Serial { .. } => "serial",
            Self::Crazyradio { .. } => "crazyradio",
            Self::CrazyflieUsb => "crazyflie_usb",
            Self::Fake => "fake",
        }
    }
}

/// Events emitted by a transport connection.
#[derive(Clone, Debug)]
pub enum TransportEvent {
    /// New bytes received from the transport.
    Bytes(Vec<u8>),
    /// Transport connection established.
    Connected,
    /// Transport connection lost with reason.
    Disconnected(String),
    /// Transport error that may be recoverable.
    Error(String),
}

/// Result of a transport operation.
pub type TransportResult<T> = Result<T, TransportError>;

/// Errors that can occur in transport operations.
#[derive(Clone, Debug)]
pub enum TransportError {
    /// Connection failed.
    ConnectionFailed(String),
    /// Read failed.
    ReadFailed(String),
    /// Write failed.
    WriteFailed(String),
    /// Transport not available (e.g., no Crazyradio dongle found).
    NotAvailable(String),
    /// Operation timed out.
    Timeout(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionFailed(msg) => write!(f, "connection failed: {msg}"),
            Self::ReadFailed(msg) => write!(f, "read failed: {msg}"),
            Self::WriteFailed(msg) => write!(f, "write failed: {msg}"),
            Self::NotAvailable(msg) => write!(f, "not available: {msg}"),
            Self::Timeout(msg) => write!(f, "timeout: {msg}"),
        }
    }
}

impl std::error::Error for TransportError {}

/// Trait for byte-stream transports.
///
/// Implementors provide a blocking read/write interface. The framing layer
/// calls `read_chunk` in a loop and feeds bytes to protocol decoders.
pub trait ByteTransport: Send {
    /// Returns the transport kind for this connection.
    fn kind(&self) -> &TransportKind;

    /// Reads a chunk of bytes from the transport.
    ///
    /// Returns `Ok(None)` on timeout or `Ok(Some(bytes))` on data.
    /// Returns `Err` on unrecoverable failure.
    fn read_chunk(&mut self) -> TransportResult<Option<Vec<u8>>>;

    /// Writes bytes to the transport.
    fn write_all(&mut self, data: &[u8]) -> TransportResult<()>;

    /// Flushes any buffered write data.
    fn flush(&mut self) -> TransportResult<()>;

    /// Returns true if the transport is still connected.
    fn is_connected(&self) -> bool;
}

/// Blanket impl so `Box<dyn ByteTransport>` can be used anywhere `dyn ByteTransport` is expected.
impl<T: ByteTransport + ?Sized> ByteTransport for Box<T> {
    fn kind(&self) -> &TransportKind {
        (**self).kind()
    }

    fn read_chunk(&mut self) -> TransportResult<Option<Vec<u8>>> {
        (**self).read_chunk()
    }

    fn write_all(&mut self, data: &[u8]) -> TransportResult<()> {
        (**self).write_all(data)
    }

    fn flush(&mut self) -> TransportResult<()> {
        (**self).flush()
    }

    fn is_connected(&self) -> bool {
        (**self).is_connected()
    }
}
