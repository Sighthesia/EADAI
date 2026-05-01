use eadai::app::App;
use eadai::bus::{BusSubscription, MessageBus};
use eadai::cli::{Command, InteractiveConfig, LoopbackConfig, SendConfig, parse_args};
use eadai::error::AppError;
use eadai::message::{LineDirection, MessageKind};
use eadai::serial::{self, LineFramer};
use std::io::{self, BufRead};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    match parse_args(std::env::args().skip(1))? {
        Command::Ports => {
            for port in serial::list_ports()? {
                println!("{port}");
            }
            Ok(())
        }
        Command::Run(config) => {
            let bus = MessageBus::new();
            let subscription = bus.subscribe();
            let app = App::new(config, bus.clone());
            let _printer = std::thread::spawn(move || print_messages(subscription));
            let result = app.run();
            drop(bus);
            result
        }
        Command::Send(config) => send_once(&config),
        Command::LoopbackTest(config) => run_loopback_test(&config),
        Command::Interactive(config) => run_interactive(&config),
    }
}

fn send_once(config: &SendConfig) -> Result<(), AppError> {
    let mut port = serial::open_send_port(config)?;
    let payload = serial::payload_bytes(config);
    serial::write_payload(&mut *port, &payload)?;
    println!(
        "[send] port={} baud={} bytes={} payload={:?}",
        config.port,
        config.baud_rate,
        payload.len(),
        String::from_utf8_lossy(&payload)
    );
    Ok(())
}

fn run_loopback_test(config: &LoopbackConfig) -> Result<(), AppError> {
    let mut port = serial::open_send_port(&config.send)?;
    let payload = serial::payload_bytes(&config.send);
    let expected_text = config.send.payload.clone();
    let mut framer = LineFramer::new();

    serial::write_payload(&mut *port, &payload)?;
    let echoed = serial::read_expected_line(
        &mut *port,
        &mut framer,
        &expected_text,
        config.loopback_timeout,
    )?;

    if echoed.text != expected_text {
        return Err(AppError::LoopbackMismatch {
            expected: expected_text,
            received: echoed.text,
        });
    }

    println!(
        "[loopback-ok] port={} baud={} sent={:?} received={:?}",
        config.send.port,
        config.send.baud_rate,
        String::from_utf8_lossy(&payload),
        echoed.text
    );
    Ok(())
}

fn run_interactive(config: &InteractiveConfig) -> Result<(), AppError> {
    let mut write_port = serial::open_interactive_port(config)?;
    let mut read_port = write_port.try_clone()?;
    let port_name = config.port.clone();

    let _reader = std::thread::spawn(move || -> Result<(), AppError> {
        let mut framer = LineFramer::new();

        loop {
            serial::pump_port(&mut *read_port, &mut framer, |line| {
                println!(
                    "[rx] port={} bytes={} frame_status={:?} text={}",
                    port_name,
                    line.payload.raw.len(),
                    line.status,
                    line.payload.text
                );
            })?;
        }
    });

    println!(
        "[interactive] connected to {} at {} baud",
        config.port, config.baud_rate
    );
    println!("[interactive] type one line per command, use /quit to exit");

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line == "/quit" {
            break;
        }

        let payload = serial::payload_bytes_for_text(&line, config.append_newline);
        serial::write_payload(&mut *write_port, &payload)?;
        println!(
            "[tx] port={} bytes={} text={}",
            config.port,
            payload.len(),
            line
        );
    }

    drop(write_port);
    Ok(())
}

fn print_messages(subscription: BusSubscription) {
    while let Ok(message) = subscription.recv() {
        match message.kind {
            MessageKind::Connection(event) => {
                let reason = event.reason.unwrap_or_else(|| "-".to_string());
                println!(
                    "[conn] state={:?} port={} baud={} attempt={} retry_ms={:?} reason={}",
                    event.state,
                    message.source.port,
                    message.source.baud_rate,
                    event.attempt,
                    event.retry_delay_ms,
                    reason,
                );
            }
            MessageKind::Line(line) => {
                let parser_name = message
                    .parser
                    .parser_name
                    .unwrap_or_else(|| "raw".to_string());
                let direction = match line.direction {
                    LineDirection::Rx => "rx",
                    LineDirection::Tx => "tx",
                };
                println!(
                    "[line] dir={} port={} bytes={} parser={} parse_status={:?} fields={:?} text={}",
                    direction,
                    message.source.port,
                    line.payload.raw.len(),
                    parser_name,
                    message.parser.status,
                    message.parser.fields,
                    line.payload.text
                );
            }
            MessageKind::ShellOutput(line) => {
                println!(
                    "[shell-output] port={} bytes={} text={}",
                    message.source.port,
                    line.payload.raw.len(),
                    line.payload.text
                );
            }
            MessageKind::Analysis(frame) => {
                println!(
                    "[analysis] channel={} samples={} freq={:?} duty={:?} rms={:?} triggers={:?}",
                    frame.channel_id,
                    frame.sample_count,
                    frame.frequency_hz,
                    frame.duty_cycle,
                    frame.rms_value,
                    frame.trigger_hits
                );
            }
            MessageKind::Trigger(trigger) => {
                println!(
                    "[trigger] channel={} rule={} severity={:?} reason={}",
                    trigger.channel_id, trigger.rule_id, trigger.severity, trigger.reason
                );
            }
            MessageKind::TelemetryIdentity(identity) => {
                println!(
                    "[telemetry-identity] device={} board={} firmware={} protocol={} transport={}",
                    identity.device_name,
                    identity.board_name,
                    identity.firmware_version,
                    identity.protocol_version,
                    identity.transport_name
                );
            }
            MessageKind::TelemetrySchema(schema) => {
                println!(
                    "[telemetry-schema] rate_hz={} sample_len={} fields={}",
                    schema.rate_hz,
                    schema.sample_len,
                    schema.fields.len()
                );
            }
            MessageKind::TelemetrySample(sample) => {
                println!("[telemetry-sample] fields={}", sample.fields.len());
            }
            MessageKind::MavlinkPacket(packet) => {
                println!(
                    "[mavlink] msg_id=0x{:04X} sys={} comp={} seq={} payload_len={}",
                    packet.message_id,
                    packet.system_id,
                    packet.component_id,
                    packet.sequence,
                    packet.payload.len()
                );
            }
            MessageKind::CrtpPacket(packet) => {
                println!(
                    "[crtp] port={} channel={} payload_len={}",
                    packet.port.label(),
                    packet.channel,
                    packet.payload.len()
                );
            }
            MessageKind::Capability(event) => {
                println!("[capability] {:?}", event);
            }
            MessageKind::SelfDescribingIdentity(identity) => {
                println!(
                    "[self-describing-identity] device={} firmware={} vars={} cmds={}",
                    identity.device_name,
                    identity.firmware_version,
                    identity.variable_count,
                    identity.command_count
                );
            }
            MessageKind::SelfDescribingVariableCatalog(catalog) => {
                println!(
                    "[self-describing-variable-catalog] page={}/{} vars={}",
                    catalog.page + 1,
                    catalog.total_pages,
                    catalog.variables.len()
                );
            }
            MessageKind::SelfDescribingCommandCatalog(catalog) => {
                println!(
                    "[self-describing-command-catalog] page={}/{} cmds={}",
                    catalog.page + 1,
                    catalog.total_pages,
                    catalog.commands.len()
                );
            }
            MessageKind::SelfDescribingSample(sample) => {
                println!(
                    "[self-describing-sample] seq={} bitmap_len={} values_len={}",
                    sample.seq,
                    sample.changed_bitmap.len(),
                    sample.values.len()
                );
            }
            MessageKind::SelfDescribingSetVariable(set_var) => {
                println!(
                    "[self-describing-set-variable] seq={} var_idx={} value_len={}",
                    set_var.seq,
                    set_var.variable_index,
                    set_var.value.len()
                );
            }
            MessageKind::SelfDescribingAckResult(result) => {
                println!(
                    "[self-describing-ack-result] seq={} code={} msg={}",
                    result.seq, result.code, result.message
                );
            }
            MessageKind::ProtocolDetected(event) => {
                println!("[protocol] detected: {}", event.protocol);
            }
        }
    }
}
