//! `vox ci scientia-worthiness-contract` — JSON Schema validate default worthiness YAML + invariants.

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde_json::Value as JsonValue;

const SCHEMA_REL: &str = "contracts/scientia/publication-worthiness.schema.json";
const DATA_REL: &str = "contracts/scientia/publication-worthiness.default.yaml";

pub fn run(repo_root: &Path) -> Result<()> {
    let schema_path = repo_root.join(SCHEMA_REL);
    let data_path = repo_root.join(DATA_REL);

    let schema_src = std::fs::read_to_string(&schema_path)
        .with_context(|| format!("read {}", schema_path.display()))?;
    let yaml_src = std::fs::read_to_string(&data_path)
        .with_context(|| format!("read {}", data_path.display()))?;

    let schema_val: JsonValue = serde_json::from_str(&schema_src)
        .with_context(|| format!("parse {}", schema_path.display()))?;
    let data_val: JsonValue = serde_yaml::from_str(&yaml_src)
        .with_context(|| format!("parse YAML {}", data_path.display()))?;

    let validator = jsonschema::validator_for(&schema_val)
        .with_context(|| format!("compile JSON Schema {}", schema_path.display()))?;
    validator.validate(&data_val).map_err(|e| {
        anyhow!(
            "{} failed validation against {}: {e}",
            data_path.display(),
            schema_path.display()
        )
    })?;

    let contract = vox_publisher::publication_worthiness::load_contract_from_str(&yaml_src)?;
    vox_publisher::publication_worthiness::validate_contract_invariants(&contract)
        .context("worthiness contract invariants")?;

    println!(
        "OK: {} + invariants match {}",
        data_path.display(),
        schema_path.display()
    );
    Ok(())
}
