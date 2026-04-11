use eadai::ai_adapter::AiContextAdapter;
use eadai::app::{App, StopSignal};
use eadai::bus::MessageBus;
use eadai::cli::{
    DEFAULT_BAUD_RATE, DEFAULT_READ_TIMEOUT_MS, DEFAULT_RETRY_DELAY_MS, ParserKind, RunConfig,
};
use eadai::error::AppError;
use eadai::fake_session::{self, FakeSessionHandle};
use eadai::mcp_server::TelemetryMcpServer;
use rmcp::{ServiceExt, transport::stdio};
use std::error::Error;
use std::thread::{self, JoinHandle};
use std::time::Duration;

fn main() {
    if let Err(error) = run_main() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run_main() -> Result<(), Box<dyn Error>> {
    let runtime = parse_args(std::env::args().skip(1))?;
    let tokio_runtime = tokio::runtime::Runtime::new()?;
    tokio_runtime.block_on(run(runtime))
}

async fn run(runtime: McpRuntimeConfig) -> Result<(), Box<dyn Error>> {
    let bus = MessageBus::new();
    let adapter = AiContextAdapter::default();
    let adapter_worker = adapter.spawn(bus.subscribe());
    let session = spawn_runtime(runtime, bus.clone());
    let server = TelemetryMcpServer::new(adapter);
    let service = match server.serve(stdio()).await {
        Ok(service) => service,
        Err(error) => {
            session.stop();
            drop(bus);
            let _ = adapter_worker.join();
            return Err(Box::new(error));
        }
    };

    let result = service.waiting().await;
    session.stop();
    drop(bus);
    let _ = adapter_worker.join();
    result
        .map(|_| ())
        .map_err(|error| Box::new(error) as Box<dyn Error>)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum McpRuntimeConfig {
    Serial(RunConfig),
    Fake { profile: String, baud_rate: u32 },
}

struct RuntimeSession {
    control: RuntimeControl,
    worker: JoinHandle<()>,
}

enum RuntimeControl {
    Real(StopSignal),
    Fake(FakeSessionHandle),
}

impl RuntimeSession {
    fn stop(self) {
        match self.control {
            RuntimeControl::Real(signal) => signal.request_stop(),
            RuntimeControl::Fake(handle) => handle.request_stop(),
        }
        let _ = self.worker.join();
    }
}

fn spawn_runtime(config: McpRuntimeConfig, bus: MessageBus) -> RuntimeSession {
    match config {
        McpRuntimeConfig::Serial(config) => {
            let app = App::new(config, bus);
            let stop_signal = app.stop_signal();
            let worker = thread::spawn(move || {
                let _ = app.run();
            });

            RuntimeSession {
                control: RuntimeControl::Real(stop_signal),
                worker,
            }
        }
        McpRuntimeConfig::Fake { profile, baud_rate } => {
            let port = fake_session::fake_port_label(&profile);
            let (handle, worker) = fake_session::spawn(
                fake_session::FakeSessionConfig {
                    port,
                    baud_rate,
                    profile,
                },
                bus,
            );

            RuntimeSession {
                control: RuntimeControl::Fake(handle),
                worker,
            }
        }
    }
}

fn parse_args<I>(args: I) -> Result<McpRuntimeConfig, AppError>
where
    I: IntoIterator<Item = String>,
{
    let collected: Vec<String> = args.into_iter().collect();
    let mut port = None;
    let mut fake_profile = None;
    let mut baud_rate = DEFAULT_BAUD_RATE;
    let mut retry_ms = DEFAULT_RETRY_DELAY_MS;
    let mut read_timeout_ms = DEFAULT_READ_TIMEOUT_MS;
    let mut index = 0;

    while index < collected.len() {
        match collected[index].as_str() {
            "--port" => port = Some(next_value(&collected, &mut index, "--port")?),
            "--fake-profile" => {
                fake_profile = Some(next_value(&collected, &mut index, "--fake-profile")?)
            }
            "--baud" => {
                baud_rate = parse_number(&next_value(&collected, &mut index, "--baud")?, "--baud")?
            }
            "--retry-ms" => {
                retry_ms = parse_number(
                    &next_value(&collected, &mut index, "--retry-ms")?,
                    "--retry-ms",
                )?
            }
            "--read-timeout-ms" => {
                read_timeout_ms = parse_number(
                    &next_value(&collected, &mut index, "--read-timeout-ms")?,
                    "--read-timeout-ms",
                )?
            }
            _ => return Err(AppError::Usage(usage())),
        }

        index += 1;
    }

    if port.is_some() == fake_profile.is_some() {
        return Err(AppError::Usage(usage()));
    }

    if let Some(port) = port {
        return Ok(McpRuntimeConfig::Serial(RunConfig {
            port,
            baud_rate,
            retry_delay: Duration::from_millis(retry_ms),
            read_timeout: Duration::from_millis(read_timeout_ms),
            parser: ParserKind::Auto,
            max_frame_bytes: eadai::cli::DEFAULT_MAX_FRAME_BYTES,
        }));
    }

    Ok(McpRuntimeConfig::Fake {
        profile: fake_profile.unwrap_or_else(|| fake_session::default_profile().to_string()),
        baud_rate,
    })
}

fn usage() -> String {
    format!(
        "Usage:\n  eadai-mcp --port <name> [--baud <rate>] [--retry-ms <ms>] [--read-timeout-ms <ms>]\n  eadai-mcp --fake-profile <name> [--baud <rate>]\n\nRuns a read-only MCP stdio server backed by the serial runtime or fake telemetry stream."
    )
}

fn next_value(values: &[String], index: &mut usize, flag: &str) -> Result<String, AppError> {
    *index += 1;

    values
        .get(*index)
        .cloned()
        .ok_or_else(|| AppError::Usage(format!("Missing value for {flag}\n\n{}", usage())))
}

fn parse_number<T>(value: &str, flag: &str) -> Result<T, AppError>
where
    T: std::str::FromStr,
{
    value.parse::<T>().map_err(|_| {
        AppError::Usage(format!(
            "Invalid numeric value for {flag}: {value}\n\n{}",
            usage()
        ))
    })
}
