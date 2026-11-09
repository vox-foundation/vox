//! Versioned gate state evaluation and policy hashing inputs for MENS receipts and spoke gates.
//!
//! Enforces the gate state truth table and canonical JSON representations across
//! `vox mens eval-gate`, `vox-populi` publish/pull, and Hub card facts.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const RECEIPT_SCHEMA: &str = "vox_mens_gate_receipt_v2";
pub const SPOKE_GATES_SCHEMA: &str = "vox_mens_spoke_gates_v1";

/// Four canonical states for receipts and spoke gate evaluations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateState {
    Passed,
    Failed,
    NeedsReGate,
    NotRun,
}

impl GateState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::NeedsReGate => "needs_re_gate",
            Self::NotRun => "not_run",
        }
    }
}

impl std::fmt::Display for GateState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The currently expected policy hash and optional adapter weight hash for gate verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentGate {
    pub policy_blake3: String,
    pub adapter_sha256: Option<String>,
}

/// Recursively canonicalize JSON by sorting map keys.
/// Serde-json workspace configuration has `preserve_order` enabled,
/// so sorting must be done explicitly into `BTreeMap`.
pub fn canonical_json(v: &Value) -> String {
    fn canonicalize(v: &Value) -> Value {
        match v {
            Value::Object(map) => {
                let mut sorted = BTreeMap::new();
                for (k, val) in map {
                    sorted.insert(k.clone(), canonicalize(val));
                }
                serde_json::to_value(sorted).unwrap()
            }
            Value::Array(arr) => {
                let sorted_arr: Vec<_> = arr.iter().map(canonicalize).collect();
                Value::Array(sorted_arr)
            }
            other => other.clone(),
        }
    }
    serde_json::to_string(&canonicalize(v)).expect("canonical json string")
}

/// Canonical payload for hashing effective gate policies.
/// Combines the parsed policy with code constants from `vox-eval`.
pub fn policy_hash_input(policy: &Value) -> Vec<u8> {
    let root = serde_json::json!({
        "policy": policy,
        "code": {
            "paired_rule": crate::reasoning_slice::PAIRED_RULE,
            "grader": crate::qwen_reasoning::GRADER_ID,
            "slice_size": crate::reasoning_bench::SLICE_SIZE,
        }
    });
    canonical_json(&root).into_bytes()
}

/// Count substantive gates in a receipt (excluding skipped, missing, or not applicable).
pub fn substantive_gate_count(receipt: &Value) -> usize {
    receipt["gates"].as_array().map_or(0, |g| {
        g.iter()
            .filter(|r| {
                let m = r["message"].as_str().unwrap_or("");
                !m.contains("not applicable") && !m.contains("skipped") && !m.contains("missing")
            })
            .count()
    })
}

/// Truth table resolution of gate state from a receipt and current policy/adapter expectation.
pub fn gate_state(receipt: Option<&Value>, current: &CurrentGate) -> (GateState, String) {
    let Some(r) = receipt else {
        return (GateState::NotRun, "no receipt".to_string());
    };

    // 1. Schema v1 or missing policy_blake3 -> NeedsReGate
    let is_v1 = r.get("schema").and_then(|s| s.as_str()) == Some("vox_mens_gate_receipt_v1");
    let policy_blake3 = r.get("policy_blake3").and_then(|s| s.as_str());
    if is_v1 || policy_blake3.is_none() {
        return (
            GateState::NeedsReGate,
            "receipt lacks policy_blake3 (re-run `vox mens eval-gate` for this size)".to_string(),
        );
    }

    let got_hash = policy_blake3.unwrap();
    // 2. policy_blake3 differs -> NeedsReGate
    if got_hash != current.policy_blake3 {
        return (
            GateState::NeedsReGate,
            format!(
                "policy_blake3 differs: receipt {got_hash}, current {} (re-run `vox mens eval-gate` for this size)",
                current.policy_blake3
            ),
        );
    }

    // 3. adapter_sha256 differs -> NeedsReGate
    if let Some(ref want_sha) = current.adapter_sha256 {
        let got_sha = r.pointer("/inputs/adapter_sha256").and_then(|s| s.as_str());
        if got_sha != Some(want_sha.as_str()) {
            return (
                GateState::NeedsReGate,
                format!("adapter_sha256 differs: receipt {got_sha:?}, current Some({want_sha:?})"),
            );
        }
    }

    // 4. Blocking row with passed == false -> Failed
    if let Some(gates) = r.get("gates").and_then(|g| g.as_array()) {
        for g in gates {
            let passed = g.get("passed").and_then(|b| b.as_bool()).unwrap_or(false);
            let block = g.get("block").and_then(|b| b.as_bool()).unwrap_or(false);
            if block && !passed {
                let name = g.get("name").and_then(|s| s.as_str()).unwrap_or("unknown");
                let msg = g
                    .get("message")
                    .and_then(|s| s.as_str())
                    .unwrap_or("gate failed");
                return (GateState::Failed, format!("gate {name} failed: {msg}"));
            }
        }
    }

    // 5. Non-discriminating row -> NotRun
    if let Some(gates) = r.get("gates").and_then(|g| g.as_array()) {
        for g in gates {
            if let Some(false) = g.get("discriminating").and_then(|b| b.as_bool()) {
                let name = g.get("name").and_then(|s| s.as_str()).unwrap_or("unknown");
                return (GateState::NotRun, format!("non_discriminating: {name}"));
            }
        }
    }

    // 6. Fewer than two substantive gates -> NotRun
    let count = substantive_gate_count(r);
    if count < 2 {
        return (
            GateState::NotRun,
            format!("fewer than two substantive gates ({count})"),
        );
    }

    // 7. Otherwise Passed
    (GateState::Passed, "all gates passed".to_string())
}

/// Row representing one spoke's gate status in `spoke_gates.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpokeGateRow {
    pub spoke: String,
    pub policy_file: String,
    pub policy_blake3: String,
    pub receipt_file: Option<String>,
    pub state: GateState,
    pub reason: String,
    pub applies: bool,
}

/// Format the top-level `spoke_gates.json` structure.
pub fn spoke_gates_json(
    adapter_sha256: Option<&str>,
    base_pin: Option<&str>,
    rows: &[SpokeGateRow],
) -> Value {
    serde_json::json!({
        "schema": SPOKE_GATES_SCHEMA,
        "adapter_sha256": adapter_sha256,
        "base_pin": base_pin,
        "rows": rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_hash_canonical_and_sensitive() {
        let v1: Value = serde_json::json!({
            "version": "1",
            "collateral_damage": {
                "alpha": 0.05,
                "block": true
            },
            "throughput": {
                "min_tokens_per_sec": 100.0
            }
        });
        let v2: Value = serde_json::json!({
            "throughput": {
                "min_tokens_per_sec": 100.0
            },
            "collateral_damage": {
                "block": true,
                "alpha": 0.05
            },
            "version": "1"
        });
        let v3: Value = serde_json::json!({
            "version": "1",
            "collateral_damage": {
                "alpha": 0.04,
                "block": true
            },
            "throughput": {
                "min_tokens_per_sec": 100.0
            }
        });

        let h1 = policy_hash_input(&v1);
        let h2 = policy_hash_input(&v2);
        let h3 = policy_hash_input(&v3);

        assert_eq!(
            h1, h2,
            "key ordering in input must produce identical canonical hash input"
        );
        assert_ne!(h1, h3, "value changes must change canonical hash input");
    }

    fn sample_valid_receipt(policy_blake3: &str, adapter_sha: &str) -> Value {
        serde_json::json!({
            "schema": RECEIPT_SCHEMA,
            "policy_blake3": policy_blake3,
            "source_commit": "abc1234",
            "overall_passed": true,
            "inputs": {
                "adapter_sha256": adapter_sha,
                "base_pin": "base_123",
                "slice_blake3": {},
                "grader": crate::qwen_reasoning::GRADER_ID,
                "decode_blake3": "dec123",
                "suites": {},
                "datasets": []
            },
            "gates": [
                {"name": "throughput", "passed": true, "block": true, "message": "150 tok/s >= 100"},
                {"name": "pass_at_k", "passed": true, "block": true, "message": "0.60 >= 0.50"}
            ]
        })
    }

    #[test]
    fn truth_table_1_no_receipt() {
        let current = CurrentGate {
            policy_blake3: "hash123".to_string(),
            adapter_sha256: Some("sha123".to_string()),
        };
        let (state, reason) = gate_state(None, &current);
        assert_eq!(state, GateState::NotRun);
        assert_eq!(reason, "no receipt");
    }

    #[test]
    fn truth_table_2_v1_or_missing_policy_blake3() {
        let current = CurrentGate {
            policy_blake3: "hash123".to_string(),
            adapter_sha256: Some("sha123".to_string()),
        };
        let mut r = sample_valid_receipt("hash123", "sha123");
        r["schema"] = serde_json::json!("vox_mens_gate_receipt_v1");
        let (state, reason) = gate_state(Some(&r), &current);
        assert_eq!(state, GateState::NeedsReGate);
        assert!(reason.contains("receipt lacks policy_blake3"));

        r["schema"] = serde_json::json!(RECEIPT_SCHEMA);
        r.as_object_mut().unwrap().remove("policy_blake3");
        let (state, reason) = gate_state(Some(&r), &current);
        assert_eq!(state, GateState::NeedsReGate);
        assert!(reason.contains("receipt lacks policy_blake3"));
    }

    #[test]
    fn truth_table_3_policy_blake3_differs() {
        let current = CurrentGate {
            policy_blake3: "hash_current".to_string(),
            adapter_sha256: Some("sha123".to_string()),
        };
        let r = sample_valid_receipt("hash_old", "sha123");
        let (state, reason) = gate_state(Some(&r), &current);
        assert_eq!(state, GateState::NeedsReGate);
        assert!(reason.contains("policy_blake3 differs"), "{reason}");
        assert!(reason.contains("hash_old") && reason.contains("hash_current"));
    }

    #[test]
    fn truth_table_4_adapter_sha256_differs() {
        let current = CurrentGate {
            policy_blake3: "hash123".to_string(),
            adapter_sha256: Some("sha_new".to_string()),
        };
        let r = sample_valid_receipt("hash123", "sha_old");
        let (state, reason) = gate_state(Some(&r), &current);
        assert_eq!(state, GateState::NeedsReGate);
        assert!(reason.contains("adapter_sha256 differs"), "{reason}");
    }

    #[test]
    fn truth_table_5_blocking_row_failed() {
        let current = CurrentGate {
            policy_blake3: "hash123".to_string(),
            adapter_sha256: Some("sha123".to_string()),
        };
        let mut r = sample_valid_receipt("hash123", "sha123");
        r["gates"][0]["passed"] = serde_json::json!(false);
        r["gates"][0]["message"] = serde_json::json!("10 tok/s < 100");
        let (state, reason) = gate_state(Some(&r), &current);
        assert_eq!(state, GateState::Failed);
        assert!(reason.contains("gate throughput failed"), "{reason}");
    }

    #[test]
    fn truth_table_6_discriminating_false() {
        let current = CurrentGate {
            policy_blake3: "hash123".to_string(),
            adapter_sha256: Some("sha123".to_string()),
        };
        let mut r = sample_valid_receipt("hash123", "sha123");
        r["gates"][0]["discriminating"] = serde_json::json!(false);
        let (state, reason) = gate_state(Some(&r), &current);
        assert_eq!(state, GateState::NotRun);
        assert!(reason.starts_with("non_discriminating:"), "{reason}");
    }

    #[test]
    fn truth_table_7_fewer_than_two_substantive_rows() {
        let current = CurrentGate {
            policy_blake3: "hash123".to_string(),
            adapter_sha256: Some("sha123".to_string()),
        };
        let mut r = sample_valid_receipt("hash123", "sha123");
        r["gates"][0]["message"] = serde_json::json!("skipped regression check");
        let (state, reason) = gate_state(Some(&r), &current);
        assert_eq!(state, GateState::NotRun);
        assert!(
            reason.contains("fewer than two substantive gates"),
            "{reason}"
        );
    }

    #[test]
    fn truth_table_8_passed() {
        let current = CurrentGate {
            policy_blake3: "hash123".to_string(),
            adapter_sha256: Some("sha123".to_string()),
        };
        let r = sample_valid_receipt("hash123", "sha123");
        let (state, reason) = gate_state(Some(&r), &current);
        assert_eq!(state, GateState::Passed);
        assert_eq!(reason, "all gates passed");
    }
}
