use eadai::message::{
    BusMessage, ConnectionState, LineDirection, LinePayload, MessageKind, MessageSource,
    ParserMeta, ParserStatus,
};
use std::collections::BTreeMap;

#[test]
fn creates_received_line_messages() {
    let source = MessageSource::serial("/dev/ttyUSB0", 115_200);
    let message = BusMessage::rx_line(
        &source,
        LinePayload {
            text: "temp:42".to_string(),
            raw: b"temp:42".to_vec(),
        },
    );

    match message.kind {
        MessageKind::Line(line) => {
            assert_eq!(line.direction, LineDirection::Rx);
            assert_eq!(line.payload.text, "temp:42");
        }
        _ => panic!("expected line message"),
    }
}

#[test]
fn preserves_connection_messages() {
    let source = MessageSource::serial("/dev/ttyUSB0", 115_200);
    let message = BusMessage::connection(&source, ConnectionState::Connected, None, 1, None);

    match message.kind {
        MessageKind::Connection(event) => assert_eq!(event.state, ConnectionState::Connected),
        _ => panic!("expected connection message"),
    }
}

#[test]
fn creates_transmitted_line_messages() {
    let source = MessageSource::serial("/dev/ttyUSB0", 115_200);
    let message = BusMessage::tx_line(
        &source,
        LinePayload {
            text: "set:1".to_string(),
            raw: b"set:1".to_vec(),
        },
    );

    match message.kind {
        MessageKind::Line(line) => {
            assert_eq!(line.direction, LineDirection::Tx);
            assert_eq!(line.payload.raw, b"set:1".to_vec());
        }
        _ => panic!("expected line message"),
    }
}

#[test]
fn creates_shell_output_messages() {
    let source = MessageSource::serial("/dev/ttyUSB0", 115_200);
    let message = BusMessage::shell_output(
        &source,
        LinePayload {
            text: "hello shell".to_string(),
            raw: b"hello shell".to_vec(),
        },
    );

    match message.kind {
        MessageKind::ShellOutput(line) => {
            assert_eq!(line.direction, LineDirection::Rx);
            assert_eq!(line.payload.text, "hello shell");
        }
        _ => panic!("expected shell output message"),
    }
}

#[test]
fn preserves_parser_metadata_on_line_messages() {
    let source = MessageSource::serial("/dev/ttyUSB0", 115_200);
    let mut fields = BTreeMap::new();
    fields.insert("channel_id".to_string(), "temp".to_string());

    let message = BusMessage::rx_line(
        &source,
        LinePayload {
            text: "temp:42".to_string(),
            raw: b"temp:42".to_vec(),
        },
    )
    .with_parser(ParserMeta::parsed("measurements", fields));

    assert_eq!(message.parser.status, ParserStatus::Parsed);
    assert_eq!(message.parser.parser_name.as_deref(), Some("measurements"));
    assert_eq!(
        message.parser.fields.get("channel_id").map(String::as_str),
        Some("temp")
    );
}
