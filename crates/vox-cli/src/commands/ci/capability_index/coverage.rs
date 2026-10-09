//! Graph-coverage snapshot (D-07): written only by `--refresh-coverage`, read by check mode.
//!
//! The snapshot is a committed file with its own lifecycle. Check mode and `ssot-drift` read the
//! file and never build or open the code graph. Coverage is never invented: a row with no graph
//! key keeps `kind: null` and a reason.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use vox_bounded_fs::read_utf8_path_capped;
use vox_capability_registry::{CoverageRef, CoverageSnapshotMeta, IndexRow, RowKind};
use vox_graph_reader::coverage::{CoverageReport, compute_coverage};

use super::super::operations_catalog::write_or_verify_text;

pub(crate) const SNAPSHOT_REL: &str = "contracts/reports/graph-coverage.v1.json";
pub(crate) const CORPUS_ID: &str = "vox-gui-surface";
/// Graph node kinds that join to capability rows. The Tauri `command` kind cannot be joined.
const KINDS: [&str; 3] = ["tool", "cli-command", "surface"];

/// One graph node's verdict; `status` is the snake_case `CoverageStatus` string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SnapshotEntry {
    pub id: String,
    pub label: String,
    pub status: String,
}

/// `contracts/reports/graph-coverage.v1.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CoverageSnapshot {
    pub schema_version: u32,
    pub source: CoverageSnapshotMeta,
    /// Graph node kind -> entries sorted by id.
    pub entries: BTreeMap<String, Vec<SnapshotEntry>>,
}

fn status_str(status: &impl Serialize) -> Result<String> {
    serde_json::to_value(status)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow!("coverage status did not serialize to a string"))
}

/// Pure: group reports by kind, sort each by id, carry `source` verbatim.
pub(crate) fn build_snapshot(
    source: CoverageSnapshotMeta,
    reports: Vec<(&str, CoverageReport)>,
) -> Result<CoverageSnapshot> {
    let mut entries = BTreeMap::new();
    for (kind, report) in reports {
        let mut list = report
            .entries
            .iter()
            .map(|e| {
                Ok(SnapshotEntry {
                    id: e.id.clone(),
                    label: e.label.clone(),
                    status: status_str(&e.status)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        list.sort_by(|a, b| a.id.cmp(&b.id));
        entries.insert(kind.to_string(), list);
    }
    Ok(CoverageSnapshot {
        schema_version: 1,
        source,
        entries,
    })
}

/// The exact on-disk text of a snapshot (pretty JSON, trailing newline, no clock of our own).
pub(crate) fn render(snapshot: &CoverageSnapshot) -> Result<String> {
    let mut text = serde_json::to_string_pretty(snapshot).context("serialize coverage snapshot")?;
    text.push('\n');
    Ok(text)
}

fn missing_graph(detail: &str) -> anyhow::Error {
    anyhow!(
        "no graph for corpus {CORPUS_ID} ({detail}); run `vox graph rebuild --corpus {CORPUS_ID}` first"
    )
}

/// Re-snapshot coverage from the already-built `vox-gui-surface` graph (D-07). Does not build the
/// graph: run `vox graph rebuild --corpus vox-gui-surface` first.
pub fn refresh_coverage_snapshot(repo_root: &Path) -> Result<()> {
    let reg = vox_config::graphify::load_all_corpora(repo_root)
        .map_err(|e| missing_graph(&e.to_string()))?;
    let corpus = reg
        .corpora
        .iter()
        .find(|c| c.id == CORPUS_ID)
        .ok_or_else(|| missing_graph("corpus is not registered"))?;
    let graph_path = repo_root.join(&corpus.graph_path);
    if !graph_path.is_file() {
        return Err(missing_graph(&format!(
            "{} is absent",
            graph_path.display()
        )));
    }
    let graph: serde_json::Value = serde_json::from_str(&read_utf8_path_capped(&graph_path)?)
        .with_context(|| format!("parse graph JSON {}", graph_path.display()))?;
    let manifest_path = repo_root.join(&corpus.manifest_path);
    let manifest: vox_config::graphify::GraphifyManifest =
        serde_json::from_str(&read_utf8_path_capped(&manifest_path)?)
            .with_context(|| format!("parse manifest {}", manifest_path.display()))?;
    let (Some(git_sha), Some(built_at)) = (manifest.git_sha, manifest.built_at) else {
        bail!(
            "{} lacks git_sha or built_at; rebuild with `vox graph rebuild --corpus {CORPUS_ID}`",
            manifest_path.display()
        );
    };
    let source = CoverageSnapshotMeta {
        corpus_id: CORPUS_ID.to_string(),
        git_sha,
        built_at,
    };
    let reports = KINDS
        .iter()
        .map(|k| (*k, compute_coverage(&graph, k)))
        .collect();
    let snapshot = build_snapshot(source, reports)?;
    write_or_verify_text(
        &repo_root.join(SNAPSHOT_REL),
        &render(&snapshot)?,
        true,
        "run `vox ci capability-index --refresh-coverage`",
    )
    .map_err(|e| anyhow!("{e:#}"))
}

const TOP_LEVEL: &str =
    "top-level commands are cli-group nodes; the graph has no per-command coverage";
const NO_CLI_KEY: &str = "no cli:<group>:<leaf> node in the vox-gui-surface snapshot";
const NO_TOOL_KEY: &str = "no `tool:` node (the graph only extracts GUI-wired tools)";
const NO_SURFACE_KEY: &str = "no surface: node in the snapshot";

/// Graph node id a row would have: `Ok(key)`, or `Err(reason)` when its shape has no node.
/// `None` for row kinds coverage does not apply to.
fn graph_key(r: &IndexRow) -> Option<std::result::Result<String, &'static str>> {
    match r.kind {
        RowKind::Cli => {
            let p = &r.cli.as_ref()?.path;
            Some(match (p.first(), p.last()) {
                (Some(g), Some(l)) if p.len() >= 2 => Ok(format!("cli:{g}:{l}")),
                _ => Err(TOP_LEVEL),
            })
        }
        RowKind::McpTool => Some(Ok(format!("tool:{}", r.mcp.as_ref()?.name))),
        RowKind::Surface => Some(Ok(format!(
            "surface:{}",
            r.gui.as_ref()?.view_key.as_deref()?
        ))),
        _ => None,
    }
}

fn miss_reason(r: &IndexRow, shape_err: Option<&'static str>) -> &'static str {
    shape_err.unwrap_or(match r.kind {
        RowKind::Cli => NO_CLI_KEY,
        RowKind::McpTool => NO_TOOL_KEY,
        _ => NO_SURFACE_KEY,
    })
}

/// Join the snapshot onto cli / mcp / surface rows (D-07). A row whose graph key exists gets the
/// snapshot status; otherwise `kind` is null with a reason. Rows sharing one key all stay rows
/// and are marked `ambiguous`. With no snapshot the rows keep their 06.2-01 reason.
pub(crate) fn apply_coverage(rows: &mut [IndexRow], snapshot: Option<&CoverageSnapshot>) {
    let Some(snap) = snapshot else { return };
    let status: HashMap<&str, &str> = snap
        .entries
        .values()
        .flatten()
        .map(|e| (e.id.as_str(), e.status.as_str()))
        .collect();
    let keys: Vec<Option<std::result::Result<String, &'static str>>> =
        rows.iter().map(graph_key).collect();
    let mut shared: HashMap<&str, usize> = HashMap::new();
    for k in keys.iter().flatten().filter_map(|k| k.as_ref().ok()) {
        *shared.entry(k.as_str()).or_default() += 1;
    }
    for (r, key) in rows.iter_mut().zip(&keys) {
        let Some(key) = key else { continue };
        let hit = key.as_ref().ok().and_then(|k| {
            status
                .get(k.as_str())
                .map(|s| (*s, shared.get(k.as_str()).copied().unwrap_or(0) > 1))
        });
        r.coverage = Some(match hit {
            Some((kind, ambiguous)) => CoverageRef {
                kind: Some(kind.to_string()),
                basis: "graph-key".to_string(),
                ambiguous,
                reason: None,
            },
            None => CoverageRef {
                kind: None,
                basis: "none".to_string(),
                ambiguous: false,
                reason: Some(miss_reason(r, key.as_ref().err().copied()).to_string()),
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_graph_reader::coverage::{CoverageEntry, CoverageStatus};

    fn entry(id: &str, status: CoverageStatus) -> CoverageEntry {
        CoverageEntry {
            id: id.to_string(),
            label: id.to_string(),
            status,
        }
    }

    fn meta() -> CoverageSnapshotMeta {
        CoverageSnapshotMeta {
            corpus_id: CORPUS_ID.to_string(),
            git_sha: "abc123".to_string(),
            built_at: "2026-10-09T00:00:00+00:00".to_string(),
        }
    }

    fn reports() -> Vec<(&'static str, CoverageReport)> {
        vec![
            (
                "tool",
                CoverageReport {
                    entries: vec![
                        entry("tool:vox_b", CoverageStatus::Surfaced),
                        entry("tool:vox_a", CoverageStatus::OrphanBackend),
                    ],
                },
            ),
            (
                "cli-command",
                CoverageReport {
                    entries: vec![entry("cli:ars:status", CoverageStatus::CliOnly)],
                },
            ),
            (
                "surface",
                CoverageReport {
                    entries: vec![entry("surface:mesh", CoverageStatus::DeadEnd)],
                },
            ),
        ]
    }

    #[test]
    fn snapshot_is_sorted_and_keyed_by_kind() {
        let snap = build_snapshot(meta(), reports()).expect("build");
        assert_eq!(snap.source, meta(), "source carried verbatim");
        let kinds: Vec<&str> = snap.entries.keys().map(String::as_str).collect();
        assert_eq!(kinds, vec!["cli-command", "surface", "tool"]);
        let tool: Vec<(&str, &str)> = snap.entries["tool"]
            .iter()
            .map(|e| (e.id.as_str(), e.status.as_str()))
            .collect();
        assert_eq!(
            tool,
            vec![("tool:vox_a", "orphan_backend"), ("tool:vox_b", "surfaced")]
        );
        assert_eq!(snap.entries["cli-command"][0].status, "cli_only");
        assert_eq!(snap.entries["surface"][0].status, "dead_end");
    }

    #[test]
    fn snapshot_render_is_deterministic() {
        let one = render(&build_snapshot(meta(), reports()).expect("build")).expect("render");
        let two = render(&build_snapshot(meta(), reports()).expect("build")).expect("render");
        assert_eq!(one, two);
        assert!(one.ends_with("}\n"));
    }

    #[test]
    fn missing_graph_names_rebuild_command() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let err = refresh_coverage_snapshot(dir.path()).expect_err("no graph must fail");
        assert!(
            err.to_string()
                .contains("vox graph rebuild --corpus vox-gui-surface"),
            "got: {err}"
        );
        assert!(!dir.path().join(SNAPSHOT_REL).exists());
    }

    // --- apply_coverage / join (D-07, D-08) ---

    use super::super::inputs::{Inputs, OperationRow, test_op};
    use super::super::join::build_index;
    use vox_capability_registry::CapabilityIndex;

    fn snap(entries: &[(&str, &str, &str)]) -> CoverageSnapshot {
        let mut map: BTreeMap<String, Vec<SnapshotEntry>> = BTreeMap::new();
        for (kind, id, status) in entries {
            map.entry((*kind).to_string())
                .or_default()
                .push(SnapshotEntry {
                    id: (*id).to_string(),
                    label: (*id).to_string(),
                    status: (*status).to_string(),
                });
        }
        CoverageSnapshot {
            schema_version: 1,
            source: meta(),
            entries: map,
        }
    }

    fn index_with(ops: Vec<OperationRow>, coverage: Option<CoverageSnapshot>) -> CapabilityIndex {
        build_index(&Inputs {
            ops,
            coverage,
            ..Default::default()
        })
        .expect("build")
    }

    fn cov(idx: &CapabilityIndex, id: &str) -> CoverageRef {
        idx.row(id)
            .and_then(|r| r.coverage.clone())
            .unwrap_or_else(|| panic!("{id} has no coverage object"))
    }

    #[test]
    fn cli_key_hit() {
        let idx = index_with(
            vec![test_op(
                "ars.ludus.status",
                Some((&["ars", "ludus", "status"], "active")),
                None,
                "d",
            )],
            Some(snap(&[("cli-command", "cli:ars:status", "cli_only")])),
        );
        let c = cov(&idx, "cli.ars.ludus.status");
        assert_eq!(c.kind.as_deref(), Some("cli_only"));
        assert_eq!(c.basis, "graph-key");
        assert!(!c.ambiguous && c.reason.is_none());
    }

    #[test]
    fn ambiguous_key_marks_every_sharing_row() {
        let idx = index_with(
            vec![
                test_op(
                    "a",
                    Some((&["ars", "ludus", "status"], "active")),
                    None,
                    "d",
                ),
                test_op(
                    "b",
                    Some((&["ars", "other", "status"], "active")),
                    None,
                    "d",
                ),
                test_op("c", Some((&["ars", "solo"], "active")), None, "d"),
            ],
            Some(snap(&[
                ("cli-command", "cli:ars:status", "cli_only"),
                ("cli-command", "cli:ars:solo", "surfaced"),
            ])),
        );
        for id in ["cli.ars.ludus.status", "cli.ars.other.status"] {
            assert!(cov(&idx, id).ambiguous, "{id} shares a graph key");
        }
        assert!(!cov(&idx, "cli.ars.solo").ambiguous);
        assert_eq!(idx.rows.len(), 3, "rows are never merged");
    }

    #[test]
    fn top_level_cli_has_reason() {
        let idx = index_with(
            vec![test_op("check", Some((&["check"], "active")), None, "d")],
            Some(snap(&[("cli-command", "cli:ars:status", "cli_only")])),
        );
        let c = cov(&idx, "cli.check");
        assert!(c.kind.is_none());
        let reason = c.reason.expect("reason");
        assert!(reason.contains("top-level commands"), "got: {reason}");
    }

    #[test]
    fn mcp_hit_and_miss() {
        let idx = index_with(
            vec![
                test_op("x", None, Some("vox_x"), "d"),
                test_op("y", None, Some("vox_y"), "d"),
            ],
            Some(snap(&[("tool", "tool:vox_x", "surfaced")])),
        );
        assert_eq!(cov(&idx, "mcp.vox_x").kind.as_deref(), Some("surfaced"));
        let miss = cov(&idx, "mcp.vox_y");
        assert!(miss.kind.is_none());
        assert!(
            miss.reason.expect("reason").contains("`tool:`"),
            "reason must name the missing key shape"
        );
    }

    #[test]
    fn surface_hit() {
        use crate::commands::ci::gui_surface_registry::{
            RepresentationTier, SurfaceEntry, SurfaceRegistry,
        };
        let entry = SurfaceEntry {
            view_key: Some("mesh".to_string()),
            cli_group: None,
            representation_tier: RepresentationTier::LiveBackend,
            nav_label: Some("Mesh".to_string()),
            nav_icon: None,
            nav_group: None,
            parent_surface: None,
            notes: None,
        };
        let idx = build_index(&Inputs {
            surfaces: SurfaceRegistry {
                x_vox_version: 1,
                schema_version: 1,
                surfaces: vec![entry],
            },
            coverage: Some(snap(&[("surface", "surface:mesh", "surfaced")])),
            ..Default::default()
        })
        .expect("build");
        assert_eq!(cov(&idx, "surface.mesh").kind.as_deref(), Some("surfaced"));
    }

    #[test]
    fn no_snapshot_keeps_reason() {
        let idx = index_with(
            vec![test_op(
                "x",
                Some((&["x", "y"], "active")),
                Some("vox_x"),
                "d",
            )],
            None,
        );
        for id in ["cli.x.y", "mcp.vox_x"] {
            let c = cov(&idx, id);
            assert!(c.kind.is_none());
            assert_eq!(
                c.reason.as_deref(),
                Some("no graph-coverage snapshot committed")
            );
        }
        assert!(idx.coverage_snapshot.is_none());
    }

    #[test]
    fn snapshot_meta_copied_verbatim() {
        let s = snap(&[("tool", "tool:vox_x", "surfaced")]);
        let idx = index_with(
            vec![test_op("x", None, Some("vox_x"), "d")],
            Some(s.clone()),
        );
        assert_eq!(idx.coverage_snapshot.as_ref(), Some(&s.source));
    }
}
