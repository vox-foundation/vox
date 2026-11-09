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

/// Extract the reasoning pre-score entries from a `reasoning_before_after.v1`
/// report: all six when `status` is `compared`, or the two of each slice in
/// `slices_requested` when it is `partial` (a `--slices` run). A partial
/// baseline feeds only `eval-collateral-damage --fail-fast`.
pub(crate) fn baseline_json_from_compare_value(compare: &Value) -> Result<Value> {
    let schema = compare.get("schema").and_then(Value::as_str).unwrap_or("");
    if schema != BEFORE_AFTER_SCHEMA {
        bail!("expected schema {BEFORE_AFTER_SCHEMA}, got {schema:?}");
    }
    let status = compare.get("status").and_then(Value::as_str).unwrap_or("");
    let names: Vec<&str> = match status {
        "compared" => SLICE_NAMES.to_vec(),
        "partial" => {
            let requested: Vec<&str> = compare
                .get("slices_requested")
                .and_then(Value::as_array)
                .context("partial compare missing slices_requested")?
                .iter()
                .map(|v| v.as_str().context("slices_requested entry is not a string"))
                .collect::<Result<_>>()?;
            if requested.is_empty() {
                bail!("partial compare names no slices_requested");
            }
            if let Some(bad) = requested.iter().find(|n| !SLICE_NAMES.contains(n)) {
                bail!("partial compare requests unknown slice {bad:?}");
            }
            requested
        }
        _ => bail!("compare status must be compared or partial, got {status:?}"),
    };
    let slices = compare
        .get("slices")
        .and_then(Value::as_object)
        .context("compare missing slices object")?;

    let mut benchmarks: Map<String, Value> = Map::new();
    for name in names {
        let slice = slices
            .get(name)
            .with_context(|| format!("compare missing slice {name}"))?;
        let acc = base_side_score(slice, name, "accuracy")?;
        let init = base_side_score(slice, name, "think_initiation_rate")?;
        benchmarks.insert(name.to_string(), json!(acc));
        benchmarks.insert(format!("{name}{INIT_SUFFIX}"), json!(init));
    }

    if status == "compared" {
        for key in reasoning_benchmark_keys() {
            if !benchmarks.contains_key(key) {
                bail!("internal error: baseline missing key {key}");
            }
        }
    }

    let mut out = json!({ "benchmarks": benchmarks });
    if let Some(decode) = compare.get("decode") {
        out["decode"] = decode.clone();
    }
    Ok(out)
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

    fn partial_fixture() -> Value {
        let mut doc = minimal_compared_fixture();
        let s = doc["slices"].as_object_mut().unwrap();
        s.retain(|k, _| k == "vox_think");
        doc["status"] = json!("partial");
        doc["slices_requested"] = json!(["vox_think"]);
        doc
    }

    #[test]
    fn a_partial_compare_yields_only_its_slice_keys() {
        let v = baseline_json_from_compare_value(&partial_fixture()).unwrap();
        let keys: Vec<&String> = v["benchmarks"].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["vox_think", "vox_think_init"]);
    }

    #[test]
    fn a_partial_compare_missing_its_requested_slice_is_refused() {
        let mut doc = partial_fixture();
        doc["slices_requested"] = json!(["vox_think", "gsm8k_think"]);
        let err = baseline_json_from_compare_value(&doc)
            .unwrap_err()
            .to_string();
        assert!(err.contains("gsm8k_think"), "{err}");
    }

    #[test]
    fn an_incomplete_compare_is_still_refused() {
        let mut doc = minimal_compared_fixture();
        doc["status"] = json!("incomplete");
        assert!(baseline_json_from_compare_value(&doc).is_err());
    }

    #[test]
    fn baseline_copies_decode_object_when_present_and_omits_when_absent() {
        let doc_no_decode = minimal_compared_fixture();
        let baseline = baseline_json_from_compare_value(&doc_no_decode).unwrap();
        assert!(baseline.get("decode").is_none());

        let mut doc_with_decode = minimal_compared_fixture();
        let decode_obj = json!({
            "protocol": "bf16",
            "bf16_gemm_rows": 32,
            "max_tokens": 2048,
            "temperature": 0.0,
            "seed": 0,
            "thinking": true,
        });
        doc_with_decode["decode"] = decode_obj.clone();
        let baseline_with = baseline_json_from_compare_value(&doc_with_decode).unwrap();
        assert_eq!(baseline_with["decode"], decode_obj);
    }
}
