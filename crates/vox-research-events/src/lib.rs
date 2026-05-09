//! Typed SCIENTIA research event types, `PreregistrationV1`, and `ResearchEventEmitter` trait.
//!
//! This is an L1 crate (pure data — no async runtime, no DB). All SCIENTIA pipeline
//! components communicate through these types to avoid circular crate dependencies.

pub mod emitter;
pub mod events;
pub mod preregistration;

pub use emitter::{NoopEmitter, ResearchEventEmitter};
pub use events::{ResearchEvent, ResearchEventKind};
pub use preregistration::{
    DecisionRule, MetricSpec, PreregistrationV1, StatisticalTest, StopRule, SubstrateRef,
    TestSpec,
};
