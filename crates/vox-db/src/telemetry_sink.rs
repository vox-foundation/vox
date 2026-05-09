//! [`ResearchMetricsSink`] — writes `TelemetryEvent::ResearchMetric` events to the
//! `research_metrics` table via [`crate::VoxDb::append_research_metric`].
//!
//! Other event variants are silently ignored; they are handled by higher-layer sinks
//! introduced in Phases B–D.

use std::sync::Arc;

use vox_telemetry::{ResearchMetricEvent, TelemetryEvent, TelemetryRecorder};

/// `TelemetryRecorder` sink backed by a live `VoxDb` connection.
///
/// `record` spawns a background tokio task for the async DB write so the caller
/// is never blocked. Write failures are logged at WARN and swallowed — telemetry
/// must never surface as a user-visible error.
pub struct ResearchMetricsSink {
    db: Arc<crate::VoxDb>,
}

impl ResearchMetricsSink {
    pub fn new(db: crate::VoxDb) -> Self {
        Self { db: Arc::new(db) }
    }
}

impl TelemetryRecorder for ResearchMetricsSink {
    fn record(&self, event: &TelemetryEvent) {
        let TelemetryEvent::ResearchMetric(e) = event else {
            return;
        };
        let db = Arc::clone(&self.db);
        let e: ResearchMetricEvent = e.clone();
        tokio::spawn(async move {
            if let Err(err) = db
                .append_research_metric(
                    &e.session_id,
                    &e.metric_type,
                    e.metric_value,
                    e.metadata_json.as_deref(),
                )
                .await
            {
                tracing::warn!(?err, "ResearchMetricsSink: append_research_metric failed");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_metrics_sink_is_recorder() {
        fn _assert_recorder<T: vox_telemetry::TelemetryRecorder>() {}
        _assert_recorder::<ResearchMetricsSink>();
    }
}
