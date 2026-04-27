use crate::analysis::AnalysisEngine;
use crate::bmi088::{
    self, encode_host_command, encode_host_command_with_payload, Bmi088Frame, Bmi088HostCommand,
    Bmi088SessionState, TelemetryPacket, host_command_from_text, host_command_label,
};
use crate::bus::MessageBus;
use crate::cli::RunConfig;
use crate::error::AppError;
use crate::message::{BusMessage, ConnectionState, LinePayload, MessageSource, ParserMeta};
use crate::parser;
use crate::serial;
use std::collections::BTreeMap;
use std::cmp::min;
use std::io::ErrorKind;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RETRY_SLEEP_SLICE_MS: u64 = 100;

enum RuntimeCommand {
    Send { payload: Vec<u8> },
    SendBmi088 { command: Bmi088HostCommand, payload: Option<Vec<u8>> },
}

#[derive(Clone, Debug)]
pub struct RuntimeCommandHandle {
    sender: Sender<RuntimeCommand>,
}

impl RuntimeCommandHandle {
    pub fn send_payload(&self, payload: Vec<u8>) -> Result<(), AppError> {
        self.sender
            .send(RuntimeCommand::Send { payload })
            .map_err(|_| {
                AppError::Io(std::io::Error::new(
                    ErrorKind::BrokenPipe,
                    "runtime command channel is closed",
                ))
            })
    }

    pub fn send_bmi088_command(
        &self,
        command: Bmi088HostCommand,
        payload: Option<Vec<u8>>,
    ) -> Result<(), AppError> {
        self.sender
            .send(RuntimeCommand::SendBmi088 { command, payload })
            .map_err(|_| {
                AppError::Io(std::io::Error::new(
                    ErrorKind::BrokenPipe,
                    "runtime command channel is closed",
                ))
            })
    }
}

#[derive(Clone, Debug, Default)]
pub struct StopSignal {
    requested: Arc<AtomicBool>,
}

impl StopSignal {
    pub fn request_stop(&self) {
        self.requested.store(true, Ordering::SeqCst);
    }

    pub fn is_requested(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconnectController {
    attempt: u32,
    retry_delay: Duration,
}

impl ReconnectController {
    pub fn new(retry_delay: Duration) -> Self {
        Self {
            attempt: 0,
            retry_delay,
        }
    }

    pub fn start_attempt(&mut self) -> u32 {
        self.attempt += 1;
        self.attempt
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    pub fn retry_delay_ms(&self) -> u64 {
        self.retry_delay.as_millis() as u64
    }
}

pub struct App {
    config: RunConfig,
    bus: MessageBus,
    stop_signal: StopSignal,
    command_rx: Receiver<RuntimeCommand>,
    command_tx: Sender<RuntimeCommand>,
}

impl App {
    pub fn new(config: RunConfig, bus: MessageBus) -> Self {
        let (command_tx, command_rx) = mpsc::channel();

        Self {
            config,
            bus,
            stop_signal: StopSignal::default(),
            command_rx,
            command_tx,
        }
    }

    pub fn stop_signal(&self) -> StopSignal {
        self.stop_signal.clone()
    }

    pub fn command_handle(&self) -> RuntimeCommandHandle {
        RuntimeCommandHandle {
            sender: self.command_tx.clone(),
        }
    }

    pub fn run(&self) -> Result<(), AppError> {
        let source = MessageSource::serial(self.config.port.clone(), self.config.baud_rate);
        self.bus.publish(BusMessage::connection(
            &source,
            ConnectionState::Idle,
            None,
            0,
            None,
        ));

        let mut reconnect = ReconnectController::new(self.config.retry_delay);

        loop {
            if self.stop_signal.is_requested() {
                self.publish_stopped(&source, 0, Some("stop requested".to_string()));
                return Ok(());
            }

            let attempt = reconnect.start_attempt();
            self.bus.publish(BusMessage::connection(
                &source,
                ConnectionState::Connecting,
                None,
                attempt,
                None,
            ));

            match serial::open_port(&self.config) {
                Ok(mut port) => {
                    reconnect.reset();
                    self.bus.publish(BusMessage::connection(
                        &source,
                        ConnectionState::Connected,
                        None,
                        attempt,
                        None,
                    ));

                    let mut analysis = AnalysisEngine::new();
                    let mut binary_decoder =
                        bmi088::Bmi088StreamDecoder::new(self.config.max_frame_bytes);
                    let mut bmi088_session = Bmi088SessionState::new();

                    if self.config.parser == crate::cli::ParserKind::Bmi088 {
                        for command in bmi088_session.boot_commands() {
                            send_bmi088_command(
                                &self.bus,
                                &source,
                                &mut *port,
                                &mut bmi088_session,
                                command,
                                None,
                            )?;
                        }
                    }

                    loop {
                        if self.stop_signal.is_requested() {
                            self.publish_stopped(
                                &source,
                                attempt,
                                Some("stop requested".to_string()),
                            );
                            return Ok(());
                        }

                        if let Err(error) = self.drain_commands(
                            &source,
                            &mut *port,
                            &mut bmi088_session,
                            self.config.parser == crate::cli::ParserKind::Bmi088,
                        ) {
                            self.publish_retry(&source, attempt, error.to_string(), &reconnect);
                            break;
                        }

                        if let Some(chunk) = serial::read_bytes(&mut *port)? {
                            for packet in binary_decoder.push(&chunk) {
                                match packet {
                                    TelemetryPacket::Text(line) => {
                                        let parser =
                                            parser::parse_framed_line(self.config.parser, &line);
                                        publish_rx_with_analysis(
                                            &self.bus,
                                            &source,
                                            &mut analysis,
                                            line.payload,
                                            parser,
                                        );
                                    }
                                    TelemetryPacket::ShellOutput(output) => {
                                        self.bus.publish(BusMessage::shell_output(&source, output));
                                    }
                                    TelemetryPacket::Identity(identity) => {
                                        bmi088_session
                                            .on_frame(&Bmi088Frame::Identity(identity.clone()));
                                        publish_identity(&self.bus, &source, identity);
                                    }
                                    TelemetryPacket::Schema(schema) => {
                                        for command in bmi088_session
                                            .on_frame(&Bmi088Frame::Schema(schema.clone()))
                                        {
                                            send_bmi088_command(
                                                &self.bus,
                                                &source,
                                                &mut *port,
                                                &mut bmi088_session,
                                                command,
                                                None,
                                            )?;
                                        }
                                        publish_schema(&self.bus, &source, schema);
                                    }
                                    TelemetryPacket::Sample(sample) => {
                                        bmi088_session
                                            .on_frame(&Bmi088Frame::Sample(sample.clone()));
                                        publish_sample(&self.bus, &source, sample);
                                    }
                                }
                            }
                        }

                        if self.config.parser == crate::cli::ParserKind::Bmi088
                            && bmi088_session.phase() == bmi088::Bmi088SessionPhase::AwaitingSchema
                        {
                            for command in bmi088_session.boot_commands() {
                                send_bmi088_command(
                                    &self.bus,
                                    &source,
                                    &mut *port,
                                    &mut bmi088_session,
                                    command,
                                    None,
                                )?;
                            }
                        }
                    }
                }
                Err(error) => {
                    self.publish_retry(&source, attempt, error.to_string(), &reconnect);
                }
            }

            if !sleep_with_stop(&self.stop_signal, self.config.retry_delay) {
                self.publish_stopped(&source, attempt, Some("stop requested".to_string()));
                return Ok(());
            }
        }
    }

    fn publish_retry(
        &self,
        source: &MessageSource,
        attempt: u32,
        reason: String,
        reconnect: &ReconnectController,
    ) {
        self.bus.publish(BusMessage::connection(
            source,
            ConnectionState::WaitingRetry,
            Some(reason),
            attempt,
            Some(reconnect.retry_delay_ms()),
        ));
    }

    fn publish_stopped(&self, source: &MessageSource, attempt: u32, reason: Option<String>) {
        self.bus.publish(BusMessage::connection(
            source,
            ConnectionState::Stopped,
            reason,
            attempt,
            None,
        ));
    }

    fn drain_commands<T>(
        &self,
        source: &MessageSource,
        port: &mut T,
        bmi088_session: &mut Bmi088SessionState,
        bmi088_mode: bool,
    ) -> Result<(), AppError>
    where
        T: std::io::Write + ?Sized,
    {
        loop {
            match self.command_rx.try_recv() {
                Ok(RuntimeCommand::Send { payload }) => {
                    if bmi088_mode
                        && let Some(command) =
                            host_command_from_text(&String::from_utf8_lossy(&payload))
                    {
                        send_bmi088_command(&self.bus, source, port, bmi088_session, command, None)?;
                    } else {
                        serial::write_payload(port, &payload)?;
                        self.bus
                            .publish(BusMessage::tx_line(source, outbound_payload(&payload)));
                    }
                }
                Ok(RuntimeCommand::SendBmi088 { command, payload }) => {
                    send_bmi088_command(&self.bus, source, port, bmi088_session, command, payload)?;
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return Ok(()),
            }
        }
    }
}

fn send_bmi088_command<T>(
    bus: &MessageBus,
    source: &MessageSource,
    port: &mut T,
    bmi088_session: &mut Bmi088SessionState,
    command: Bmi088HostCommand,
    payload: Option<Vec<u8>>,
) -> Result<(), AppError>
where
    T: std::io::Write + ?Sized,
{
    let encoded = match payload.as_deref() {
        Some(payload) => encode_host_command_with_payload(command.clone(), payload),
        None => encode_host_command(command.clone()),
    };
    serial::write_payload(port, &encoded)?;
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

fn publish_rx_with_analysis(
    bus: &MessageBus,
    source: &MessageSource,
    analysis: &mut AnalysisEngine,
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

fn publish_schema(bus: &MessageBus, source: &MessageSource, schema: bmi088::Bmi088SchemaFrame) {
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

fn publish_identity(
    bus: &MessageBus,
    source: &MessageSource,
    identity: bmi088::Bmi088IdentityFrame,
) {
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

fn publish_sample(bus: &MessageBus, source: &MessageSource, sample: bmi088::Bmi088SampleFrame) {
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

fn timestamp_ms(timestamp: SystemTime) -> u64 {
    timestamp
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn outbound_payload(payload: &[u8]) -> LinePayload {
    let mut raw = payload.to_vec();

    while matches!(raw.last(), Some(b'\n' | b'\r')) {
        raw.pop();
    }

    let text = String::from_utf8_lossy(&raw).into_owned();
    LinePayload { text, raw }
}

fn sleep_with_stop(stop_signal: &StopSignal, total_delay: Duration) -> bool {
    let mut remaining_ms = total_delay.as_millis() as u64;

    while remaining_ms > 0 {
        if stop_signal.is_requested() {
            return false;
        }

        let sleep_ms = min(remaining_ms, RETRY_SLEEP_SLICE_MS);
        thread::sleep(Duration::from_millis(sleep_ms));
        remaining_ms -= sleep_ms;
    }

    true
}
