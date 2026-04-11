use crate::ai_contract::{
    AiAdapterLimits, AiLineEventRecord, AiRecentEvent, AiRecentEventKind, AiSessionSnapshot,
    AnalysisFramesResource, ChannelAnalysisResource, DEFAULT_CHANNEL_TRIGGER_CONTEXT_LIMIT,
    RecentEventsQuery, RecentEventsResource, TelemetryChannelSummary, TelemetrySummaryResource,
    TriggerHistoryResource,
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
    triggers: VecDeque<TriggerEvent>,
    recent_events: VecDeque<AiRecentEvent>,
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

impl AiContextState {
    fn new(limits: AiAdapterLimits) -> Self {
        Self {
            limits,
            session: AiSessionSnapshot::default(),
            telemetry: BTreeMap::new(),
            analysis: BTreeMap::new(),
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
            MessageKind::Analysis(frame) => {
                let summary = self
                    .telemetry
                    .entry(frame.channel_id.clone())
                    .or_insert_with(|| empty_summary(&frame.channel_id, timestamp_ms, None));
                summary.has_analysis = true;
                summary.updated_at_ms = timestamp_ms;
                self.analysis
                    .insert(frame.channel_id.clone(), frame.clone());
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
