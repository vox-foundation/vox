//! Guardrail: keep `contracts/orchestration/context-lifecycle-telemetry.fixtures.json` in lockstep with
//! `context-lifecycle-telemetry.schema.json` whenever shadow telemetry fields change.

use std::path::PathBuf;

#[test]
fn context_lifecycle_telemetry_fixtures_validate_against_schema() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_path = root.join("../../contracts/orchestration/context-lifecycle-telemetry.schema.json");
    let schema_text = std::fs::read_to_string(&schema_path).expect("read schema");
    let schema: serde_json::Value = serde_json::from_str(&schema_text).expect("parse schema");
    let validator = jsonschema::validator_for(&schema).expect("compile schema validator");

    let fixtures_path = root.join("../../contracts/orchestration/context-lifecycle-telemetry.fixtures.json");
    let fixtures_raw = std::fs::read_to_string(&fixtures_path).expect("read fixtures");
    let fixtures: Vec<serde_json::Value> =
        serde_json::from_str(&fixtures_raw).expect("parse fixtures array");

    for (i, instance) in fixtures.iter().enumerate() {
        validator
            .validate(instance)
            .unwrap_or_else(|e| panic!("fixture {i} failed schema validation: {e}"));
    }
}
