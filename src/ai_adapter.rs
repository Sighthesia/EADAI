use crate::ai_contract::{
    AiAdapterLimits, AiLineEventRecord, AiRecentEvent, AiRecentEventKind, AiSamplePoint,
    AiSessionSnapshot, AnalysisFramesResource, ChannelAnalysisResource, ChannelStatisticsQuery,
    ChannelStatisticsResource, HistoricalAnalysisQuery, HistoricalAnalysisResource,
    RecentEventsQuery, RecentEventsResource, TelemetryChannelSummary, TelemetrySummaryResource,
    TriggerHistoryResource, DEFAULT_CHANNEL_TRIGGER_CONTEXT_LIMIT,
};
use crate::analysis::{AnalysisFrame, TriggerEvent};
use crate::bus::BusSubscription;
use crate::message::{BusMessage, ConnectionState, MessageKind};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

/// Read-only adapter that reshapes the live backend bus into AI-facing snapshots.
#[derive(Clone, Debug)]
pub struct AiContextAdapter {
    inner: Arc<Mutex<AiContextState>>,
}

#[derive(Debug)]
struct AiContextState {
    limits: AiAdapterLimits,
    session: AiSessionSnapshot,
    telemetry: BTreeMap<String, TelemetryChannelSummary>,
    analysis: BTreeMap<String, AnalysisFrame>,
    analysis_history: BTreeMap<String, VecDeque<HistoricalAnalysisEntry>>,
    sample_history: BTreeMap<String, VecDeque<AiSamplePoint>>,
    triggers: VecDeque<TriggerEvent>,
    recent_events: VecDeque<AiRecentEvent>,
}

#[derive(Clone, Debug)]
struct HistoricalAnalysisEntry {
    timestamp_ms: u64,
    frame: AnalysisFrame,
}

impl Default for AiContextAdapter {
    fn default() -> Self {
        Self::new(AiAdapterLimits::default())
    }
}

impl AiContextAdapter {
    /// Creates a new adapter with bounded in-memory history.
    pub fn new(limits: AiAdapterLimits) -> Self {
        Self {
            inner: Arc::new(Mutex::new(AiContextState::new(limits))),
        }
    }

    /// Starts consuming a bus subscription on a background thread.
    pub fn spawn(&self, subscription: BusSubscription) -> JoinHandle<()> {
        let adapter = self.clone();
        thread::spawn(move || {
            while let Ok(message) = subscription.recv() {
                adapter.ingest(message);
            }
        })
    }

    /// Applies one backend bus message to the AI-facing snapshot.
    pub fn ingest(&self, message: BusMessage) {
        let mut state = lock_state(&self.inner);
        state.ingest(message);
    }

    /// Clears all bounded snapshots so a new runtime session starts from a clean state.
    pub fn reset(&self) {
        let mut state = lock_state(&self.inner);
        state.reset();
    }

    /// Returns the latest session snapshot.
    pub fn session_snapshot(&self) -> AiSessionSnapshot {
        lock_state(&self.inner).session.clone()
    }

    /// Returns the latest telemetry summaries sorted by channel id.
    pub fn telemetry_summary(&self) -> TelemetrySummaryResource {
        let state = lock_state(&self.inner);
        TelemetrySummaryResource {
            channels: state.telemetry.values().cloned().collect(),
        }
    }

    /// Returns the latest analysis frame per channel.
    pub fn analysis_frames(&self) -> AnalysisFramesResource {
        let state = lock_state(&self.inner);
        AnalysisFramesResource {
            frames: state.analysis.values().cloned().collect(),
        }
    }

    /// Returns sampled statistics for one channel.
    pub fn channel_statistics(
        &self,
        query: &ChannelStatisticsQuery,
    ) -> Option<ChannelStatisticsResource> {
        let state = lock_state(&self.inner);
        let frame = state.analysis.get(&query.channel_id)?.clone();
        let samples = state.sample_history.get(&query.channel_id);
        let window_ms = query
            .window_ms
            .unwrap_or(crate::ai_contract::DEFAULT_CHANNEL_STATISTICS_WINDOW_MS)
            .max(1);
        let Some(samples) = samples else {
            return Some(ChannelStatisticsResource {
                channel_id: query.channel_id.clone(),
                window_ms,
                sample_count: 0,
                time_span_ms: None,
                min_value: None,
                max_value: None,
                mean_value: None,
                median_value: None,
                rms_value: None,
                variance: None,
                trend: None,
                change_rate: None,
                frequency_hz: frame.frequency_hz,
                period_ms: frame.period_ms,
                duty_cycle: frame.duty_cycle,
                period_stability: frame.period_stability,
                raw_samples: None,
            });
        };

        let newest_timestamp = samples.back()?.timestamp_ms;
        let min_timestamp = newest_timestamp.saturating_sub(window_ms);
        let window_samples = samples
            .iter()
            .filter(|sample| sample.timestamp_ms >= min_timestamp)
            .cloned()
            .collect::<Vec<_>>();
        if window_samples.is_empty() {
            return None;
        }

        let sample_count = window_samples.len();
        let values = window_samples
            .iter()
            .map(|sample| sample.value)
            .collect::<Vec<_>>();
        let min_value = values.iter().copied().reduce(f64::min);
        let max_value = values.iter().copied().reduce(f64::max);
        let window_mean_value = Some(values.iter().sum::<f64>() / sample_count as f64);
        let cycle_values = min_value.zip(max_value).and_then(|(min_value, max_value)| {
            latest_cycle_values(&window_samples, min_value, max_value)
        });
        let mean_value = cycle_values
            .as_ref()
            .map(|values| values.iter().sum::<f64>() / values.len() as f64)
            .or(window_mean_value);
        let median_value = cycle_values
            .as_ref()
            .and_then(|values| compute_median(values))
            .or_else(|| compute_median(&values));
        let rms_value = Some(
            (values.iter().map(|value| value * value).sum::<f64>() / sample_count as f64).sqrt(),
        );
        let variance = window_mean_value.and_then(|mean| compute_variance(&values, mean));
        let time_span_ms = window_samples
            .first()
            .zip(window_samples.last())
            .map(|(first, last)| last.timestamp_ms.saturating_sub(first.timestamp_ms) as f64);
        let trend = window_samples
            .first()
            .zip(window_samples.last())
            .map(|(first, last)| last.value - first.value);
        let change_rate = match (trend, time_span_ms) {
            (Some(delta), Some(span_ms)) if span_ms >= 1.0 => Some(delta / (span_ms / 1000.0)),
            _ => None,
        };
        let raw_samples = if query.include_raw_samples {
            Some(window_samples.clone())
        } else {
            None
        };

        Some(ChannelStatisticsResource {
            channel_id: query.channel_id.clone(),
            window_ms,
            sample_count,
            time_span_ms,
            min_value,
            max_value,
            mean_value,
            median_value,
            rms_value,
            variance,
            trend,
            change_rate,
            frequency_hz: frame.frequency_hz,
            period_ms: frame.period_ms,
            duty_cycle: frame.duty_cycle,
            period_stability: frame.period_stability,
            raw_samples,
        })
    }

    /// Returns historical analysis frames for one channel.
    pub fn historical_analysis(
        &self,
        query: &HistoricalAnalysisQuery,
    ) -> Option<HistoricalAnalysisResource> {
        let state = lock_state(&self.inner);
        let history = state.analysis_history.get(&query.channel_id)?;
        let start_time_ms = query.start_time_ms.min(query.end_time_ms);
        let end_time_ms = query.start_time_ms.max(query.end_time_ms);
        let max_frames = query.max_frames.unwrap_or(state.limits.analysis_history);

        let frames = history
            .iter()
            .filter(|entry| {
                entry.timestamp_ms >= start_time_ms && entry.timestamp_ms <= end_time_ms
            })
            .map(|entry| entry.frame.clone())
            .take(max_frames)
            .collect();

        Some(HistoricalAnalysisResource {
            channel_id: query.channel_id.clone(),
            frames,
        })
    }

    /// Returns the bounded trigger history.
    pub fn trigger_history(&self) -> TriggerHistoryResource {
        let state = lock_state(&self.inner);
        TriggerHistoryResource {
            triggers: state.triggers.iter().cloned().collect(),
        }
    }

    /// Returns a channel-focused snapshot for AI diagnostics.
    pub fn channel_analysis(
        &self,
        channel_id: &str,
        include_trigger_context: bool,
    ) -> Option<ChannelAnalysisResource> {
        let state = lock_state(&self.inner);
        let telemetry = state.telemetry.get(channel_id).cloned();
        let analysis = state.analysis.get(channel_id).cloned();
        let recent_triggers = if include_trigger_context {
            state
                .triggers
                .iter()
                .rev()
                .filter(|trigger| trigger.channel_id == channel_id)
                .take(DEFAULT_CHANNEL_TRIGGER_CONTEXT_LIMIT)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect()
        } else {
            Vec::new()
        };

        if telemetry.is_none() && analysis.is_none() && recent_triggers.is_empty() {
            return None;
        }

        Some(ChannelAnalysisResource {
            channel_id: channel_id.to_string(),
            telemetry,
            analysis,
            recent_triggers,
        })
    }

    /// Returns recent events using the requested filter.
    pub fn recent_events(&self, query: &RecentEventsQuery) -> RecentEventsResource {
        let state = lock_state(&self.inner);
        let limit = query
            .limit
            .unwrap_or(state.limits.recent_events)
            .min(state.limits.recent_events);

        let events = state
            .recent_events
            .iter()
            .rev()
            .filter(|event| match query.kind {
                Some(kind) => event.kind == kind,
                None => true,
            })
            .filter(|event| match query.channel_id.as_deref() {
                Some(channel_id) => event.channel_id() == Some(channel_id),
                None => true,
            })
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        RecentEventsResource { events }
    }
}

fn compute_median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let center = sorted.len() / 2;

    if sorted.len() % 2 == 0 {
        Some((sorted[center - 1] + sorted[center]) / 2.0)
    } else {
        Some(sorted[center])
    }
}

fn latest_cycle_values(
    samples: &[AiSamplePoint],
    min_value: f64,
    max_value: f64,
) -> Option<Vec<f64>> {
    let range = max_value - min_value;
    if samples.len() < 2 || !range.is_finite() || range < 0.2 {
        return None;
    }

    let midpoint = min_value + (range / 2.0);
    let hysteresis = (range * 0.1).max(0.02);
    let upper = midpoint + hysteresis;
    let lower = midpoint - hysteresis;
    let mut state = samples
        .first()
        .map(|sample| sample.value >= midpoint)
        .unwrap_or(false);
    let mut rising_times = Vec::new();
    let mut falling_times = Vec::new();

    for sample in samples.iter().skip(1) {
        let next_state = if sample.value >= upper {
            true
        } else if sample.value <= lower {
            false
        } else {
            state
        };

        if next_state != state {
            if next_state {
                rising_times.push(sample.timestamp_ms);
            } else {
                falling_times.push(sample.timestamp_ms);
            }
            state = next_state;
        }
    }

    let cycle_starts = if rising_times.len() >= 2 {
        rising_times
    } else if falling_times.len() >= 2 {
        falling_times
    } else {
        return None;
    };

    let start_ms = cycle_starts[cycle_starts.len() - 2];
    let end_ms = cycle_starts[cycle_starts.len() - 1];
    let values = samples
        .iter()
        .filter(|sample| sample.timestamp_ms >= start_ms && sample.timestamp_ms < end_ms)
        .map(|sample| sample.value)
        .collect::<Vec<_>>();

    (!values.is_empty()).then_some(values)
}

impl AiContextState {
    fn new(limits: AiAdapterLimits) -> Self {
        Self {
            limits,
            session: AiSessionSnapshot::default(),
            telemetry: BTreeMap::new(),
            analysis: BTreeMap::new(),
            analysis_history: BTreeMap::new(),
            sample_history: BTreeMap::new(),
            triggers: VecDeque::new(),
            recent_events: VecDeque::new(),
        }
    }

    fn ingest(&mut self, message: BusMessage) {
        let timestamp_ms = timestamp_ms(message.timestamp);
        let source = message.source.clone();

        match message.kind {
            MessageKind::Connection(connection) => {
                self.session = AiSessionSnapshot {
                    is_running: connection.state != ConnectionState::Stopped,
                    source: Some(source.clone()),
                    connection: Some(connection.clone()),
                    last_event_at_ms: Some(timestamp_ms),
                };
                self.push_event(AiRecentEvent {
                    timestamp_ms,
                    source,
                    kind: AiRecentEventKind::Connection,
                    connection: Some(connection),
                    line: None,
                    analysis: None,
                    trigger: None,
                });
            }
            MessageKind::Line(line) => {
                if let Some(channel_id) = message.parser.fields.get("channel_id") {
                    let summary = self.telemetry.entry(channel_id.clone()).or_insert_with(|| {
                        empty_summary(channel_id, timestamp_ms, message.parser.parser_name.clone())
                    });
                    summary.current_value = message.parser.fields.get("value").cloned();
                    summary.numeric_value = parse_numeric_value(&message.parser.fields);
                    summary.parser_name = message.parser.parser_name.clone();
                    summary.updated_at_ms = timestamp_ms;

                    if let Some(value) = summary.numeric_value {
                        self.sample_history
                            .entry(channel_id.clone())
                            .or_default()
                            .push_back(AiSamplePoint {
                                timestamp_ms,
                                value,
                            });
                        if let Some(history) = self.sample_history.get_mut(channel_id) {
                            trim_queue(history, self.limits.analysis_history);
                        }
                    }
                }

                self.push_event(AiRecentEvent {
                    timestamp_ms,
                    source,
                    kind: AiRecentEventKind::Line,
                    connection: None,
                    line: Some(AiLineEventRecord {
                        direction: line.direction,
                        text: line.payload.text,
                        raw_length: line.payload.raw.len(),
                        parser: message.parser,
                    }),
                    analysis: None,
                    trigger: None,
                });
            }
            MessageKind::TelemetrySchema(schema) => {
                self.push_event(AiRecentEvent {
                    timestamp_ms,
                    source,
                    kind: AiRecentEventKind::Line,
                    connection: None,
                    line: Some(AiLineEventRecord {
                        direction: crate::message::LineDirection::Rx,
                        text: format!("schema rate={} len={}", schema.rate_hz, schema.sample_len),
                        raw_length: schema.fields.len(),
                        parser: crate::message::ParserMeta::parsed(
                            "bmi088_schema",
                            schema
                                .fields
                                .iter()
                                .enumerate()
                                .map(|(index, field)| {
                                    (format!("field.{index}"), field.name.clone())
                                })
                                .collect(),
                        ),
                    }),
                    analysis: None,
                    trigger: None,
                });
            }
            MessageKind::TelemetrySample(sample) => {
                self.push_event(AiRecentEvent {
                    timestamp_ms,
                    source,
                    kind: AiRecentEventKind::Line,
                    connection: None,
                    line: Some(AiLineEventRecord {
                        direction: crate::message::LineDirection::Rx,
                        text: format!("sample fields={}", sample.fields.len()),
                        raw_length: sample.fields.len(),
                        parser: crate::message::ParserMeta::parsed(
                            "bmi088_sample",
                            sample
                                .fields
                                .iter()
                                .map(|field| (field.name.clone(), field.value.to_string()))
                                .collect(),
                        ),
                    }),
                    analysis: None,
                    trigger: None,
                });
            }
            MessageKind::Analysis(frame) => {
                let summary = self
                    .telemetry
                    .entry(frame.channel_id.clone())
                    .or_insert_with(|| empty_summary(&frame.channel_id, timestamp_ms, None));
                summary.has_analysis = true;
                summary.updated_at_ms = timestamp_ms;
                self.analysis
                    .insert(frame.channel_id.clone(), frame.clone());
                self.analysis_history
                    .entry(frame.channel_id.clone())
                    .or_default()
                    .push_back(HistoricalAnalysisEntry {
                        timestamp_ms,
                        frame: frame.clone(),
                    });
                if let Some(history) = self.analysis_history.get_mut(&frame.channel_id) {
                    trim_queue(history, self.limits.analysis_history);
                }
                self.push_event(AiRecentEvent {
                    timestamp_ms,
                    source,
                    kind: AiRecentEventKind::Analysis,
                    connection: None,
                    line: None,
                    analysis: Some(frame),
                    trigger: None,
                });
            }
            MessageKind::Trigger(trigger) => {
                let summary = self
                    .telemetry
                    .entry(trigger.channel_id.clone())
                    .or_insert_with(|| empty_summary(&trigger.channel_id, timestamp_ms, None));
                summary.trigger_count += 1;
                summary.updated_at_ms = timestamp_ms;
                summary.latest_trigger_severity = Some(trigger.severity);
                summary.latest_trigger_reason = Some(trigger.reason.clone());
                self.triggers.push_back(trigger.clone());
                trim_queue(&mut self.triggers, self.limits.recent_triggers);
                self.push_event(AiRecentEvent {
                    timestamp_ms,
                    source,
                    kind: AiRecentEventKind::Trigger,
                    connection: None,
                    line: None,
                    analysis: None,
                    trigger: Some(trigger),
                });
            }
        }
    }

    fn push_event(&mut self, event: AiRecentEvent) {
        self.recent_events.push_back(event);
        trim_queue(&mut self.recent_events, self.limits.recent_events);
    }

    fn reset(&mut self) {
        *self = Self::new(self.limits);
    }
}

impl From<HistoricalAnalysisEntry> for AiSamplePoint {
    fn from(value: HistoricalAnalysisEntry) -> Self {
        AiSamplePoint {
            timestamp_ms: value.timestamp_ms,
            value: value.frame.mean_value.unwrap_or_default(),
        }
    }
}

fn empty_summary(
    channel_id: &str,
    updated_at_ms: u64,
    parser_name: Option<String>,
) -> TelemetryChannelSummary {
    TelemetryChannelSummary {
        channel_id: channel_id.to_string(),
        current_value: None,
        numeric_value: None,
        parser_name,
        updated_at_ms,
        has_analysis: false,
        trigger_count: 0,
        latest_trigger_severity: None,
        latest_trigger_reason: None,
    }
}

fn parse_numeric_value(fields: &BTreeMap<String, String>) -> Option<f64> {
    fields
        .get("numeric_value")
        .or_else(|| fields.get("value"))
        .and_then(|value| value.parse::<f64>().ok())
}

fn trim_queue<T>(queue: &mut VecDeque<T>, limit: usize) {
    while queue.len() > limit {
        queue.pop_front();
    }
}

fn compute_variance(values: &[f64], mean: f64) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }

    Some(
        values
            .iter()
            .map(|value| {
                let delta = value - mean;
                delta * delta
            })
            .sum::<f64>()
            / values.len() as f64,
    )
}

fn lock_state<'a>(state: &'a Arc<Mutex<AiContextState>>) -> MutexGuard<'a, AiContextState> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn timestamp_ms(timestamp: SystemTime) -> u64 {
    timestamp
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
