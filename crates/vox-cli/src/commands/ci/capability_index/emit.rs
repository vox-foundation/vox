//! Pure text emitters over the capability index: generated regions for `llms.txt` /
//! `llms-full.txt`. No IO; `mod.rs` owns reading and writing.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use vox_capability_registry::{CapabilityIndex, IndexRow, RowKind};

use super::join::single_line;

pub(crate) const BEGIN_MARKER: &str =
    "<!-- BEGIN GENERATED capability-index: vox ci capability-index --write -->";
pub(crate) const END_MARKER: &str = "<!-- END GENERATED capability-index -->";

const SITE: &str = "https://voxlang.org";
const INDEX_URL: &str = "https://github.com/vox-foundation/vox/blob/main/contracts/capabilities/capability-index.v1.json";
const QUERY_HINT: &str = "Query it: `vox explain <thing>`, `vox features --domain <d>`, MCP `vox_capability_explain` / `vox_capability_features`.";

/// Replace the text between the (unique) marker pair with `body`; bytes outside are untouched.
/// Never appends: a missing, duplicated or reversed marker is an error.
pub(crate) fn splice_region(doc: &str, body: &str) -> Result<String> {
    let (begins, ends) = (
        doc.matches(BEGIN_MARKER).count(),
        doc.matches(END_MARKER).count(),
    );
    if begins == 0 || ends == 0 {
        bail!("missing marker: expected one `{BEGIN_MARKER}` and one `{END_MARKER}`");
    }
    if begins != 1 || ends != 1 {
        bail!(
            "marker order/duplicates: found {begins} BEGIN and {ends} END markers, expected one each"
        );
    }
    let (Some(b), Some(e)) = (doc.find(BEGIN_MARKER), doc.find(END_MARKER)) else {
        bail!("missing marker");
    };
    let head_end = b + BEGIN_MARKER.len();
    if e < head_end {
        bail!("marker order/duplicates: END precedes BEGIN");
    }
    Ok(format!(
        "{}\n{}\n{}",
        &doc[..head_end],
        body.trim_end(),
        &doc[e..]
    ))
}

/// One printable line with no HTML comment delimiters (markup/prompt-injection guard).
pub(crate) fn safe_line(s: &str) -> String {
    let mut line = single_line(s).unwrap_or_default();
    // Loop: removing one delimiter can splice the surrounding text into a new one.
    while line.contains("<!--") || line.contains("-->") {
        line = line.replace("<!--", "").replace("-->", "");
    }
    line
}

/// Published site URL of a repo doc path (`docs/src/<x>.md` -> `https://voxlang.org/<x>/`).
pub(crate) fn site_url(repo_path: &str) -> String {
    let rest = repo_path.strip_prefix("docs/src/").unwrap_or(repo_path);
    let rest = rest.strip_suffix(".md").unwrap_or(rest);
    format!("{SITE}/{rest}/")
}

/// Inline-code span that cannot be closed early by the content.
fn code(s: &str) -> String {
    format!("`{}`", safe_line(s).replace('`', ""))
}

fn count_kind(index: &CapabilityIndex, domain: &str, kind: RowKind) -> usize {
    index
        .rows
        .iter()
        .filter(|r| r.kind == kind && r.domain.as_deref() == Some(domain))
        .count()
}

/// Compact capability region for `llms.txt` (per-domain counts plus links).
pub(crate) fn llms_compact(index: &CapabilityIndex) -> String {
    let mut out = String::from("## Capabilities (generated from the capability index)\n");
    for d in &index.domains {
        out.push_str(&format!(
            "- {}: {} CLI commands, {} MCP tools, {} GUI surfaces\n",
            safe_line(&d.label),
            count_kind(index, &d.id, RowKind::Cli),
            count_kind(index, &d.id, RowKind::McpTool),
            count_kind(index, &d.id, RowKind::Surface),
        ));
    }
    // CLI surface links the hand-written reference: the Starlight slug of
    // `cli-command-surface.generated.md` is unverified (RESEARCH A2).
    out.push_str(&format!(
        "\n- [MCP tool reference]({}): every MCP tool, its schema and contract\n\
         - [GUI navigation]({}): desktop app surfaces and how to reach them\n\
         - [CLI reference]({}): all `vox` subcommands\n\
         - [Capability index JSON]({INDEX_URL}): the machine-readable index this section is generated from\n\
         \n{QUERY_HINT}\n",
        site_url("docs/src/reference/mcp-tool-registry-contract.md"),
        site_url("docs/src/reference/gui-navigation.md"),
        site_url("docs/src/reference/cli.md"),
    ));
    out
}

/// Full capability region for `llms-full.txt`: one line per operation (a CLI row and its MCP
/// equivalent share a line), grouped by domain in vocabulary order.
pub(crate) fn llms_full(index: &CapabilityIndex) -> String {
    let mut ops: BTreeMap<String, (Option<&IndexRow>, Option<&IndexRow>)> = BTreeMap::new();
    for r in &index.rows {
        let slot = ops
            .entry(r.op_id.clone().unwrap_or_else(|| r.id.clone()))
            .or_default();
        match r.kind {
            RowKind::Cli => slot.0 = Some(r),
            RowKind::McpTool => slot.1 = Some(r),
            _ => {}
        }
    }
    let mut by_domain: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (cli, mcp) in ops.into_values() {
        let Some(primary) = cli.or(mcp) else { continue };
        let names: Vec<String> = [
            cli.and_then(|r| r.cli.as_ref())
                .map(|c| code(&c.invocation)),
            mcp.and_then(|r| r.mcp.as_ref()).map(|m| code(&m.name)),
        ]
        .into_iter()
        .flatten()
        .collect();
        let domain = primary
            .domain
            .clone()
            .unwrap_or_else(|| "other".to_string());
        by_domain.entry(domain).or_default().push((
            primary.id.clone(),
            format!("- {}: {}", names.join(" / "), safe_line(&primary.title)),
        ));
    }
    let mut out = String::from(
        "## Capability index (generated)\nEvery CLI command and MCP tool, grouped by domain. \
         Query it: `vox explain <thing>` or MCP `vox_capability_explain`.\n",
    );
    let mut section = |label: &str, mut items: Vec<(String, String)>| {
        items.sort();
        out.push_str(&format!("\n### {}\n", safe_line(label)));
        for (_, line) in items {
            out.push_str(&line);
            out.push('\n');
        }
    };
    for d in &index.domains {
        if let Some(items) = by_domain.remove(&d.id) {
            section(&d.label, items);
        }
    }
    for (id, items) in by_domain {
        section(&id, items);
    }
    out
}

/// Set the generator-owned top-level `capabilities` key of `vox-docs.json`; every other key keeps
/// its value and position (`serde_json` `preserve_order` is unified workspace-wide).
pub(crate) fn docs_json_with_capabilities(
    existing: &str,
    index: &CapabilityIndex,
) -> Result<String> {
    let mut doc: serde_json::Value =
        serde_json::from_str(existing).context("parse vox-docs.json")?;
    let Some(obj) = doc.as_object_mut() else {
        bail!("vox-docs.json: top level is not an object");
    };
    let domains: Vec<serde_json::Value> = index
        .domains
        .iter()
        .map(|d| {
            serde_json::json!({
                "id": d.id,
                "label": safe_line(&d.label),
                "cli": count_kind(index, &d.id, RowKind::Cli),
                "mcp": count_kind(index, &d.id, RowKind::McpTool),
                "surfaces": count_kind(index, &d.id, RowKind::Surface),
            })
        })
        .collect();
    obj.insert(
        "capabilities".to_string(),
        serde_json::json!({
            "index": "contracts/capabilities/capability-index.v1.json",
            "explain_cli": "vox explain <thing>",
            "features_cli": "vox features --domain <d>",
            "explain_mcp": "vox_capability_explain",
            "features_mcp": "vox_capability_features",
            "mcp_tool_reference": site_url("docs/src/reference/mcp-tool-registry-contract.md"),
            "gui_navigation": site_url("docs/src/reference/gui-navigation.md"),
            "domains": domains,
        }),
    );
    let mut text = serde_json::to_string_pretty(&doc).context("serialize vox-docs.json")?;
    text.push('\n');
    Ok(text)
}

/// A TypeScript string literal (JSON string syntax is valid TS and escapes quotes, backslashes
/// and control characters, so no raw text is ever interpolated).
pub(super) fn ts_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

pub(super) fn ts_opt(s: Option<&str>) -> String {
    s.map_or_else(|| "null".to_string(), ts_str)
}

fn ts_kind(kind: RowKind) -> &'static str {
    match kind {
        RowKind::Cli => "cli",
        RowKind::McpTool => "mcp_tool",
        RowKind::Surface => "surface",
        RowKind::Crate => "crate",
        RowKind::Skill => "skill",
        RowKind::Doc => "doc",
    }
}

/// Rows per emitted `ROWS_<n>` chunk: one 1544-row array literal trips TS2590 ("union type too
/// complex"); the limit sits between 800 and 1544 rows, and 200 leaves margin for the wider rows.
const TS_CHUNK_ROWS: usize = 200;

/// Deterministic TypeScript projection of the index for the GUI build (trimmed fields).
pub(crate) fn generate_ts(
    index: &CapabilityIndex,
    groups: &[super::groups::CapabilityGroup],
) -> String {
    let mut out = String::from(
        "// AUTO-GENERATED by `vox ci capability-index --write`. DO NOT EDIT.\n\
         // Source: contracts/capabilities/capability-index.v1.json\n\
         export interface CapabilityIndexEntry {\n  \
         id: string;\n  \
         kind: 'cli' | 'mcp_tool' | 'surface' | 'crate' | 'skill' | 'doc';\n  \
         title: string;\n  \
         summary: string | null;\n  \
         domain: string | null;\n  \
         cli: string | null;\n  \
         mcp: string | null;\n  \
         viewKey: string | null;\n  \
         crate: string | null;\n  \
         layer: number | null;\n  \
         docs: string | null;\n  \
         coverage: string | null;\n  \
         equivalents: string[];\n  \
         refs: string[];\n  \
         tier: string | null;\n  \
         status: string | null;\n  \
         sideEffect: string | null;\n  \
         lane: string | null;\n  \
         catalogForm: boolean;\n  \
          navLabel: string | null;\n  \
          source: string | null;\n  \
          docOrigin: string | null;\n  \
          coverageReason: string | null;\n  \
          coverageAmbiguous: boolean;\n}\n\
         export const CAPABILITY_DOMAINS: { id: string; label: string }[] = [\n",
    );
    for d in &index.domains {
        out.push_str(&format!(
            "  {{ id: {}, label: {} }},\n",
            ts_str(&d.id),
            ts_str(&d.label)
        ));
    }
    out.push_str(
        "];\nexport const CAPABILITY_COVERAGE_AS_OF: { sha: string; date: string } | null = ",
    );
    match &index.coverage_snapshot {
        Some(s) => out.push_str(&format!(
            "{{ sha: {}, date: {} }};\n",
            ts_str(&s.git_sha),
            ts_str(s.built_at.split('T').next().unwrap_or(&s.built_at))
        )),
        None => out.push_str("null;\n"),
    }
    out.push_str(&super::groups::ts_groups(groups));
    let reasons: Vec<&str> = index
        .rows
        .iter()
        .filter_map(|r| r.coverage.as_ref()?.reason.as_deref())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    for (i, r) in reasons.iter().enumerate() {
        out.push_str(&format!("const R_{i} = {};\n", ts_str(r)));
    }
    for (n, chunk) in index.rows.chunks(TS_CHUNK_ROWS).enumerate() {
        out.push_str(&format!("const ROWS_{n}: CapabilityIndexEntry[] = [\n"));
        for r in chunk {
            out.push_str(&ts_row(r, &reasons));
        }
        out.push_str("];\n");
    }
    let spread: Vec<String> = (0..index.rows.chunks(TS_CHUNK_ROWS).count())
        .map(|n| format!("...ROWS_{n}"))
        .collect();
    out.push_str(&format!(
        "export const CAPABILITY_INDEX: CapabilityIndexEntry[] = [{}];\n",
        spread.join(", ")
    ));
    out
}

/// One row literal. Every string goes through `ts_str`; nothing raw is interpolated.
fn ts_row(r: &IndexRow, reasons: &[&str]) -> String {
    let summary: Option<String> = r.summary.as_ref().map(|s| s.chars().take(120).collect());
    let layer = r
        .owner
        .as_ref()
        .and_then(|o| o.layer)
        .map_or_else(|| "null".to_string(), |l| l.to_string());
    let ids = |v: &[String]| v.iter().map(|s| ts_str(s)).collect::<Vec<_>>().join(", ");
    let (c_reason, c_ambig) = match &r.coverage {
        Some(c) => (
            c.reason
                .as_deref()
                .and_then(|s| reasons.binary_search(&s).ok())
                .map_or_else(|| "null".into(), |i| format!("R_{i}")),
            c.ambiguous,
        ),
        None => ("null".into(), false),
    };
    format!(
        "  {{ id: {}, kind: '{}', title: {}, summary: {}, domain: {}, cli: {}, mcp: {}, viewKey: {}, crate: {}, layer: {}, docs: {}, coverage: {}, equivalents: [{}], refs: [{}], tier: {}, status: {}, sideEffect: {}, lane: {}, catalogForm: {}, navLabel: {}, source: {}, docOrigin: {}, coverageReason: {}, coverageAmbiguous: {} }},\n",
        ts_str(&r.id),
        ts_kind(r.kind),
        ts_str(&r.title),
        ts_opt(summary.as_deref()),
        ts_opt(r.domain.as_deref()),
        ts_opt(r.cli.as_ref().map(|c| c.invocation.as_str())),
        ts_opt(r.mcp.as_ref().map(|m| m.name.as_str())),
        ts_opt(r.gui.as_ref().and_then(|g| g.view_key.as_deref())),
        ts_opt(r.owner.as_ref().map(|o| o.crate_name.as_str())),
        layer,
        ts_opt(r.docs.first().map(|d| d.path.as_str())),
        ts_opt(r.coverage.as_ref().and_then(|c| c.kind.as_deref())),
        ids(&r.equivalents),
        ids(&r.refs),
        ts_opt(r.tier.as_deref()),
        ts_opt(r.status.as_deref()),
        ts_opt(r.side_effect_class.as_deref()),
        ts_opt(r.product_lane.as_deref()),
        r.gui.as_ref().is_some_and(|g| g.catalog_form),
        ts_opt(r.gui.as_ref().and_then(|g| g.nav_label.as_deref())),
        ts_opt(r.source_path.as_deref()),
        ts_opt(r.docs.first().map(|d| d.origin.as_str())),
        c_reason,
        c_ambig,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_capability_registry::{
        CliRef, CoverageRef, DocLink, DomainDef, IndexRow, McpRef, RowKind,
    };

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
    fn sanitizer_strips_bidi_zero_width_and_tag_characters() {
        // RLO, zero-width space, BOM, isolates and a Unicode tag letter (hidden-text carrier).
        let s = safe_line("a\u{202E}b\u{200B}c\u{FEFF}d\u{2066}e\u{2069}f\u{E0041}g");
        assert_eq!(s, "abcdefg");
        let mut idx = fixture();
        idx.rows[0].title = "ti\u{E0049}\u{202E}tle".into();
        let out = llms_full(&idx);
        assert!(out.contains(": title\n"), "title not emitted: {out}");
        assert!(
            !out.contains('\u{202E}')
                && !out
                    .chars()
                    .any(|c| ('\u{E0000}'..='\u{E007F}').contains(&c)),
            "invisible characters survived"
        );
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

    // --- vox-docs.json + GUI TypeScript artifact ---

    fn with_snapshot() -> CapabilityIndex {
        let mut idx = fixture();
        idx.coverage_snapshot = Some(vox_capability_registry::CoverageSnapshotMeta {
            corpus_id: "c".into(),
            git_sha: "abc123".into(),
            built_at: "2026-10-09T08:03:00+00:00".into(),
        });
        idx
    }

    #[test]
    fn docs_json_sets_capabilities_and_keeps_other_keys() {
        let existing = r#"{"project":"Vox","key_pages":{"a":"b"},"status":"pre-1.0"}"#;
        let once = docs_json_with_capabilities(existing, &fixture()).expect("once");
        let v: serde_json::Value = serde_json::from_str(&once).expect("json");
        let keys: Vec<&String> = v.as_object().expect("object").keys().collect();
        assert_eq!(keys, ["project", "key_pages", "status", "capabilities"]);
        assert_eq!(v["key_pages"]["a"], "b");
        let twice = docs_json_with_capabilities(&once, &fixture()).expect("twice");
        assert_eq!(once, twice, "regenerating is idempotent");
        assert!(once.ends_with("}\n"));
    }

    #[test]
    fn docs_json_capabilities_shape() {
        let out = docs_json_with_capabilities("{}", &fixture()).expect("json");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        let c = &v["capabilities"];
        for k in [
            "index",
            "explain_cli",
            "explain_mcp",
            "features_cli",
            "features_mcp",
            "mcp_tool_reference",
            "gui_navigation",
        ] {
            assert!(c[k].is_string(), "capabilities.{k} missing in {out}");
        }
        assert_eq!(c["explain_mcp"], "vox_capability_explain");
        let d0 = &c["domains"][0];
        assert_eq!(d0["id"], "compiler");
        assert_eq!(
            (
                d0["cli"].as_u64(),
                d0["mcp"].as_u64(),
                d0["surfaces"].as_u64()
            ),
            (Some(1), Some(2), Some(0))
        );
        assert_eq!(c["domains"][1]["surfaces"], 1);
        assert!(docs_json_with_capabilities("[]", &fixture()).is_err());
    }

    #[test]
    fn generate_ts_is_typed_and_deterministic() {
        let ts = generate_ts(&with_snapshot(), &[]);
        assert!(
            ts.starts_with("// AUTO-GENERATED by `vox ci capability-index --write`. DO NOT EDIT.")
        );
        for needle in [
            "export interface CapabilityIndexEntry",
            "export const CAPABILITY_INDEX: CapabilityIndexEntry[]",
            "export const CAPABILITY_DOMAINS",
            "export const CAPABILITY_COVERAGE_AS_OF",
            "{ sha: \"abc123\", date: \"2026-10-09\" }",
            "id: \"cli.check\"",
            "kind: 'mcp_tool'",
            "mcp: \"vox_check\"",
            "const ROWS_0: CapabilityIndexEntry[] = [",
            "equivalents: [",
            "sideEffect: ",
            "catalogForm: ",
            "navLabel: ",
        ] {
            assert!(ts.contains(needle), "missing {needle:?} in:\n{ts}");
        }
        assert_eq!(ts, generate_ts(&with_snapshot(), &[]));
        assert!(
            generate_ts(&fixture(), &[]).contains(
                "CAPABILITY_COVERAGE_AS_OF: { sha: string; date: string } | null = null;"
            )
        );
    }

    #[test]
    fn generate_ts_empty_index() {
        let mut idx = fixture();
        idx.rows.clear();
        let ts = generate_ts(&idx, &[]);
        assert!(ts.contains("export const CAPABILITY_INDEX: CapabilityIndexEntry[] = [];"));
        assert!(!ts.contains("const ROWS_"));
    }

    #[test]
    fn generate_ts_chunks_every_row() {
        use std::collections::BTreeSet;
        let mut idx = fixture();
        idx.rows = (0..450)
            .map(|i| cli_row(&format!("chunk{i}"), "T", "d", "op"))
            .collect();
        let ts = generate_ts(&idx, &[]);
        let declared: Vec<&str> = ts
            .lines()
            .filter_map(|l| l.strip_prefix("const ")?.split(':').next())
            .filter(|n| n.starts_with("ROWS_"))
            .collect();
        assert_eq!(declared.len(), 3, "expected 450 rows in 3 chunks:\n{ts}");
        let export = ts
            .lines()
            .find(|l| l.starts_with("export const CAPABILITY_INDEX"))
            .expect("export line");
        let spread: Vec<&str> = export
            .split("...")
            .skip(1)
            .map(|s| s.trim_end_matches(|c| c == ',' || c == ' ' || c == ']' || c == ';'))
            .collect();
        assert_eq!(spread.len(), declared.len(), "spread: {export}");
        assert_eq!(
            spread.iter().collect::<BTreeSet<_>>(),
            declared.iter().collect::<BTreeSet<_>>(),
            "every declared chunk must be spread exactly once: {export}"
        );
        assert_eq!(
            ts.lines()
                .filter(|l| l.starts_with("  { id: ") && l.contains(", kind: '"))
                .count(),
            450
        );
    }

    #[test]
    fn ts_strings_are_json_escaped() {
        let mut idx = fixture();
        let nasty = "say \"hi\" \\ back\nnext";
        idx.rows[0].title = nasty.into();
        let ts = generate_ts(&idx, &[]);
        let literal = serde_json::to_string(nasty).expect("literal");
        assert!(ts.contains(&format!("title: {literal}")), "got:\n{ts}");
        assert!(!ts.contains("say \"hi\" \\ back\nnext"));
    }

    #[test]
    fn ts_summary_is_trimmed() {
        let mut idx = fixture();
        idx.rows[0].summary = Some("x".repeat(300));
        let ts = generate_ts(&idx, &[]);
        assert!(ts.contains(&"x".repeat(120)));
        assert!(!ts.contains(&"x".repeat(121)));
    }

    fn cov(k: Option<&str>, r: Option<&str>, a: bool) -> Option<CoverageRef> {
        Some(CoverageRef {
            kind: k.map(Into::into),
            basis: "b".into(),
            ambiguous: a,
            reason: r.map(Into::into),
        })
    }
    fn doc_link(p: &str, o: &str) -> DocLink {
        DocLink {
            path: p.into(),
            origin: o.into(),
        }
    }

    #[test]
    fn generate_ts_emits_groups_and_parity_fields() {
        let mut idx = fixture();
        let mut r1 = cli_row("r1", "R1", "d", "op");
        r1.coverage = cov(Some("k"), None, true);
        let mut r2 = cli_row("r2", "R2", "d", "op");
        r2.coverage = cov(None, Some("r1"), false);
        let mut r3 = cli_row("r3", "R3", "d", "op");
        r3.docs = vec![doc_link("p1", "claim"), doc_link("p2", "pinned")];
        idx.rows = vec![r1, r2, r3];
        let ts = generate_ts(&idx, &[]);
        let needles = [
            "export const CAPABILITY_GROUPS",
            "coverageAmbiguous: true",
            "const R_0 = \"r1\";",
            "coverageReason: R_0",
            "docOrigin: \"claim\"",
            "docOrigin: string | null;",
            "coverageReason: string | null;",
            "coverageAmbiguous: boolean;",
        ];
        for s in needles {
            assert!(ts.contains(s), "missing {s}");
        }
        let ambig_count = ts
            .lines()
            .filter(|l| l.starts_with("  { id: ") && l.contains("coverageAmbiguous: "))
            .count();
        assert_eq!(ambig_count, 3);
    }

    #[test]
    fn reason_consts_are_deduplicated_and_escaped() {
        let mut idx = fixture();
        let mut r1 = cli_row("r1", "R1", "d", "op");
        r1.coverage = cov(None, Some("reason \"q\""), false);
        let mut r2 = cli_row("r2", "R2", "d", "op");
        r2.coverage = cov(None, Some("reason \"q\""), false);
        idx.rows = vec![r1, r2];
        let ts = generate_ts(&idx, &[]);
        assert_eq!(ts.lines().filter(|l| l.starts_with("const R_")).count(), 1);
        assert!(ts.contains("const R_0 = \"reason \\\"q\\\"\";"));
    }
}
