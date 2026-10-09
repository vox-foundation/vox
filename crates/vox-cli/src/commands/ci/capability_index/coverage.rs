//! Graph-coverage snapshot (D-07): written only by `--refresh-coverage`, read by check mode.
//!
//! The snapshot is a committed file with its own lifecycle. Check mode and `ssot-drift` read the
//! file and never build or open the code graph. Coverage is never invented: a row with no graph
//! key keeps `kind: null` and a reason.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use vox_bounded_fs::read_utf8_path_capped;
use vox_capability_registry::CoverageSnapshotMeta;
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
}
