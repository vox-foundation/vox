//! Plain-text renderers over the capability index: the four-group explain card (D-11), the
//! shortlist / no-match text, and the domain listing (D-09).
//!
//! Only index data is printed. Anything absent renders as `—`; nothing is ever invented.
//! Coverage age is shown, never gated on (D-08).

use anyhow::{Result, bail};

use crate::explain::Resolution;
use crate::index::{CapabilityIndex, IndexRow};

/// The four-group card (Equivalents, Location, Status, Related) for one row.
#[must_use]
pub fn render_card(index: &CapabilityIndex, row: &IndexRow) -> String {
    let _ = (index, row);
    "RED stub: render_card not implemented".to_string()
}

/// Card, shortlist or no-match text for a [`Resolution`] of `query`.
#[must_use]
pub fn render_resolution(index: &CapabilityIndex, query: &str, r: &Resolution) -> String {
    let _ = (index, query, r);
    "RED stub: render_resolution not implemented".to_string()
}

/// Domain overview (`None`) or the operations of one domain (`Some`).
pub fn render_features(index: &CapabilityIndex, domain: Option<&str>) -> Result<String> {
    let _ = (index, domain);
    bail!("RED stub: render_features not implemented")
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
