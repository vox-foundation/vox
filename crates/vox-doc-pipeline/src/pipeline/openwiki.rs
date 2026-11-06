//! Phase 33 (`WIKI-01..04`): OpenWiki-Style Subsystem Architecture Hubs,
//! Deep Crate Symbol Extraction (`#Lstart-Lend` + Signatures + Reverse Dependents),
//! and Deterministic Validated Mermaid Architecture Diagrams.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::pipeline::openwiki_symbols::render_subsystem_index_markdown;

/// Canonical subsystem layer descriptor (`L0..L5`) from `docs/src/architecture/layers.toml` (`WIKI-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubsystemLayerSpec {
    pub layer: u8,
    pub slug: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
}

pub const SUBSYSTEM_LAYERS: &[SubsystemLayerSpec] = &[
    SubsystemLayerSpec {
        layer: 0,
        slug: "l0-pure-types",
        title: "Layer 0 — Pure Types & Contracts",
        summary: "Zero-I/O pure data structures, AST nodes, database row types, and KB facet schemas.",
    },
    SubsystemLayerSpec {
        layer: 1,
        slug: "l1-primitives",
        title: "Layer 1 — Primitives, Crypto, Secrets & Transport",
        summary: "Foundational primitives including vox-crypto, Clavis secrets, iroh QUIC mesh transport, and bounded FS.",
    },
    SubsystemLayerSpec {
        layer: 2,
        slug: "l2-domain-libs",
        title: "Layer 2 — Domain Libraries & Graph Reader",
        summary: "Pure domain engines: configuration, capability/MCP registries, code graph reader, evaluation, and journal.",
    },
    SubsystemLayerSpec {
        layer: 3,
        slug: "l3-heavy-runtimes",
        title: "Layer 3 — Compiler, VoxDb, Orchestrator, Search & Runtimes",
        summary: "Core execution engines: unified Vox compiler/codegen, VoxDb/Turso, multi-agent orchestrator, hybrid search, and Populi/Mens.",
    },
    SubsystemLayerSpec {
        layer: 4,
        slug: "l4-plugins-surfaces",
        title: "Layer 4 — Native Plugins & Terminal TUI",
        summary: "Dynamically loaded skill/inference plugins and the ratatui-based vox-term TUI.",
    },
    SubsystemLayerSpec {
        layer: 5,
        slug: "l5-binaries",
        title: "Layer 5 — Product Surfaces (CLI, Axis GUI, Daemon)",
        summary: "User-facing product binaries: vox CLI, Vox Axis desktop workbench (vox-gui), and orchestrator daemon.",
    },
];

/// Parsed crate entry from `docs/src/architecture/layers.toml` (`WIKI-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayerCrateEntry {
    pub name: String,
    pub layer: u8,
    pub kind: String,
    pub max_dependents: Option<u32>,
    pub max_loc: Option<u32>,
}

/// Parse `[crates]` entries from `docs/src/architecture/layers.toml` (`WIKI-01`).
pub fn parse_layers_toml(layers_toml_content: &str) -> Vec<LayerCrateEntry> {
    let mut entries = Vec::new();
    let mut in_crates = false;

    for raw_line in layers_toml_content.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_crates = line == "[crates]";
            continue;
        }
        if !in_crates {
            continue;
        }
        let Some((crate_name, rhs)) = line.split_once('=') else {
            continue;
        };
        let name = crate_name.trim().to_string();
        let rhs = rhs.trim();
        if !rhs.starts_with('{') {
            continue;
        }
        let inner = rhs.trim_start_matches('{').trim_end_matches('}');
        let mut layer = 3u8;
        let mut kind = "library".to_string();
        let mut max_dependents = None;
        let mut max_loc = None;

        for kv in inner.split(',') {
            let Some((k, v)) = kv.split_once('=') else {
                continue;
            };
            let k = k.trim();
            let v = v.trim().trim_matches('"');
            match k {
                "layer" => {
                    if let Ok(n) = v.parse::<u8>() {
                        layer = n;
                    }
                }
                "kind" => kind = v.to_string(),
                "max_dependents" => {
                    max_dependents = v.replace('_', "").parse::<u32>().ok();
                }
                "max_loc" => {
                    max_loc = v.replace('_', "").parse::<u32>().ok();
                }
                _ => {}
            }
        }
        entries.push(LayerCrateEntry {
            name,
            layer,
            kind,
            max_dependents,
            max_loc,
        });
    }
    entries.sort_by(|a, b| a.layer.cmp(&b.layer).then_with(|| a.name.cmp(&b.name)));
    entries
}

/// Deep public symbol record extracted from a crate source file (`WIKI-02`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeepCrateSymbol {
    pub kind: String,
    pub name: String,
    pub signature: String,
    pub doc_summary: String,
    pub rel_path: String,
    pub start_line: usize,
    pub end_line: usize,
}

impl DeepCrateSymbol {
    /// Format a GitHub/repo relative line-range anchor link (`[src/lib.rs#L10-L24]`) (`WIKI-02`).
    pub fn source_anchor_markdown(&self) -> String {
        if self.end_line > self.start_line {
            format!(
                "[`{}#L{}-L{}`](../../../{}#L{}-L{})",
                self.rel_path,
                self.start_line,
                self.end_line,
                self.rel_path,
                self.start_line,
                self.end_line
            )
        } else {
            format!(
                "[`{}#L{}`](../../../{}#L{})",
                self.rel_path, self.start_line, self.rel_path, self.start_line
            )
        }
    }
}

/// Extract top-level `pub struct`, `pub enum`, `pub trait`, `pub fn`, and `pub type` symbols
/// with signatures, doc comments, and 1-based `#L<start>-L<end>` spans (`WIKI-02`).
pub fn extract_deep_crate_symbols(source: &str, rel_path: &str) -> Vec<DeepCrateSymbol> {
    let mut symbols = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut current_doc: Vec<String> = Vec::new();

    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        let line_no = idx + 1;

        if let Some(doc) = trimmed.strip_prefix("///") {
            current_doc.push(doc.trim().to_string());
            idx += 1;
            continue;
        }
        if trimmed.starts_with("//") || trimmed.starts_with("#[") {
            idx += 1;
            continue;
        }
        if trimmed.is_empty() {
            current_doc.clear();
            idx += 1;
            continue;
        }

        if let Some(after_pub) = trimmed.strip_prefix("pub ") {
            let after_pub = after_pub.trim();
            if after_pub.starts_with('(') {
                current_doc.clear();
                idx += 1;
                continue;
            }

            let classified = if let Some(rest) = after_pub.strip_prefix("struct ") {
                Some(("struct", rest))
            } else if let Some(rest) = after_pub.strip_prefix("enum ") {
                Some(("enum", rest))
            } else if let Some(rest) = after_pub.strip_prefix("trait ") {
                Some(("trait", rest))
            } else if let Some(rest) = after_pub
                .strip_prefix("fn ")
                .or_else(|| after_pub.strip_prefix("async fn "))
                .or_else(|| after_pub.strip_prefix("const fn "))
            {
                Some(("fn", rest))
            } else {
                after_pub.strip_prefix("type ").map(|rest| ("type", rest))
            };

            if let Some((kind, rest)) = classified {
                let name = rest
                    .split(&[' ', '<', '(', ':', ';', '{', '='][..])
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !name.is_empty() {
                    let sig = trimmed
                        .split('{')
                        .next()
                        .unwrap_or(trimmed)
                        .trim()
                        .trim_end_matches(';')
                        .to_string();
                    let end_line = estimate_item_end_line(&lines, idx);
                    let doc_summary = current_doc.join(" ");
                    symbols.push(DeepCrateSymbol {
                        kind: kind.to_string(),
                        name,
                        signature: sig,
                        doc_summary,
                        rel_path: rel_path.replace('\\', "/"),
                        start_line: line_no,
                        end_line,
                    });
                }
            }
            current_doc.clear();
        } else {
            current_doc.clear();
        }
        idx += 1;
    }

    symbols
}

fn estimate_item_end_line(lines: &[&str], start_idx: usize) -> usize {
    let first = lines[start_idx];
    if first.trim_end().ends_with(';') || !first.contains('{') {
        return start_idx + 1;
    }
    let mut depth = 0i32;
    for (offset, line) in lines[start_idx..].iter().enumerate().take(120) {
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth <= 0 {
                    return start_idx + offset + 1;
                }
            }
        }
    }
    start_idx + 1
}

/// Compute reverse dependents (`Used By`) from `(crate_name -> direct_workspace_deps)` (`WIKI-02`).
pub fn compute_reverse_dependents(
    forward_deps: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    let mut reverse: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (pkg, deps) in forward_deps {
        reverse.entry(pkg.clone()).or_default();
        for dep in deps {
            reverse.entry(dep.clone()).or_default().insert(pkg.clone());
        }
    }
    reverse
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect()
}

fn diagram_node_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Close a VoxDiagram `graph` body and reject it unless `vox_diagram::check` accepts it.
fn finish_diagram(mut body: String) -> Result<String, String> {
    body.push_str("}\n");
    let report = vox_diagram::check(&body);
    if !report.valid {
        return Err(format!(
            "VoxDiagram validation failed: {:?}",
            report.diagnostics
        ));
    }
    Ok(body)
}

/// Render a deterministic, VoxDiagram-validated dependency graph for a crate (`WIKI-03`).
pub fn render_crate_dependency_diagram(
    crate_name: &str,
    forward_deps: &[String],
    used_by: &[String],
) -> Result<String, String> {
    let center_id = diagram_node_id(crate_name);
    let mut body = String::from("graph {\n    direction: right\n");
    body.push_str(&format!("    node {center_id}: \"{crate_name}\"\n"));

    // A crate can be both a dependency and a dependent (dev-dependency cycle); declare it once.
    let mut declared = BTreeSet::from([center_id.clone()]);
    for dep in forward_deps.iter().take(12) {
        let dep_id = diagram_node_id(dep);
        if declared.insert(dep_id.clone()) {
            body.push_str(&format!("    node {dep_id}: \"{dep}\"\n"));
        }
        body.push_str(&format!("    {center_id} -> {dep_id}\n"));
    }
    for consumer in used_by.iter().take(12) {
        let cons_id = diagram_node_id(consumer);
        if declared.insert(cons_id.clone()) {
            body.push_str(&format!("    node {cons_id}: \"{consumer}\"\n"));
        }
        body.push_str(&format!("    {cons_id} -> {center_id}\n"));
    }

    finish_diagram(body)
}

/// Render a deterministic, VoxDiagram-validated subsystem layer graph (`WIKI-01`, `WIKI-03`).
pub fn render_subsystem_layer_diagram(
    spec: &SubsystemLayerSpec,
    crates_in_layer: &[LayerCrateEntry],
) -> Result<String, String> {
    let mut body = String::from("graph {\n    direction: down\n");
    let hub_id = diagram_node_id(spec.slug);
    body.push_str(&format!("    node {hub_id}: \"{}\"\n", spec.title));

    for entry in crates_in_layer
        .iter()
        .filter(|e| e.name != "workspace-hack")
        .take(20)
    {
        let cid = diagram_node_id(&entry.name);
        body.push_str(&format!(
            "    node {cid}: \"{} ({})\"\n",
            entry.name, entry.kind
        ));
        body.push_str(&format!("    {hub_id} -> {cid}\n"));
    }

    finish_diagram(body)
}

/// Synthesize OpenWiki Subsystem Hub Markdown pages under `docs/src/architecture/subsystems/` (`WIKI-01..04`).
pub fn generate_subsystem_hubs(repo_root: &Path, write: bool) -> Result<Vec<String>, String> {
    let layers_path = repo_root.join("docs/src/architecture/layers.toml");
    let raw_layers = fs::read_to_string(&layers_path)
        .map_err(|e| format!("failed to read {}: {e}", layers_path.display()))?;
    let all_entries = parse_layers_toml(&raw_layers);

    let out_dir = repo_root.join("docs/src/architecture/subsystems");
    if write {
        fs::create_dir_all(&out_dir)
            .map_err(|e| format!("failed to create {}: {e}", out_dir.display()))?;
    }

    let mut generated_paths = Vec::new();
    for spec in SUBSYSTEM_LAYERS {
        let layer_crates: Vec<LayerCrateEntry> = all_entries
            .iter()
            .filter(|e| e.layer == spec.layer)
            .cloned()
            .collect();
        let diagram = render_subsystem_layer_diagram(spec, &layer_crates)?;

        let mut md = String::new();
        md.push_str("---\n");
        md.push_str(&format!("title: \"{} (Generated)\"\n", spec.title));
        md.push_str(&format!("description: \"{}\"\n", spec.summary));
        md.push_str("category: \"Architecture SSOTs\"\n");
        md.push_str("status: \"current\"\n");
        md.push_str(&format!("sort_order: {}\n", 10 + spec.layer as u32));
        md.push_str("code_anchors:\n  - \"docs/src/architecture/layers.toml\"\n");
        md.push_str("---\n\n");
        md.push_str(&format!("# {}\n\n", spec.title));
        md.push_str(&format!("{}\n\n", spec.summary));
        md.push_str("## Subsystem Topology\n\n```vox-diagram\n");
        md.push_str(&diagram);
        md.push_str("```\n\n## Crates in Layer\n\n");
        md.push_str("| Crate | Kind | Fan-In Cap | LoC Baseline |\n|---|---|---|---|\n");
        for c in &layer_crates {
            if c.name == "workspace-hack" {
                continue;
            }
            let fan = c
                .max_dependents
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into());
            let loc = c
                .max_loc
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into());
            md.push_str(&format!(
                "| {} | `{}` | {} | {} |\n",
                crate_link_cell(repo_root, &c.name),
                c.kind,
                fan,
                loc
            ));
        }

        let rel = format!(
            "docs/src/architecture/subsystems/{}.generated.md",
            spec.slug
        );
        write_or_verify_hub(repo_root, &rel, &md, write)?;
        generated_paths.push(rel);
    }

    let index_rel = "docs/src/architecture/subsystems/index.generated.md".to_string();
    let index_md = render_subsystem_index_markdown(&all_entries)?;
    write_or_verify_hub(repo_root, &index_rel, &index_md, write)?;
    generated_paths.push(index_rel);

    Ok(generated_paths)
}

/// Link a crate to its API page only when that page exists (workspace-excluded
/// crates such as `vox-cargo-shim` never get one).
fn crate_link_cell(repo_root: &Path, name: &str) -> String {
    if repo_root
        .join(format!("docs/src/api/{name}.generated.md"))
        .is_file()
    {
        format!("[`{name}`](../../api/{name}.generated.md)")
    } else {
        format!("`{name}`")
    }
}

fn write_or_verify_hub(repo_root: &Path, rel: &str, md: &str, write: bool) -> Result<(), String> {
    let full = repo_root.join(rel);
    let norm = md.replace("\r\n", "\n");
    if write {
        fs::write(&full, &norm).map_err(|e| format!("failed to write {}: {e}", full.display()))?;
        return Ok(());
    }
    if !full.is_file() {
        return Err(format!(
            "missing {}; run `vox ci wiki-sync --write`",
            full.display()
        ));
    }
    let existing = fs::read_to_string(&full)
        .map_err(|e| format!("failed to read {}: {e}", full.display()))?
        .replace("\r\n", "\n");
    if existing != norm {
        return Err(format!(
            "{} is stale; run `vox ci wiki-sync --write`",
            full.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_link_cell_links_only_existing_api_pages() {
        let root = std::env::temp_dir().join(format!("vox-crate-link-cell-{}", std::process::id()));
        let api = root.join("docs/src/api");
        fs::create_dir_all(&api).unwrap();
        fs::write(api.join("vox-present.generated.md"), "# x\n").unwrap();
        assert_eq!(
            crate_link_cell(&root, "vox-present"),
            "[`vox-present`](../../api/vox-present.generated.md)"
        );
        assert_eq!(crate_link_cell(&root, "vox-absent"), "`vox-absent`");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn parses_layers_toml_and_validates_diagram() {
        let sample = r#"
[crates]
vox-ast = { layer = 0, kind = "library", max_dependents = 20, max_loc = 5_000 }
vox-crypto = { layer = 1, kind = "library" }
"#;
        let parsed = parse_layers_toml(sample);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "vox-ast");
        assert_eq!(parsed[0].layer, 0);

        let m = render_subsystem_layer_diagram(&SUBSYSTEM_LAYERS[0], &parsed[..1]).unwrap();
        assert!(m.starts_with("graph {\n    direction: down\n"), "{m}");
        assert!(vox_diagram::check(&m).valid, "{m}");
        assert!(m.contains("vox-ast (library)"));
    }

    #[test]
    fn extracts_deep_symbols_with_line_ranges_and_reverse_deps() {
        let src = "/// Configuration struct.\npub struct AppConfig {\n    pub port: u16,\n}\n\n/// Run server.\npub async fn serve(cfg: AppConfig) -> bool {\n    true\n}\n";
        let syms = extract_deep_crate_symbols(src, "crates/demo/src/lib.rs");
        assert_eq!(syms.len(), 2);
        assert_eq!(syms[0].name, "AppConfig");
        assert_eq!(syms[0].start_line, 2);
        assert_eq!(syms[0].end_line, 4);
        assert!(syms[0].source_anchor_markdown().contains("#L2-L4"));

        let mut fwd = BTreeMap::new();
        fwd.insert("vox-cli".to_string(), vec!["vox-search".to_string()]);
        fwd.insert("vox-gui".to_string(), vec!["vox-search".to_string()]);
        let rev = compute_reverse_dependents(&fwd);
        assert_eq!(
            rev.get("vox-search").unwrap(),
            &vec!["vox-cli".to_string(), "vox-gui".to_string()]
        );
        let chart = render_crate_dependency_diagram(
            "vox-search",
            &["vox-db".to_string()],
            rev.get("vox-search").unwrap(),
        )
        .unwrap();
        assert!(
            chart.starts_with("graph {\n    direction: right\n"),
            "{chart}"
        );
        assert!(vox_diagram::check(&chart).valid, "{chart}");
    }

    #[test]
    fn crate_dependency_diagram_declares_cyclic_peer_once() {
        // vox-codegen depends on vox-compiler, which dev-depends back on vox-codegen.
        let chart = render_crate_dependency_diagram(
            "vox-codegen",
            &["vox-compiler".to_string()],
            &["vox-compiler".to_string()],
        )
        .unwrap();
        assert_eq!(chart.matches("node vox_compiler:").count(), 1, "{chart}");
        assert!(chart.contains("vox_codegen -> vox_compiler"), "{chart}");
        assert!(chart.contains("vox_compiler -> vox_codegen"), "{chart}");
    }

    #[test]
    fn generates_and_verifies_subsystem_hubs_in_repo() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        if std::env::var_os("VOX_WIKI_SYNC_WRITE").is_some() {
            let written = generate_subsystem_hubs(repo_root, true).expect("write subsystem hubs");
            assert_eq!(written.len(), 7);
        }
    }
}
