//! Typed SCIENTIA research event types, `PreregistrationV1`, and `ResearchEventEmitter` trait.
//!
//! This is an L1 crate (pure data — no async runtime, no DB). All SCIENTIA pipeline
//! components communicate through these types to avoid circular crate dependencies.

pub mod emitter;
pub mod events;
pub mod preregistration;
pub mod schema_types;

pub use emitter::{NoopEmitter, ResearchEventEmitter};
pub use events::{ResearchEvent, ResearchEventKind};
pub use preregistration::{
    DecisionRule, MetricSpec, PreregistrationV1, StatisticalTest, StopRule, SubstrateRef,
    TestSpec,
};
pub use schema_types::{
    DiscoverySignal, DiscoverySignalFamily, DiscoverySignalStrength,
    EvidencePackV1, FindingCandidateClass, FindingCandidateV1,
    NoveltyEvidenceBundle, SignalProvenance, WorthinessSignalsV2,
};
