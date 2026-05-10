//! Pure-types L0 leaf for the populi/mesh subsystem (a2a envelopes, donation policy, federation, kudos).

pub mod a2a;
/// Signed result attestation envelope (P5-T4).
pub mod attestation;
/// A2A wire types for content-addressed bundle requests/responses (P2-T4).
pub mod bundle;
pub mod donation_policy;
pub mod federation;
pub mod kudos;
pub mod model_registry;
pub mod quorum;
pub mod secret_sync;
pub mod task;
pub mod trace;

pub use a2a::A2ADeliverRequest;
pub use attestation::Attestation;
pub use donation_policy::*;
pub use federation::*;
pub use kudos::*;
pub use model_registry::*;
pub use quorum::*;
pub use secret_sync::*;
pub use task::*;
pub use trace::{MeshTraceContext, ParseTraceparentError, SpanId, TraceId};
