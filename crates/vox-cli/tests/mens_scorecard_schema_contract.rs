use anyhow::Result;
use serde_json::Value;
use vox_jsonschema_util::compile_validator;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn read_json(path: &std::path::Path) -> Result<Value> {
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

#[test]
fn scorecard_input_schema_accepts_baseline_and_rejects_invalid_fixture() -> Result<()> {
    let root = repo_root();
    let schema = read_json(&root.join("contracts/eval/mens-scorecard.schema.json"))?;
    let validator = compile_validator(&schema, "mens-scorecard.schema.json")?;

    let baseline = read_json(&root.join("contracts/eval/mens-scorecard.baseline.json"))?;
    assert!(validator.is_valid(&baseline));

    let invalid = read_json(&root.join("contracts/eval/mens-scorecard.invalid.json"))?;
    assert!(!validator.is_valid(&invalid));
    Ok(())
}

#[test]
fn scorecard_output_schemas_accept_sample_golden_artifacts() -> Result<()> {
    let root = repo_root();

    let summary_schema =
        read_json(&root.join("contracts/eval/mens-scorecard-summary.schema.json"))?;
    let summary_sample =
        read_json(&root.join("crates/vox-cli/tests/fixtures/mens_scorecard_summary_sample.json"))?;
    assert!(
        compile_validator(&summary_schema, "mens-scorecard-summary.schema.json")?
            .is_valid(&summary_sample)
    );

    let event_schema = read_json(&root.join("contracts/eval/mens-scorecard-event.schema.json"))?;
    let event_sample =
        read_json(&root.join("crates/vox-cli/tests/fixtures/mens_scorecard_event_sample.json"))?;
    assert!(
        compile_validator(&event_schema, "mens-scorecard-event.schema.json")?.is_valid(&event_sample)
    );
    Ok(())
}
