use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Telemetry callback signature for SolvraAI integration.
pub type TelemetryHook = Arc<dyn Fn(&TelemetryEvent) + Send + Sync>;

/// Event emitted on runtime milestones.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    pub kind: TelemetryEventKind,
    pub task_label: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub timeout_threshold_ms: Option<u64>,
    pub stack_depth: usize,
    pub timestamp: Instant,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub enum TelemetryEventKind {
    TaskSpawn,
    TaskTimeout,
    TaskJoin,
    TaskPanic,
    TaskCancel,
    RuntimeSummary,
}

/// JSON-serialisable view of telemetry emitted by the runtime.
#[derive(Debug, Clone, Serialize)]
pub struct TelemetryRecord {
    pub kind: TelemetryEventKind,
    pub task_label: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub timeout_threshold_ms: Option<u64>,
    pub stack_depth: usize,
    pub timestamp_utc: String,
}

impl TelemetryRecord {
    fn from_event(event: &TelemetryEvent) -> Self {
        Self {
            kind: event.kind.clone(),
            task_label: event.task_label.clone(),
            elapsed_ms: event.elapsed_ms,
            timeout_threshold_ms: event.timeout_threshold_ms,
            stack_depth: event.stack_depth,
            timestamp_utc: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

/// Collects runtime telemetry events for later inspection.
#[derive(Clone, Default)]
pub struct TelemetryCollector {
    events: Arc<Mutex<Vec<TelemetryRecord>>>,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hook(&self) -> TelemetryHook {
        let collector = self.clone();
        Arc::new(move |event: &TelemetryEvent| {
            collector.record(event);
        })
    }

    fn record(&self, event: &TelemetryEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.push(TelemetryRecord::from_event(event));
        }
    }

    pub fn snapshot(&self) -> Vec<TelemetryRecord> {
        self.events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }
}
