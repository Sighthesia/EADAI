/// Self-describing protocol session state management.
///
/// This module provides runtime state management for the self-describing protocol,
/// tracking handshake progress and sample reconstruction state.
use super::bitmap::BitmapCodec;
use super::frame::*;
use super::state::{HandshakeMachine, HandshakeState};

/// Session state for the self-describing protocol.
pub struct SelfDescribingSession {
    /// Handshake state machine.
    handshake: HandshakeMachine,
    /// Bitmap codec for sample reconstruction.
    bitmap_codec: Option<BitmapCodec>,
    /// Whether we've received identity.
    has_identity: bool,
    /// Whether streaming is active.
    is_streaming: bool,
}

impl SelfDescribingSession {
    /// Create a new session.
    pub fn new() -> Self {
        Self {
            handshake: HandshakeMachine::new(),
            bitmap_codec: None,
            has_identity: false,
            is_streaming: false,
        }
    }

    /// Get the current handshake state.
    pub fn handshake_state(&self) -> &HandshakeState {
        self.handshake.state()
    }

    /// Check if streaming is active.
    pub fn is_streaming(&self) -> bool {
        self.is_streaming
    }

    /// Get the variable catalog (if received).
    pub fn variable_catalog(&self) -> Vec<VariableDescriptor> {
        self.handshake.variable_catalog()
    }

    /// Get the command catalog (if received).
    pub fn command_catalog(&self) -> Vec<CommandDescriptor> {
        self.handshake.command_catalog()
    }

    /// Get the device identity (if received).
    pub fn identity(&self) -> Option<&Identity> {
        self.handshake.identity()
    }

    /// Process an incoming frame and return any host commands to send.
    ///
    /// Returns a list of frames that should be sent back to the device.
    pub fn on_frame(&mut self, frame: &Frame) -> Vec<Frame> {
        let mut responses = Vec::new();

        match frame {
            Frame::Identity(identity) => {
                eprintln!(
                    "[self-describing][session] rx IDENTITY device={} firmware={} vars={} cmds={}",
                    identity.device_name,
                    identity.firmware_version,
                    identity.variable_count,
                    identity.command_count
                );

                if let Err(e) = self.handshake.on_identity(identity.clone()) {
                    eprintln!("[self-describing][session] identity error: {e}");
                    return responses;
                }

                self.has_identity = true;
            }
            Frame::CommandCatalogPage(page) => {
                eprintln!(
                    "[self-describing][session] rx COMMAND_CATALOG page={}/{} cmds={}",
                    page.page + 1,
                    page.total_pages,
                    page.commands.len()
                );

                if let Err(e) = self.handshake.on_command_catalog_page(page.clone()) {
                    eprintln!("[self-describing][session] command catalog error: {e}");
                }
            }
            Frame::VariableCatalogPage(page) => {
                eprintln!(
                    "[self-describing][session] rx VARIABLE_CATALOG page={}/{} vars={}",
                    page.page + 1,
                    page.total_pages,
                    page.variables.len()
                );

                if let Err(e) = self.handshake.on_variable_catalog_page(page.clone()) {
                    eprintln!("[self-describing][session] variable catalog error: {e}");
                }

                // If we just completed the variable catalog, create the bitmap codec
                if matches!(self.handshake.state(), HandshakeState::WaitingHostAck) {
                    let vars = self.handshake.variable_catalog();
                    self.bitmap_codec = Some(BitmapCodec::new(vars));

                    // Send host acknowledgment
                    responses.push(Frame::HostAck(HostAck {
                        stage: AckStage::VariableCatalog,
                        status: 0,
                        message: "OK".to_string(),
                    }));
                }
            }
            Frame::TelemetrySample(sample) => {
                if !self.is_streaming {
                    eprintln!(
                        "[self-describing][session] rx SAMPLE seq={} transitioning to streaming",
                        sample.seq
                    );
                    self.is_streaming = true;
                }

                // Decode the sample using the bitmap codec
                if let Some(codec) = &self.bitmap_codec {
                    match codec.decode(sample) {
                        Ok(values) => {
                            eprintln!(
                                "[self-describing][session] rx SAMPLE seq={} decoded values_len={}",
                                sample.seq,
                                values.len()
                            );
                        }
                        Err(e) => {
                            eprintln!("[self-describing][session] sample decode error: {e}");
                        }
                    }
                }
            }
            Frame::AckResult(result) => {
                eprintln!(
                    "[self-describing][session] rx ACK_RESULT seq={} code={} msg={}",
                    result.seq, result.code, result.message
                );
            }
            Frame::HostAck(ack) => {
                eprintln!(
                    "[self-describing][session] rx HOST_ACK stage={:?} status={}",
                    ack.stage, ack.status
                );

                if let Err(e) = self.handshake.on_host_ack(ack) {
                    eprintln!("[self-describing][session] host ack error: {e}");
                }

                // If we're ready to stream, send start acknowledgment
                if matches!(self.handshake.state(), HandshakeState::ReadyToStream) {
                    responses.push(Frame::HostAck(HostAck {
                        stage: AckStage::Streaming,
                        status: 0,
                        message: "OK".to_string(),
                    }));
                    self.is_streaming = true;
                }
            }
            Frame::SetVariable(set_var) => {
                eprintln!(
                    "[self-describing][session] rx SET_VARIABLE seq={} var_idx={} value_len={}",
                    set_var.seq,
                    set_var.variable_index,
                    set_var.value.len()
                );

                // For now, just acknowledge the set variable request
                // In a real implementation, this would apply the value and get device confirmation
                responses.push(Frame::AckResult(AckResult {
                    seq: set_var.seq,
                    code: 0,
                    message: "OK".to_string(),
                }));
            }
        }

        responses
    }

    /// Create a SetVariable frame to send to the device.
    pub fn create_set_variable(&self, variable_index: u16, value: Vec<u8>, seq: u32) -> Frame {
        Frame::SetVariable(SetVariable {
            seq,
            variable_index,
            value,
        })
    }

    /// Reset the session state.
    pub fn reset(&mut self) {
        self.handshake = HandshakeMachine::new();
        self.bitmap_codec = None;
        self.has_identity = false;
        self.is_streaming = false;
    }
}

impl Default for SelfDescribingSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_identity() -> Identity {
        Identity {
            protocol_version: 1,
            device_name: "Test Device".to_string(),
            firmware_version: "1.0.0".to_string(),
            sample_rate_hz: 100,
            variable_count: 2,
            command_count: 1,
            sample_payload_len: 8,
        }
    }

    #[test]
    fn test_session_handshake_flow() {
        let mut session = SelfDescribingSession::new();
        assert_eq!(*session.handshake_state(), HandshakeState::WaitingIdentity);
        assert!(!session.is_streaming());

        // Receive identity
        let responses = session.on_frame(&Frame::Identity(sample_identity()));
        assert!(responses.is_empty());
        assert_eq!(
            *session.handshake_state(),
            HandshakeState::WaitingCommandCatalog
        );

        // Receive command catalog
        let cmd_catalog = CommandCatalogPage {
            page: 0,
            total_pages: 1,
            commands: vec![CommandDescriptor {
                id: "start".to_string(),
                params: "".to_string(),
                docs: "Start streaming".to_string(),
            }],
        };
        let responses = session.on_frame(&Frame::CommandCatalogPage(cmd_catalog));
        assert!(responses.is_empty());
        assert_eq!(
            *session.handshake_state(),
            HandshakeState::WaitingVariableCatalog
        );

        // Receive variable catalog
        let var_catalog = VariableCatalogPage {
            page: 0,
            total_pages: 1,
            variables: vec![
                VariableDescriptor {
                    name: "acc_x".to_string(),
                    order: 0,
                    unit: "m/s^2".to_string(),
                    adjustable: false,
                    value_type: ValueType::I16,
                },
                VariableDescriptor {
                    name: "gain".to_string(),
                    order: 1,
                    unit: "".to_string(),
                    adjustable: true,
                    value_type: ValueType::F32,
                },
            ],
        };
        let responses = session.on_frame(&Frame::VariableCatalogPage(var_catalog));
        assert_eq!(responses.len(), 1); // Should send HostAck
        assert_eq!(*session.handshake_state(), HandshakeState::WaitingHostAck);

        // Receive host acknowledgment
        let ack = HostAck {
            stage: AckStage::VariableCatalog,
            status: 0,
            message: "OK".to_string(),
        };
        let responses = session.on_frame(&Frame::HostAck(ack));
        assert_eq!(responses.len(), 1); // Should send Streaming ack
        assert!(session.is_streaming());
    }

    #[test]
    fn test_session_sample_decode() {
        let mut session = SelfDescribingSession::new();

        // Complete handshake
        session.on_frame(&Frame::Identity(sample_identity()));
        session.on_frame(&Frame::CommandCatalogPage(CommandCatalogPage {
            page: 0,
            total_pages: 1,
            commands: vec![],
        }));
        session.on_frame(&Frame::VariableCatalogPage(VariableCatalogPage {
            page: 0,
            total_pages: 1,
            variables: vec![
                VariableDescriptor {
                    name: "acc_x".to_string(),
                    order: 0,
                    unit: "m/s^2".to_string(),
                    adjustable: false,
                    value_type: ValueType::I16,
                },
                VariableDescriptor {
                    name: "gain".to_string(),
                    order: 1,
                    unit: "".to_string(),
                    adjustable: true,
                    value_type: ValueType::F32,
                },
            ],
        }));
        session.on_frame(&Frame::HostAck(HostAck {
            stage: AckStage::VariableCatalog,
            status: 0,
            message: "OK".to_string(),
        }));

        // Send a sample
        let sample = TelemetrySample {
            seq: 1,
            changed_bitmap: vec![0b00000011], // Both variables changed
            values: vec![
                100i16.to_le_bytes()[0],
                100i16.to_le_bytes()[1],
                0x00,
                0x00,
                0x80,
                0x3F,
            ], // acc_x=100, gain=1.0
        };
        let responses = session.on_frame(&Frame::TelemetrySample(sample));
        assert!(responses.is_empty());
        assert!(session.is_streaming());
    }

    #[test]
    fn test_session_set_variable() {
        let mut session = SelfDescribingSession::new();

        // Create a set variable frame
        let set_var = session.create_set_variable(1, vec![0x00, 0x00, 0x80, 0x3F], 10);

        match set_var {
            Frame::SetVariable(sv) => {
                assert_eq!(sv.seq, 10);
                assert_eq!(sv.variable_index, 1);
                assert_eq!(sv.value, vec![0x00, 0x00, 0x80, 0x3F]);
            }
            _ => panic!("expected set variable frame"),
        }
    }
}
