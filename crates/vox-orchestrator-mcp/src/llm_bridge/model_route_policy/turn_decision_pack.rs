//! Calibrated pre-turn decision pack (`DEC-01`, `DEC-03`, `DEC-04`).
//!
//! Feeds structured routing priors (`task`, `complexity`, `prefer_local`, and
//! conditional `allow_free_in_performance_mode`) upstream of
//! [`super::resolve::build_selection_request`] without ever emitting or
//! accepting raw model IDs, and evaluates selective-prediction calibration
//! metrics (`accuracy`, `ece`, `coverage_at_error`) with strict local-only
//! privacy gating (`!vox_orchestrator::route_policy::inference_privacy_local_only_from_env()`).

use serde::{Deserialize, Serialize};
use vox_orchestrator::mode::ClutchProfile;
use vox_orchestrator::types::TaskCategory;

/// Schema version tag for `TurnDecisionPack` (`DEC-01`).
pub const TURN_DECISION_PACK_SCHEMA_V1: &str = "turn_decision_pack.v1";

/// Compile-time invariant (`DEC-04`): Jev / System One turn-decision outputs
/// are harness control telemetry only and MUST NEVER be distilled into MENS
/// chat SFT training corpora.
pub const ALLOW_JEV_DISTILLATION_INTO_MENS_SFT: bool = false;

/// Versioned pre-turn decision pack (`DEC-01`).
///
/// Uses `#[serde(deny_unknown_fields)]` and [`Self::from_json_rejecting_model_ids`]
/// so any attempt to smuggle `model_id`, `model`, or provider pins through the
/// pack fails closed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurnDecisionPack {
    pub schema_version: String,
    #[serde(default)]
    pub task: Option<TaskCategory>,
    #[serde(default)]
    pub complexity: Option<u8>,
    #[serde(default)]
    pub prefer_local: Option<bool>,
    /// Optional request to let `:free` specs compete under `CostPreference::Performance`.
    ///
    /// **Exact semantics (`DEC-01`):**
    /// - Only honored when `res.clutch.is_none()`.
    /// - `ClutchProfile::Free` already sets `allow_free_in_performance_mode = true`.
    /// - `ClutchProfile::Performance`, `ClutchProfile::Genius`, and `ClutchProfile::Balanced`
    ///   are NEVER overridden by the pack.
    /// - Setting this flag only prevents `best_for_internal` from skipping `:free`
    ///   specs under `CostPreference::Performance`; it does **not** invoke
    ///   `PreferFree` or reorder `FREE_FLOOR` / `OPENROUTER_FREE_FALLBACK_MODELS`.
    #[serde(default)]
    pub allow_free_in_performance_mode: Option<bool>,
    /// Calibrated confidence in `[0.0, 1.0]` for the pre-turn classification.
    pub confidence: f64,
}

impl TurnDecisionPack {
    /// Env key holding an operator-supplied pack JSON (`turn_decision_pack.v1`). Unset ⇒ flag off.
    pub const ENV_KEY: &'static str = "VOX_TURN_DECISION_PACK_JSON";

    /// Producer for the live resolver: parse [`Self::ENV_KEY`], failing closed (`None`, with a warning)
    /// on malformed JSON, unknown fields, model ids, or a bad schema version.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var(Self::ENV_KEY).ok()?;
        if raw.trim().is_empty() {
            return None;
        }
        match Self::from_json_rejecting_model_ids(&raw) {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!(error = %e, "ignoring invalid {}", Self::ENV_KEY);
                None
            }
        }
    }

    /// Parse a JSON payload into a [`TurnDecisionPack`], explicitly rejecting
    /// any payload that contains a model identifier key (`model_id`, `model`,
    /// `pinned_model`, `target_model`) or an unsupported `schema_version`.
    pub fn from_json_rejecting_model_ids(raw: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|e| format!("invalid turn decision pack JSON: {e}"))?;
        if let Some(obj) = value.as_object() {
            for forbidden in ["model_id", "model", "pinned_model", "target_model"] {
                if obj.contains_key(forbidden) {
                    return Err(format!(
                        "TurnDecisionPack forbids `{forbidden}` (`DEC-01`: packs never emit model IDs)"
                    ));
                }
            }
        }
        let pack: Self = serde_json::from_value(value)
            .map_err(|e| format!("TurnDecisionPack schema violation: {e}"))?;
        if pack.schema_version != TURN_DECISION_PACK_SCHEMA_V1 {
            return Err(format!(
                "unsupported TurnDecisionPack schema_version `{}` (expected `{TURN_DECISION_PACK_SCHEMA_V1}`)",
                pack.schema_version
            ));
        }
        if !pack.confidence.is_finite() || !(0.0..=1.0).contains(&pack.confidence) {
            return Err(format!(
                "TurnDecisionPack confidence must be in 0.0..=1.0, got {}",
                pack.confidence
            ));
        }
        Ok(pack)
    }
}

/// Caller-explicitness flags indicating which fields were explicitly set by the
/// caller before consulting a [`TurnDecisionPack`] (`DEC-01`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CallerExplicitRouteFlags {
    pub caller_set_task: bool,
    pub caller_set_complexity: bool,
    pub caller_set_prefer_local: bool,
}

/// Patched inputs produced upstream of [`super::resolve::build_selection_request`] (`DEC-01`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnDecisionPatchOutcome {
    pub task: TaskCategory,
    pub complexity: u8,
    pub prefer_local: bool,
    /// `Some(true)` only when `clutch.is_none()` and the pack requested
    /// `allow_free_in_performance_mode = Some(true)`.
    pub allow_free_in_performance_mode_override: Option<bool>,
    pub pack_applied: bool,
}

/// Apply an optional [`TurnDecisionPack`] upstream of `build_selection_request` (`DEC-01`).
///
/// - When `pack` is `None`, returns the caller inputs byte-for-byte unchanged
///   (`pack_applied = false`, `allow_free_in_performance_mode_override = None`).
/// - Patches `task`, `complexity` (clamped to `1..=10`), and `prefer_local`
///   ONLY when the corresponding flag in `explicit` is `false`.
/// - May set `allow_free_in_performance_mode_override = Some(true)` ONLY when
///   `clutch.is_none()`; when `clutch` is `Some(_)` (`Free`, `Balanced`,
///   `Performance`, or `Genius`), `clutch` remains authoritative and is never
///   overridden by the pack.
#[must_use]
pub fn apply_turn_decision_pack_upstream(
    base_task: TaskCategory,
    base_complexity: u8,
    base_prefer_local: bool,
    clutch: Option<ClutchProfile>,
    explicit: CallerExplicitRouteFlags,
    pack: Option<&TurnDecisionPack>,
) -> TurnDecisionPatchOutcome {
    let Some(pack) = pack else {
        return TurnDecisionPatchOutcome {
            task: base_task,
            complexity: base_complexity,
            prefer_local: base_prefer_local,
            allow_free_in_performance_mode_override: None,
            pack_applied: false,
        };
    };

    let task = if explicit.caller_set_task {
        base_task
    } else {
        pack.task.unwrap_or(base_task)
    };
    let complexity = if explicit.caller_set_complexity {
        base_complexity
    } else {
        pack.complexity
            .map(|c| c.clamp(1, 10))
            .unwrap_or(base_complexity)
    };
    let prefer_local = if explicit.caller_set_prefer_local {
        base_prefer_local
    } else {
        pack.prefer_local.unwrap_or(base_prefer_local)
    };
    let allow_free_in_performance_mode_override = if clutch.is_none() {
        pack.allow_free_in_performance_mode.filter(|&allow| allow)
    } else {
        None
    };

    TurnDecisionPatchOutcome {
        task,
        complexity,
        prefer_local,
        allow_free_in_performance_mode_override,
        pack_applied: true,
    }
}

/// Gate optional hosted Jev / System One consultation (`DEC-04`).
///
/// Strictly blocked whenever `vox_orchestrator::route_policy::inference_privacy_local_only_from_env()`
/// is true (`VOX_INFERENCE_PRIVACY=local_only`) or when `pack_feature_enabled` is false.
/// Does not touch `SIZE-02` / `MESH-01` memory selection.
#[must_use]
pub fn hosted_jev_consultation_allowed(pack_feature_enabled: bool) -> bool {
    pack_feature_enabled && !vox_orchestrator::route_policy::inference_privacy_local_only_from_env()
}

/// Enforce the `DEC-04` invariant that Jev / System One decision-pack traces are
/// never admitted into MENS chat SFT training sets.
pub fn validate_jev_training_isolation(dataset_source_tag: &str) -> Result<(), String> {
    let normalized = dataset_source_tag.trim().to_ascii_lowercase();
    if normalized.contains("jev")
        || normalized.contains("system_one")
        || normalized.contains("turn_decision_pack")
    {
        return Err(format!(
            "DEC-04 violation: source `{dataset_source_tag}` is a harness turn-decision artifact and must never be distilled into MENS chat SFT (ALLOW_JEV_DISTILLATION_INTO_MENS_SFT={ALLOW_JEV_DISTILLATION_INTO_MENS_SFT})"
        ));
    }
    Ok(())
}

/// One labeled confidence observation for selective-prediction evaluation (`DEC-04`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalibrationSample {
    pub confidence: f64,
    pub correct: bool,
}

/// Selective-prediction and calibration summary (`DEC-04`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectivePredictionMetrics {
    /// Empirical accuracy across all samples (`0.0..=1.0`).
    pub accuracy: f64,
    /// Expected Calibration Error across `num_bins` equal-width bins in `[0.0, 1.0]`.
    pub ece: f64,
    /// Maximum fraction of samples (`0.0..=1.0`) retained when sorting by
    /// descending confidence such that empirical selective error `<= target_error_bound`.
    pub coverage_at_error: f64,
    pub target_error_bound: f64,
    pub sample_count: usize,
    pub num_bins: usize,
}

/// Compute selective-prediction metrics (`accuracy`, `ece`, `coverage_at_error`) over
/// turn-decision calibration observations (`DEC-04`).
#[must_use]
pub fn evaluate_selective_prediction(
    samples: &[CalibrationSample],
    num_bins: usize,
    target_error_bound: f64,
) -> SelectivePredictionMetrics {
    let bins = num_bins.max(1);
    let clamped_bound = target_error_bound.clamp(0.0, 1.0);
    if samples.is_empty() {
        return SelectivePredictionMetrics {
            accuracy: 0.0,
            ece: 0.0,
            coverage_at_error: 0.0,
            target_error_bound: clamped_bound,
            sample_count: 0,
            num_bins: bins,
        };
    }

    let n = samples.len();
    let n_f64 = n as f64;
    let total_correct = samples.iter().filter(|s| s.correct).count();
    let accuracy = total_correct as f64 / n_f64;

    let mut bin_counts = vec![0usize; bins];
    let mut bin_correct = vec![0usize; bins];
    let mut bin_conf_sum = vec![0.0f64; bins];
    for s in samples {
        let c = s.confidence.clamp(0.0, 1.0);
        let idx = ((c * bins as f64).floor() as usize).min(bins - 1);
        bin_counts[idx] += 1;
        if s.correct {
            bin_correct[idx] += 1;
        }
        bin_conf_sum[idx] += c;
    }

    let mut ece = 0.0f64;
    for b in 0..bins {
        let count = bin_counts[b];
        if count > 0 {
            let bin_acc = bin_correct[b] as f64 / count as f64;
            let bin_conf = bin_conf_sum[b] / count as f64;
            ece += (count as f64 / n_f64) * (bin_acc - bin_conf).abs();
        }
    }

    // Sort by confidence descending (tie-breaking deterministically with correct first)
    // and find the largest prefix `k` whose selective error rate `(k - correct_k) / k <= clamped_bound`.
    let mut sorted: Vec<CalibrationSample> = samples.to_vec();
    sorted.sort_by(|a, b| {
        b.confidence
            .total_cmp(&a.confidence)
            .then_with(|| b.correct.cmp(&a.correct))
    });
    let mut prefix_errors = 0usize;
    let mut max_valid_k = 0usize;
    for (i, sample) in sorted.iter().enumerate() {
        if !sample.correct {
            prefix_errors += 1;
        }
        let k = i + 1;
        let error_rate = prefix_errors as f64 / k as f64;
        if error_rate <= clamped_bound + 1e-12 {
            max_valid_k = k;
        }
    }
    let coverage_at_error = max_valid_k as f64 / n_f64;

    SelectivePredictionMetrics {
        accuracy,
        ece,
        coverage_at_error,
        target_error_bound: clamped_bound,
        sample_count: n,
        num_bins: bins,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_bridge::model_route_policy::resolve::{
        build_selection_request, build_selection_request_with_decision_pack,
    };
    use vox_orchestrator::config::CostPreference;
    use vox_orchestrator::mode::{ClutchProfile, RiskPosture};
    use vox_orchestrator::models::Capability;

    #[test]
    fn turn_decision_pack_rejects_model_id_fields() {
        let raw_with_model_id = r#"{
            "schema_version": "turn_decision_pack.v1",
            "task": "CodeGen",
            "complexity": 6,
            "confidence": 0.91,
            "model_id": "anthropic/claude-sonnet-4"
        }"#;
        let err = TurnDecisionPack::from_json_rejecting_model_ids(raw_with_model_id)
            .expect_err("model_id must be rejected by TurnDecisionPack");
        assert!(err.contains("model_id"));

        let valid_raw = r#"{
            "schema_version": "turn_decision_pack.v1",
            "task": "Debugging",
            "complexity": 8,
            "prefer_local": true,
            "allow_free_in_performance_mode": true,
            "confidence": 0.88
        }"#;
        let pack =
            TurnDecisionPack::from_json_rejecting_model_ids(valid_raw).expect("valid pack parses");
        assert_eq!(pack.task, Some(TaskCategory::Debugging));
        assert_eq!(pack.complexity, Some(8));
        assert_eq!(pack.prefer_local, Some(true));
        assert_eq!(pack.allow_free_in_performance_mode, Some(true));
    }

    #[test]
    fn turn_decision_pack_patches_only_unset_caller_fields_and_respects_clutch() {
        let pack = TurnDecisionPack {
            schema_version: TURN_DECISION_PACK_SCHEMA_V1.to_string(),
            task: Some(TaskCategory::Debugging),
            complexity: Some(9),
            prefer_local: Some(false),
            allow_free_in_performance_mode: Some(true),
            confidence: 0.92,
        };

        // 1. Caller set nothing, clutch is None -> pack patches all 3 + allow_free_in_performance_mode.
        let req_patched = build_selection_request_with_decision_pack(
            TaskCategory::CodeGen,
            5,
            true,
            false,
            CostPreference::Performance,
            vec![Capability::SupportsToolUse],
            None,
            None,
            None,
            CallerExplicitRouteFlags::default(),
            Some(&pack),
        );
        assert_eq!(req_patched.intent.task, TaskCategory::Debugging);
        assert_eq!(req_patched.intent.complexity, 9);
        assert!(!req_patched.intent.prefer_local);
        assert!(
            req_patched.intent.allow_free_in_performance_mode,
            "when clutch is None, pack may set allow_free_in_performance_mode"
        );

        // 2. Caller explicitly set task & complexity, and clutch is Balanced ->
        //    pack must NOT override caller fields and must NOT set allow_free_in_performance_mode.
        let req_clutch_bal = build_selection_request_with_decision_pack(
            TaskCategory::Research,
            3,
            false,
            false,
            CostPreference::Performance,
            vec![],
            Some(ClutchProfile::Balanced),
            Some(RiskPosture::Moderate),
            None,
            CallerExplicitRouteFlags {
                caller_set_task: true,
                caller_set_complexity: true,
                caller_set_prefer_local: true,
            },
            Some(&pack),
        );
        assert_eq!(req_clutch_bal.intent.task, TaskCategory::Research);
        assert_eq!(req_clutch_bal.intent.complexity, 3);
        assert!(!req_clutch_bal.intent.prefer_local);
        assert!(
            !req_clutch_bal.intent.allow_free_in_performance_mode,
            "ClutchProfile::Balanced must never be overridden by pack allow_free_in_performance_mode"
        );

        // 3. ClutchProfile::Genius and ClutchProfile::Balanced must never be overridden.
        for clutch in [ClutchProfile::Genius, ClutchProfile::Balanced] {
            let req = build_selection_request_with_decision_pack(
                TaskCategory::Research,
                4,
                false,
                false,
                CostPreference::Performance,
                vec![],
                Some(clutch),
                None,
                None,
                CallerExplicitRouteFlags::default(),
                Some(&pack),
            );
            assert!(
                !req.intent.allow_free_in_performance_mode,
                "clutch {clutch:?} must never have allow_free_in_performance_mode overridden by pack"
            );
        }
    }

    #[test]
    fn dec_03_flag_off_build_selection_request_is_byte_identical() {
        let baseline = build_selection_request(
            TaskCategory::CodeGen,
            7,
            true,
            false,
            CostPreference::Economy,
            vec![Capability::SupportsToolUse, Capability::SupportsReasoning],
            Some(ClutchProfile::Balanced),
            Some(RiskPosture::Moderate),
            Some(0.42),
        );
        let flag_off = build_selection_request_with_decision_pack(
            TaskCategory::CodeGen,
            7,
            true,
            false,
            CostPreference::Economy,
            vec![Capability::SupportsToolUse, Capability::SupportsReasoning],
            Some(ClutchProfile::Balanced),
            Some(RiskPosture::Moderate),
            Some(0.42),
            CallerExplicitRouteFlags::default(),
            None,
        );
        assert_eq!(
            format!("{baseline:?}").as_bytes(),
            format!("{flag_off:?}").as_bytes(),
            "flag-off build_selection_request_with_decision_pack must be byte-for-byte identical"
        );
    }

    #[test]
    fn dec_04_selective_prediction_metrics_and_privacy_gate() {
        let samples = vec![
            CalibrationSample {
                confidence: 0.95,
                correct: true,
            },
            CalibrationSample {
                confidence: 0.90,
                correct: true,
            },
            CalibrationSample {
                confidence: 0.80,
                correct: true,
            },
            CalibrationSample {
                confidence: 0.60,
                correct: false,
            },
            CalibrationSample {
                confidence: 0.20,
                correct: false,
            },
        ];
        let metrics = evaluate_selective_prediction(&samples, 5, 0.05);
        assert!((metrics.accuracy - 0.60).abs() < 1e-9);
        assert!(metrics.ece >= 0.0 && metrics.ece <= 1.0);
        // Top 3 predictions (0.95, 0.90, 0.80) have 0 errors -> error rate 0.0 <= 0.05 -> coverage = 3/5 = 0.60.
        assert!((metrics.coverage_at_error - 0.60).abs() < 1e-9);

        // Privacy gate: local_only blocks hosted Jev consultation.
        vox_orchestrator::route_policy::set_test_privacy_override(Some("local_only"));
        assert!(
            !hosted_jev_consultation_allowed(true),
            "VOX_INFERENCE_PRIVACY=local_only must block hosted Jev consultation"
        );
        vox_orchestrator::route_policy::set_test_privacy_override(Some("any"));
        assert!(hosted_jev_consultation_allowed(true));
        assert!(!hosted_jev_consultation_allowed(false));
        vox_orchestrator::route_policy::set_test_privacy_override(None);

        // Training isolation guard: Jev / System One never distilled into MENS SFT.
        const { assert!(!ALLOW_JEV_DISTILLATION_INTO_MENS_SFT) };
        assert!(validate_jev_training_isolation("jev_system_one_trace").is_err());
        assert!(validate_jev_training_isolation("turn_decision_pack.v1").is_err());
        assert!(validate_jev_training_isolation("vox_golden_compiler_corpus").is_ok());
    }
}
