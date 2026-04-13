use crate::analysis::{AnalysisFrame, TriggerEvent, TriggerSeverity};
use crate::message::{ConnectionEvent, LineDirection, MessageSource, ParserMeta};
use serde::{Deserialize, Serialize};

// FIXME: Make adapter history capacities configurable from operator-facing config when larger windows are needed.
pub const DEFAULT_RECENT_EVENTS_LIMIT: usize = 64;
// FIXME: Make adapter history capacities configurable from operator-facing config when larger windows are needed.
pub const DEFAULT_RECENT_TRIGGERS_LIMIT: usize = 24;
// FIXME: Make adapter history capacities configurable from operator-facing config when larger windows are needed.
pub const DEFAULT_ANALYSIS_HISTORY_LIMIT: usize = 48;
// FIXME: Make channel trigger context depth configurable once AI workflows need longer retrospectives.
pub const DEFAULT_CHANNEL_TRIGGER_CONTEXT_LIMIT: usize = 8;
// FIXME: Make channel statistics window configurable once AI workflows need more dynamic defaults.
pub const DEFAULT_CHANNEL_STATISTICS_WINDOW_MS: u64 = 1_000;

/// Bounded cache sizes for the AI-facing adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AiAdapterLimits {
    pub recent_events: usize,
    pub recent_triggers: usize,
    pub analysis_history: usize,
}

impl Default for AiAdapterLimits {
    fn default() -> Self {
        Self {
            recent_events: DEFAULT_RECENT_EVENTS_LIMIT,
            recent_triggers: DEFAULT_RECENT_TRIGGERS_LIMIT,
            analysis_history: DEFAULT_ANALYSIS_HISTORY_LIMIT,
        }
    }
}

/// One bounded raw sample mirrored from the backend bus.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AiSamplePoint {
    pub timestamp_ms: u64,
    pub value: f64,
}

/// Current runtime session state exported to AI clients.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct AiSessionSnapshot {
    pub is_running: bool,
    pub source: Option<MessageSource>,
    pub connection: Option<ConnectionEvent>,
    pub last_event_at_ms: Option<u64>,
}

/// Stable telemetry summary for one observed channel.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TelemetryChannelSummary {
    pub channel_id: String,
    pub current_value: Option<String>,
    pub numeric_value: Option<f64>,
    pub parser_name: Option<String>,
    pub updated_at_ms: u64,
    pub has_analysis: bool,
    pub trigger_count: usize,
    pub latest_trigger_severity: Option<TriggerSeverity>,
    pub latest_trigger_reason: Option<String>,
}

/// Read-only line event shape for recent event inspection.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AiLineEventRecord {
    pub direction: LineDirection,
    pub text: String,
    pub raw_length: usize,
    pub parser: ParserMeta,
}

/// Supported recent event categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiRecentEventKind {
    Connection,
    Line,
    Analysis,
    Trigger,
}

/// One event mirrored from the backend bus for AI diagnostics.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AiRecentEvent {
    pub timestamp_ms: u64,
    pub source: MessageSource,
    pub kind: AiRecentEventKind,
    pub connection: Option<ConnectionEvent>,
    pub line: Option<AiLineEventRecord>,
    pub analysis: Option<AnalysisFrame>,
    pub trigger: Option<TriggerEvent>,
}

impl AiRecentEvent {
    pub fn channel_id(&self) -> Option<&str> {
        if let Some(analysis) = &self.analysis {
            return Some(&analysis.channel_id);
        }

        if let Some(trigger) = &self.trigger {
            return Some(&trigger.channel_id);
        }

        None
    }
}

/// Resource payload for recent telemetry summaries.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TelemetrySummaryResource {
    pub channels: Vec<TelemetryChannelSummary>,
}

/// Resource payload for latest analysis frames.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AnalysisFramesResource {
    pub frames: Vec<AnalysisFrame>,
}

/// Resource payload for historical analysis frames.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HistoricalAnalysisResource {
    pub channel_id: String,
    pub frames: Vec<AnalysisFrame>,
}

/// Resource payload for sampled channel statistics.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChannelStatisticsResource {
    pub channel_id: String,
    pub window_ms: u64,
    pub sample_count: usize,
    pub time_span_ms: Option<f64>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub mean_value: Option<f64>,
    pub rms_value: Option<f64>,
    pub variance: Option<f64>,
    pub trend: Option<f64>,
    pub change_rate: Option<f64>,
    pub frequency_hz: Option<f64>,
    pub period_ms: Option<f64>,
    pub duty_cycle: Option<f64>,
    pub period_stability: Option<f64>,
    pub raw_samples: Option<Vec<AiSamplePoint>>,
}

/// Resource payload for recent triggers.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TriggerHistoryResource {
    pub triggers: Vec<TriggerEvent>,
}

/// Tool payload for one channel-specific query.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChannelAnalysisResource {
    pub channel_id: String,
    pub telemetry: Option<TelemetryChannelSummary>,
    pub analysis: Option<AnalysisFrame>,
    pub recent_triggers: Vec<TriggerEvent>,
}

/// Tool payload for recent event lookups.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RecentEventsResource {
    pub events: Vec<AiRecentEvent>,
}

/// Query parameters for per-channel analysis requests.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChannelAnalysisQuery {
    pub channel_id: String,
    #[serde(default)]
    pub include_trigger_context: bool,
}

/// Query parameters for sampled statistics requests.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChannelStatisticsQuery {
    pub channel_id: String,
    #[serde(default)]
    pub window_ms: Option<u64>,
    #[serde(default)]
    pub include_raw_samples: bool,
}

/// Query parameters for historical analysis lookups.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HistoricalAnalysisQuery {
    pub channel_id: String,
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    #[serde(default)]
    pub max_frames: Option<usize>,
}

/// Query parameters for recent event requests.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecentEventsQuery {
    pub limit: Option<usize>,
    pub kind: Option<AiRecentEventKind>,
    pub channel_id: Option<String>,
}
