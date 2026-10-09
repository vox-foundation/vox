//! `vox ci capability-index` — generate or verify `contracts/capabilities/capability-index.v1.json`.
//!
//! The index is derived only from committed inputs (the operations catalog today), so check mode
//! is safe inside `vox ci ssot-drift`: it never builds the code graph.

mod inputs;
mod join;
mod nodes;
mod owner;
mod ratchet;

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

/// Read the committed owner-ratchet baseline; a missing file is an error (never silently empty).
fn load_baseline(repo_root: &Path) -> Result<ratchet::OwnerRatchet> {
    let path = repo_root.join(ratchet::BASELINE_REL);
    if !path.is_file() {
        return Err(anyhow!(
            "missing {}; run `vox ci capability-index --tighten` once",
            ratchet::BASELINE_REL
        ));
    }
    serde_json::from_str(&read_utf8_path_capped(&path)?)
        .with_context(|| format!("parse {}", ratchet::BASELINE_REL))
}

/// Generate (`write`) or verify the committed capability index. Never writes the owner-ratchet
/// baseline, even with `write`: only `tighten_baseline` does.
pub fn run(repo_root: &Path, write: bool) -> Result<()> {
    let loaded = inputs::load(repo_root)?;
    let index = join::build_index(&loaded)?;
    // Before any artifact write/verify, so a new ownerless command reports the ratchet error
    // rather than a generic "stale".
    ratchet::check(&ratchet::unresolved_ids(&index), &load_baseline(repo_root)?)?;
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

/// `--tighten`: shrink the unresolved-owner baseline (removal-only; bootstraps a missing file).
pub fn tighten_baseline(repo_root: &Path) -> Result<()> {
    let loaded = inputs::load(repo_root)?;
    let index = join::build_index(&loaded)?;
    let path = repo_root.join(ratchet::BASELINE_REL);
    let current = if path.is_file() {
        Some(load_baseline(repo_root)?)
    } else {
        None
    };
    let next = ratchet::tighten(&ratchet::unresolved_ids(&index), current.as_ref())?;
    write_or_verify_text(
        &path,
        &next.render()?,
        true,
        "run `vox ci capability-index --tighten`",
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

    // --- DISC-02: check mode must fail on drift (mutation-style, tempdir only) ---

    fn synthetic(extra_op: bool) -> CapabilityIndex {
        use inputs::test_op;
        let mut ops = vec![
            test_op("a", Some((&["a"], "active")), Some("vox_a"), "first"),
            test_op("b", Some((&["b"], "active")), None, "second"),
        ];
        if extra_op {
            ops.push(test_op("c", Some((&["c"], "active")), None, "new op"));
        }
        join::build_index(&inputs::Inputs {
            ops,
            ..Default::default()
        })
        .expect("build synthetic index")
    }

    #[test]
    fn drift_detects_deleted_row() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("capability-index.v1.json");
        let fresh = render(&synthetic(false)).expect("render");

        write_or_verify_text(&path, &fresh, true, STALE_HINT).expect("write");
        write_or_verify_text(&path, &fresh, false, STALE_HINT).expect("fresh file verifies");

        let mut mutated = synthetic(false);
        mutated.rows.remove(0);
        std::fs::write(&path, render(&mutated).expect("render")).expect("write mutated");
        let err = write_or_verify_text(&path, &fresh, false, STALE_HINT)
            .expect_err("a deleted row must be reported");
        assert!(err.to_string().contains("is stale"), "got: {err}");
    }

    #[test]
    fn drift_detects_new_op() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("capability-index.v1.json");
        write_or_verify_text(
            &path,
            &render(&synthetic(false)).expect("render"),
            true,
            STALE_HINT,
        )
        .expect("write committed index");
        let err = write_or_verify_text(
            &path,
            &render(&synthetic(true)).expect("render"),
            false,
            STALE_HINT,
        )
        .expect_err("a catalog op added without regeneration must be reported");
        assert!(err.to_string().contains("is stale"), "got: {err}");
    }

    #[test]
    fn drift_detects_missing_file() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("absent.json");
        let err = write_or_verify_text(
            &path,
            &render(&synthetic(false)).expect("render"),
            false,
            STALE_HINT,
        )
        .expect_err("absent index must be reported");
        assert!(err.to_string().contains("missing"), "got: {err}");
    }
}
