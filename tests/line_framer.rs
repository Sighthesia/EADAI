use eadai::serial::{FrameStatus, LineFramer};

#[test]
fn joins_a_line_across_multiple_chunks() {
    let mut framer = LineFramer::new();

    assert!(framer.push(b"imu:").is_empty());
    let lines = framer.push(b"123\n");

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].status, FrameStatus::Complete);
    assert_eq!(lines[0].payload.text, "imu:123");
    assert_eq!(lines[0].payload.raw, b"imu:123");
}

#[test]
fn trims_crlf_before_emitting() {
    let mut framer = LineFramer::new();

    let lines = framer.push(b"temp:42\r\n");

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].payload.text, "temp:42");
    assert_eq!(lines[0].payload.raw, b"temp:42");
}

#[test]
fn keeps_partial_data_without_newline() {
    let mut framer = LineFramer::new();

    let lines = framer.push(b"gyro:999");

    assert!(lines.is_empty());
    assert_eq!(framer.buffered_len(), 8);
}

#[test]
fn splits_multiple_lines_from_one_chunk() {
    let mut framer = LineFramer::new();

    let lines = framer.push(b"a:1\nb:2\n");

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].payload.text, "a:1");
    assert_eq!(lines[1].payload.text, "b:2");
}

#[test]
fn emits_overflow_line_when_buffer_limit_is_exceeded() {
    let mut framer = LineFramer::with_max_buffer(4);

    let lines = framer.push(b"abcdef");

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].status, FrameStatus::Overflow);
    assert_eq!(lines[0].payload.text, "abcdef");
    assert_eq!(framer.buffered_len(), 0);
}
