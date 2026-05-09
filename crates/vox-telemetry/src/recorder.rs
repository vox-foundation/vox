use std::sync::{Arc, OnceLock};

use crate::types::TelemetryEvent;

/// Trait implemented by every telemetry sink.
///
/// `record` is called synchronously on the caller's thread/task. Implementations
/// MUST return quickly (fire-and-forget internally via `tokio::spawn` or a channel).
pub trait TelemetryRecorder: Send + Sync + 'static {
    fn record(&self, event: &TelemetryEvent);
}

static GLOBAL_RECORDER: OnceLock<Arc<dyn TelemetryRecorder>> = OnceLock::new();

/// Register the process-wide recorder. Silently ignored if called more than once
/// (first writer wins). Call once at binary startup before any `record_event!`.
pub fn set_global_recorder(recorder: Arc<dyn TelemetryRecorder>) {
    let _ = GLOBAL_RECORDER.set(recorder);
}

/// Returns the global recorder, or `None` if not yet initialized.
///
/// Used by the `record_event!` macro; callers should prefer that macro.
pub fn global_recorder() -> Option<&'static Arc<dyn TelemetryRecorder>> {
    GLOBAL_RECORDER.get()
}

/// Fan-out recorder: delegates every `record` call to all inner recorders.
pub struct CompositeRecorder {
    inner: Vec<Arc<dyn TelemetryRecorder>>,
}

impl CompositeRecorder {
    pub fn new(inner: Vec<Arc<dyn TelemetryRecorder>>) -> Self {
        Self { inner }
    }
}

impl TelemetryRecorder for CompositeRecorder {
    fn record(&self, event: &TelemetryEvent) {
        for r in &self.inner {
            r.record(event);
        }
    }
}
