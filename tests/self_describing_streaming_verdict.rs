use eadai::message::{BusMessage, MessageKind, MessageSource, TransportKind};
use eadai::protocols::self_describing::crtp_adapter::{
    RawSelfDescribingDecodeContext, RawSelfDescribingDecodeOutcome, RawSelfDescribingDecoder,
};
use eadai::protocols::self_describing::frame::{
    AckStage, CommandCatalogPage, CommandDescriptor, Frame, HostAck, Identity,
    ValueType, VariableDescriptor, VariableCatalogPage,
};
use eadai::protocols::self_describing::{
    SelfDescribingSession, SelfDescribingStreamingDriftVerdict,
};
use eadai::protocols::HandshakeState;

#[path = "../src-tauri/src/model/mod.rs"]
mod model;

use model::{UiBusEvent, UiTransportKind};

fn complete_handshake(session: &mut SelfDescribingSession) {
    session.on_frame(&Frame::Identity(Identity {
        protocol_version: 1,
        device_name: "Test Device".to_string(),
        firmware_version: "1.0.0".to_string(),
        sample_rate_hz: 100,
        variable_count: 1,
        command_count: 1,
        sample_payload_len: 4,
    }));
    session.on_frame(&Frame::CommandCatalogPage(CommandCatalogPage {
        page: 0,
        total_pages: 1,
        commands: vec![CommandDescriptor {
            id: "start".to_string(),
            params: String::new(),
            docs: String::new(),
        }],
    }));
    session.on_frame(&Frame::VariableCatalogPage(VariableCatalogPage {
        page: 0,
        total_pages: 1,
        variables: vec![VariableDescriptor {
            name: "value".to_string(),
            order: 0,
            unit: String::new(),
            adjustable: false,
            value_type: ValueType::U32,
        }],
    }));
    session.on_frame(&Frame::HostAck(HostAck {
        stage: AckStage::VariableCatalog,
    }));
    session.on_frame(&Frame::HostAck(HostAck {
        stage: AckStage::Streaming,
    }));
}

#[test]
fn verdict_triggers_after_three_consecutive_matching_failures() {
    let mut session = SelfDescribingSession::new();
    complete_handshake(&mut session);

    let failure = eadai::protocols::self_describing::crtp_adapter::RawSelfDescribingDecodeFailure {
        phase: "streaming",
        first_payload_byte: Some(0x00),
        payload_len: 3,
        hint: Some("likely bare telemetry sample payload missing frame type 0x05"),
    };

    assert!(session.observe_streaming_drift(&failure).is_none());
    assert!(session.observe_streaming_drift(&failure).is_none());

    let verdict = session
        .observe_streaming_drift(&failure)
        .expect("verdict should fire on third consecutive hit");

    assert_eq!(
        verdict,
        SelfDescribingStreamingDriftVerdict {
            reason_code: "non_canonical_streaming_frame_envelope",
            evidence: eadai::protocols::self_describing::SelfDescribingStreamingDriftEvidence {
                phase: "streaming",
                consecutive_hit_count: 3,
                first_payload_byte: Some(0x00),
                payload_len: 3,
                hint: Some("likely bare telemetry sample payload missing frame type 0x05"),
            },
        }
    );
}

#[test]
fn verdict_resets_when_evidence_changes() {
    let mut session = SelfDescribingSession::new();
    complete_handshake(&mut session);

    let first = eadai::protocols::self_describing::crtp_adapter::RawSelfDescribingDecodeFailure {
        phase: "streaming",
        first_payload_byte: Some(0x00),
        payload_len: 3,
        hint: Some("likely bare telemetry sample payload missing frame type 0x05"),
    };
    let second = eadai::protocols::self_describing::crtp_adapter::RawSelfDescribingDecodeFailure {
        phase: "streaming",
        first_payload_byte: Some(0x05),
        payload_len: 9,
        hint: Some("likely truncated telemetry sample payload after frame type 0x05"),
    };

    assert!(session.observe_streaming_drift(&first).is_none());
    assert!(session.observe_streaming_drift(&second).is_none());
    assert!(session.observe_streaming_drift(&second).is_none());
}

#[test]
fn verdict_propagates_through_message_and_ui_model() {
    let source = MessageSource {
        transport: TransportKind::Serial,
        port: "ttyUSB0".to_string(),
        baud_rate: 115200,
    };
    let verdict = SelfDescribingStreamingDriftVerdict {
        reason_code: "non_canonical_streaming_frame_envelope",
        evidence: eadai::protocols::self_describing::SelfDescribingStreamingDriftEvidence {
            phase: "streaming",
            consecutive_hit_count: 3,
            first_payload_byte: Some(0x00),
            payload_len: 3,
            hint: Some("likely bare telemetry sample payload missing frame type 0x05"),
        },
    };

    let bus = BusMessage::self_describing_verdict(&source, verdict.clone());
    match bus.kind {
        MessageKind::SelfDescribingVerdict(ref payload) => assert_eq!(payload, &verdict),
        _ => panic!("expected verdict message"),
    }

    let ui_event = UiBusEvent::from(bus);
    match ui_event {
        UiBusEvent::SelfDescribingVerdict {
            source: ui_source,
            verdict: ui_verdict,
            ..
        } => {
            assert_eq!(ui_verdict, verdict);
            assert!(matches!(ui_source.transport, UiTransportKind::Serial));
        }
        _ => panic!("expected ui verdict event"),
    }
}

#[test]
fn raw_decoder_reports_failure_outcome_for_bare_sample_body() {
    let mut decoder = RawSelfDescribingDecoder::new(64);
    let context = RawSelfDescribingDecodeContext {
        handshake_state: HandshakeState::Streaming,
        is_streaming: true,
    };

    let sample_body = vec![0x73, 0x03, 0x00, 0x83, 0x00];
    let outcomes = decoder.push_with_context(&sample_body, Some(&context));
    assert_eq!(outcomes.len(), 1);
    match &outcomes[0] {
        RawSelfDescribingDecodeOutcome::Failure(failure) => {
            assert_eq!(failure.phase, "streaming");
            assert_eq!(failure.first_payload_byte, Some(0x00));
            assert_eq!(failure.payload_len, 3);
        }
        _ => panic!("expected failure outcome"),
    }
}
