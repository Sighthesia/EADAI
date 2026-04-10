use eadai::cli::{LoopbackConfig, SendConfig};
use eadai::serial::{LineFramer, payload_bytes, read_expected_line, write_payload};
use std::io::{Cursor, Read, Result as IoResult, Write};
use std::time::Duration;

#[test]
fn appends_newline_to_default_payload() {
    let config = SendConfig {
        port: "/dev/ttyUSB0".to_string(),
        baud_rate: 115_200,
        read_timeout: Duration::from_millis(50),
        payload: "ping:42".to_string(),
        append_newline: true,
    };

    assert_eq!(payload_bytes(&config), b"ping:42\n");
}

#[test]
fn preserves_payload_without_newline_when_requested() {
    let config = SendConfig {
        port: "/dev/ttyUSB0".to_string(),
        baud_rate: 115_200,
        read_timeout: Duration::from_millis(50),
        payload: "ping:42".to_string(),
        append_newline: false,
    };

    assert_eq!(payload_bytes(&config), b"ping:42");
}

#[test]
fn writes_full_payload_to_port_like_writer() {
    let mut writer = Cursor::new(Vec::<u8>::new());

    write_payload(&mut writer, b"hello\n").unwrap();

    assert_eq!(writer.into_inner(), b"hello\n");
}

#[test]
fn reads_expected_echo_from_loopback_like_stream() {
    let send = SendConfig {
        port: "/dev/ttyUSB0".to_string(),
        baud_rate: 115_200,
        read_timeout: Duration::from_millis(10),
        payload: "temp:42".to_string(),
        append_newline: true,
    };
    let _loopback = LoopbackConfig {
        send: send.clone(),
        loopback_timeout: Duration::from_millis(100),
    };
    let mut stream = FakeLoopback::new(payload_bytes(&send));
    let mut framer = LineFramer::new();

    write_payload(&mut stream, &payload_bytes(&send)).unwrap();
    let echoed = read_expected_line(
        &mut stream,
        &mut framer,
        &send.payload,
        Duration::from_millis(100),
    )
    .unwrap();

    assert_eq!(echoed.text, "temp:42");
}

struct FakeLoopback {
    read_cursor: Cursor<Vec<u8>>,
    written: Vec<u8>,
}

impl FakeLoopback {
    fn new(initial_read: Vec<u8>) -> Self {
        Self {
            read_cursor: Cursor::new(initial_read),
            written: Vec::new(),
        }
    }
}

impl Read for FakeLoopback {
    fn read(&mut self, buffer: &mut [u8]) -> IoResult<usize> {
        self.read_cursor.read(buffer)
    }
}

impl Write for FakeLoopback {
    fn write(&mut self, buffer: &[u8]) -> IoResult<usize> {
        self.written.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}
