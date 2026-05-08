//! `vox mens eval-gate` — check training/eval metrics against policy thresholds.
//!
//! Reads `mens/config/eval-gates.yaml` and validates run artifacts.
//! Exits non-zero if any blocking gate fails.

// Re-exports are consumed by feature-gated `populi` / `ai::train` call sites; default `cargo check` may not reference every name.
#![allow(unused_imports)]

mod check_run;
mod io;
mod legacy;
mod policy;
mod run_gate;
#[cfg(test)]
mod tests;

pub use check_run::{GateResult, check_run};
#[cfg(all(feature = "mens-dei", feature = "gpu"))]
pub(crate) use legacy::{
    LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_COVERAGE, LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_PARSE_RATE,
    run_legacy_train_post_eval_gate,
};
pub use policy::{
    ContextGateEntry, EvalGatePolicy, EvalLocalGate, McpToolSchemaGate, ModalMixGate, PassAtKGate,
    PerplexityGate, ReviewRecurrenceGate, SupervisedRatioGate, ThroughputGate, TruncationGate,
    load_policy,
};
pub use run_gate::run_eval_gate;
