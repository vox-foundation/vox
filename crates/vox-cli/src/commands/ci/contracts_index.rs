//! `vox ci contracts-index` — validate [`contracts/index.yaml`](../../../../../contracts/index.yaml) against JSON Schema and on-disk paths.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::fs;
use std::path::Path;

const INDEX_REL: &str = "contracts/index.yaml";
const SCHEMA_REL: &str = "contracts/index.schema.json";

#[derive(Debug, Deserialize)]
struct IndexFile {
    #[allow(dead_code)]
    schema_version: u32,
    contracts: Vec<IndexContract>,
}

#[derive(Debug, Deserialize)]
struct IndexContract {
    id: String,
    path: String,
}

/// Validate contract index: JSON Schema + every listed `path` exists.
pub fn run(repo_root: &Path) -> Result<()> {
    let index_path = repo_root.join(INDEX_REL);
    let raw = fs::read_to_string(&index_path)
        .with_context(|| format!("read {}", index_path.display()))?;

    let schema_path = repo_root.join(SCHEMA_REL);
    if !schema_path.is_file() {
        return Err(anyhow!("missing {}", schema_path.display()));
    }
    let schema_val: JsonValue = serde_json::from_str(&fs::read_to_string(&schema_path)?)
        .with_context(|| format!("parse {}", schema_path.display()))?;
    let instance: JsonValue =
        serde_yaml::from_str(&raw).context("parse contracts/index.yaml as JSON value")?;
    let validator =
        jsonschema::validator_for(&schema_val).context("compile contracts/index.schema.json")?;
    validator
        .validate(&instance)
        .map_err(|e| anyhow!("contracts/index.yaml does not match schema: {e}"))?;

    let idx: IndexFile = serde_yaml::from_str(&raw).context("parse contracts/index.yaml")?;
    for c in &idx.contracts {
        let p = repo_root.join(&c.path);
        if !p.is_file() {
            return Err(anyhow!(
                "contracts index: missing file for `{}`: {}",
                c.id,
                p.display()
            ));
        }
    }

    println!(
        "contracts-index OK (schema v{}, {} entries)",
        idx.schema_version,
        idx.contracts.len()
    );
    Ok(())
}
