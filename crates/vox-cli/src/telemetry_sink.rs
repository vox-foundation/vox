//! [`SpoolSink`] — serializes `TelemetryEvent` to the local upload queue.
//!
//! Only S0–S1 events should reach this sink by default. The spool is drained
//! by `vox telemetry upload` when the user has configured upload credentials
//! (ADR 023).

use std::path::PathBuf;

use vox_telemetry::{TelemetryEvent, TelemetryRecorder};

/// `TelemetryRecorder` sink that writes events as JSON files to the local spool.
///
/// `record` spawns a tokio task to avoid holding an async executor during file I/O.
/// Errors are logged at DEBUG (spool failure is non-critical).
pub struct SpoolSink {
    root: PathBuf,
}

impl SpoolSink {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl TelemetryRecorder for SpoolSink {
    fn record(&self, event: &TelemetryEvent) {
        let root = self.root.clone();
        let event = event.clone();
        tokio::spawn(async move {
            if let Err(err) = crate::telemetry_spool::enqueue(&root, &event) {
                tracing::debug!(?err, "SpoolSink: enqueue failed");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spool_sink_is_recorder() {
        fn _assert_recorder<T: vox_telemetry::TelemetryRecorder>() {}
        _assert_recorder::<SpoolSink>();
    }
}
