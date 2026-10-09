//! `vox ci capability-index` — generate or verify `contracts/capabilities/capability-index.v1.json`.
//!
//! The index is derived only from committed inputs (the operations catalog today), so check mode
//! is safe inside `vox ci ssot-drift`: it never builds the code graph.

mod inputs;
mod join;

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use vox_bounded_fs::read_utf8_path_capped;
use vox_capability_registry::CapabilityIndex;

use super::operations_catalog::write_or_verify_text;

const INDEX_REL: &str = "contracts/capabilities/capability-index.v1.json";
const SCHEMA_REL: &str = "contracts/capabilities/capability-index.v1.schema.json";
const STALE_HINT: &str = "run `vox ci capability-index --write`";

/// Validate a serialized index document against the committed JSON Schema.
fn validate_against_schema(repo_root: &Path, instance: &serde_json::Value) -> Result<()> {
    let schema_path = repo_root.join(SCHEMA_REL);
    let schema_val: serde_json::Value = serde_json::from_str(&read_utf8_path_capped(&schema_path)?)
        .with_context(|| format!("parse {} as JSON", schema_path.display()))?;
    let validator = vox_jsonschema_util::compile_validator(&schema_val, schema_path.display())
        .context("compile capability index JSON Schema")?;
    vox_jsonschema_util::validate(instance, &validator, "capability-index.v1.json vs schema")
}

/// The exact on-disk text of an index (pretty JSON, trailing newline, no timestamps).
fn render(index: &CapabilityIndex) -> Result<String> {
    let mut text = serde_json::to_string_pretty(index).context("serialize capability index")?;
    text.push('\n');
    Ok(text)
}

/// Generate (`write`) or verify the committed capability index.
pub fn run(repo_root: &Path, write: bool) -> Result<()> {
    let loaded = inputs::load(repo_root)?;
    let index = join::build_index(&loaded)?;
    validate_against_schema(
        repo_root,
        &serde_json::to_value(&index).context("capability index to JSON value")?,
    )?;
    write_or_verify_text(
        &repo_root.join(INDEX_REL),
        &render(&index)?,
        write,
        STALE_HINT,
    )
    .map_err(|e| anyhow!("{e:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn repo_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root")
    }

    #[test]
    fn repo_index_valid() {
        let root = repo_root();
        let raw = read_utf8_path_capped(&root.join(INDEX_REL)).expect("read committed index");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("index is JSON");
        validate_against_schema(&root, &value).expect("committed index validates");
        let index: CapabilityIndex = serde_json::from_value(value).expect("index deserializes");

        let index_ops: BTreeSet<String> =
            index.rows.iter().filter_map(|r| r.op_id.clone()).collect();
        let catalog_ops: BTreeSet<String> = inputs::load(&root)
            .expect("load catalog")
            .ops
            .into_iter()
            .map(|o| o.id)
            .collect();
        assert_eq!(
            index_ops, catalog_ops,
            "every catalog op must have an index row (run `vox ci capability-index --write`)"
        );
    }
}
