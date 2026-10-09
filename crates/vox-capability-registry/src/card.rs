//! Plain-text renderers over the capability index: the four-group explain card (D-11), the
//! shortlist / no-match text, and the domain listing (D-09).
//!
//! Only index data is printed. Anything absent renders as `—`; nothing is ever invented.
//! Coverage age is shown, never gated on (D-08).

use std::collections::HashSet;
use std::fmt::Write as _;

use anyhow::Result;

use crate::explain::{Resolution, domain_counts, rows_in_domain};
use crate::index::{CapabilityIndex, IndexRow, RowKind};

const DASH: &str = "—";
/// Most MCP names listed on one card line before `+N more`.
const MAX_MCP_NAMES: usize = 5;
/// Most sibling rows in the Related group.
const RELATED_LIMIT: usize = 5;

fn kind_str(k: RowKind) -> &'static str {
    match k {
        RowKind::Cli => "cli",
        RowKind::McpTool => "mcp_tool",
        RowKind::Surface => "surface",
        RowKind::Crate => "crate",
        RowKind::Skill => "skill",
        RowKind::Doc => "doc",
    }
}

/// Rows linked to `row` through `equivalents` / `refs`, sorted and deduped by id.
fn linked<'a>(index: &'a CapabilityIndex, row: &IndexRow) -> Vec<&'a IndexRow> {
    let mut v: Vec<&IndexRow> = row
        .equivalents
        .iter()
        .chain(&row.refs)
        .filter_map(|id| index.row(id))
        .collect();
    v.sort_by(|a, b| a.id.cmp(&b.id));
    v.dedup_by(|a, b| a.id == b.id);
    v
}

/// `f(row)` if the row has it itself, else `f` over the linked rows (sorted, deduped).
fn gather(
    index: &CapabilityIndex,
    row: &IndexRow,
    f: impl Fn(&IndexRow) -> Option<String>,
) -> Vec<String> {
    if let Some(own) = f(row) {
        return vec![own];
    }
    let mut v: Vec<String> = linked(index, row).into_iter().filter_map(f).collect();
    v.sort();
    v.dedup();
    v
}

fn or_dash(v: &[String]) -> String {
    if v.is_empty() {
        DASH.to_string()
    } else {
        v.join(", ")
    }
}

fn gui_text(r: &IndexRow) -> Option<String> {
    let g = r.gui.as_ref()?;
    match (&g.nav_label, &g.view_key) {
        (Some(n), Some(k)) => Some(format!("{n} (view {k})")),
        (None, Some(k)) => Some(format!("view {k}")),
        (Some(n), None) => Some(n.clone()),
        (None, None) => None,
    }
}

fn owner_text(r: &IndexRow) -> String {
    match &r.owner {
        Some(o) => match o.layer {
            Some(l) => format!("{} (L{l})", o.crate_name),
            None => o.crate_name.clone(),
        },
        None => "unresolved".to_string(),
    }
}

fn domain_label(index: &CapabilityIndex, id: Option<&str>) -> String {
    let Some(id) = id else {
        return DASH.to_string();
    };
    index
        .domains
        .iter()
        .find(|d| d.id == id)
        .map_or_else(|| id.to_string(), |d| d.label.clone())
}

fn coverage_text(index: &CapabilityIndex, row: &IndexRow) -> String {
    let cov = match &row.coverage {
        Some(c) => match &c.kind {
            Some(k) if c.ambiguous => format!("{k} (ambiguous)"),
            Some(k) => k.clone(),
            None => format!(
                "none ({})",
                c.reason.as_deref().unwrap_or("no reason recorded")
            ),
        },
        None => "none (not computed)".to_string(),
    };
    let snap = match &index.coverage_snapshot {
        Some(s) => {
            let sha: String = s.git_sha.chars().take(9).collect();
            format!(
                "coverage as of {sha} ({})",
                s.built_at.split('T').next().unwrap_or(&s.built_at)
            )
        }
        None => "no snapshot".to_string(),
    };
    format!("{cov} — {snap}")
}

/// The four-group card (Equivalents, Location, Status, Related) for one row.
#[must_use]
pub fn render_card(index: &CapabilityIndex, row: &IndexRow) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "{} [{}] {}", row.title, kind_str(row.kind), row.id);
    if let Some(sum) = &row.summary {
        let _ = writeln!(s, "{sum}");
    }

    let mut mcp = gather(index, row, |r| r.mcp.as_ref().map(|m| m.name.clone()));
    if mcp.len() > MAX_MCP_NAMES {
        let more = mcp.len() - MAX_MCP_NAMES;
        mcp.truncate(MAX_MCP_NAMES);
        mcp.push(format!("+{more} more"));
    }
    let cli = gather(index, row, |r| r.cli.as_ref().map(|c| c.invocation.clone()));
    let gui = gather(index, row, gui_text);
    let generic = std::iter::once(row)
        .chain(linked(index, row))
        .any(|r| r.gui.as_ref().is_some_and(|g| g.catalog_form));
    let _ = writeln!(
        s,
        "\nEquivalents\n  CLI: {}\n  MCP: {}",
        or_dash(&cli),
        or_dash(&mcp)
    );
    let _ = writeln!(s, "  GUI: {}", gui.first().map_or(DASH, String::as_str));
    let _ = writeln!(
        s,
        "  Catalog: {}",
        if generic { "generic form" } else { DASH }
    );

    let _ = writeln!(s, "\nLocation\n  Crate: {}", owner_text(row));
    let _ = writeln!(
        s,
        "  Source: {}",
        row.source_path.as_deref().unwrap_or(DASH)
    );
    if row.docs.is_empty() {
        let _ = writeln!(s, "  Docs: {DASH}");
    }
    for d in &row.docs {
        let _ = writeln!(s, "  Docs: {} ({})", d.path, d.origin);
    }

    let _ = writeln!(s, "\nStatus\n  Coverage: {}", coverage_text(index, row));
    let _ = writeln!(s, "  Tier: {}", row.tier.as_deref().unwrap_or(DASH));
    let _ = writeln!(
        s,
        "  Side effects: {}",
        row.side_effect_class.as_deref().unwrap_or("unknown")
    );
    let _ = writeln!(
        s,
        "  Domain: {} · lane {}",
        domain_label(index, row.domain.as_deref()),
        row.product_lane.as_deref().unwrap_or(DASH)
    );

    let skip: HashSet<&str> = row
        .equivalents
        .iter()
        .chain(&row.refs)
        .map(String::as_str)
        .collect();
    let same_owner = |r: &IndexRow| {
        row.owner
            .as_ref()
            .zip(r.owner.as_ref())
            .is_some_and(|(a, b)| a.crate_name == b.crate_name)
    };
    let mut related: Vec<&IndexRow> = index
        .rows
        .iter()
        .filter(|r| matches!(r.kind, RowKind::Cli | RowKind::McpTool | RowKind::Surface))
        .filter(|r| r.id != row.id && !skip.contains(r.id.as_str()))
        .filter(|r| (row.domain.is_some() && r.domain == row.domain) || same_owner(r))
        .collect();
    related.sort_by(|a, b| a.id.cmp(&b.id));
    let _ = writeln!(s, "\nRelated");
    if related.is_empty() {
        let _ = writeln!(s, "  {DASH}");
    }
    for r in related.into_iter().take(RELATED_LIMIT) {
        let _ = writeln!(s, "  {} — {}", r.id, r.title);
    }
    s
}

/// Card, shortlist or no-match text for a [`Resolution`] of `query`.
#[must_use]
pub fn render_resolution(index: &CapabilityIndex, query: &str, r: &Resolution) -> String {
    let q = query.trim();
    match r {
        Resolution::Exact { primary, also } => {
            let mut s = render_card(index, primary);
            if !also.is_empty() {
                s.push_str("\nAlso matches:\n");
                for a in also {
                    let _ = writeln!(s, "  {}", a.id);
                }
            }
            s
        }
        Resolution::Shortlist(c) => {
            let mut s = format!("No exact match for `{q}`. Did you mean:\n");
            for (i, cand) in c.iter().enumerate() {
                let _ = writeln!(s, "{}. {} — {}", i + 1, cand.row.id, cand.row.title);
            }
            s
        }
        Resolution::NoMatch => {
            format!("No capability matches `{q}`. Try `vox features` to browse by domain.\n")
        }
    }
}

/// One operation line: `<lead> · MCP <names> · GUI <view> · <crate> · <first doc>`.
fn op_line(index: &CapabilityIndex, r: &IndexRow) -> String {
    let lead = r
        .cli
        .as_ref()
        .map(|c| c.invocation.as_str())
        .or(r.mcp.as_ref().map(|m| m.name.as_str()))
        .unwrap_or(&r.id);
    let mcp = gather(index, r, |x| x.mcp.as_ref().map(|m| m.name.clone()));
    let view = gather(index, r, |x| {
        x.gui.as_ref().and_then(|g| g.view_key.clone())
    });
    let doc = r.docs.first().map_or(DASH, |d| d.path.as_str());
    format!(
        "  {lead} · MCP {} · GUI {} · {} · {doc}",
        or_dash(&mcp),
        view.first().map_or(DASH, String::as_str),
        owner_text(r)
    )
}

/// Domain overview (`None`) or the operations of one domain (`Some`).
pub fn render_features(index: &CapabilityIndex, domain: Option<&str>) -> Result<String> {
    let mut s = String::new();
    let Some(d) = domain else {
        s.push_str("Capability domains (vox features --domain <id>):\n");
        for (d, cli, mcp, gui) in domain_counts(index) {
            let _ = writeln!(
                s,
                "  {} — {} · cli {cli} · mcp {mcp} · gui {gui}",
                d.id, d.label
            );
        }
        return Ok(s);
    };
    let rows = rows_in_domain(index, d)?;
    let id = d.trim().to_lowercase();
    let _ = writeln!(s, "{} ({id})", domain_label(index, Some(&id)));
    let mut paired: HashSet<&str> = HashSet::new();
    let mut ops: Vec<String> = Vec::new();
    for r in rows.iter().filter(|r| r.kind == RowKind::Cli) {
        paired.extend(
            linked(index, r)
                .into_iter()
                .filter(|p| p.kind == RowKind::McpTool)
                .map(|p| p.id.as_str()),
        );
        ops.push(op_line(index, r));
    }
    for r in rows
        .iter()
        .filter(|r| r.kind == RowKind::McpTool && !paired.contains(r.id.as_str()))
    {
        ops.push(op_line(index, r));
    }
    if ops.is_empty() {
        ops.push(format!("  {DASH}"));
    }
    s.push_str(&ops.join("\n"));
    s.push('\n');
    let mut section = |title: &str, kind: RowKind, f: &dyn Fn(&IndexRow) -> String| {
        let lines: Vec<String> = rows
            .iter()
            .filter(|r| r.kind == kind)
            .map(|r| f(r))
            .collect();
        if !lines.is_empty() {
            let _ = write!(s, "\n{title}:\n{}\n", lines.join("\n"));
        }
    };
    section("Surfaces", RowKind::Surface, &|r| {
        let view = r
            .gui
            .as_ref()
            .and_then(|g| g.view_key.as_deref())
            .unwrap_or(DASH);
        format!("  {} · GUI {view} · {}", r.title, r.id)
    });
    section("Crates", RowKind::Crate, &|r| {
        format!("  {} · {}", r.title, owner_text(r))
    });
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RowKind;
    use crate::explain::resolve;
    use crate::index::CoverageSnapshotMeta;
    use crate::test_fixture::{cli_ref, fixture_index, row};

    fn card(index: &CapabilityIndex, id: &str) -> String {
        render_card(index, index.row(id).expect("fixture row"))
    }

    const GOLDEN: &str = "\
Secrets Get [cli] cli.secrets.get
CLI operation `vox secrets get`

Equivalents
  CLI: vox secrets get
  MCP: vox_secrets_get
  GUI: —
  Catalog: —

Location
  Crate: vox-secrets (L2)
  Source: crates/vox-secrets/
  Docs: docs/src/reference/secrets-ssot.md (claim)

Status
  Coverage: Surfaced — no snapshot
  Tier: advanced
  Side effects: unknown
  Domain: Secrets · lane platform

Related
  cli.secrets — Secrets
";

    #[test]
    fn golden_card_for_paired_cli_row() {
        let idx = fixture_index();
        let c = card(&idx, "cli.secrets.get");
        assert_eq!(c, GOLDEN);
        let pos = |h: &str| c.find(h).unwrap_or_else(|| panic!("missing {h}"));
        assert!(pos("Equivalents") < pos("Location"));
        assert!(pos("Location") < pos("Status"));
        assert!(pos("Status") < pos("Related"));
    }

    #[test]
    fn absent_equivalents_render_as_dash() {
        let c = card(&fixture_index(), "cli.check");
        assert!(c.contains("MCP: —"), "{c}");
        assert!(c.contains("GUI: —"), "{c}");
    }

    #[test]
    fn surface_card_lists_linked_mcp_tools() {
        let c = card(&fixture_index(), "surface.mesh");
        assert!(c.contains("GUI: Mesh (view mesh)"), "{c}");
        assert!(c.contains("MCP: vox_mesh_nodes"), "{c}");
    }

    #[test]
    fn coverage_as_of_when_snapshot_present() {
        let mut idx = fixture_index();
        idx.coverage_snapshot = Some(CoverageSnapshotMeta {
            corpus_id: "vox".into(),
            git_sha: "abc123def456".into(),
            built_at: "2026-10-08T01:02:03Z".into(),
        });
        let c = card(&idx, "cli.secrets.get");
        assert!(c.contains("as of abc123def (2026-10-08)"), "{c}");
    }

    #[test]
    fn no_snapshot_message() {
        let c = card(&fixture_index(), "cli.check");
        assert!(c.contains("no snapshot"), "{c}");
        assert!(c.contains("no graph-coverage snapshot committed"), "{c}");
    }

    #[test]
    fn related_is_capped_sorted_and_excludes_self() {
        let mut idx = fixture_index();
        for n in (1..=7).rev() {
            let id = format!("cli.secrets.s{n}");
            let mut r = row(&id, RowKind::Cli, &format!("S{n}"));
            r.domain = Some("secrets".into());
            r.cli = cli_ref(&["secrets", &format!("s{n}")]);
            idx.rows.push(r);
        }
        let c = card(&idx, "cli.secrets.get");
        let related: Vec<&str> = c
            .split("Related\n")
            .nth(1)
            .expect("related group")
            .lines()
            .map(str::trim)
            .collect();
        let ids: Vec<&str> = related
            .iter()
            .filter_map(|l| l.split(" — ").next())
            .collect();
        assert_eq!(
            ids,
            [
                "cli.secrets",
                "cli.secrets.s1",
                "cli.secrets.s2",
                "cli.secrets.s3",
                "cli.secrets.s4"
            ]
        );
    }

    #[test]
    fn render_resolution_shortlist_and_no_match() {
        let idx = fixture_index();
        let s = render_resolution(&idx, "secrts get", &resolve(&idx, "secrts get").unwrap());
        assert!(s.contains("1. cli.secrets.get — Secrets Get"), "{s}");
        let n = render_resolution(&idx, "zzqqxx", &resolve(&idx, "zzqqxx").unwrap());
        assert!(n.contains("vox features"), "{n}");
    }

    #[test]
    fn features_listing_shows_layer_and_docs() {
        let idx = fixture_index();
        let out = render_features(&idx, Some("secrets")).unwrap();
        let line = out
            .lines()
            .find(|l| l.contains("vox secrets get"))
            .expect("op line");
        for want in [
            "vox_secrets_get",
            "vox-secrets (L2)",
            "docs/src/reference/secrets-ssot.md",
        ] {
            assert!(line.contains(want), "{want} missing in {line}");
        }
        assert_eq!(
            out.lines()
                .filter(|l| l.contains("vox_secrets_get"))
                .count(),
            1,
            "{out}"
        );
        assert!(out.contains("Crates:"), "{out}");
        assert!(render_features(&idx, Some("nope")).is_err());
    }

    #[test]
    fn features_overview_counts() {
        let out = render_features(&fixture_index(), None).unwrap();
        assert!(
            out.lines()
                .any(|l| l.contains("secrets") && l.contains("cli 2 · mcp 1 · gui 0")),
            "{out}"
        );
        assert!(
            out.lines()
                .any(|l| l.contains("mesh") && l.contains("cli 0 · mcp 1 · gui 1")),
            "{out}"
        );
        assert!(out.contains("platform"), "{out}");
    }
}
