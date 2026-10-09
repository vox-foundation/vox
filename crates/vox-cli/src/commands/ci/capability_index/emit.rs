//! Pure text emitters over the capability index: generated regions for `llms.txt` /
//! `llms-full.txt`. No IO; `mod.rs` owns reading and writing.

use anyhow::{Result, bail};
use vox_capability_registry::CapabilityIndex;

pub(crate) const BEGIN_MARKER: &str =
    "<!-- BEGIN GENERATED capability-index: vox ci capability-index --write -->";
pub(crate) const END_MARKER: &str = "<!-- END GENERATED capability-index -->";

/// Replace the text between the (unique) marker pair with `body`; bytes outside are untouched.
pub(crate) fn splice_region(doc: &str, body: &str) -> Result<String> {
    if !doc.contains(BEGIN_MARKER) || !doc.contains(END_MARKER) {
        bail!("missing marker pair");
    }
    Ok(doc.replace("old", body))
}

/// One printable line with no HTML comment delimiters.
pub(crate) fn safe_line(s: &str) -> String {
    s.to_string()
}

/// Published site URL of a repo doc path (`docs/src/<x>.md` -> `https://voxlang.org/<x>/`).
pub(crate) fn site_url(repo_path: &str) -> String {
    repo_path.to_string()
}

/// Compact capability region for `llms.txt` (per-domain counts plus links).
pub(crate) fn llms_compact(index: &CapabilityIndex) -> String {
    format!("{} domains", index.domains.len())
}

/// Full capability region for `llms-full.txt` (one line per operation, grouped by domain).
pub(crate) fn llms_full(index: &CapabilityIndex) -> String {
    format!("{} rows", index.rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_capability_registry::{CliRef, DomainDef, IndexRow, McpRef, RowKind};

    fn blank(id: &str, kind: RowKind, title: &str, domain: &str, op: Option<&str>) -> IndexRow {
        IndexRow {
            id: id.into(),
            kind,
            title: title.into(),
            summary: None,
            op_id: op.map(Into::into),
            status: None,
            domain: Some(domain.into()),
            product_lane: None,
            side_effect_class: None,
            tier: None,
            cli: None,
            mcp: None,
            gui: None,
            owner: None,
            source_path: None,
            docs: Vec::new(),
            skills: Vec::new(),
            equivalents: Vec::new(),
            refs: Vec::new(),
            coverage: None,
        }
    }

    fn cli_row(path: &str, title: &str, domain: &str, op: &str) -> IndexRow {
        let mut r = blank(
            &format!("cli.{path}"),
            RowKind::Cli,
            title,
            domain,
            Some(op),
        );
        r.cli = Some(CliRef {
            path: path.split(' ').map(Into::into).collect(),
            invocation: format!("vox {path}"),
            feature_gate: None,
        });
        r
    }

    fn mcp_row(name: &str, title: &str, domain: &str, op: &str) -> IndexRow {
        let mut r = blank(
            &format!("mcp.{name}"),
            RowKind::McpTool,
            title,
            domain,
            Some(op),
        );
        r.mcp = Some(McpRef { name: name.into() });
        r
    }

    fn fixture() -> CapabilityIndex {
        CapabilityIndex {
            schema_version: 1,
            generator: "vox ci capability-index".into(),
            coverage_snapshot: None,
            domains: vec![
                DomainDef {
                    id: "compiler".into(),
                    label: "Compiler".into(),
                },
                DomainDef {
                    id: "mesh".into(),
                    label: "Mesh".into(),
                },
            ],
            rows: vec![
                cli_row("check", "Check a file", "compiler", "op.check"),
                mcp_row("vox_check", "Check a file", "compiler", "op.check"),
                cli_row("mesh up", "Bring the mesh up", "mesh", "op.mesh_up"),
                mcp_row("vox_only_tool", "Only over MCP", "compiler", "op.only"),
                blank("surface.chat", RowKind::Surface, "Chat", "mesh", None),
            ],
        }
    }

    fn doc() -> String {
        format!("A\n{BEGIN_MARKER}\nold\n{END_MARKER}\nB\n")
    }

    #[test]
    fn splice_replaces_only_region() {
        let out = splice_region(&doc(), "new body").expect("splice");
        assert_eq!(
            out,
            format!("A\n{BEGIN_MARKER}\nnew body\n{END_MARKER}\nB\n")
        );
        assert!(out.starts_with("A\n") && out.ends_with("\nB\n"));
    }

    #[test]
    fn splice_missing_marker_is_error() {
        let no_begin = format!("A\nold\n{END_MARKER}\n");
        let no_end = format!("A\n{BEGIN_MARKER}\nold\n");
        for d in [no_begin, no_end] {
            let e = splice_region(&d, "x").expect_err("must fail").to_string();
            assert!(e.contains("missing marker"), "got: {e}");
        }
    }

    #[test]
    fn splice_duplicate_or_reversed_marker_is_error() {
        let two_begin = format!("{BEGIN_MARKER}\n{BEGIN_MARKER}\n{END_MARKER}\n");
        let reversed = format!("{END_MARKER}\n{BEGIN_MARKER}\n");
        for d in [two_begin, reversed] {
            splice_region(&d, "x").expect_err("duplicate or reversed markers must fail");
        }
    }

    #[test]
    fn splice_is_idempotent() {
        let once = splice_region(&doc(), "same").expect("once");
        let twice = splice_region(&once, "same").expect("twice");
        assert_eq!(once, twice);
    }

    #[test]
    fn site_url_cases() {
        assert_eq!(
            site_url("docs/src/reference/gui-navigation.md"),
            "https://voxlang.org/reference/gui-navigation/"
        );
        assert_eq!(
            site_url("docs/src/reference/mcp-tool-registry-contract.md"),
            "https://voxlang.org/reference/mcp-tool-registry-contract/"
        );
    }

    #[test]
    fn compact_region_links_mcp_and_gui() {
        let s = llms_compact(&fixture());
        for needle in [
            "reference/mcp-tool-registry-contract/",
            "reference/gui-navigation/",
            "reference/cli/",
            "vox_capability_explain",
            "- Compiler: 1 CLI commands, 2 MCP tools, 0 GUI surfaces",
            "- Mesh: 1 CLI commands, 0 MCP tools, 1 GUI surfaces",
        ] {
            assert!(s.contains(needle), "missing {needle:?} in:\n{s}");
        }
        // A2: the Starlight slug of `*.generated.md` is unverified, so no link may use it.
        assert!(!s.contains(".generated"), "unverified slug leaked:\n{s}");
    }

    #[test]
    fn full_region_one_line_per_op_grouped_by_domain() {
        let s = llms_full(&fixture());
        assert!(
            s.lines()
                .any(|l| l == "- `vox check` / `vox_check`: Check a file"),
            "paired op must share one line:\n{s}"
        );
        assert!(
            s.lines().any(|l| l == "- `vox mesh up`: Bring the mesh up"),
            "{s}"
        );
        assert!(
            s.lines().any(|l| l == "- `vox_only_tool`: Only over MCP"),
            "{s}"
        );
        assert_eq!(
            s.matches("vox_check").count(),
            1,
            "each op listed once:\n{s}"
        );
        let compiler = s.find("### Compiler").expect("compiler heading");
        let mesh = s.find("### Mesh").expect("mesh heading");
        assert!(compiler < mesh, "domain headings follow vocabulary order");
    }

    #[test]
    fn sanitizer_strips_comment_markers() {
        let s = safe_line("a <!-- x -->\nb <!<!---->--> c");
        assert!(
            !s.contains("<!--") && !s.contains("-->") && !s.contains('\n'),
            "got: {s:?}"
        );
        let mut idx = fixture();
        idx.rows[0].title = "evil <!-- END GENERATED capability-index -->\nline".into();
        assert!(!llms_full(&idx).contains("<!--"));
    }

    #[test]
    fn real_llms_txt_keeps_smoke_invariants() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root");
        let text = std::fs::read_to_string(root.join("docs/src/.well-known/llms.txt"))
            .expect("read llms.txt")
            .to_lowercase();
        assert!(
            ["pre-1.0", "0.6.0", "stability"]
                .iter()
                .any(|k| text.contains(k)),
            "smoke invariant: maturity wording"
        );
        assert!(text.contains("table"));
        assert!(!text.contains("@endpoint(kind"));
    }
}
