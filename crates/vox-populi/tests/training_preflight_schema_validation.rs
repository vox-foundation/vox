//! Emit [`TrainingPreflightRecord`] JSON validates against `contracts/mens/training-preflight.schema.json`.

#![cfg(feature = "mens-train")]

use jsonschema::Validator;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[test]
fn training_preflight_record_validates_against_contract_schema() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_path = manifest_dir.join("../../contracts/mens/training-preflight.schema.json");
    let schema_raw = fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display()));
    let schema_val: serde_json::Value = serde_json::from_str(&schema_raw).expect("schema JSON");
    let compiled = Validator::new(&schema_val).expect("compile JSON Schema");

    let good = json!({
        "schema_version": "vox.mens.preflight.v0",
        "contract_digest": "sha256:test",
        "execution_kernel": "qlora",
        "notes": ["ok"]
    });
    compiled
        .validate(&good)
        .expect("valid preflight record must validate");

    let missing_digest = json!({
        "schema_version": "vox.mens.preflight.v0",
        "execution_kernel": "qlora"
    });
    assert!(
        compiled.validate(&missing_digest).is_err(),
        "schema should require contract_digest"
    );
}
