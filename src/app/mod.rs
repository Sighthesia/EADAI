mod bmi088_publish;
mod command;
mod helpers;

pub use command::{ReconnectController, RuntimeCommandHandle, StopSignal};

use crate::analysis::AnalysisEngine;
use crate::bmi088::{self, Bmi088Frame, Bmi088SessionState, TelemetryPacket, host_command_label};
use crate::bus::MessageBus;
use crate::cli::{ParserKind, RunConfig};
use crate::error::AppError;
use crate::message::{BusMessage, ConnectionState, MessageSource};
use crate::parser;
use crate::protocols::{CrtpDecoder, MavlinkDecoder};
use crate::serial;
use std::sync::mpsc;

use bmi088_publish::{publish_identity, publish_rx_with_analysis, publish_sample, publish_schema, send_bmi088_command};
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
                    let mut bmi088_decoder =
                        bmi088::Bmi088StreamDecoder::new(self.config.max_frame_bytes);
                    let mut mavlink_decoder =
                        MavlinkDecoder::new(self.config.max_frame_bytes);
                    let mut crtp_decoder =
                        CrtpDecoder::new(self.config.max_frame_bytes);
                    let mut bmi088_session = Bmi088SessionState::new();
                    let parser = self.config.parser;

                    if parser == ParserKind::Bmi088 {
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
                            parser == ParserKind::Bmi088,
                        ) {
                            self.publish_retry(&source, attempt, error.to_string(), &reconnect);
                            break;
                        }

                        if let Some(chunk) = serial::read_bytes(&mut *port)? {
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
                                &mut *port,
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
        use crate::bmi088::host_command_from_text;

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
                Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn process_chunk<T>(
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
        port: &mut T,
    ) -> Result<(), AppError>
    where
        T: std::io::Write + ?Sized,
    {
        match parser {
            ParserKind::Bmi088 => {
                for packet in bmi088_decoder.push(chunk) {
                    self.handle_bmi088_packet(
                        packet, source, analysis, bmi088_session, port,
                    )?;
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
                    self.bus.publish(BusMessage::mavlink_packet(source, packet));
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
                    self.bus.publish(BusMessage::crtp_packet(source, packet));
                }
            }
            ParserKind::Auto => {
                let mut handled = false;

                for packet in bmi088_decoder.push(chunk) {
                    handled = true;
                    self.handle_bmi088_packet(
                        packet, source, analysis, bmi088_session, port,
                    )?;
                }

                if !handled {
                    for packet in mavlink_decoder.push(chunk) {
                        handled = true;
                        eprintln!(
                            "[mavlink][auto] decoded msg_id=0x{:04X} sys={} comp={}",
                            packet.message_id, packet.system_id, packet.component_id,
                        );
                        self.bus.publish(BusMessage::mavlink_packet(source, packet));
                    }
                }

                if !handled {
                    for packet in crtp_decoder.push(chunk) {
                        eprintln!(
                            "[crtp][auto] decoded port={} channel={}",
                            packet.port.label(), packet.channel,
                        );
                        self.bus.publish(BusMessage::crtp_packet(source, packet));
                    }
                }

                if !handled {
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
                publish_rx_with_analysis(
                    &self.bus,
                    source,
                    analysis,
                    framed.payload,
                    parser_meta,
                );
            }
        }

        Ok(())
    }

    fn handle_bmi088_packet<T>(
        &self,
        packet: TelemetryPacket,
        source: &MessageSource,
        analysis: &mut AnalysisEngine,
        bmi088_session: &mut Bmi088SessionState,
        port: &mut T,
    ) -> Result<(), AppError>
    where
        T: std::io::Write + ?Sized,
    {
        match packet {
            TelemetryPacket::Text(line) => {
                let parser_meta = parser::parse_framed_line(ParserKind::Auto, &line);
                publish_rx_with_analysis(
                    &self.bus,
                    source,
                    analysis,
                    line.payload,
                    parser_meta,
                );
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
                        port,
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
}
