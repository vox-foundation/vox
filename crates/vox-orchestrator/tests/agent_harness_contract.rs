use std::path::PathBuf;

#[test]
fn agent_harness_projection_validates_against_contract_schema() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_path = root.join("../../contracts/orchestration/agent-harness.schema.json");
    let schema_text = std::fs::read_to_string(&schema_path).expect("read schema");
    let schema: serde_json::Value = serde_json::from_str(&schema_text).expect("parse schema");
    let validator = jsonschema::validator_for(&schema).expect("compile schema validator");

    let harness = vox_orchestrator::AgentHarnessSpec::minimal_contract_first(
        "repo-contract-test",
        "Validate the portable harness contract against the SSOT schema.",
        Some("sid-contract-test"),
        Some("thread-contract-test"),
        &["artifacts/response.md".to_string()],
    );
    let instance = serde_json::to_value(&harness).expect("serialize harness");
    validator.validate(&instance).expect("validate against schema");
}
