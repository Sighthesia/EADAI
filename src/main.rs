use eadai::app::App;
use eadai::bus::{BusSubscription, MessageBus};
use eadai::cli::{Command, LoopbackConfig, SendConfig, parse_args};
use eadai::error::AppError;
use eadai::message::MessageKind;
use eadai::serial::{self, LineFramer};

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
