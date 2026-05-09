//! `ResearchEventEmitter` — object-safe trait for publishing SCIENTIA events.
//!
//! The orchestrator holds an `Arc<dyn ResearchEventEmitter>` and calls `emit()` at
//! each signal-ladder transition. Default implementation is `NoopEmitter` (drops events).
//! Production wiring replaces it with a multi-sink fanout that writes to the event bus,
//! gamify bridge, and telemetry store.

use crate::events::ResearchEvent;

/// Object-safe trait for emitting SCIENTIA research lifecycle events.
///
/// Implementations must be `Send + Sync` so they can be held behind `Arc`.
pub trait ResearchEventEmitter: Send + Sync {
    /// Emit a research lifecycle event.
    ///
    /// Implementations should be non-blocking; use an internal channel if persistence
    /// is needed. The caller does not await completion.
    fn emit(&self, event: ResearchEvent);

    /// Emit multiple events in order.
    ///
    /// Default impl calls `emit` in a loop. Override for batching.
    fn emit_batch(&self, events: Vec<ResearchEvent>) {
        for evt in events {
            self.emit(evt);
        }
    }
}

/// No-op emitter — silently drops all events. Used in tests and during bootstrap.
pub struct NoopEmitter;

impl ResearchEventEmitter for NoopEmitter {
    fn emit(&self, _event: ResearchEvent) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ResearchEvent;

    #[test]
    fn noop_emitter_accepts_any_event() {
        let emitter = NoopEmitter;
        emitter.emit(ResearchEvent::CampaignStarted {
            campaign_id: "c1".to_string(),
            prereg_id: "p1".to_string(),
            cost_cap_usd: 10.0,
        });
        // NoopEmitter drops events silently — just verifying no panic.
    }

    #[test]
    fn boxed_emitter_is_object_safe() {
        let _boxed: Box<dyn ResearchEventEmitter> = Box::new(NoopEmitter);
    }
}
