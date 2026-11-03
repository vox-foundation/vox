//! `vox ci capability-index` — generate or verify `contracts/capabilities/capability-index.v1.json`.
//!
//! The index is derived only from committed inputs (the operations catalog today), so check mode
//! is safe inside `vox ci ssot-drift`: it never builds the code graph.

mod coverage;
mod docs_claims;
mod emit;
mod groups;
mod inputs;
mod join;
mod nodes;
mod owner;
pub(crate) mod ratchet;

pub use coverage::refresh_coverage_snapshot;

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use vox_bounded_fs::read_utf8_path_capped;
use vox_capability_registry::{CapabilityIndex, RowKind};

use super::operations_catalog::write_or_verify_text;

const INDEX_REL: &str = "contracts/capabilities/capability-index.v1.json";
const SCHEMA_REL: &str = "contracts/capabilities/capability-index.v1.schema.json";
const STALE_HINT: &str = "run `vox ci capability-index --write`";
const LLMS_REL: &str = "docs/src/.well-known/llms.txt";
const LLMS_FULL_REL: &str = "docs/src/.well-known/llms-full.txt";
const DOCS_JSON_REL: &str = "docs/src/.well-known/vox-docs.json";
const TS_REL: &str = "crates/vox-gui/ui/src/generated/capabilityIndex.generated.ts";

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

/// MCP tools with an empty equivalents list (D-17).
pub(crate) fn mcp_only_names(index: &CapabilityIndex) -> BTreeSet<String> {
    index
        .rows
        .iter()
        .filter(|r| r.kind == RowKind::McpTool && r.equivalents.is_empty())
        .map(|r| {
            r.mcp
                .as_ref()
                .map(|m| m.name.clone())
                .unwrap_or_else(|| r.id.clone())
        })
        .collect()
}

/// Tools MCP-only in `new` that were not in `old` (D-27).
pub(crate) fn new_mcp_only_names(old: &CapabilityIndex, new: &CapabilityIndex) -> BTreeSet<String> {
    let old_names = mcp_only_names(old);
    let new_names = mcp_only_names(new);
    new_names.difference(&old_names).cloned().collect()
}

/// Generate (`write`) or verify the committed capability index. Never writes a ratchet
/// baseline (owner or side-effect), even with `write`: only `tighten_baseline` does.
pub fn run(repo_root: &Path, write: bool) -> Result<()> {
    let loaded = inputs::load(repo_root)?;
    let index = join::build_index(&loaded)?;

    let total_mcp = index
        .rows
        .iter()
        .filter(|r| r.kind == RowKind::McpTool)
        .count();
    let mcp_only = mcp_only_names(&index).len();
    println!(
        "capability-index: {mcp_only} of {total_mcp} MCP tools are MCP-only (no CLI twin); warning only (D-17)"
    );

    if write {
        if let Ok(committed_text) = read_utf8_path_capped(&repo_root.join(INDEX_REL)) {
            if let Ok(committed_index) = serde_json::from_str::<CapabilityIndex>(&committed_text) {
                for name in new_mcp_only_names(&committed_index, &index) {
                    println!(
                        "::warning::capability-index: new MCP-only tool without CLI twin: {name}"
                    );
                }
            }
        }
    }
    let snap = loaded.coverage.as_ref().ok_or_else(|| {
        anyhow!(
            "missing graph-coverage snapshot (contracts/reports/graph-coverage.v1.json); run `vox graph rebuild --corpus vox-gui-surface && vox ci capability-index --refresh-coverage --write`"
        )
    })?;
    ratchet::check(
        &ratchet::orphan_ids(snap),
        &ratchet::load_baseline(
            repo_root,
            &ratchet::ORPHAN,
            "vox ci capability-index --tighten",
        )?,
        &ratchet::ORPHAN,
    )?;
    for (spec, ids) in [
        (&ratchet::OWNER, ratchet::owner_ids(&index)),
        (&ratchet::SIDE_EFFECT, ratchet::side_effect_ids(&index)),
    ] {
        ratchet::check(
            &ids,
            &ratchet::load_baseline(repo_root, spec, "vox ci capability-index --tighten")?,
            spec,
        )?;
    }
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
    .map_err(|e| anyhow!("{e:#}"))?;
    sync_llms_regions(repo_root, &index, write)?;
    let docs_path = repo_root.join(DOCS_JSON_REL);
    let docs_json = emit::docs_json_with_capabilities(&read_utf8_path_capped(&docs_path)?, &index)?;
    write_or_verify_text(&docs_path, &docs_json, write, STALE_HINT)
        .map_err(|e| anyhow!("{e:#}"))?;
    let top_level = super::gui_surface_registry::top_level_groups(repo_root)?;
    let mut groups = groups::build_groups(&index, &loaded.surfaces, &top_level, &loaded.domains)?;
    groups::attach_exemptions(
        &mut groups,
        &super::gui_coverage_exemptions::load_exemptions(repo_root)?,
    );
    write_or_verify_text(
        &repo_root.join(TS_REL),
        &emit::generate_ts(&index, &groups),
        write,
        STALE_HINT,
    )
    .map_err(|e| anyhow!("{e:#}"))
}

/// Crate names declared in `layers.toml [crates]` (exemption owners must be one of these).
pub(crate) fn layers_crates(repo_root: &Path) -> Result<std::collections::BTreeSet<String>> {
    Ok(inputs::load_layers(repo_root)?.into_keys().collect())
}

/// Rewrite (or verify) only the generated marker region of each hand-written llms file.
fn sync_llms_regions(repo_root: &Path, index: &CapabilityIndex, write: bool) -> Result<()> {
    for (rel, body) in [
        (LLMS_REL, emit::llms_compact(index)),
        (LLMS_FULL_REL, emit::llms_full(index)),
    ] {
        let path = repo_root.join(rel);
        if rel == LLMS_FULL_REL && !path.is_file() {
            // Retired under TRUST-03 (docs-freshness Phase 24): /llms-full.txt is generated by
            // starlight-llms-txt at Astro build time and .well-known/llms-full.txt 301-redirects.
            continue;
        }
        let current = read_utf8_path_capped(&path)?;
        let next = emit::splice_region(&current, &body).with_context(|| {
            format!(
                "{rel}: add the marker pair `{}` / `{}` where the section belongs",
                emit::BEGIN_MARKER,
                emit::END_MARKER
            )
        })?;
        write_or_verify_text(&path, &next, write, STALE_HINT).map_err(|e| anyhow!("{e:#}"))?;
    }
    Ok(())
}

/// `--tighten`: shrink both ratchet baselines (removal-only; each bootstraps if missing).
pub fn tighten_baseline(repo_root: &Path) -> Result<()> {
    let loaded = inputs::load(repo_root)?;
    let index = join::build_index(&loaded)?;
    for (spec, ids) in [
        (&ratchet::OWNER, ratchet::owner_ids(&index)),
        (&ratchet::SIDE_EFFECT, ratchet::side_effect_ids(&index)),
    ] {
        let path = repo_root.join(spec.baseline_rel);
        let current: Option<ratchet::RatchetBaseline> = if path.is_file() {
            Some(ratchet::load_baseline(
                repo_root,
                spec,
                "vox ci capability-index --tighten",
            )?)
        } else {
            None
        };
        let next = ratchet::tighten(&ids, current.as_ref(), spec)?;
        write_or_verify_text(
            &path,
            &next.render()?,
            true,
            "run `vox ci capability-index --tighten`",
        )
        .map_err(|e| anyhow!("{e:#}"))?;
    }
    let snap = loaded.coverage.as_ref().ok_or_else(|| {
        anyhow!(
            "missing graph-coverage snapshot (contracts/reports/graph-coverage.v1.json); run `vox graph rebuild --corpus vox-gui-surface && vox ci capability-index --refresh-coverage --write`"
        )
    })?;
    let path = repo_root.join(ratchet::ORPHAN.baseline_rel);
    let current: Option<ratchet::RatchetBaseline> = if path.is_file() {
        Some(ratchet::load_baseline(
            repo_root,
            &ratchet::ORPHAN,
            "vox ci capability-index --tighten",
        )?)
    } else {
        None
    };
    let next = ratchet::tighten(
        &ratchet::orphan_ids(snap),
        current.as_ref(),
        &ratchet::ORPHAN,
    )?;
    write_or_verify_text(
        &path,
        &next.render()?,
        true,
        "run `vox ci capability-index --tighten`",
    )
    .map_err(|e| anyhow!("{e:#}"))?;
    Ok(())
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

        // Coverage is never silently unknown: kind null always carries a reason, and every
        // cli / mcp / surface row has a coverage object.
        for r in &index.rows {
            let needs = matches!(
                r.kind,
                vox_capability_registry::RowKind::Cli
                    | vox_capability_registry::RowKind::McpTool
                    | vox_capability_registry::RowKind::Surface
            );
            match &r.coverage {
                Some(c) if c.kind.is_none() => assert!(
                    c.reason.as_deref().is_some_and(|x| !x.is_empty()),
                    "{}: coverage kind is null without a reason",
                    r.id
                ),
                Some(_) => {}
                None => assert!(!needs, "{}: missing coverage object", r.id),
            }
        }
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

    #[test]
    fn mcp_only_names_behavior() {
        use vox_capability_registry::IndexRow;
        let mcp_row = |id: &str, name: &str, eq: &[&str]| {
            serde_json::from_value::<IndexRow>(serde_json::json!({
                "id": id, "kind": "mcp_tool", "title": id, "summary": null, "op_id": null,
                "status": null, "domain": null, "product_lane": null,
                "side_effect_class": null, "tier": null,
                "cli": null, "mcp": { "name": name }, "gui": null, "owner": null,
                "source_path": null, "docs": [], "skills": [],
                "equivalents": eq, "refs": [], "coverage": null
            }))
            .expect("row parses")
        };
        let cli_row = serde_json::from_value::<IndexRow>(serde_json::json!({
            "id": "cli.x", "kind": "cli", "title": "cli.x", "summary": null, "op_id": null,
            "status": null, "domain": null, "product_lane": null,
            "side_effect_class": null, "tier": null,
            "cli": null, "mcp": null, "gui": null, "owner": null,
            "source_path": null, "docs": [], "skills": [],
            "equivalents": [], "refs": [], "coverage": null
        }))
        .expect("cli row parses");

        let index = CapabilityIndex {
            schema_version: 1,
            generator: "test".into(),
            coverage_snapshot: None,
            domains: vec![],
            rows: vec![
                mcp_row("mcp.vox_only", "vox_only", &[]),
                mcp_row("mcp.vox_twin", "vox_twin", &["cli.twin"]),
                cli_row,
            ],
        };
        let names = mcp_only_names(&index);
        assert_eq!(names, ["vox_only".to_string()].into_iter().collect());
    }

    #[test]
    fn new_mcp_only_names_behavior() {
        use vox_capability_registry::IndexRow;
        let mcp_row = |id: &str, name: &str| {
            serde_json::from_value::<IndexRow>(serde_json::json!({
                "id": id, "kind": "mcp_tool", "title": id, "summary": null, "op_id": null,
                "status": null, "domain": null, "product_lane": null,
                "side_effect_class": null, "tier": null,
                "cli": null, "mcp": { "name": name }, "gui": null, "owner": null,
                "source_path": null, "docs": [], "skills": [],
                "equivalents": [], "refs": [], "coverage": null
            }))
            .expect("row parses")
        };
        let old = CapabilityIndex {
            schema_version: 1,
            generator: "test".into(),
            coverage_snapshot: None,
            domains: vec![],
            rows: vec![mcp_row("mcp.vox_old", "vox_old")],
        };
        let new = CapabilityIndex {
            schema_version: 1,
            generator: "test".into(),
            coverage_snapshot: None,
            domains: vec![],
            rows: vec![
                mcp_row("mcp.vox_old", "vox_old"),
                mcp_row("mcp.vox_new", "vox_new"),
            ],
        };
        let diff = new_mcp_only_names(&old, &new);
        assert_eq!(diff, ["vox_new".to_string()].into_iter().collect());
    }
}
