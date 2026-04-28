#[path = "../src-tauri/src/model.rs"]
mod model;

use model::UiBmi088HostCommand;

#[test]
fn only_payload_commands_forward_bytes() {
    assert_eq!(
        UiBmi088HostCommand::SetTuning.payload_bytes(Some("pid roll=1")),
        Some(b"pid roll=1".to_vec())
    );
    assert_eq!(
        UiBmi088HostCommand::ShellExec.payload_bytes(Some("fc status")),
        Some(b"fc status".to_vec())
    );
    assert_eq!(UiBmi088HostCommand::ReqIdentity.payload_bytes(Some("ignored")), None);
    assert_eq!(UiBmi088HostCommand::ReqTuning.payload_bytes(Some("ignored")), None);
    assert_eq!(UiBmi088HostCommand::Ack.payload_bytes(Some("ignored")), None);
}
