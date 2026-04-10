use eadai::message::{
    BusMessage, ConnectionState, LineDirection, LinePayload, MessageKind, MessageSource,
};

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
