use eadai::cli::{Command, LoopbackConfig, ParserKind, SendConfig, parse_args};
use eadai::message::{LinePayload, ParserStatus};
use eadai::parser;
use eadai::serial::{
    FrameStatus, FramedLine, LineFramer, payload_bytes, payload_bytes_for_text, read_expected_line,
    write_payload,
};
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
fn appends_newline_for_interactive_text() {
    assert_eq!(payload_bytes_for_text("ping", true), b"ping\n");
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

#[test]
fn ignores_overflowed_frames_when_matching_loopback_echo() {
    let mut stream = FakeLoopback::new(b"abcdef".to_vec());
    let mut framer = LineFramer::with_max_buffer(4);

    let error = read_expected_line(&mut stream, &mut framer, "abcdef", Duration::from_millis(5))
        .unwrap_err();

    assert!(error.to_string().contains("loopback timeout"));
}

#[test]
fn parses_interactive_command_flags() {
    let command = parse_args([
        "interactive".to_string(),
        "--port".to_string(),
        "/dev/ttyACM0".to_string(),
        "--baud".to_string(),
        "9600".to_string(),
        "--no-newline".to_string(),
    ])
    .unwrap();

    match command {
        Command::Interactive(config) => {
            assert_eq!(config.port, "/dev/ttyACM0");
            assert_eq!(config.baud_rate, 9600);
            assert!(!config.append_newline);
        }
        _ => panic!("expected interactive command"),
    }
}

#[test]
fn parses_run_command_parser_flags() {
    let command = parse_args([
        "run".to_string(),
        "--port".to_string(),
        "/dev/ttyUSB0".to_string(),
        "--parser".to_string(),
        "measurements".to_string(),
        "--max-frame-bytes".to_string(),
        "8192".to_string(),
    ])
    .unwrap();

    match command {
        Command::Run(config) => {
            assert_eq!(config.port, "/dev/ttyUSB0");
            assert_eq!(config.parser, ParserKind::Measurements);
            assert_eq!(config.max_frame_bytes, 8192);
        }
        _ => panic!("expected run command"),
    }
}

#[test]
fn auto_parser_keeps_legacy_key_value_semantics() {
    let parsed = parser::parse_framed_line(
        ParserKind::Auto,
        &FramedLine {
            payload: LinePayload {
                text: "temp: 42".to_string(),
                raw: b"temp: 42".to_vec(),
            },
            status: FrameStatus::Complete,
        },
    );

    assert_eq!(parsed.parser_name.as_deref(), Some("key_value"));
    assert_eq!(parsed.status, ParserStatus::Parsed);
    assert_eq!(parsed.fields.get("value").map(String::as_str), Some("42"));
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
