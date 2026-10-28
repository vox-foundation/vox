//! Multi-size ladder before/after aggregate (BENCH-04). Operator/milestone evidence
//! only — never a serve-gate input (`gate_input` is always false at the root).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::reasoning_bench::BEFORE_AFTER_SCHEMA;

/// Top-level schema tag for the size-ladder rollup report.
pub const LADDER_BEFORE_AFTER_SCHEMA: &str = "mens_size_ladder_before_after.v1";

/// Paths and sidecar JSON for one **trained** ladder rung.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrainedSizeArtifacts {
    /// Adapter run directory (e.g. `mens/runs/qwen3-0.6b`).
    pub run_dir: PathBuf,
    /// `reasoning_before_after.v1` compare output for this size.
    pub reasoning_compare: PathBuf,
    /// Base-side `eval-local` report on heldout_bench.
    pub base_eval_local: PathBuf,
    /// Adapter-side `eval-local` report on heldout_bench.
    pub adapter_eval_local: PathBuf,
}

/// Score-like keys that must never appear on `not_trained` rungs.
const FORBIDDEN_NOT_TRAINED_KEYS: &[&str] = &[
    "accuracy",
    "think_initiation_rate",
    "pass_at_1",
    "pass_rate_at_1",
];

/// Read `pass_rate_at_1` (alias `pass_at_1`) and task count `n` from an eval-local JSON file.
pub fn intelligence_from_eval_local(path: &Path) -> Result<(f64, u64), String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let v: Value =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    let n = v
        .get("total")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{} missing numeric total", path.display()))?;
    let pass_at_1 = v
        .get("pass_at_1")
        .or_else(|| v.get("pass_rate_at_1"))
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{} missing pass_rate_at_1", path.display()))?;
    Ok((pass_at_1, n))
}

fn run_dir_for_size(size_label: &str) -> PathBuf {
    let slug = size_label.to_ascii_lowercase();
    PathBuf::from(format!("mens/runs/qwen3-{slug}"))
}

fn trained_row(size: &str, artifacts: &TrainedSizeArtifacts) -> Result<Value, String> {
    let (pass_at_1, n) = intelligence_from_eval_local(&artifacts.adapter_eval_local)?;
    Ok(serde_json::json!({
        "size": size,
        "status": "trained",
        "run_dir": artifacts.run_dir.display().to_string(),
        "reasoning": {
            "compare_report": artifacts.reasoning_compare.display().to_string(),
            "compare_schema": BEFORE_AFTER_SCHEMA,
        },
        "intelligence": {
            "base_eval_local": artifacts.base_eval_local.display().to_string(),
            "adapter_eval_local": artifacts.adapter_eval_local.display().to_string(),
            "pass_at_1": pass_at_1,
            "n": n,
        },
    }))
}

fn not_trained_row(size: &str, reason: &str) -> Value {
    serde_json::json!({
        "size": size,
        "status": "not_trained",
        "reason": reason,
    })
}

/// Build the ladder rollup from Phase 320 `ladder.json` (or equivalent) and per-size artifacts
/// for trained rungs only.
pub fn build_ladder_report(
    ladder_ssot: &Value,
    trained: &HashMap<String, TrainedSizeArtifacts>,
) -> Result<Value, String> {
    let sizes = ladder_ssot
        .get("sizes")
        .and_then(Value::as_array)
        .ok_or_else(|| "ladder_ssot missing sizes array".to_string())?;

    let mut rows: Vec<Value> = Vec::new();
    for entry in sizes {
        let size = entry
            .get("size")
            .and_then(Value::as_str)
            .ok_or_else(|| "ladder size entry missing size".to_string())?;
        let status = entry
            .get("s4_status")
            .and_then(Value::as_str)
            .unwrap_or("not_trained");
        if status == "trained" {
            let art = trained
                .get(size)
                .ok_or_else(|| format!("trained size {size} missing TrainedSizeArtifacts"))?;
            rows.push(trained_row(size, art)?);
        } else {
            let reason = entry
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("not trained on this host");
            rows.push(not_trained_row(size, reason));
        }
    }

    Ok(serde_json::json!({
        "schema": LADDER_BEFORE_AFTER_SCHEMA,
        "gate_input": false,
        "sizes": rows,
    }))
}

/// Returns true if `row` (a single size entry) contains forbidden score keys or a numeric `n`.
pub fn not_trained_row_has_forbidden_scores(row: &Value) -> bool {
    if row.get("status").and_then(Value::as_str) != Some("not_trained") {
        return false;
    }
    forbidden_score_keys_present(row)
}

/// Recursive walk: forbidden key names, or key `n` with a numeric value.
pub fn forbidden_score_keys_present(v: &Value) -> bool {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                if FORBIDDEN_NOT_TRAINED_KEYS.iter().any(|f| f == &k.as_str()) {
                    return true;
                }
                if k == "n" && child.is_number() {
                    return true;
                }
                if forbidden_score_keys_present(child) {
                    return true;
                }
            }
            false
        }
        Value::Array(items) => items.iter().any(forbidden_score_keys_present),
        _ => false,
    }
}

/// Convenience: default run dir from ladder size label (`0.6B` → `mens/runs/qwen3-0.6b`).
#[must_use]
pub fn default_run_dir_for_ladder_size(size_label: &str) -> PathBuf {
    run_dir_for_size(size_label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ladder_before_after")
    }

    fn load_fixture(name: &str) -> Value {
        let path = fixtures().join(name);
        let text = std::fs::read_to_string(&path).expect("read fixture");
        serde_json::from_str(&text).expect("parse fixture")
    }

    #[test]
    fn trained_fixture_row_has_reasoning_and_intelligence_with_n() {
        let ladder = load_fixture("ladder_ssot.json");
        let fix = fixtures();
        let mut trained = HashMap::new();
        trained.insert(
            "0.6B".to_string(),
            TrainedSizeArtifacts {
                run_dir: PathBuf::from("mens/runs/qwen3-0.6b"),
                reasoning_compare: fix.join("reasoning_compare_0.6b.json"),
                base_eval_local: fix.join("eval_local_base.json"),
                adapter_eval_local: fix.join("eval_local_adapter.json"),
            },
        );
        let report = build_ladder_report(&ladder, &trained).expect("build");
        assert_eq!(report["schema"], LADDER_BEFORE_AFTER_SCHEMA);
        assert_eq!(report["gate_input"], false);
        let rows = report["sizes"].as_array().expect("sizes array");
        let trained_row = rows
            .iter()
            .find(|r| r["status"] == "trained")
            .expect("trained row");
        assert_eq!(trained_row["run_dir"], "mens/runs/qwen3-0.6b");
        assert_eq!(
            trained_row["reasoning"]["compare_schema"],
            "reasoning_before_after.v1"
        );
        assert!(trained_row["reasoning"]["compare_report"].is_string());
        assert_eq!(trained_row["intelligence"]["n"], 52);
        assert!(
            trained_row["intelligence"]["pass_at_1"]
                .as_f64()
                .is_some_and(|x| x > 0.0)
        );
    }

    #[test]
    fn not_trained_rows_have_no_forbidden_score_keys() {
        let ladder = load_fixture("ladder_ssot.json");
        let mut ladder_only_nt = ladder.clone();
        if let Some(arr) = ladder_only_nt
            .get_mut("sizes")
            .and_then(Value::as_array_mut)
        {
            arr.retain(|e| e.get("s4_status").and_then(Value::as_str) != Some("trained"));
        }
        let report = build_ladder_report(&ladder_only_nt, &HashMap::new()).expect("build");
        for row in report["sizes"].as_array().expect("sizes") {
            assert_eq!(row["status"], "not_trained");
            assert!(
                !not_trained_row_has_forbidden_scores(row),
                "forbidden keys in {row}"
            );
            assert!(row.get("reason").and_then(Value::as_str).is_some());
        }
    }

    #[test]
    fn full_fixture_report_validates_schema_and_not_trained_invariants() {
        let ladder = load_fixture("ladder_ssot.json");
        let fix = fixtures();
        let mut trained = HashMap::new();
        trained.insert(
            "0.6B".to_string(),
            TrainedSizeArtifacts {
                run_dir: PathBuf::from("mens/runs/qwen3-0.6b"),
                reasoning_compare: fix.join("reasoning_compare_0.6b.json"),
                base_eval_local: fix.join("eval_local_base.json"),
                adapter_eval_local: fix.join("eval_local_adapter.json"),
            },
        );
        let report = build_ladder_report(&ladder, &trained).expect("build");
        let text = serde_json::to_string(&report).expect("serialize");
        let back: Value = serde_json::from_str(&text).expect("round-trip");
        assert_eq!(back["schema"], LADDER_BEFORE_AFTER_SCHEMA);
        for row in back["sizes"].as_array().expect("sizes") {
            if row["status"] == "not_trained" {
                assert!(!forbidden_score_keys_present(row));
            }
        }
    }

    #[test]
    fn forbidden_key_walk_detects_nested_accuracy() {
        let bad = serde_json::json!({
            "status": "not_trained",
            "detail": { "accuracy": 1.0 }
        });
        assert!(forbidden_score_keys_present(&bad));
    }
}
