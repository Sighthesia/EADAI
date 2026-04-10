use eadai::app::App;
use eadai::bus::{BusSubscription, MessageBus};
use eadai::cli::{Command, InteractiveConfig, LoopbackConfig, SendConfig, parse_args};
use eadai::error::AppError;
use eadai::message::MessageKind;
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
                    "[rx] port={} bytes={} text={}",
                    port_name,
                    line.raw.len(),
                    line.text
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
            MessageKind::Line(payload) => {
                let parser_name = message
                    .parser
                    .parser_name
                    .unwrap_or_else(|| "raw".to_string());
                println!(
                    "[line] port={} bytes={} parser={} fields={:?} text={}",
                    message.source.port,
                    payload.raw.len(),
                    parser_name,
                    message.parser.fields,
                    payload.text
                );
            }
        }
    }
}
