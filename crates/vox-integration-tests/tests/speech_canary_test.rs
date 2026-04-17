#![allow(missing_docs)]

//! Optional canary gate: set `VOX_SPEECH_CANARY_KPI` to a JSON file validated against `kpi-baseline.schema.json`
//! and checked against `contracts/speech-to-code/canary_policy.example.json` thresholds.

use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[derive(Debug, Deserialize)]
struct CanaryPolicy {
    #[allow(dead_code)]
    schema_version: String,
    #[allow(dead_code)]
    description: Option<String>,
    min_compile_pass_at_1: f64,
    min_compile_pass_at_k: f64,
    max_latency_ms_p95: f64,
    max_wer: f64,
    max_cer: f64,
}

#[derive(Debug, Deserialize)]
struct KpiSnapshot {
    compile_pass_at_1: Option<f64>,
    compile_pass_at_k: Option<f64>,
    latency_ms_p95: Option<f64>,
    wer: Option<f64>,
    cer: Option<f64>,
}

#[test]
fn canary_example_policy_parses() {
    let root = workspace_root();
    let raw = fs::read_to_string(root.join("contracts/speech-to-code/canary_policy.example.json"))
        .expect("read canary policy example");
    let _: CanaryPolicy = serde_json::from_str(&raw).expect("parse canary policy");
}

#[test]
fn speech_canary_kpi_env_optional_gate() {
    let Some(path) = std::env::var_os("VOX_SPEECH_CANARY_KPI") else {
        return;
    };
    let root = workspace_root();
    let policy_raw =
        fs::read_to_string(root.join("contracts/speech-to-code/canary_policy.example.json"))
            .expect("policy example");
    let policy: CanaryPolicy = serde_json::from_str(&policy_raw).expect("policy");

    let kpi_raw =
        fs::read_to_string(PathBuf::from(&path)).expect("read VOX_SPEECH_CANARY_KPI file");
    let kpi_val: serde_json::Value = serde_json::from_str(&kpi_raw).expect("parse kpi json");
    let schema_src =
        fs::read_to_string(root.join("contracts/speech-to-code/kpi-baseline.schema.json"))
            .expect("kpi schema");
    let schema_val: serde_json::Value = serde_json::from_str(&schema_src).expect("parse schema");
    let validator = jsonschema::validator_for(&schema_val).expect("compile kpi schema");
    validator
        .validate(&kpi_val)
        .expect("KPI snapshot must match kpi-baseline.schema.json");

    let kpi: KpiSnapshot = serde_json::from_value(kpi_val).expect("kpi struct");

    if let Some(v) = kpi.compile_pass_at_1 {
        assert!(
            v + f64::EPSILON >= policy.min_compile_pass_at_1,
            "compile_pass_at_1 {v} below {}",
            policy.min_compile_pass_at_1
        );
    }
    if let Some(v) = kpi.compile_pass_at_k {
        assert!(
            v + f64::EPSILON >= policy.min_compile_pass_at_k,
            "compile_pass_at_k {v} below {}",
            policy.min_compile_pass_at_k
        );
    }
    if let Some(v) = kpi.latency_ms_p95 {
        assert!(
            v <= policy.max_latency_ms_p95 + f64::EPSILON,
            "latency_ms_p95 {v} above {}",
            policy.max_latency_ms_p95
        );
    }
    if let Some(v) = kpi.wer {
        assert!(
            v <= policy.max_wer + f64::EPSILON,
            "wer {v} above {}",
            policy.max_wer
        );
    }
    if let Some(v) = kpi.cer {
        assert!(
            v <= policy.max_cer + f64::EPSILON,
            "cer {v} above {}",
            policy.max_cer
        );
    }
}
