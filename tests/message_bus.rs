use eadai::bus::MessageBus;
use eadai::message::{BusMessage, ConnectionState, MessageKind, MessageSource};
use std::time::Duration;

#[test]
fn broadcasts_messages_to_multiple_subscribers() {
    let bus = MessageBus::new();
    let first = bus.subscribe();
    let second = bus.subscribe();
    let source = MessageSource::serial("/dev/ttyUSB0", 115_200);
    let message = BusMessage::connection(&source, ConnectionState::Connected, None, 1, None);

    bus.publish(message.clone());

    let first_message = first.recv_timeout(Duration::from_millis(200)).unwrap();
    let second_message = second.recv_timeout(Duration::from_millis(200)).unwrap();

    assert_eq!(first_message, message);
    assert_eq!(second_message, message);
}

#[test]
fn protocol_detected_message_has_correct_kind() {
    let source = MessageSource::serial("/dev/ttyUSB0", 115_200);
    let message = BusMessage::protocol_detected(&source, "mavlink");

    match &message.kind {
        MessageKind::ProtocolDetected(event) => {
            assert_eq!(event.protocol, "mavlink");
        }
        other => panic!("expected ProtocolDetected, got {:?}", other),
    }
}

#[test]
fn crazyradio_source_has_correct_transport() {
    let source = MessageSource::crazyradio("radio://0/60/2M/E7E7E7E7E7", 0);
    assert_eq!(
        source.transport,
        eadai::message::TransportKind::Crazyradio
    );
    assert_eq!(source.port, "radio://0/60/2M/E7E7E7E7E7");
}
