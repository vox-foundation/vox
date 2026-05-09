use crate::{recorder::TelemetryRecorder, types::TelemetryEvent};

/// Default recorder used when no sink is registered. Discards all events silently.
pub struct NoOpRecorder;

impl TelemetryRecorder for NoOpRecorder {
    #[inline]
    fn record(&self, _event: &TelemetryEvent) {}
}
