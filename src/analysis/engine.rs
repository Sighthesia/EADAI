use super::model::{AnalysisFrame, TriggerEvent, TriggerSeverity};
use crate::message::{BusMessage, LineDirection, MessageSource, ParserMeta};
use std::collections::{BTreeMap, VecDeque};

const DEFAULT_WINDOW_MS: u64 = 2_000;
const MIN_SIGNAL_RANGE: f64 = 0.2;
const MIN_EDGE_SPAN_MS: f64 = 1.0;

#[derive(Clone, Debug, Default)]
pub struct AnalysisEngine {
    window_ms: u64,
    rules: Vec<TriggerRule>,
    channels: BTreeMap<String, ChannelState>,
}

#[derive(Clone, Debug, Default)]
struct ChannelState {
    samples: VecDeque<SamplePoint>,
    active_rules: BTreeMap<String, bool>,
}

#[derive(Clone, Copy, Debug)]
struct SamplePoint {
    timestamp_ms: u64,
    value: f64,
}

#[derive(Clone, Debug)]
struct TriggerRule {
    id: &'static str,
    channel_id: &'static str,
    condition: TriggerCondition,
    severity: TriggerSeverity,
}

#[derive(Clone, Debug)]
enum TriggerCondition {
    Threshold {
        metric: MetricKind,
        comparison: ThresholdComparison,
        value: f64,
    },
    Range {
        metric: MetricKind,
        min: f64,
        max: f64,
        mode: RangeMode,
    },
    EdgeCount {
        min_edges: usize,
    },
}

#[derive(Clone, Copy, Debug)]
enum MetricKind {
    DutyCycle,
    MeanValue,
    RmsValue,
}

#[derive(Clone, Copy, Debug)]
enum ThresholdComparison {
    Above,
    Below,
}

#[derive(Clone, Copy, Debug)]
enum RangeMode {
    Outside,
}

struct EdgeMetrics {
    frequency_hz: Option<f64>,
    period_ms: Option<f64>,
    duty_cycle: Option<f64>,
    edge_count: usize,
    rising_edges: usize,
    falling_edges: usize,
}

impl AnalysisEngine {
    pub fn new() -> Self {
        Self::with_window_ms(DEFAULT_WINDOW_MS)
    }

    pub fn with_window_ms(window_ms: u64) -> Self {
        Self {
            window_ms,
            rules: default_rules(),
            channels: BTreeMap::new(),
        }
    }

    pub fn ingest_line(
        &mut self,
        source: &MessageSource,
        direction: &LineDirection,
        parser: &ParserMeta,
        timestamp_ms: u64,
    ) -> Option<Vec<BusMessage>> {
        if direction != &LineDirection::Rx {
            return None;
        }

        let channel_id = parser.fields.get("channel_id")?.clone();
        let value = parser
            .fields
            .get("numeric_value")
            .or_else(|| parser.fields.get("value"))?
            .parse::<f64>()
            .ok()?;
        let timestamp_ms = parser
            .fields
            .get("timestamp")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(timestamp_ms);

        let channel = self.channels.entry(channel_id.clone()).or_default();
        channel.samples.push_back(SamplePoint {
            timestamp_ms,
            value,
        });
        prune_old_samples(&mut channel.samples, self.window_ms, timestamp_ms);

        let mut frame = compute_frame(&channel_id, self.window_ms, &channel.samples)?;
        let triggers = evaluate_rules(&self.rules, &mut channel.active_rules, &frame, timestamp_ms);
        frame.trigger_hits = triggers
            .iter()
            .map(|trigger| trigger.rule_id.clone())
            .collect();

        let mut messages = vec![BusMessage::analysis(source, frame.clone())];
        messages.extend(
            triggers
                .into_iter()
                .map(|trigger| BusMessage::trigger(source, trigger)),
        );
        Some(messages)
    }
}

fn prune_old_samples(
    samples: &mut VecDeque<SamplePoint>,
    window_ms: u64,
    newest_timestamp_ms: u64,
) {
    let min_timestamp = newest_timestamp_ms.saturating_sub(window_ms);
    while samples.len() > 1 {
        let Some(sample) = samples.front() else {
            break;
        };
        if sample.timestamp_ms >= min_timestamp {
            break;
        }
        samples.pop_front();
    }
}

fn compute_frame(
    channel_id: &str,
    window_ms: u64,
    samples: &VecDeque<SamplePoint>,
) -> Option<AnalysisFrame> {
    let first = *samples.front()?;
    let last = *samples.back()?;
    let values = samples
        .iter()
        .map(|sample| sample.value)
        .collect::<Vec<_>>();
    let sample_count = values.len();
    let min_value = values.iter().copied().reduce(f64::min);
    let max_value = values.iter().copied().reduce(f64::max);
    let mean_value = Some(values.iter().sum::<f64>() / sample_count as f64);
    let rms_value =
        Some((values.iter().map(|value| value * value).sum::<f64>() / sample_count as f64).sqrt());
    let trend = (sample_count >= 2).then_some(last.value - first.value);
    let span_ms = last.timestamp_ms.saturating_sub(first.timestamp_ms) as f64;
    let change_rate = if span_ms >= MIN_EDGE_SPAN_MS {
        trend.map(|delta| delta / (span_ms / 1000.0))
    } else {
        None
    };

    let edge_metrics = compute_edge_metrics(samples, min_value?, max_value?);
    Some(AnalysisFrame {
        channel_id: channel_id.to_string(),
        window_ms,
        sample_count,
        frequency_hz: edge_metrics.frequency_hz,
        period_ms: edge_metrics.period_ms,
        duty_cycle: edge_metrics.duty_cycle,
        min_value,
        max_value,
        mean_value,
        rms_value,
        edge_count: edge_metrics.edge_count,
        rising_edge_count: edge_metrics.rising_edges,
        falling_edge_count: edge_metrics.falling_edges,
        trend,
        change_rate,
        trigger_hits: Vec::new(),
    })
}

fn compute_edge_metrics(
    samples: &VecDeque<SamplePoint>,
    min_value: f64,
    max_value: f64,
) -> EdgeMetrics {
    let range = max_value - min_value;
    if samples.len() < 2 || range < MIN_SIGNAL_RANGE {
        return EdgeMetrics {
            frequency_hz: None,
            period_ms: None,
            duty_cycle: None,
            edge_count: 0,
            rising_edges: 0,
            falling_edges: 0,
        };
    }

    let midpoint = min_value + (range / 2.0);
    let hysteresis = (range * 0.1).max(0.02);
    let upper = midpoint + hysteresis;
    let lower = midpoint - hysteresis;
    let ordered = samples.iter().copied().collect::<Vec<_>>();
    let mut state = ordered
        .first()
        .map(|sample| sample.value >= midpoint)
        .unwrap_or(false);
    let mut high_ms = 0.0;
    let mut rising_times = Vec::new();
    let mut falling_times = Vec::new();

    for pair in ordered.windows(2) {
        let current = pair[0];
        let next = pair[1];
        let next_state = if next.value >= upper {
            true
        } else if next.value <= lower {
            false
        } else {
            state
        };

        let delta_ms = next.timestamp_ms.saturating_sub(current.timestamp_ms) as f64;
        if state {
            high_ms += delta_ms;
        }

        if next_state != state {
            if next_state {
                rising_times.push(next.timestamp_ms as f64);
            } else {
                falling_times.push(next.timestamp_ms as f64);
            }
            state = next_state;
        }
    }

    let span_ms = ordered
        .last()
        .zip(ordered.first())
        .map(|(last, first)| last.timestamp_ms.saturating_sub(first.timestamp_ms) as f64)
        .unwrap_or_default();
    let period_ms = average_period(&rising_times).or_else(|| average_period(&falling_times));
    let frequency_hz =
        period_ms.and_then(|period| (period >= MIN_EDGE_SPAN_MS).then_some(1000.0 / period));
    let duty_cycle = (span_ms >= MIN_EDGE_SPAN_MS).then_some((high_ms / span_ms) * 100.0);

    EdgeMetrics {
        frequency_hz,
        period_ms,
        duty_cycle,
        edge_count: rising_times.len() + falling_times.len(),
        rising_edges: rising_times.len(),
        falling_edges: falling_times.len(),
    }
}

fn average_period(edges: &[f64]) -> Option<f64> {
    if edges.len() < 2 {
        return None;
    }

    let deltas = edges
        .windows(2)
        .map(|window| window[1] - window[0])
        .collect::<Vec<_>>();
    Some(deltas.iter().sum::<f64>() / deltas.len() as f64)
}

fn evaluate_rules(
    rules: &[TriggerRule],
    active_rules: &mut BTreeMap<String, bool>,
    frame: &AnalysisFrame,
    fired_at_ms: u64,
) -> Vec<TriggerEvent> {
    let mut triggers = Vec::new();

    for rule in rules
        .iter()
        .filter(|rule| rule.channel_id == frame.channel_id)
    {
        let (is_match, reason) = match_rule(rule, frame);
        let was_active = active_rules.get(rule.id).copied().unwrap_or(false);
        active_rules.insert(rule.id.to_string(), is_match);

        if is_match && !was_active {
            triggers.push(TriggerEvent {
                channel_id: frame.channel_id.clone(),
                rule_id: rule.id.to_string(),
                severity: rule.severity,
                fired_at_ms,
                reason,
                snapshot: Some(frame.clone()),
            });
        }
    }

    triggers
}

fn match_rule(rule: &TriggerRule, frame: &AnalysisFrame) -> (bool, String) {
    match &rule.condition {
        TriggerCondition::Threshold {
            metric,
            comparison,
            value,
        } => {
            let Some(actual) = metric_value(frame, *metric) else {
                return (false, format!("{} missing", metric_label(*metric)));
            };
            let is_match = match comparison {
                ThresholdComparison::Above => actual > *value,
                ThresholdComparison::Below => actual < *value,
            };
            (
                is_match,
                format!("{} {:.3} vs {:.3}", metric_label(*metric), actual, value),
            )
        }
        TriggerCondition::Range {
            metric,
            min,
            max,
            mode,
        } => {
            let Some(actual) = metric_value(frame, *metric) else {
                return (false, format!("{} missing", metric_label(*metric)));
            };
            let inside = actual >= *min && actual <= *max;
            let is_match = !inside;
            let label = match mode {
                RangeMode::Outside => "outside",
            };
            (
                is_match,
                format!(
                    "{} {:.3} {} [{:.3}, {:.3}]",
                    metric_label(*metric),
                    actual,
                    label,
                    min,
                    max
                ),
            )
        }
        TriggerCondition::EdgeCount { min_edges } => (
            frame.edge_count >= *min_edges,
            format!("edgeCount {} vs {}", frame.edge_count, min_edges),
        ),
    }
}

fn metric_value(frame: &AnalysisFrame, metric: MetricKind) -> Option<f64> {
    match metric {
        MetricKind::DutyCycle => frame.duty_cycle,
        MetricKind::MeanValue => frame.mean_value,
        MetricKind::RmsValue => frame.rms_value,
    }
}

fn metric_label(metric: MetricKind) -> &'static str {
    match metric {
        MetricKind::DutyCycle => "dutyCycle",
        MetricKind::MeanValue => "mean",
        MetricKind::RmsValue => "rms",
    }
}

fn default_rules() -> Vec<TriggerRule> {
    vec![
        TriggerRule {
            id: "voltage-high-mean",
            channel_id: "voltage",
            condition: TriggerCondition::Threshold {
                metric: MetricKind::MeanValue,
                comparison: ThresholdComparison::Above,
                value: 12.3,
            },
            severity: TriggerSeverity::Warning,
        },
        TriggerRule {
            id: "vibration-rms-high",
            channel_id: "vibration_g",
            condition: TriggerCondition::Threshold {
                metric: MetricKind::RmsValue,
                comparison: ThresholdComparison::Above,
                value: 0.48,
            },
            severity: TriggerSeverity::Warning,
        },
        TriggerRule {
            id: "pulse-edge-burst",
            channel_id: "pulse_signal",
            condition: TriggerCondition::EdgeCount { min_edges: 6 },
            severity: TriggerSeverity::Info,
        },
        TriggerRule {
            id: "pwm-duty-outside-band",
            channel_id: "pwm_signal",
            condition: TriggerCondition::Range {
                metric: MetricKind::DutyCycle,
                min: 35.0,
                max: 65.0,
                mode: RangeMode::Outside,
            },
            severity: TriggerSeverity::Critical,
        },
        TriggerRule {
            id: "temp-low-mean",
            channel_id: "temp",
            condition: TriggerCondition::Threshold {
                metric: MetricKind::MeanValue,
                comparison: ThresholdComparison::Below,
                value: 22.5,
            },
            severity: TriggerSeverity::Info,
        },
    ]
}
