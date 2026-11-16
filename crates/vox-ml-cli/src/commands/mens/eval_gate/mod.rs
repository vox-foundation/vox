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
    AntiStubGate, CollateralDamageGate, EvalGatePolicy, PassAtKGate, SlicePin, SuiteGate,
    SupervisedRatioGate, load_policy, load_policy_value,
};
pub use run_gate::run_eval_gate;

/// Source used to determine which domain spokes apply to a training run directory (`338-06`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpokeDerivation {
    AdapterCardSpokes,
    TrainLanes,
    ManifestProfile,
    DefaultFallback,
}

/// Derive applying spoke names for `run_dir` using `registry`, reporting the derivation source.
pub fn spokes_of_run(
    run_dir: &std::path::Path,
    registry: &vox_populi::mens::tensor::domain_profiles::DomainProfilesFile,
) -> (Vec<String>, SpokeDerivation) {
    let card_path = run_dir.join("adapter_card.json");
    if let Ok(raw) = std::fs::read_to_string(&card_path)
        && let Ok(card) = serde_json::from_str::<serde_json::Value>(&raw)
    {
        if let Some(arr) = card.get("spokes").and_then(|v| v.as_array()) {
            let spokes: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .filter(|s| !s.trim().is_empty())
                .collect();
            if !spokes.is_empty() {
                return (spokes, SpokeDerivation::AdapterCardSpokes);
            }
        }
        if let Some(arr) = card.get("train_lanes").and_then(|v| v.as_array()) {
            let lanes: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect();
            let mapped = registry.spokes_for_lanes(&lanes);
            if !mapped.is_empty() {
                return (mapped, SpokeDerivation::TrainLanes);
            }
        }
    }

    let manifest_path = run_dir.join("training_manifest.json");
    if let Ok(raw) = std::fs::read_to_string(&manifest_path)
        && let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&raw)
        && let Some(sp) = manifest
            .get("spoke")
            .or_else(|| manifest.get("domain_profile"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
    {
        return (vec![sp.to_string()], SpokeDerivation::ManifestProfile);
    }

    (Vec::new(), SpokeDerivation::DefaultFallback)
}
