//! Pure evaluation evidence checker for collateral reports.
//!
//! Enforces that collateral damage reports are current evidence (D-18, SPOKE-01):
//! matching current slice pins, policy rule, policy alpha, and canonical grader ID,
//! with sample-level counts re-verified by [`crate::reasoning_slice::verify_reasoning_slices`].

use std::collections::HashMap;

/// Current collateral gating requirements against which a report is evaluated.
#[derive(Debug, Clone, PartialEq)]
pub struct CollateralRequirement {
    /// Slice name -> expected Blake3 hash of the item set.
    pub slices: HashMap<String, String>,
    /// Canonical rule (e.g. [`crate::reasoning_slice::PAIRED_RULE`]).
    pub rule: String,
    /// Test significance level alpha (e.g. 0.05).
    pub alpha: f64,
    /// Canonical grader ID (e.g. [`crate::qwen_reasoning::GRADER_ID`]).
    pub grader: String,
}

impl CollateralRequirement {
    /// Parse from `eval-gates-ladder.yaml` loaded as a JSON/YAML Value.
    pub fn from_policy_json(value: &serde_json::Value) -> Result<Self, String> {
        let cd = value.get("collateral_damage").unwrap_or(value);
        let alpha = cd
            .get("alpha")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| "collateral_damage.alpha is missing or not a float".to_string())?;

        let rule = match cd.get("rule").and_then(|v| v.as_str()) {
            Some("fixed_n") | None => crate::reasoning_slice::PAIRED_RULE.to_string(),
            Some(r) => r.to_string(),
        };

        let grader = cd
            .get("grader")
            .or_else(|| cd.get("grader_id"))
            .and_then(|v| v.as_str())
            .unwrap_or(crate::qwen_reasoning::GRADER_ID)
            .to_string();

        let slices_val = cd
            .get("slices")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "collateral_damage.slices is missing or not an object".to_string())?;

        let mut slices = HashMap::new();
        for (name, val) in slices_val {
            let hash = val
                .get("blake3")
                .and_then(|v| v.as_str())
                .or_else(|| val.as_str())
                .ok_or_else(|| format!("slice {name} has no blake3 hash"))?;
            slices.insert(name.clone(), hash.to_string());
        }

        Ok(Self {
            slices,
            rule,
            alpha,
            grader,
        })
    }
}

/// Refusal reason when a collateral report cannot be accepted as evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollateralRefusal {
    /// A genuine regression or evaluation failure.
    Failed(String),
    /// A report that is stale, unpinned, or using mismatched rules/graders.
    Stale(String),
}

impl std::fmt::Display for CollateralRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed(msg) => write!(f, "collateral check failed: {msg}"),
            Self::Stale(msg) => write!(f, "collateral check stale: {msg}"),
        }
    }
}

impl std::error::Error for CollateralRefusal {}

/// Check reasoning_slices.rule and reasoning_slices.alpha against requirement.
fn require_rule_alpha(
    report_json: &serde_json::Value,
    req: &CollateralRequirement,
) -> Result<(), CollateralRefusal> {
    let rs = report_json.get("reasoning_slices");
    let rule = rs.and_then(|r| r.get("rule")).and_then(|v| v.as_str());
    match rule {
        Some(r) if r == req.rule => {}
        Some(r) => {
            return Err(CollateralRefusal::Stale(format!(
                "report reasoning_slices.rule {r:?} does not match required {} (re-run `vox mens eval-collateral-damage` for this size)",
                req.rule
            )));
        }
        None => {
            return Err(CollateralRefusal::Stale(format!(
                "report lacks reasoning_slices.rule field (required: {}) (re-run `vox mens eval-collateral-damage` for this size)",
                req.rule
            )));
        }
    }

    let alpha = rs.and_then(|r| r.get("alpha")).and_then(|v| v.as_f64());
    match alpha {
        Some(a) if (a - req.alpha).abs() <= 1e-6 => {}
        Some(a) => {
            return Err(CollateralRefusal::Stale(format!(
                "report reasoning_slices.alpha {a} does not match required {} (re-run `vox mens eval-collateral-damage` for this size)",
                req.alpha
            )));
        }
        None => {
            return Err(CollateralRefusal::Stale(format!(
                "report lacks reasoning_slices.alpha field (required: {}) (re-run `vox mens eval-collateral-damage` for this size)",
                req.alpha
            )));
        }
    }
    Ok(())
}

/// Check slice BLAKE3 pins against requirement.
fn require_slice_pins(
    report_json: &serde_json::Value,
    req: &CollateralRequirement,
) -> Result<(), CollateralRefusal> {
    let report_slices = report_json
        .get("slice_blake3")
        .or_else(|| report_json.get("slices"))
        .and_then(|v| v.as_object());

    let Some(slices_obj) = report_slices else {
        return Err(CollateralRefusal::Stale(
            "report lacks slice_blake3 or slices object (re-run `vox mens eval-collateral-damage` for this size)".to_string(),
        ));
    };

    for (name, expected_hash) in &req.slices {
        let got_hash = slices_obj.get(name).and_then(|v| {
            v.get("blake3")
                .and_then(|b| b.as_str())
                .or_else(|| v.as_str())
        });
        match got_hash {
            Some(got) if got == expected_hash => {}
            Some(got) => {
                return Err(CollateralRefusal::Stale(format!(
                    "slice {name} was scored on {got}, expected {expected_hash} (re-run `vox mens eval-collateral-damage` for this size)"
                )));
            }
            None => {
                return Err(CollateralRefusal::Stale(format!(
                    "report is missing slice {name} (expected {expected_hash}) (re-run `vox mens eval-collateral-damage` for this size)"
                )));
            }
        }
    }
    Ok(())
}

/// Check grader ID against requirement.
fn require_grader(
    report_json: &serde_json::Value,
    req: &CollateralRequirement,
) -> Result<(), CollateralRefusal> {
    let grader = report_json
        .get("grader")
        .or_else(|| report_json.get("grader_id"))
        .and_then(|v| v.as_str());
    match grader {
        Some(g) if g == req.grader => Ok(()),
        Some(g) => Err(CollateralRefusal::Stale(format!(
            "report grader {g:?} does not match required {} (re-run `vox mens eval-collateral-damage` for this size)",
            req.grader
        ))),
        None => Err(CollateralRefusal::Stale(format!(
            "report lacks grader field (required: {}) (re-run `vox mens eval-collateral-damage` for this size)",
            req.grader
        ))),
    }
}

/// Pure evaluation evidence checker.
///
/// Verifies a collateral report against the required pins, rule, alpha, and grader.
pub fn check_collateral_report(
    report_json: &serde_json::Value,
    req: &CollateralRequirement,
) -> Result<(), CollateralRefusal> {
    // 1. If top-level report overall_passed or status is false / "fail" / "failed", return Failed.
    let status_str = report_json
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let overall_passed = report_json.get("overall_passed").and_then(|v| v.as_bool());

    if overall_passed == Some(false) || status_str == "fail" || status_str == "failed" {
        let msg = match report_json.get("failed_on").and_then(|v| v.as_str()) {
            Some(bench) => format!("the collateral report status is `{status_str}` on `{bench}`"),
            None => format!("the collateral report status is `{status_str}`"),
        };
        return Err(CollateralRefusal::Failed(msg));
    }
    if status_str != "pass" {
        let msg = match report_json.get("failed_on").and_then(|v| v.as_str()) {
            Some(bench) => format!("the collateral report status is `{status_str}` on `{bench}`"),
            None => format!("the collateral report status is `{status_str}`"),
        };
        return Err(CollateralRefusal::Failed(msg));
    }

    // 2. Rule and alpha check.
    require_rule_alpha(report_json, req)?;

    // 3. Slices pins check.
    require_slice_pins(report_json, req)?;

    // 4. Grader check.
    require_grader(report_json, req)?;

    // 5. Delegate sample-level counts to verify_reasoning_slices.
    if let Err(e) = crate::reasoning_slice::verify_reasoning_slices(report_json, req.alpha) {
        if e.contains("regressed") || e.contains("regression") {
            return Err(CollateralRefusal::Failed(e));
        } else {
            return Err(CollateralRefusal::Stale(e));
        }
    }

    // 6. All checks pass.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning_bench::{SLICE_NAMES, SLICE_SIZE};
    use crate::reasoning_slice::{
        PAIRED_RULE, ReasoningSliceVerdict, eval_reasoning_slice, pooled_accuracy,
        reasoning_slices_json,
    };

    const ALPHA: f64 = 0.05;

    fn flags(n: usize, f: impl Fn(usize) -> bool) -> Vec<(String, bool)> {
        (0..n).map(|i| (format!("item_{i}"), f(i))).collect()
    }

    fn passing_verdicts() -> Vec<ReasoningSliceVerdict> {
        let base = flags(SLICE_SIZE, |i| i < 16);
        let adapter = flags(SLICE_SIZE, |i| i < 15);
        let all_true = flags(SLICE_SIZE, |_| true);
        SLICE_NAMES
            .iter()
            .map(|s| {
                eval_reasoning_slice(s, (&base, &adapter), (&all_true, &all_true), ALPHA).unwrap()
            })
            .collect()
    }

    fn sample_requirement() -> CollateralRequirement {
        let mut slices = HashMap::new();
        slices.insert("gsm8k_think".to_string(), "hash_gsm8k".to_string());
        slices.insert("math500_think".to_string(), "hash_math500".to_string());
        slices.insert("vox_think".to_string(), "hash_vox".to_string());
        CollateralRequirement {
            slices,
            rule: PAIRED_RULE.to_string(),
            alpha: ALPHA,
            grader: crate::qwen_reasoning::GRADER_ID.to_string(),
        }
    }

    fn fresh_passing_report(req: &CollateralRequirement) -> serde_json::Value {
        let verdicts = passing_verdicts();
        let pooled = pooled_accuracy(&verdicts, req.alpha).unwrap();
        let mut slices_map = serde_json::Map::new();
        for (k, v) in &req.slices {
            slices_map.insert(k.clone(), serde_json::Value::String(v.clone()));
        }
        serde_json::json!({
            "status": "pass",
            "failed_on": null,
            "threshold": req.alpha,
            "reports": [],
            "reasoning_slices": reasoning_slices_json(&verdicts, Some(&pooled), req.alpha),
            "slice_blake3": slices_map,
            "grader": req.grader,
        })
    }

    #[test]
    fn test_fresh_passed_report_accepted() {
        let req = sample_requirement();
        let report = fresh_passing_report(&req);
        assert_eq!(check_collateral_report(&report, &req), Ok(()));
    }

    #[test]
    fn test_stale_pin_refused() {
        let req = sample_requirement();
        let mut report = fresh_passing_report(&req);
        report["slice_blake3"]["gsm8k_think"] = serde_json::json!("wrong_hash");
        let result = check_collateral_report(&report, &req);
        match result {
            Err(CollateralRefusal::Stale(msg)) => {
                assert!(msg.contains("gsm8k_think"), "{msg}");
                assert!(msg.contains("expected hash_gsm8k"), "{msg}");
            }
            other => panic!("expected CollateralRefusal::Stale, got {other:?}"),
        }

        // Mutation check: missing pin
        let mut report_missing = fresh_passing_report(&req);
        report_missing["slice_blake3"]
            .as_object_mut()
            .unwrap()
            .remove("vox_think");
        match check_collateral_report(&report_missing, &req) {
            Err(CollateralRefusal::Stale(msg)) => {
                assert!(msg.contains("vox_think"), "{msg}");
            }
            other => panic!("expected CollateralRefusal::Stale for missing pin, got {other:?}"),
        }
    }

    #[test]
    fn test_stale_rule_or_alpha_refused() {
        let req = sample_requirement();

        // Stale rule
        let mut report_bad_rule = fresh_passing_report(&req);
        report_bad_rule["reasoning_slices"]["rule"] = serde_json::json!("retired_rule_v0");
        match check_collateral_report(&report_bad_rule, &req) {
            Err(CollateralRefusal::Stale(msg)) => {
                assert!(msg.contains("rule"), "{msg}");
            }
            other => panic!("expected CollateralRefusal::Stale for bad rule, got {other:?}"),
        }

        // Stale alpha
        let mut report_bad_alpha = fresh_passing_report(&req);
        report_bad_alpha["reasoning_slices"]["alpha"] = serde_json::json!(0.01);
        match check_collateral_report(&report_bad_alpha, &req) {
            Err(CollateralRefusal::Stale(msg)) => {
                assert!(msg.contains("alpha"), "{msg}");
            }
            other => panic!("expected CollateralRefusal::Stale for bad alpha, got {other:?}"),
        }
    }

    #[test]
    fn test_stale_grader_refused() {
        let req = sample_requirement();
        let mut report = fresh_passing_report(&req);
        report["grader"] = serde_json::json!("grade_final_answer v1");
        match check_collateral_report(&report, &req) {
            Err(CollateralRefusal::Stale(msg)) => {
                assert!(msg.contains("grader"), "{msg}");
            }
            other => panic!("expected CollateralRefusal::Stale for bad grader, got {other:?}"),
        }
    }

    #[test]
    fn test_genuine_failure_returns_failed_not_stale() {
        let req = sample_requirement();
        let mut report = fresh_passing_report(&req);
        report["status"] = serde_json::json!("fail");
        report["failed_on"] = serde_json::json!("math500_think");
        match check_collateral_report(&report, &req) {
            Err(CollateralRefusal::Failed(msg)) => {
                assert!(msg.contains("math500_think"), "{msg}");
            }
            other => panic!("expected CollateralRefusal::Failed, got {other:?}"),
        }
    }

    #[test]
    fn test_from_policy_json() {
        let val = serde_json::json!({
            "version": "1",
            "collateral_damage": {
                "alpha": 0.05,
                "rule": "fixed_n",
                "slices": {
                    "gsm8k_think": {
                        "blake3": "8425aff2fe6078b4ee8ee879fdaa2bfbc74df8fab96311800a6586748f383a5e"
                    },
                    "math500_think": {
                        "blake3": "277b1ce69176891d1ed81ef0195b2616dcba3a695d9c5a386061a0d2a80719ed"
                    },
                    "vox_think": {
                        "blake3": "21a201449c03a7fc02ddff45e7a65df537b4ec3f1733bbbaee39e84e378c6102"
                    }
                }
            }
        });
        let req = CollateralRequirement::from_policy_json(&val).unwrap();
        assert_eq!(req.alpha, 0.05);
        assert_eq!(req.rule, PAIRED_RULE);
        assert_eq!(req.grader, crate::qwen_reasoning::GRADER_ID);
        assert_eq!(
            req.slices.get("gsm8k_think").unwrap(),
            "8425aff2fe6078b4ee8ee879fdaa2bfbc74df8fab96311800a6586748f383a5e"
        );
    }
}
