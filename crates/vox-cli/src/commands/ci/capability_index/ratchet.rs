//! Unresolved-count ratchets (D-05 owner, D-02 side-effect): the number of unresolved rows may only
//! fall. Each baseline file is written by `tighten` alone (removal-only), never by `--write`.
//! One implementation serves both; a [`RatchetSpec`] names the baseline, the predicate and the fix.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use vox_bounded_fs::read_utf8_path_capped;
use vox_capability_registry::{CapabilityIndex, RowKind};

use super::coverage::{CoverageSnapshot, EXCLUDED_KINDS};

/// What a ratchet counts and where its baseline lives.
pub(crate) struct RatchetSpec {
    pub baseline_rel: &'static str,
    /// Noun phrase for error text, e.g. "cli/mcp row(s) have no resolvable owner".
    pub subject: &'static str,
    pub fix_hint: &'static str,
}

pub(crate) fn owner_ids(index: &CapabilityIndex) -> BTreeSet<String> {
    index
        .rows
        .iter()
        .filter(|r| matches!(r.kind, RowKind::Cli | RowKind::McpTool) && r.owner.is_none())
        .map(|r| r.id.clone())
        .collect()
}

pub(crate) fn side_effect_ids(index: &CapabilityIndex) -> BTreeSet<String> {
    index
        .rows
        .iter()
        .filter(|r| r.kind == RowKind::Cli && r.side_effect_class.is_none())
        .map(|r| r.id.clone())
        .collect()
}

/// Graph nodes in snapshot kinds not excluded that have status `orphan_backend` or `dead_end` (D-07, D-21).
pub(crate) fn orphan_ids(snapshot: &CoverageSnapshot) -> BTreeSet<String> {
    let excluded: BTreeSet<&str> = EXCLUDED_KINDS.iter().map(|(k, _)| *k).collect();
    let mut ids = BTreeSet::new();
    for (kind, entries) in &snapshot.entries {
        if !excluded.contains(kind.as_str()) {
            for entry in entries {
                if entry.status == "orphan_backend" || entry.status == "dead_end" {
                    ids.insert(entry.id.clone());
                }
            }
        }
    }
    ids
}

pub(crate) const OWNER: RatchetSpec = RatchetSpec {
    baseline_rel: "contracts/capabilities/owner-ratchet.v1.json",
    subject: "cli/mcp row(s) have no resolvable owner",
    fix_hint: "Fix the source, not the baseline: add the group to `cli_groups` (with an `owner`) in \
               contracts/capabilities/domains.v1.yaml, or set `owner_crate` on the catalog op",
};

pub(crate) const SIDE_EFFECT: RatchetSpec = RatchetSpec {
    baseline_rel: "contracts/capabilities/side-effect-ratchet.v1.json",
    subject: "cli row(s) have no side_effect_class",
    fix_hint: "Fix the source, not the baseline: classify the op by setting `side_effect_class` in \
               contracts/operations/catalog.v1.yaml after inspecting its handler (never by name)",
};

pub(crate) const EXEMPTION: RatchetSpec = RatchetSpec {
    baseline_rel: "contracts/gui/coverage-exemption-ratchet.v1.json",
    subject: "exemption(s) in contracts/gui/coverage-exemptions.v1.yaml",
    fix_hint: "A new exemption is user-authorized — add it to the baseline in the same reviewed change, or give the group a GUI path",
};

pub(crate) const DEDICATED_FLOOR: RatchetSpec = RatchetSpec {
    baseline_rel: "contracts/gui/dedicated-floor-ratchet.v1.json",
    subject: "dedicated surface group(s) lost",
    fix_hint: "Restore the dedicated surface or have the user lower the floor by a reviewed hand edit",
};

pub(crate) const ORPHAN: RatchetSpec = RatchetSpec {
    baseline_rel: "contracts/capabilities/orphan-ratchet.v1.json",
    subject: "graph node(s) are orphan_backend/dead_end in contracts/reports/graph-coverage.v1.json",
    fix_hint: "fix or delete, never exempt (D-06): repair the ingest source or delete the dead code, rebuild the graph, refresh the snapshot",
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RatchetBaseline {
    pub schema_version: u32,
    pub max_unresolved: usize,
    pub unresolved: Vec<String>,
}

impl RatchetBaseline {
    pub(crate) fn from_set(ids: &BTreeSet<String>) -> Self {
        Self {
            schema_version: 1,
            max_unresolved: ids.len(),
            unresolved: ids.iter().cloned().collect(),
        }
    }

    /// Exact on-disk text: pretty JSON, ids sorted, trailing newline.
    pub(crate) fn render(&self) -> Result<String> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FloorBaseline {
    pub schema_version: u32,
    pub members: Vec<String>,
}

impl FloorBaseline {
    pub(crate) fn from_set(members: &BTreeSet<String>) -> Self {
        Self {
            schema_version: 1,
            members: members.iter().cloned().collect(),
        }
    }

    pub(crate) fn render(&self) -> Result<String> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

/// Read a committed baseline file; missing file returns error naming tighten_cmd.
pub(crate) fn load_baseline<T: DeserializeOwned>(
    repo_root: &Path,
    spec: &RatchetSpec,
    tighten_cmd: &str,
) -> Result<T> {
    let path = repo_root.join(spec.baseline_rel);
    if !path.is_file() {
        return Err(anyhow!(
            "missing {}; run `{}` once",
            spec.baseline_rel,
            tighten_cmd
        ));
    }
    serde_json::from_str(&read_utf8_path_capped(&path)?)
        .with_context(|| format!("parse {}", spec.baseline_rel))
}

/// Fail when a row is unresolved and not already in the baseline, or the count exceeds the max.
pub(crate) fn check(
    live: &BTreeSet<String>,
    baseline: &RatchetBaseline,
    spec: &RatchetSpec,
) -> Result<()> {
    let known: BTreeSet<&str> = baseline.unresolved.iter().map(String::as_str).collect();
    let added: Vec<&str> = live
        .iter()
        .map(String::as_str)
        .filter(|id| !known.contains(id))
        .collect();
    if !added.is_empty() {
        bail!(
            "{} {} and are not in {}: {}. {}",
            added.len(),
            spec.subject,
            spec.baseline_rel,
            added.join(", "),
            spec.fix_hint
        );
    }
    if live.len() > baseline.max_unresolved {
        bail!(
            "{} unresolved ({}) exceed max_unresolved {} in {}",
            live.len(),
            spec.subject,
            baseline.max_unresolved,
            spec.baseline_rel
        );
    }
    Ok(())
}

/// Shrink the baseline to the rows still unresolved. `None` bootstraps it from `live`.
/// Additions are refused: raising the baseline is a visible hand edit, never automation.
pub(crate) fn tighten(
    live: &BTreeSet<String>,
    baseline: Option<&RatchetBaseline>,
    spec: &RatchetSpec,
) -> Result<RatchetBaseline> {
    let Some(base) = baseline else {
        return Ok(RatchetBaseline::from_set(live));
    };
    let known: BTreeSet<&str> = base.unresolved.iter().map(String::as_str).collect();
    let added: Vec<&str> = live
        .iter()
        .map(String::as_str)
        .filter(|id| !known.contains(id))
        .collect();
    if !added.is_empty() {
        bail!(
            "--tighten would ADD {} unresolved row(s) to {} (tighten is removal-only): {}",
            added.len(),
            spec.baseline_rel,
            added.join(", ")
        );
    }
    Ok(RatchetBaseline::from_set(live))
}

/// Check that every dedicated member in baseline is still in live.
pub(crate) fn check_floor(
    live: &BTreeSet<String>,
    baseline: &FloorBaseline,
    spec: &RatchetSpec,
) -> Result<()> {
    let live_set: BTreeSet<&str> = live.iter().map(String::as_str).collect();
    let lost: Vec<&str> = baseline
        .members
        .iter()
        .map(String::as_str)
        .filter(|m| !live_set.contains(m))
        .collect();
    if !lost.is_empty() {
        bail!(
            "{} {} from {}: {}. {}",
            lost.len(),
            spec.subject,
            spec.baseline_rel,
            lost.join(", "),
            spec.fix_hint
        );
    }
    Ok(())
}

/// Raise the floor to include newly dedicated groups. `None` bootstraps it from `live`.
/// Dropping members is refused: lowering the floor is a visible hand edit, never automation.
pub(crate) fn tighten_floor(
    live: &BTreeSet<String>,
    baseline: Option<&FloorBaseline>,
    spec: &RatchetSpec,
) -> Result<FloorBaseline> {
    let Some(base) = baseline else {
        return Ok(FloorBaseline::from_set(live));
    };
    let live_set: BTreeSet<&str> = live.iter().map(String::as_str).collect();
    let lost: Vec<&str> = base
        .members
        .iter()
        .map(String::as_str)
        .filter(|m| !live_set.contains(m))
        .collect();
    if !lost.is_empty() {
        bail!(
            "--tighten would drop {} dedicated member(s) from {} (tighten would lower the floor): {}",
            lost.len(),
            spec.baseline_rel,
            lost.join(", ")
        );
    }
    let mut updated: BTreeSet<String> = base.members.iter().cloned().collect();
    updated.extend(live.iter().cloned());
    Ok(FloorBaseline::from_set(&updated))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    fn baseline(ids: &[&str]) -> RatchetBaseline {
        RatchetBaseline::from_set(&set(ids))
    }

    fn floor(members: &[&str]) -> FloorBaseline {
        FloorBaseline::from_set(&set(members))
    }

    /// Index with the given (id, kind, side_effect_class, has_owner) rows.
    fn index(rows: &[(&str, &str, Option<&str>, bool)]) -> CapabilityIndex {
        let row = |(id, kind, class, owned): &(&str, &str, Option<&str>, bool)| {
            serde_json::json!({
                "id": id, "kind": kind, "title": id, "summary": null, "op_id": null,
                "status": null, "domain": null, "product_lane": null,
                "side_effect_class": class, "tier": null,
                "cli": null, "mcp": null, "gui": null,
                "owner": if *owned { serde_json::json!({"crate": "c", "layer": 1, "rule": "t"}) } else { serde_json::Value::Null },
                "source_path": null, "docs": [], "skills": [], "equivalents": [], "refs": [],
                "coverage": null
            })
        };
        serde_json::from_value(serde_json::json!({
            "schema_version": 1, "generator": "t", "coverage_snapshot": null, "domains": [],
            "rows": rows.iter().map(row).collect::<Vec<_>>()
        }))
        .expect("synthetic index parses")
    }

    #[test]
    fn new_unresolved_id_fails() {
        let err = check(&set(&["a", "b"]), &baseline(&["a"]), &OWNER).expect_err("b is new");
        let msg = err.to_string();
        assert!(
            msg.contains('b') && msg.contains("domains.v1.yaml") && msg.contains("owner_crate"),
            "got: {msg}"
        );
    }

    #[test]
    fn count_above_max_fails() {
        let mut b = baseline(&["a", "b"]);
        b.max_unresolved = 1;
        let err = check(&set(&["a", "b"]), &b, &OWNER).expect_err("count above max");
        assert!(err.to_string().contains("max_unresolved"), "got: {err}");
    }

    #[test]
    fn shrink_passes() {
        check(&set(&["a"]), &baseline(&["a", "b"]), &OWNER).expect("a shrinking set is fine");
        check(&set(&[]), &baseline(&["a"]), &OWNER).expect("fully resolved is fine");
    }

    #[test]
    fn tighten_refuses_additions() {
        let err = tighten(&set(&["a", "b"]), Some(&baseline(&["a"])), &OWNER).expect_err("refuse");
        assert!(
            err.to_string().contains("tighten is removal-only"),
            "got: {err}"
        );
    }

    #[test]
    fn tighten_removes_resolved() {
        let out = tighten(&set(&["a"]), Some(&baseline(&["a", "b"])), &OWNER).expect("tighten");
        assert_eq!(out.unresolved, vec!["a".to_string()]);
        assert_eq!(out.max_unresolved, 1);
    }

    #[test]
    fn tighten_bootstraps_from_live() {
        let out = tighten(&set(&["x", "y"]), None, &OWNER).expect("bootstrap");
        assert_eq!(out, baseline(&["x", "y"]));
    }

    #[test]
    fn baseline_render_is_sorted_and_stable() {
        let one = baseline(&["b", "a"]).render().expect("render");
        let two = baseline(&["a", "b"]).render().expect("render");
        assert_eq!(one, two);
        assert!(one.ends_with("}\n"));
        assert!(one.find("\"a\"").expect("a") < one.find("\"b\"").expect("b"));
        assert!(one.contains("\"max_unresolved\": 2"));
    }

    #[test]
    fn side_effect_new_unclassified_id_fails() {
        let err = check(
            &set(&["cli.a", "cli.new"]),
            &baseline(&["cli.a"]),
            &SIDE_EFFECT,
        )
        .expect_err("cli.new is new");
        let msg = err.to_string();
        assert!(
            msg.contains("cli.new") && msg.contains("contracts/operations/catalog.v1.yaml"),
            "got: {msg}"
        );
    }

    #[test]
    fn side_effect_count_above_max_fails() {
        let mut b = baseline(&["cli.a", "cli.b"]);
        b.max_unresolved = 1;
        let err = check(&set(&["cli.a", "cli.b"]), &b, &SIDE_EFFECT).expect_err("above max");
        assert!(err.to_string().contains("max_unresolved"), "got: {err}");
    }

    #[test]
    fn side_effect_shrink_passes() {
        check(
            &set(&["cli.a"]),
            &baseline(&["cli.a", "cli.b"]),
            &SIDE_EFFECT,
        )
        .expect("shrink ok");
        check(&set(&[]), &baseline(&["cli.a"]), &SIDE_EFFECT).expect("fully classified ok");
    }

    #[test]
    fn side_effect_tighten_refuses_additions() {
        let err = tighten(
            &set(&["cli.a", "cli.b"]),
            Some(&baseline(&["cli.a"])),
            &SIDE_EFFECT,
        )
        .expect_err("refuse");
        let msg = err.to_string();
        assert!(
            msg.contains("tighten is removal-only") && msg.contains(SIDE_EFFECT.baseline_rel),
            "got: {msg}"
        );
    }

    #[test]
    fn side_effect_tighten_removes_classified() {
        let out = tighten(
            &set(&["cli.a"]),
            Some(&baseline(&["cli.a", "cli.b"])),
            &SIDE_EFFECT,
        )
        .expect("tighten");
        assert_eq!(out.unresolved, vec!["cli.a".to_string()]);
        assert_eq!(out.max_unresolved, 1);
    }

    #[test]
    fn side_effect_tighten_bootstraps_from_live() {
        let out = tighten(&set(&["cli.x"]), None, &SIDE_EFFECT).expect("bootstrap");
        assert_eq!(out, baseline(&["cli.x"]));
    }

    #[test]
    fn side_effect_ids_only_cli_null() {
        let idx = index(&[
            ("cli.null", "cli", None, true),
            ("cli.ro", "cli", Some("read_only"), true),
            ("mcp.null", "mcp_tool", None, true),
        ]);
        assert_eq!(side_effect_ids(&idx), set(&["cli.null"]));
    }

    #[test]
    fn side_effect_empty_index_baseline() {
        let idx = index(&[("cli.ro", "cli", Some("none"), true)]);
        let live = side_effect_ids(&idx);
        let text = tighten(&live, None, &SIDE_EFFECT)
            .expect("bootstrap")
            .render()
            .expect("render");
        assert!(text.contains("\"max_unresolved\": 0"), "got: {text}");
        assert!(text.contains("\"unresolved\": []"), "got: {text}");
    }

    #[test]
    fn owner_ids_cover_cli_and_mcp_without_owner() {
        let idx = index(&[
            ("cli.no", "cli", None, false),
            ("mcp.no", "mcp_tool", None, false),
            ("cli.yes", "cli", None, true),
        ]);
        assert_eq!(owner_ids(&idx), set(&["cli.no", "mcp.no"]));
    }

    #[test]
    fn exemption_new_entry_not_in_baseline_fails() {
        let err = check(&set(&["dei", "extra"]), &baseline(&["dei"]), &EXEMPTION).expect_err("new");
        assert!(
            err.to_string().contains("extra")
                && err
                    .to_string()
                    .contains("coverage-exemption-ratchet.v1.json")
        );
    }

    #[test]
    fn exemption_shrink_passes() {
        check(&set(&["dei"]), &baseline(&["dei", "extra"]), &EXEMPTION).expect("shrink ok");
        check(&set(&[]), &baseline(&["dei"]), &EXEMPTION).expect("empty ok");
    }

    #[test]
    fn exemption_tighten_is_removal_only() {
        let err = tighten(
            &set(&["dei", "extra"]),
            Some(&baseline(&["dei"])),
            &EXEMPTION,
        )
        .expect_err("add");
        assert!(err.to_string().contains("tighten is removal-only"));
        let shrunk = tighten(
            &set(&["dei"]),
            Some(&baseline(&["dei", "extra"])),
            &EXEMPTION,
        )
        .expect("shrunk");
        assert_eq!(shrunk.unresolved, vec!["dei".to_string()]);
    }

    #[test]
    fn floor_lost_member_fails() {
        let err =
            check_floor(&set(&["a"]), &floor(&["a", "b"]), &DEDICATED_FLOOR).expect_err("lost");
        assert!(
            err.to_string().contains('b')
                && err.to_string().contains("dedicated-floor-ratchet.v1.json")
        );
    }

    #[test]
    fn floor_swap_still_fails() {
        let err = check_floor(&set(&["a", "c"]), &floor(&["a", "b"]), &DEDICATED_FLOOR)
            .expect_err("lost b");
        assert!(err.to_string().contains('b'));
    }

    #[test]
    fn floor_extra_member_passes() {
        check_floor(
            &set(&["a", "b", "c"]),
            &floor(&["a", "b"]),
            &DEDICATED_FLOOR,
        )
        .expect("extra members ok");
    }

    #[test]
    fn tighten_floor_adds_only() {
        let raised = tighten_floor(
            &set(&["a", "b", "c"]),
            Some(&floor(&["a", "b"])),
            &DEDICATED_FLOOR,
        )
        .expect("raise");
        assert_eq!(
            raised.members,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        let err = tighten_floor(&set(&["a"]), Some(&floor(&["a", "b"])), &DEDICATED_FLOOR)
            .expect_err("lower");
        assert!(err.to_string().contains("would lower the floor"));
        let bootstrapped =
            tighten_floor(&set(&["x", "y"]), None, &DEDICATED_FLOOR).expect("bootstrap");
        assert_eq!(bootstrapped, floor(&["x", "y"]));
    }

    #[test]
    fn floor_baseline_render_is_sorted_and_stable() {
        let one = floor(&["b", "a"]).render().expect("render");
        let two = floor(&["a", "b"]).render().expect("render");
        assert_eq!(one, two);
        assert!(one.ends_with("}\n"));
        assert!(one.find("\"a\"").expect("a") < one.find("\"b\"").expect("b"));
        let unknown = r#"{"schema_version": 1, "members": ["a"], "extra": 123}"#;
        assert!(serde_json::from_str::<FloorBaseline>(unknown).is_err());
    }

    #[test]
    fn orphan_ids_skip_excluded_kinds() {
        use super::super::coverage::SnapshotEntry;
        use vox_capability_registry::CoverageSnapshotMeta;
        let snap = CoverageSnapshot {
            schema_version: 1,
            source: CoverageSnapshotMeta {
                corpus_id: "c".into(),
                git_sha: "s".into(),
                built_at: "t".into(),
            },
            entries: [
                (
                    "tool".into(),
                    vec![
                        SnapshotEntry {
                            id: "tool:a".into(),
                            label: "a".into(),
                            status: "orphan_backend".into(),
                        },
                        SnapshotEntry {
                            id: "tool:b".into(),
                            label: "b".into(),
                            status: "surfaced".into(),
                        },
                    ],
                ),
                (
                    "cli-command".into(),
                    vec![SnapshotEntry {
                        id: "cli:x".into(),
                        label: "x".into(),
                        status: "dead_end".into(),
                    }],
                ),
                (
                    "surface".into(),
                    vec![SnapshotEntry {
                        id: "surface:s".into(),
                        label: "s".into(),
                        status: "orphan_backend".into(),
                    }],
                ),
            ]
            .into_iter()
            .collect(),
        };
        assert_eq!(orphan_ids(&snap), set(&["cli:x", "tool:a"]));
    }

    #[test]
    fn orphan_check_zero_baseline() {
        let b = baseline(&[]);
        assert!(check(&set(&[]), &b, &ORPHAN).is_ok());
        let err = check(&set(&["tool:a"]), &b, &ORPHAN).expect_err("orphan detected");
        assert!(err.to_string().contains("tool:a"));
    }
}
