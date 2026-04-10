use std::error::Error;
use std::fmt::{Display, Formatter};

/// Error type shared by the serial ingestion MVP.
#[derive(Debug)]
pub enum AppError {
    Usage(String),
    Serial(serialport::Error),
    Io(std::io::Error),
    LoopbackTimeout(String),
    LoopbackMismatch { expected: String, received: String },
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => write!(formatter, "{message}"),
            Self::Serial(error) => write!(formatter, "serial error: {error}"),
            Self::Io(error) => write!(formatter, "io error: {error}"),
            Self::LoopbackTimeout(message) => write!(formatter, "loopback timeout: {message}"),
            Self::LoopbackMismatch { expected, received } => {
                write!(
                    formatter,
                    "loopback mismatch: expected '{expected}', received '{received}'"
                )
            }
        }
    }
}

impl Error for AppError {}

impl From<serialport::Error> for AppError {
    fn from(value: serialport::Error) -> Self {
        Self::Serial(value)
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
