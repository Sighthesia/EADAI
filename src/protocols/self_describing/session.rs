/// Self-describing protocol session state management.
///
/// This module provides runtime state management for the self-describing protocol,
/// tracking handshake progress and sample reconstruction state.
use super::bitmap::BitmapCodec;
use super::crtp_adapter::RawSelfDescribingDecodeFailure;
use super::frame::*;
use super::state::{HandshakeMachine, HandshakeState};
use serde::Serialize;

const STREAMING_DRIFT_VERDICT_THRESHOLD: usize = 3;

/// Structured verdict for non-canonical streaming frame envelope drift.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SelfDescribingStreamingDriftVerdict {
    /// Fixed reason code that identifies the drift class.
    pub reason_code: &'static str,
    /// Compact evidence summary for diagnostics.
    pub evidence: SelfDescribingStreamingDriftEvidence,
}

/// Compact evidence summary for a streaming drift verdict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SelfDescribingStreamingDriftEvidence {
    /// Session phase when the drift was observed.
    pub phase: &'static str,
    /// Consecutive matching hits.
    pub consecutive_hit_count: usize,
    /// First payload byte from the failing payload.
    pub first_payload_byte: Option<u8>,
    /// Payload length.
    pub payload_len: usize,
    /// Short diagnostic hint.
    pub hint: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StreamingDriftSignature {
    phase: &'static str,
    first_payload_byte: Option<u8>,
    payload_len: usize,
    hint: Option<&'static str>,
}

#[derive(Clone, Debug, Default)]
struct StreamingDriftTracker {
    last_signature: Option<StreamingDriftSignature>,
    consecutive_hits: usize,
    verdict_emitted: bool,
}

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
    /// Consecutive streaming drift evidence tracker.
    streaming_drift: StreamingDriftTracker,
}

impl SelfDescribingSession {
    /// Create a new session.
    pub fn new() -> Self {
        Self {
            handshake: HandshakeMachine::new(),
            bitmap_codec: None,
            has_identity: false,
            is_streaming: false,
            streaming_drift: StreamingDriftTracker::default(),
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

                if !matches!(self.handshake_state(), HandshakeState::WaitingIdentity) {
                    eprintln!(
                        "[self-describing][session] unexpected IDENTITY retransmit state={:?}",
                        self.handshake_state()
                    );
                }

                if let Err(e) = self.handshake.on_identity(identity.clone()) {
                    eprintln!("[self-describing][session] identity error: {e}");
                    return responses;
                }

                self.has_identity = true;

                if matches!(self.handshake_state(), HandshakeState::WaitingCommandCatalog) {
                    eprintln!("[self-describing][session] stage complete: identity -> ack identity");
                    responses.push(Self::host_ack(AckStage::Identity));
                }
            }
            Frame::CommandCatalogPage(page) => {
                eprintln!(
                    "[self-describing][session] rx COMMAND_CATALOG page={}/{} cmds={}",
                    page.page + 1,
                    page.total_pages,
                    page.commands.len()
                );

                if !matches!(self.handshake_state(), HandshakeState::WaitingCommandCatalog) {
                    eprintln!(
                        "[self-describing][session] unexpected COMMAND_CATALOG retransmit state={:?}",
                        self.handshake_state()
                    );
                }

                if let Err(e) = self.handshake.on_command_catalog_page(page.clone()) {
                    eprintln!("[self-describing][session] command catalog error: {e}");
                    return responses;
                }

                if matches!(self.handshake_state(), HandshakeState::WaitingVariableCatalog) {
                    eprintln!("[self-describing][session] stage complete: command catalog -> ack command catalog");
                    responses.push(Self::host_ack(AckStage::CommandCatalog));
                }
            }
            Frame::VariableCatalogPage(page) => {
                eprintln!(
                    "[self-describing][session] rx VARIABLE_CATALOG page={}/{} vars={}",
                    page.page + 1,
                    page.total_pages,
                    page.variables.len()
                );

                if !matches!(self.handshake_state(), HandshakeState::WaitingVariableCatalog) {
                    eprintln!(
                        "[self-describing][session] unexpected VARIABLE_CATALOG retransmit state={:?}",
                        self.handshake_state()
                    );
                }

                if let Err(e) = self.handshake.on_variable_catalog_page(page.clone()) {
                    eprintln!("[self-describing][session] variable catalog error: {e}");
                    return responses;
                }

                // If we just completed the variable catalog, create the bitmap codec
                if matches!(self.handshake.state(), HandshakeState::WaitingHostAck) {
                    eprintln!("[self-describing][session] stage complete: variable catalog -> ack variable catalog");
                    let vars = self.handshake.variable_catalog();
                    self.bitmap_codec = Some(BitmapCodec::new(vars));

                    // Send host acknowledgment
                    responses.push(Self::host_ack(AckStage::VariableCatalog));
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

                // Decode the sample and advance bitmap state so unchanged fields remain available.
                match self.decode_telemetry_sample(sample) {
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
            Frame::AckResult(result) => {
                eprintln!(
                    "[self-describing][session] rx ACK_RESULT seq={} code={} msg={}",
                    result.seq, result.code, result.message
                );
            }
            Frame::HostAck(ack) => {
                eprintln!("[self-describing][session] rx HOST_ACK stage={:?}", ack.stage);

                if let Err(e) = self.handshake.on_host_ack(ack) {
                    eprintln!("[self-describing][session] host ack error: {e}");
                }

                // If we're ready to stream, send the final streaming acknowledgment.
                if matches!(self.handshake.state(), HandshakeState::ReadyToStream)
                    && !self.is_streaming
                {
                    responses.push(Frame::HostAck(HostAck {
                        stage: AckStage::Streaming,
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
        self.streaming_drift = StreamingDriftTracker::default();
    }

    /// Observe a raw decode failure and emit a verdict once the evidence repeats.
    pub fn observe_streaming_drift(
        &mut self,
        failure: &RawSelfDescribingDecodeFailure,
    ) -> Option<SelfDescribingStreamingDriftVerdict> {
        if !self.is_post_handshake() {
            self.streaming_drift = StreamingDriftTracker::default();
            return None;
        }

        let signature = StreamingDriftSignature {
            phase: failure.phase,
            first_payload_byte: failure.first_payload_byte,
            payload_len: failure.payload_len,
            hint: failure.hint,
        };

        let tracker = &mut self.streaming_drift;
        if tracker.last_signature.as_ref() == Some(&signature) {
            tracker.consecutive_hits = tracker.consecutive_hits.saturating_add(1);
        } else {
            tracker.last_signature = Some(signature);
            tracker.consecutive_hits = 1;
            tracker.verdict_emitted = false;
        }

        if tracker.consecutive_hits >= STREAMING_DRIFT_VERDICT_THRESHOLD && !tracker.verdict_emitted {
            tracker.verdict_emitted = true;
            return Some(SelfDescribingStreamingDriftVerdict {
                reason_code: "non_canonical_streaming_frame_envelope",
                evidence: SelfDescribingStreamingDriftEvidence {
                    phase: failure.phase,
                    consecutive_hit_count: tracker.consecutive_hits,
                    first_payload_byte: failure.first_payload_byte,
                    payload_len: failure.payload_len,
                    hint: failure.hint,
                },
            });
        }

        None
    }

    fn is_post_handshake(&self) -> bool {
        matches!(
            self.handshake_state(),
            HandshakeState::WaitingCommandCatalog
                | HandshakeState::WaitingVariableCatalog
                | HandshakeState::WaitingHostAck
                | HandshakeState::ReadyToStream
                | HandshakeState::Streaming
        ) || self.is_streaming
    }

    fn decode_telemetry_sample(&mut self, sample: &TelemetrySample) -> Result<Vec<u8>, String> {
        match self.bitmap_codec.as_mut() {
            Some(codec) => codec.decode_and_update(sample),
            None => Err("bitmap codec unavailable".to_string()),
        }
    }

    fn host_ack(stage: AckStage) -> Frame {
        Frame::HostAck(HostAck { stage })
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
        assert_eq!(responses.len(), 1);
        assert!(matches!(responses[0], Frame::HostAck(HostAck { stage: AckStage::Identity })));
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
        assert_eq!(responses.len(), 1);
        assert!(matches!(responses[0], Frame::HostAck(HostAck { stage: AckStage::CommandCatalog })));
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
        assert!(matches!(responses[0], Frame::HostAck(HostAck { stage: AckStage::VariableCatalog })));
        assert_eq!(*session.handshake_state(), HandshakeState::WaitingHostAck);

        // Receive host acknowledgment
        let ack = HostAck {
            stage: AckStage::VariableCatalog,
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
    fn test_session_sample_decode_advances_bitmap_state() {
        let mut session = SelfDescribingSession::new();

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
        }));

        let first_sample = TelemetrySample {
            seq: 1,
            changed_bitmap: vec![0b00000011],
            values: vec![
                100i16.to_le_bytes()[0],
                100i16.to_le_bytes()[1],
                0x00,
                0x00,
                0x80,
                0x3F,
            ],
        };
        let first = session
            .decode_telemetry_sample(&first_sample)
            .expect("first sample should decode");
        assert_eq!(first, vec![100i16.to_le_bytes()[0], 100i16.to_le_bytes()[1], 0x00, 0x00, 0x80, 0x3F]);

        let second_sample = TelemetrySample {
            seq: 2,
            changed_bitmap: vec![0b00000001],
            values: vec![150i16.to_le_bytes()[0], 150i16.to_le_bytes()[1]],
        };
        let second = session
            .decode_telemetry_sample(&second_sample)
            .expect("second sample should reuse prior gain value");

        assert_eq!(i16::from_le_bytes([second[0], second[1]]), 150);
        assert_eq!(f32::from_le_bytes([second[2], second[3], second[4], second[5]]), 1.0);
    }

    #[test]
    fn test_session_set_variable() {
        let session = SelfDescribingSession::new();

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
