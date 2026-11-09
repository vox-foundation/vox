//! `vox mens eval-gate` — check training/eval metrics against policy thresholds.
//!
//! Reads `mens/config/eval-gates.yaml` and validates run artifacts.
//! Exits non-zero if any blocking gate fails.

#![allow(unused_imports)]

pub mod baseline;
mod check_run;
mod io;
pub mod leakage;
mod legacy;
pub mod planning_eval;
pub mod policy;
mod run_gate;
#[cfg(test)]
mod tests;

pub use baseline::{
    BaselineEntry, BaselineReport, beat_base, capture_baseline, load_baseline, save_baseline,
    wilson_ci,
};
pub use check_run::{GateResult, check_run};
pub use leakage::{
    BenchTask, SplitManifest, assert_no_leakage, assert_no_text_leakage, leaked_bench_task,
    load_bench_answers, load_bench_texts, load_split_manifest,
};
#[cfg(all(feature = "mens-dei", feature = "gpu"))]
pub(crate) use legacy::{
    LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_COVERAGE, LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_PARSE_RATE,
    run_legacy_train_post_eval_gate,
};
pub use planning_eval::{PlanningEvalResult, evaluate_plan_sequence};
pub use policy::{
    AntiStubGate, CollateralDamageGate, EvalGatePolicy, PassAtKGate, SlicePin, SupervisedRatioGate,
    load_policy, load_policy_value,
};
pub use run_gate::run_eval_gate;
