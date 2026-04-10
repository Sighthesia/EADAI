use crate::error::AppError;
use std::time::Duration;

pub const DEFAULT_BAUD_RATE: u32 = 115_200;
pub const DEFAULT_RETRY_DELAY_MS: u64 = 1_000;
pub const DEFAULT_READ_TIMEOUT_MS: u64 = 50;
pub const DEFAULT_LOOPBACK_TIMEOUT_MS: u64 = 1_000;

/// CLI commands supported by the MVP binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Ports,
    Run(RunConfig),
    Send(SendConfig),
    LoopbackTest(LoopbackConfig),
}

/// Runtime configuration for the serial reader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunConfig {
    pub port: String,
    pub baud_rate: u32,
    pub retry_delay: Duration,
    pub read_timeout: Duration,
}

/// Runtime configuration for one-shot serial writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SendConfig {
    pub port: String,
    pub baud_rate: u32,
    pub read_timeout: Duration,
    pub payload: String,
    pub append_newline: bool,
}

/// Runtime configuration for TX/RX loopback verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopbackConfig {
    pub send: SendConfig,
    pub loopback_timeout: Duration,
}

/// Parses CLI arguments into a command.
///
/// - `args`: raw process arguments without the executable name.
pub fn parse_args<I>(args: I) -> Result<Command, AppError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Err(AppError::Usage(usage()));
    };

    match command.as_str() {
        "ports" => Ok(Command::Ports),
        "run" => parse_run_args(args),
        "send" => parse_send_args(args),
        "loopback-test" => parse_loopback_args(args),
        _ => Err(AppError::Usage(usage())),
    }
}

/// Returns CLI usage text.
pub fn usage() -> String {
    format!(
        "Usage:\n  eadai ports\n  eadai run --port <name> [--baud <rate>] [--retry-ms <ms>] [--read-timeout-ms <ms>]\n  eadai send --port <name> --payload <text> [--baud <rate>] [--read-timeout-ms <ms>] [--no-newline]\n  eadai loopback-test --port <name> --payload <text> [--baud <rate>] [--read-timeout-ms <ms>] [--loopback-timeout-ms <ms>] [--no-newline]\n\nDefaults:\n  baud = {DEFAULT_BAUD_RATE}\n  retry-ms = {DEFAULT_RETRY_DELAY_MS}\n  read-timeout-ms = {DEFAULT_READ_TIMEOUT_MS}\n  loopback-timeout-ms = {DEFAULT_LOOPBACK_TIMEOUT_MS}\n"
    )
}

fn parse_run_args<I>(args: I) -> Result<Command, AppError>
where
    I: IntoIterator<Item = String>,
{
    let mut port = None;
    let mut baud_rate = DEFAULT_BAUD_RATE;
    let mut retry_ms = DEFAULT_RETRY_DELAY_MS;
    let mut read_timeout_ms = DEFAULT_READ_TIMEOUT_MS;
    let collected: Vec<String> = args.into_iter().collect();
    let mut index = 0;

    while index < collected.len() {
        match collected[index].as_str() {
            "--port" => {
                port = Some(next_value(&collected, &mut index, "--port")?);
            }
            "--baud" => {
                baud_rate = parse_number(&next_value(&collected, &mut index, "--baud")?, "--baud")?;
            }
            "--retry-ms" => {
                retry_ms = parse_number(
                    &next_value(&collected, &mut index, "--retry-ms")?,
                    "--retry-ms",
                )?;
            }
            "--read-timeout-ms" => {
                read_timeout_ms = parse_number(
                    &next_value(&collected, &mut index, "--read-timeout-ms")?,
                    "--read-timeout-ms",
                )?;
            }
            _ => {
                return Err(AppError::Usage(format!(
                    "Unknown flag: {}\n\n{}",
                    collected[index],
                    usage()
                )));
            }
        }

        index += 1;
    }

    let Some(port) = port else {
        return Err(AppError::Usage(format!(
            "Missing required flag: --port\n\n{}",
            usage()
        )));
    };

    Ok(Command::Run(RunConfig {
        port,
        baud_rate,
        retry_delay: Duration::from_millis(retry_ms),
        read_timeout: Duration::from_millis(read_timeout_ms),
    }))
}

fn parse_send_args<I>(args: I) -> Result<Command, AppError>
where
    I: IntoIterator<Item = String>,
{
    let collected: Vec<String> = args.into_iter().collect();
    let send = parse_send_config(&collected)?;
    Ok(Command::Send(send))
}

fn parse_loopback_args<I>(args: I) -> Result<Command, AppError>
where
    I: IntoIterator<Item = String>,
{
    let collected: Vec<String> = args.into_iter().collect();
    let send = parse_send_config(&collected)?;
    let mut loopback_timeout_ms = DEFAULT_LOOPBACK_TIMEOUT_MS;
    let mut index = 0;

    while index < collected.len() {
        if collected[index].as_str() == "--loopback-timeout-ms" {
            loopback_timeout_ms = parse_number(
                &next_value(&collected, &mut index, "--loopback-timeout-ms")?,
                "--loopback-timeout-ms",
            )?;
        }

        index += 1;
    }

    Ok(Command::LoopbackTest(LoopbackConfig {
        send,
        loopback_timeout: Duration::from_millis(loopback_timeout_ms),
    }))
}

fn parse_send_config(collected: &[String]) -> Result<SendConfig, AppError> {
    let mut port = None;
    let mut payload = None;
    let mut baud_rate = DEFAULT_BAUD_RATE;
    let mut read_timeout_ms = DEFAULT_READ_TIMEOUT_MS;
    let mut append_newline = true;
    let mut index = 0;

    while index < collected.len() {
        match collected[index].as_str() {
            "--port" => {
                port = Some(next_value(collected, &mut index, "--port")?);
            }
            "--payload" => {
                payload = Some(next_value(collected, &mut index, "--payload")?);
            }
            "--baud" => {
                baud_rate = parse_number(&next_value(collected, &mut index, "--baud")?, "--baud")?;
            }
            "--read-timeout-ms" => {
                read_timeout_ms = parse_number(
                    &next_value(collected, &mut index, "--read-timeout-ms")?,
                    "--read-timeout-ms",
                )?;
            }
            "--loopback-timeout-ms" => {
                let _ = next_value(collected, &mut index, "--loopback-timeout-ms")?;
            }
            "--no-newline" => {
                append_newline = false;
            }
            _ => {
                return Err(AppError::Usage(format!(
                    "Unknown flag: {}\n\n{}",
                    collected[index],
                    usage()
                )));
            }
        }

        index += 1;
    }

    let Some(port) = port else {
        return Err(AppError::Usage(format!(
            "Missing required flag: --port\n\n{}",
            usage()
        )));
    };
    let Some(payload) = payload else {
        return Err(AppError::Usage(format!(
            "Missing required flag: --payload\n\n{}",
            usage()
        )));
    };

    Ok(SendConfig {
        port,
        baud_rate,
        read_timeout: Duration::from_millis(read_timeout_ms),
        payload,
        append_newline,
    })
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
