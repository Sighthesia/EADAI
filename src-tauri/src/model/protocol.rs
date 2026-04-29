use eadai::analysis::{AnalysisFrame, TriggerEvent, TriggerSeverity};
use eadai::bmi088::Bmi088HostCommand;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bmi088CommandRequest {
    pub command: UiBmi088HostCommand,
    #[serde(default)]
    pub payload: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UiBmi088HostCommand {
    Ack,
    Start,
    Stop,
    ReqSchema,
    ReqIdentity,
    ReqTuning,
    SetTuning,
    ShellExec,
}

impl UiBmi088HostCommand {
    pub fn payload_bytes(&self, payload: Option<&str>) -> Option<Vec<u8>> {
        match self {
            Self::ShellExec | Self::SetTuning => payload.map(|value| value.as_bytes().to_vec()),
            _ => None,
        }
    }
}

impl From<UiBmi088HostCommand> for Bmi088HostCommand {
    fn from(value: UiBmi088HostCommand) -> Self {
        match value {
            UiBmi088HostCommand::Ack => Self::Ack,
            UiBmi088HostCommand::Start => Self::Start,
            UiBmi088HostCommand::Stop => Self::Stop,
            UiBmi088HostCommand::ReqSchema => Self::ReqSchema,
            UiBmi088HostCommand::ReqIdentity => Self::ReqIdentity,
            UiBmi088HostCommand::ReqTuning => Self::ReqTuning,
            UiBmi088HostCommand::SetTuning => Self::SetTuning,
            UiBmi088HostCommand::ShellExec => Self::ShellExec,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UiTriggerSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAnalysisPayload {
    pub channel_id: String,
    pub window_ms: u64,
    pub sample_count: usize,
    pub time_span_ms: Option<f64>,
    pub frequency_hz: Option<f64>,
    pub period_ms: Option<f64>,
    pub period_stability: Option<f64>,
    pub duty_cycle: Option<f64>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub mean_value: Option<f64>,
    pub median_value: Option<f64>,
    pub rms_value: Option<f64>,
    pub variance: Option<f64>,
    pub edge_count: usize,
    pub rising_edge_count: usize,
    pub falling_edge_count: usize,
    pub trend: Option<f64>,
    pub change_rate: Option<f64>,
    pub trigger_hits: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiTriggerPayload {
    pub channel_id: String,
    pub rule_id: String,
    pub severity: UiTriggerSeverity,
    pub fired_at_ms: u64,
    pub reason: String,
    pub snapshot: Option<UiAnalysisPayload>,
}

impl From<TriggerSeverity> for UiTriggerSeverity {
    fn from(value: TriggerSeverity) -> Self {
        match value {
            TriggerSeverity::Info => Self::Info,
            TriggerSeverity::Warning => Self::Warning,
            TriggerSeverity::Critical => Self::Critical,
        }
    }
}

impl From<AnalysisFrame> for UiAnalysisPayload {
    fn from(value: AnalysisFrame) -> Self {
        Self {
            channel_id: value.channel_id,
            window_ms: value.window_ms,
            sample_count: value.sample_count,
            time_span_ms: value.time_span_ms,
            frequency_hz: value.frequency_hz,
            period_ms: value.period_ms,
            period_stability: value.period_stability,
            duty_cycle: value.duty_cycle,
            min_value: value.min_value,
            max_value: value.max_value,
            mean_value: value.mean_value,
            median_value: value.median_value,
            rms_value: value.rms_value,
            variance: value.variance,
            edge_count: value.edge_count,
            rising_edge_count: value.rising_edge_count,
            falling_edge_count: value.falling_edge_count,
            trend: value.trend,
            change_rate: value.change_rate,
            trigger_hits: value.trigger_hits,
        }
    }
}

impl From<TriggerEvent> for UiTriggerPayload {
    fn from(value: TriggerEvent) -> Self {
        Self {
            channel_id: value.channel_id,
            rule_id: value.rule_id,
            severity: value.severity.into(),
            fired_at_ms: value.fired_at_ms,
            reason: value.reason,
            snapshot: value.snapshot.map(Into::into),
        }
    }
}
