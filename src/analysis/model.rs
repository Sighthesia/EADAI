use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AnalysisFrame {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TriggerSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TriggerEvent {
    pub channel_id: String,
    pub rule_id: String,
    pub severity: TriggerSeverity,
    pub fired_at_ms: u64,
    pub reason: String,
    pub snapshot: Option<AnalysisFrame>,
}
