use eadai::app::ReconnectController;
use std::time::Duration;

#[test]
fn resets_attempts_after_successful_connection() {
    let mut controller = ReconnectController::new(Duration::from_secs(1));

    assert_eq!(controller.start_attempt(), 1);
    assert_eq!(controller.start_attempt(), 2);

    controller.reset();

    assert_eq!(controller.start_attempt(), 1);
}

#[test]
fn exposes_retry_delay_in_milliseconds() {
    let controller = ReconnectController::new(Duration::from_millis(1500));

    assert_eq!(controller.retry_delay_ms(), 1500);
}
