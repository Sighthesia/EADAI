mod bmi088_publish;
mod command;
mod helpers;

pub use command::{ReconnectController, RuntimeCommandHandle, StopSignal};

use crate::analysis::AnalysisEngine;
use crate::bmi088::{self, Bmi088Frame, Bmi088SessionState, TelemetryPacket, host_command_label};
use crate::bus::MessageBus;
use crate::cli::{ParserKind, RunConfig, TransportSelection};
use crate::error::AppError;
use crate::message::{BusMessage, ConnectionState, MessageSource};
use crate::parser;
use crate::protocols::{
    ByteTransport, CrtpDecoder, MavlinkDecoder, SerialTransport,
    capability::{crtp_to_capabilities, mavlink_to_capabilities},
    self_describing::{SelfDescribingSession, decode_crtp_packet, is_self_describing_packet},
};
use std::sync::mpsc;

use bmi088_publish::{
    publish_identity, publish_rx_with_analysis, publish_sample, publish_schema, send_bmi088_command,
};
use command::RuntimeCommand;
use helpers::{outbound_payload, sleep_with_stop};

const RETRY_SLEEP_SLICE_MS: u64 = 100;

pub struct App {
    config: RunConfig,
    bus: MessageBus,
    stop_signal: StopSignal,
    command_rx: mpsc::Receiver<RuntimeCommand>,
    command_tx: mpsc::Sender<RuntimeCommand>,
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
        RuntimeCommandHandle::new(self.command_tx.clone())
    }

    pub fn run(&self) -> Result<(), AppError> {
        let source = match &self.config.transport {
            TransportSelection::Serial => {
                MessageSource::serial(self.config.port.clone(), self.config.baud_rate)
            }
            TransportSelection::Crazyradio { .. } => {
                MessageSource::crazyradio(self.config.port.clone(), self.config.baud_rate)
            }
        };
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

            match self.open_transport() {
                Ok(mut transport) => {
                    reconnect.reset();
                    self.bus.publish(BusMessage::connection(
                        &source,
                        ConnectionState::Connected,
                        None,
                        attempt,
                        None,
                    ));

                    let mut analysis = AnalysisEngine::new();
                    let mut bmi088_decoder =
                        bmi088::Bmi088StreamDecoder::new(self.config.max_frame_bytes);
                    let mut mavlink_decoder = MavlinkDecoder::new(self.config.max_frame_bytes);
                    let mut crtp_decoder = CrtpDecoder::new(self.config.max_frame_bytes);
                    let mut bmi088_session = Bmi088SessionState::new();
                    let mut self_describing_session = SelfDescribingSession::new();
                    let parser = self.config.parser;

                    if parser == ParserKind::Bmi088 {
                        for command in bmi088_session.boot_commands() {
                            send_bmi088_command(
                                &self.bus,
                                &source,
                                &mut transport,
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
                            &mut transport,
                            &mut bmi088_session,
                            parser == ParserKind::Bmi088,
                        ) {
                            self.publish_retry(&source, attempt, error.to_string(), &reconnect);
                            break;
                        }

                        if let Some(chunk) = transport.read_chunk()? {
                            self.process_chunk(
                                &chunk,
                                parser,
                                &source,
                                attempt,
                                &mut analysis,
                                &mut bmi088_decoder,
                                &mut mavlink_decoder,
                                &mut crtp_decoder,
                                &mut bmi088_session,
                                &mut self_describing_session,
                                &mut transport,
                            )?;
                        }

                        if parser == ParserKind::Bmi088
                            && bmi088_session.phase() == bmi088::Bmi088SessionPhase::AwaitingSchema
                        {
                            for command in bmi088_session.schema_retry_commands() {
                                eprintln!(
                                    "[bmi088][app] awaiting schema -> retry {}",
                                    host_command_label(&command),
                                );
                                send_bmi088_command(
                                    &self.bus,
                                    &source,
                                    &mut transport,
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

    /// Opens a transport connection based on the configured transport selection.
    fn open_transport(&self) -> Result<Box<dyn ByteTransport>, AppError> {
        match &self.config.transport {
            TransportSelection::Serial => {
                let port = crate::serial::open_port(&self.config)?;
                Ok(Box::new(SerialTransport::from_port(port, &self.config)))
            }
            TransportSelection::Crazyradio { uri } => {
                use crate::protocols::{CrazyradioDatarate, CrazyradioTransport};
                let transport = CrazyradioTransport::connect(uri, CrazyradioDatarate::Dr2M)?;
                Ok(Box::new(transport))
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

    fn drain_commands(
        &self,
        source: &MessageSource,
        transport: &mut dyn ByteTransport,
        bmi088_session: &mut Bmi088SessionState,
        bmi088_mode: bool,
    ) -> Result<(), AppError> {
        use crate::bmi088::host_command_from_text;

        loop {
            match self.command_rx.try_recv() {
                Ok(RuntimeCommand::Send { payload }) => {
                    if bmi088_mode
                        && let Some(command) =
                            host_command_from_text(&String::from_utf8_lossy(&payload))
                    {
                        send_bmi088_command(
                            &self.bus,
                            source,
                            transport,
                            bmi088_session,
                            command,
                            None,
                        )?;
                    } else {
                        transport.write_all(&payload)?;
                        transport.flush()?;
                        self.bus
                            .publish(BusMessage::tx_line(source, outbound_payload(&payload)));
                    }
                }
                Ok(RuntimeCommand::SendBmi088 { command, payload }) => {
                    send_bmi088_command(
                        &self.bus,
                        source,
                        transport,
                        bmi088_session,
                        command,
                        payload,
                    )?;
                }
                Ok(RuntimeCommand::SendSelfDescribingSetVariable { set_variable }) => {
                    self.send_self_describing_frame(
                        source,
                        transport,
                        &crate::protocols::self_describing::Frame::SetVariable(set_variable),
                    )?;
                }
                Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => {
                    return Ok(());
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn process_chunk(
        &self,
        chunk: &[u8],
        parser: ParserKind,
        source: &MessageSource,
        _attempt: u32,
        analysis: &mut AnalysisEngine,
        bmi088_decoder: &mut bmi088::Bmi088StreamDecoder,
        mavlink_decoder: &mut MavlinkDecoder,
        crtp_decoder: &mut CrtpDecoder,
        bmi088_session: &mut Bmi088SessionState,
        self_describing_session: &mut SelfDescribingSession,
        transport: &mut dyn ByteTransport,
    ) -> Result<(), AppError> {
        match parser {
            ParserKind::Bmi088 => {
                for packet in bmi088_decoder.push(chunk) {
                    self.handle_bmi088_packet(packet, source, analysis, bmi088_session, transport)?;
                }
            }
            ParserKind::Mavlink => {
                for packet in mavlink_decoder.push(chunk) {
                    eprintln!(
                        "[mavlink][app] decoded msg_id=0x{:04X} sys={} comp={} payload_len={}",
                        packet.message_id,
                        packet.system_id,
                        packet.component_id,
                        packet.payload.len(),
                    );
                    // Emit raw protocol event
                    self.bus
                        .publish(BusMessage::mavlink_packet(source, packet.clone()));
                    // Emit unified capability events
                    for cap_event in mavlink_to_capabilities(&packet) {
                        self.bus.publish(BusMessage::capability(source, cap_event));
                    }
                }
            }
            ParserKind::Crtp => {
                for packet in crtp_decoder.push(chunk) {
                    eprintln!(
                        "[crtp][app] decoded port={} channel={} payload_len={}",
                        packet.port.label(),
                        packet.channel,
                        packet.payload.len(),
                    );
                    // Emit raw protocol event
                    self.bus
                        .publish(BusMessage::crtp_packet(source, packet.clone()));

                    // Check if this is a self-describing protocol packet
                    if is_self_describing_packet(&packet) {
                        if let Ok(Some(frame)) = decode_crtp_packet(&packet) {
                            let responses = self_describing_session.on_frame(&frame);
                            self.publish_self_describing_frame(source, &frame);

                            // Send any response frames
                            for response in responses {
                                self.publish_self_describing_frame(source, &response);
                                if let Err(e) =
                                    self.send_self_describing_frame(source, transport, &response)
                                {
                                    eprintln!(
                                        "[crtp][app] failed to send self-describing response: {e}"
                                    );
                                }
                            }
                        }
                    } else {
                        // Emit unified capability events for non-self-describing packets
                        for cap_event in crtp_to_capabilities(&packet) {
                            self.bus.publish(BusMessage::capability(source, cap_event));
                        }
                    }
                }
            }
            ParserKind::Auto => {
                let mut any_protocol_detected = false;

                // Try BMI088 first — it has the strongest framing (TLV with CRC).
                // Only run if no other protocol has been detected yet, to avoid
                // false positives from BMI088 on random binary data.
                for packet in bmi088_decoder.push(chunk) {
                    if !any_protocol_detected {
                        self.bus.publish(BusMessage::protocol_detected(source, "bmi088"));
                        any_protocol_detected = true;
                    }
                    self.handle_bmi088_packet(packet, source, analysis, bmi088_session, transport)?;
                }

                // Try MAVLink — has strong SOF (0xFD) + CRC validation.
                for packet in mavlink_decoder.push(chunk) {
                    if !any_protocol_detected {
                        self.bus
                            .publish(BusMessage::protocol_detected(source, "mavlink"));
                        any_protocol_detected = true;
                    }
                    eprintln!(
                        "[mavlink][auto] decoded msg_id=0x{:04X} sys={} comp={}",
                        packet.message_id, packet.system_id, packet.component_id,
                    );
                    self.bus
                        .publish(BusMessage::mavlink_packet(source, packet.clone()));
                    for cap_event in mavlink_to_capabilities(&packet) {
                        self.bus.publish(BusMessage::capability(source, cap_event));
                    }
                }

                // Try CRTP — has CRC-8 validation.
                for packet in crtp_decoder.push(chunk) {
                    if !any_protocol_detected {
                        self.bus
                            .publish(BusMessage::protocol_detected(source, "crtp"));
                        any_protocol_detected = true;
                    }
                    eprintln!(
                        "[crtp][auto] decoded port={} channel={} payload_len={}",
                        packet.port.label(),
                        packet.channel,
                        packet.payload.len(),
                    );
                    self.bus
                        .publish(BusMessage::crtp_packet(source, packet.clone()));

                    // Check if this is a self-describing protocol packet
                    if is_self_describing_packet(&packet) {
                        if let Ok(Some(frame)) = decode_crtp_packet(&packet) {
                            let responses = self_describing_session.on_frame(&frame);
                            self.publish_self_describing_frame(source, &frame);

                            // Send any response frames
                            for response in responses {
                                self.publish_self_describing_frame(source, &response);
                                if let Err(e) = self
                                    .send_self_describing_frame(source, transport, &response)
                                {
                                    eprintln!(
                                        "[crtp][auto] failed to send self-describing response: {e}"
                                    );
                                }
                            }
                        }
                    } else {
                        // Emit unified capability events for non-self-describing packets
                        for cap_event in crtp_to_capabilities(&packet) {
                            self.bus.publish(BusMessage::capability(source, cap_event));
                        }
                    }
                }

                // If no binary protocol claimed the data, fall back to text parsing.
                if !any_protocol_detected {
                    let framed = crate::serial::FramedLine {
                        payload: crate::message::LinePayload {
                            text: String::from_utf8_lossy(chunk).into_owned(),
                            raw: chunk.to_vec(),
                        },
                        status: crate::serial::FrameStatus::Complete,
                    };
                    let parser_meta = parser::parse_framed_line(ParserKind::Auto, &framed);
                    publish_rx_with_analysis(
                        &self.bus,
                        source,
                        analysis,
                        framed.payload,
                        parser_meta,
                    );
                }
            }
            _ => {
                let framed = crate::serial::FramedLine {
                    payload: crate::message::LinePayload {
                        text: String::from_utf8_lossy(chunk).into_owned(),
                        raw: chunk.to_vec(),
                    },
                    status: crate::serial::FrameStatus::Complete,
                };
                let parser_meta = parser::parse_framed_line(parser, &framed);
                publish_rx_with_analysis(&self.bus, source, analysis, framed.payload, parser_meta);
            }
        }

        Ok(())
    }

    fn handle_bmi088_packet(
        &self,
        packet: TelemetryPacket,
        source: &MessageSource,
        analysis: &mut AnalysisEngine,
        bmi088_session: &mut Bmi088SessionState,
        transport: &mut dyn ByteTransport,
    ) -> Result<(), AppError> {
        match packet {
            TelemetryPacket::Text(line) => {
                let parser_meta = parser::parse_framed_line(ParserKind::Auto, &line);
                publish_rx_with_analysis(&self.bus, source, analysis, line.payload, parser_meta);
            }
            TelemetryPacket::ShellOutput(output) => {
                self.bus.publish(BusMessage::shell_output(source, output));
            }
            TelemetryPacket::Identity(identity) => {
                bmi088_session.on_frame(&Bmi088Frame::Identity(identity.clone()));
                publish_identity(&self.bus, source, identity);
            }
            TelemetryPacket::Schema(schema) => {
                for command in bmi088_session.on_frame(&Bmi088Frame::Schema(schema.clone())) {
                    send_bmi088_command(
                        &self.bus,
                        source,
                        transport,
                        bmi088_session,
                        command,
                        None,
                    )?;
                }
                publish_schema(&self.bus, source, schema);
            }
            TelemetryPacket::Sample(sample) => {
                bmi088_session.on_frame(&Bmi088Frame::Sample(sample.clone()));
                publish_sample(&self.bus, source, sample);
            }
        }
        Ok(())
    }

    fn publish_self_describing_frame(
        &self,
        source: &MessageSource,
        frame: &crate::protocols::self_describing::Frame,
    ) {
        use crate::protocols::self_describing::Frame;
        match frame {
            Frame::Identity(identity) => {
                self.bus.publish(BusMessage::self_describing_identity(
                    source,
                    identity.clone(),
                ));
            }
            Frame::VariableCatalogPage(catalog) => {
                self.bus
                    .publish(BusMessage::self_describing_variable_catalog(
                        source,
                        catalog.clone(),
                    ));
            }
            Frame::CommandCatalogPage(catalog) => {
                self.bus
                    .publish(BusMessage::self_describing_command_catalog(
                        source,
                        catalog.clone(),
                    ));
            }
            Frame::TelemetrySample(sample) => {
                self.bus
                    .publish(BusMessage::self_describing_sample(source, sample.clone()));
            }
            Frame::SetVariable(set_var) => {
                self.bus.publish(BusMessage::self_describing_set_variable(
                    source,
                    set_var.clone(),
                ));
            }
            Frame::AckResult(result) => {
                self.bus.publish(BusMessage::self_describing_ack_result(
                    source,
                    result.clone(),
                ));
            }
            Frame::HostAck(ack) => {
                // HostAck is a host-to-device message, log it but don't publish as a bus event
                eprintln!(
                    "[self-describing][app] tx HOST_ACK stage={:?} status={}",
                    ack.stage, ack.status
                );
            }
        }
    }

    /// Encode a self-describing frame as a CRTP packet and send it over the transport.
    fn send_self_describing_frame(
        &self,
        source: &MessageSource,
        transport: &mut dyn ByteTransport,
        frame: &crate::protocols::self_describing::Frame,
    ) -> Result<(), AppError> {
        use crate::protocols::self_describing::encode_crtp_packet;

        let crtp_packet = encode_crtp_packet(frame);
        eprintln!(
            "[self-describing][app] sending CRTP frame port={} channel={} payload_len={}",
            crtp_packet.port.label(),
            crtp_packet.channel,
            crtp_packet.payload.len()
        );

        // Build the raw CRTP frame: [header][length][payload][crc8]
        let header = ((crate::protocols::self_describing::SELF_DESCRIBING_CRTP_PORT & 0x07) << 5)
            | (crate::protocols::self_describing::SELF_DESCRIBING_CRTP_CHANNEL & 0x03);
        let mut raw_frame = vec![header, crtp_packet.payload.len() as u8];
        raw_frame.extend_from_slice(&crtp_packet.payload);
        let crc = crc8_update(0, &raw_frame);
        raw_frame.push(crc);

        transport.write_all(&raw_frame)?;
        transport.flush()?;

        // Publish the outgoing frame as a bus event
        self.bus
            .publish(BusMessage::tx_line(source, outbound_payload(&raw_frame)));

        Ok(())
    }
}

/// CRC-8/SAE-J1850 used by CRTP.
fn crc8_update(crc: u8, data: &[u8]) -> u8 {
    let mut crc = crc;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x1D;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}
