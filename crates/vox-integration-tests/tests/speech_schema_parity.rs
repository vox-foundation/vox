#![allow(missing_docs)]

//! Parity: failure taxonomy, speech trace schemas, and MENS `$ref` wiring.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn json_obj<'a>(v: &'a serde_json::Value) -> &'a serde_json::Map<String, serde_json::Value> {
    v.as_object().expect("expected JSON object")
}

fn enum_strings(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .expect("enum array")
        .iter()
        .map(|x| x.as_str().expect("enum string").to_string())
        .collect()
}

fn failure_category_values(schema: &serde_json::Value) -> Vec<String> {
    let props = json_obj(schema)
        .get("properties")
        .and_then(|p| p.as_object())
        .expect("schema.properties");
    let fc = props
        .get("failure_category")
        .expect("failure_category property");
    enum_strings(
        json_obj(fc)
            .get("enum")
            .expect("failure_category.enum"),
    )
}

#[test]
fn failure_taxonomy_matches_speech_trace_failure_category() {
    let root = workspace_root();
    let tax: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("contracts/speech-to-code/failure-taxonomy.schema.json"))
            .expect("read failure-taxonomy"),
    )
    .expect("parse failure-taxonomy");
    let trace: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("contracts/speech-to-code/speech_trace.schema.json"))
            .expect("read speech_trace"),
    )
    .expect("parse speech_trace");
    let mens: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("contracts/speech-to-code/speech_trace.mens.schema.json"))
            .expect("read speech_trace.mens"),
    )
    .expect("parse speech_trace.mens");

    let a: HashSet<_> = enum_strings(
        json_obj(&tax)
            .get("enum")
            .expect("taxonomy enum"),
    )
    .into_iter()
    .collect();
    let b: HashSet<_> = failure_category_values(&trace).into_iter().collect();
    let c: HashSet<_> = failure_category_values(&mens).into_iter().collect();
    assert_eq!(a, b, "speech_trace.failure_category vs failure-taxonomy");
    assert_eq!(b, c, "speech_trace.mens.failure_category vs speech_trace");
}

#[test]
fn mens_schema_reexports_contracts_extension() {
    let root = workspace_root();
    let mens_entry: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("mens/schemas/speech_to_code_trace.schema.json"))
            .expect("read mens speech_to_code_trace"),
    )
    .expect("parse mens speech_to_code_trace");
    let ref_ok = mens_entry
        .get("$ref")
        .and_then(|x| x.as_str())
        == Some("../../contracts/speech-to-code/speech_trace.mens.schema.json");
    assert!(
        ref_ok,
        "mens/schemas/speech_to_code_trace.schema.json must $ref contracts speech_trace.mens.schema.json"
    );
}

#[test]
fn speech_trace_mens_validates_minimal_training_row() {
    let root = workspace_root();
    let schema_src = fs::read_to_string(
        root.join("contracts/speech-to-code/speech_trace.mens.schema.json"),
    )
    .expect("read mens trace schema");
    let schema_val: serde_json::Value =
        serde_json::from_str(&schema_src).expect("parse mens trace schema");
    let validator = jsonschema::validator_for(&schema_val).expect("compile schema");

    let ok = serde_json::json!({
        "schema_version": "1",
        "session_id": "s1",
        "refined_transcript": "add hello",
        "vox_code": "fn hello() {}",
        "correlation_id": "c1",
        "compile_ok": true,
        "failure_category": "unknown"
    });
    validator.validate(&ok).expect("valid row should validate");

    let missing_code = serde_json::json!({
        "schema_version": "1",
        "session_id": "s1",
        "refined_transcript": "add hello"
    });
    assert!(
        validator.validate(&missing_code).is_err(),
        "missing vox_code must fail MENS schema"
    );
}

#[test]
fn kpi_baseline_schema_validates_instance() {
    let root = workspace_root();
    let schema_src = fs::read_to_string(root.join("contracts/speech-to-code/kpi-baseline.schema.json"))
        .expect("read kpi schema");
    let schema_val: serde_json::Value =
        serde_json::from_str(&schema_src).expect("parse kpi schema");
    let validator = jsonschema::validator_for(&schema_val).expect("compile kpi schema");
    let inst = serde_json::json!({
        "schema_version": "1",
        "captured_at_utc": "2026-03-26T12:00:00Z",
        "wer": 0.12,
        "cer": 0.05,
        "compile_pass_at_1": 0.8,
        "latency_ms_p95": 900.0
    });
    validator.validate(&inst).expect("kpi snapshot should validate");
}
