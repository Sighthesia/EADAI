use eadai::bus::MessageBus;
use eadai::message::{BusMessage, ConnectionState, MessageSource};
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
