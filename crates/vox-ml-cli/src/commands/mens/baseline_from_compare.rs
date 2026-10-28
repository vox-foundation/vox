//! Build eval-collateral-damage pre-score JSON from a compared reasoning report.

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use vox_eval::reasoning_bench::{BEFORE_AFTER_SCHEMA, SLICE_NAMES};

const INIT_SUFFIX: &str = "_init";

/// Six reasoning keys expected in a collateral pre-score baseline.
pub(crate) fn reasoning_benchmark_keys() -> [&'static str; 6] {
    [
        "gsm8k_think",
        "gsm8k_think_init",
        "math500_think",
        "math500_think_init",
        "vox_think",
        "vox_think_init",
    ]
}

fn base_side_score(slice: &Value, slice_name: &str, field: &str) -> Result<f64> {
    let side = slice
        .get("base")
        .with_context(|| format!("compare slices.{slice_name} missing base side"))?;
    let status = side.get("status").and_then(Value::as_str).unwrap_or("");
    if status != "scored" {
        bail!("compare slices.{slice_name}.base is not scored (status={status:?})");
    }
    side.get(field)
        .and_then(Value::as_f64)
        .with_context(|| format!("compare slices.{slice_name}.base missing {field}"))
}

/// Extract the six reasoning pre-score entries from a `reasoning_before_after.v1`
/// report whose `status` is `compared`.
pub(crate) fn baseline_json_from_compare_value(compare: &Value) -> Result<Value> {
    let schema = compare.get("schema").and_then(Value::as_str).unwrap_or("");
    if schema != BEFORE_AFTER_SCHEMA {
        bail!("expected schema {BEFORE_AFTER_SCHEMA}, got {schema:?}");
    }
    let status = compare.get("status").and_then(Value::as_str).unwrap_or("");
    if status != "compared" {
        bail!("compare status must be compared, got {status:?}");
    }
    let slices = compare
        .get("slices")
        .and_then(Value::as_object)
        .context("compare missing slices object")?;

    let mut benchmarks: Map<String, Value> = Map::new();
    for name in SLICE_NAMES {
        let slice = slices
            .get(name)
            .with_context(|| format!("compare missing slice {name}"))?;
        let acc = base_side_score(slice, name, "accuracy")?;
        let init = base_side_score(slice, name, "think_initiation_rate")?;
        benchmarks.insert(name.to_string(), json!(acc));
        benchmarks.insert(format!("{name}{INIT_SUFFIX}"), json!(init));
    }

    for key in reasoning_benchmark_keys() {
        if !benchmarks.contains_key(key) {
            bail!("internal error: baseline missing key {key}");
        }
    }

    Ok(json!({ "benchmarks": benchmarks }))
}

/// Read `compare_path` and return collateral pre-score JSON (no file write).
pub(crate) fn baseline_json_from_compare(compare_path: &Path) -> Result<Value> {
    let raw = std::fs::read_to_string(compare_path)
        .with_context(|| format!("read compare report {}", compare_path.display()))?;
    let compare: Value = serde_json::from_str(&raw).context("parse compare report as JSON")?;
    baseline_json_from_compare_value(&compare)
}

#[cfg(all(test, feature = "gpu"))]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn minimal_compared_fixture() -> Value {
        let mut slices = Map::new();
        for name in SLICE_NAMES {
            slices.insert(
                (*name).to_string(),
                json!({
                    "base": {
                        "status": "scored",
                        "n": 32,
                        "accuracy": 0.25,
                        "think_initiation_rate": 0.5
                    },
                    "adapter": {
                        "status": "scored",
                        "n": 32,
                        "accuracy": 0.75,
                        "think_initiation_rate": 0.6
                    }
                }),
            );
        }
        json!({
            "schema": BEFORE_AFTER_SCHEMA,
            "status": "compared",
            "slices": slices
        })
    }

    #[test]
    fn baseline_from_compared_extracts_six_base_keys() {
        let v = baseline_json_from_compare_value(&minimal_compared_fixture()).unwrap();
        let b = v.get("benchmarks").unwrap().as_object().unwrap();
        assert_eq!(b.get("gsm8k_think").and_then(|x| x.as_f64()), Some(0.25));
        assert_eq!(
            b.get("gsm8k_think_init").and_then(|x| x.as_f64()),
            Some(0.5)
        );
        assert_eq!(b.get("vox_think_init").and_then(|x| x.as_f64()), Some(0.5));
        assert_eq!(b.len(), 6);
    }

    #[test]
    fn missing_base_slice_is_hard_error_without_partial_write() {
        let mut doc = minimal_compared_fixture();
        doc["slices"].as_object_mut().unwrap().remove("vox_think");
        let err = baseline_json_from_compare_value(&doc)
            .unwrap_err()
            .to_string();
        assert!(err.contains("vox_think"), "{err}");
    }

    #[test]
    fn baseline_from_compare_path_reads_file() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            "{}",
            serde_json::to_string(&minimal_compared_fixture()).unwrap()
        )
        .unwrap();
        let v = baseline_json_from_compare(f.path()).unwrap();
        assert!(v.get("benchmarks").unwrap().get("math500_think").is_some());
    }
}
