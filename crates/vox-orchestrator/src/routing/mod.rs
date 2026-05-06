//! Provider-agnostic routing policy + Thompson exploration (see `contracts/orchestration/model-routing.v1.yaml`).

mod bandit;
mod engine;
pub mod policy;

pub use bandit::sample_beta_thompson;
pub use engine::{ModelSelectionEngine, PickReason};
pub use policy::RoutingPolicy;
