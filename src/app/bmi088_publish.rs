use crate::bmi088::{self, Bmi088HostCommand, host_command_label};
use crate::bus::MessageBus;
use crate::message::{BusMessage, LinePayload, MessageSource, ParserMeta};
use crate::protocols::ByteTransport;
use std::collections::BTreeMap;

use super::helpers::{hex_preview, timestamp_ms};

pub(super) fn send_bmi088_command(
    bus: &MessageBus,
    source: &MessageSource,
    transport: &mut dyn ByteTransport,
    bmi088_session: &mut bmi088::Bmi088SessionState,
    command: Bmi088HostCommand,
    payload: Option<Vec<u8>>,
) -> Result<(), crate::error::AppError> {
    use bmi088::{encode_host_command, encode_host_command_with_payload};

    let encoded = match payload.as_deref() {
        Some(payload) => encode_host_command_with_payload(command.clone(), payload),
        None => encode_host_command(command.clone()),
    };
    eprintln!(
        "[bmi088][app] tx {} bytes={} preview={}",
        host_command_label(&command),
        encoded.len(),
        hex_preview(&encoded, 16),
    );
    transport.write_all(&encoded)?;
    transport.flush()?;
    bmi088_session.on_host_command(command.clone());
    bus.publish(
        BusMessage::tx_line(
            source,
            LinePayload {
                text: host_command_label(&command).to_string(),
                raw: encoded,
            },
        )
        .with_parser(bmi088_command_parser_meta(&command)),
    );
    Ok(())
}

pub(super) fn publish_rx_with_analysis(
    bus: &MessageBus,
    source: &MessageSource,
    analysis: &mut crate::analysis::AnalysisEngine,
    payload: LinePayload,
    parser: crate::message::ParserMeta,
) {
    let line_message = BusMessage::rx_line(source, payload).with_parser(parser.clone());
    let analysis_messages = analysis.ingest_line(
        source,
        &crate::message::LineDirection::Rx,
        &parser,
        timestamp_ms(line_message.timestamp),
    );
    bus.publish(line_message);

    if let Some(messages) = analysis_messages {
        for message in messages {
            bus.publish(message);
        }
    }
}

pub(super) fn publish_schema(
    bus: &MessageBus,
    source: &MessageSource,
    schema: bmi088::Bmi088SchemaFrame,
) {
    eprintln!(
        "[bmi088][app] publish SCHEMA seq={} rate_hz={} field_count={} sample_len={}",
        schema.seq,
        schema.rate_hz,
        schema.fields.len(),
        schema.sample_len,
    );
    let parser = ParserMeta::parsed(
        "bmi088_schema",
        bmi088_payload_fields(&[
            ("command", "SCHEMA".to_string()),
            ("rate_hz", schema.rate_hz.to_string()),
            ("sample_len", schema.sample_len.to_string()),
            ("field_count", schema.fields.len().to_string()),
            (
                "field_order",
                schema
                    .fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ]),
    );
    bus.publish(BusMessage::telemetry_schema(source, schema).with_parser(parser));
}

pub(super) fn publish_identity(
    bus: &MessageBus,
    source: &MessageSource,
    identity: bmi088::Bmi088IdentityFrame,
) {
    eprintln!(
        "[bmi088][app] publish IDENTITY seq={} device={} protocol={} schema_fields={} sample_len={}",
        identity.seq,
        identity.device_name,
        identity.protocol_version,
        identity.schema_field_count,
        identity.sample_payload_len,
    );
    let parser = ParserMeta::parsed(
        "bmi088_identity",
        bmi088_payload_fields(&[
            ("command", "IDENTITY".to_string()),
            ("device_name", identity.device_name.clone()),
            ("board_name", identity.board_name.clone()),
            ("firmware_version", identity.firmware_version.clone()),
            ("protocol_name", identity.protocol_name.clone()),
            ("protocol_version", identity.protocol_version.clone()),
            ("transport_name", identity.transport_name.clone()),
            ("sample_rate_hz", identity.sample_rate_hz.to_string()),
            ("schema_field_count", identity.schema_field_count.to_string()),
            ("sample_payload_len", identity.sample_payload_len.to_string()),
            ("protocol_version_byte", identity.protocol_version_byte.to_string()),
            ("feature_flags", format!("0x{:04X}", identity.feature_flags)),
            ("baud_rate", identity.baud_rate.to_string()),
            (
                "protocol_minor_version",
                identity.protocol_minor_version.to_string(),
            ),
        ]),
    );
    bus.publish(BusMessage::telemetry_identity(source, identity).with_parser(parser));
}

pub(super) fn publish_sample(
    bus: &MessageBus,
    source: &MessageSource,
    sample: bmi088::Bmi088SampleFrame,
) {
    eprintln!(
        "[bmi088][app] publish SAMPLE seq={} field_count={} first_field={}",
        sample.seq,
        sample.fields.len(),
        sample
            .fields
            .first()
            .map(|field| field.name.as_str())
            .unwrap_or("<none>"),
    );
    let parser = ParserMeta::parsed(
        "bmi088_sample",
        bmi088_payload_fields(&[
            ("command", "SAMPLE".to_string()),
            ("field_count", sample.fields.len().to_string()),
            (
                "field_order",
                sample
                    .fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ]),
    );
    bus.publish(BusMessage::telemetry_sample(source, sample).with_parser(parser));
}

fn bmi088_command_parser_meta(command: &Bmi088HostCommand) -> ParserMeta {
    ParserMeta::parsed(
        "bmi088_command",
        bmi088_payload_fields(&[
            ("command", host_command_label(command).to_string()),
            ("frame_type", "REQUEST".to_string()),
            ("payload_len", "variable".to_string()),
        ]),
    )
}

fn bmi088_payload_fields(entries: &[(&str, String)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}
