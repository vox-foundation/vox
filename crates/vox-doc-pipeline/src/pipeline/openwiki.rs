//! Phase 33 (`WIKI-01..04`): OpenWiki-Style Subsystem Architecture Hubs,
//! Deep Crate Symbol Extraction (`#Lstart-Lend` + Signatures + Reverse Dependents),
//! and Deterministic Validated Mermaid Architecture Diagrams.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::pipeline::mermaid::validate_and_repair_mermaid;

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
            } else if let Some(rest) = after_pub.strip_prefix("type ") {
                Some(("type", rest))
            } else {
                None
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

fn mermaid_node_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Render a deterministic, Mermaid-lint-validated dependency flowchart for a crate (`WIKI-03`).
pub fn render_crate_dependency_mermaid(
    crate_name: &str,
    forward_deps: &[String],
    used_by: &[String],
) -> Result<String, String> {
    let center_id = mermaid_node_id(crate_name);
    let mut body = String::from("flowchart LR\n");
    body.push_str(&format!("    {center_id}[\"{crate_name}\"]\n"));

    for dep in forward_deps.iter().take(12) {
        let dep_id = mermaid_node_id(dep);
        body.push_str(&format!("    {dep_id}[\"{dep}\"]\n"));
        body.push_str(&format!("    {center_id} --> {dep_id}\n"));
    }
    for consumer in used_by.iter().take(12) {
        let cons_id = mermaid_node_id(consumer);
        body.push_str(&format!("    {cons_id}[\"{consumer}\"]\n"));
        body.push_str(&format!("    {cons_id} --> {center_id}\n"));
    }

    let res = validate_and_repair_mermaid(&body);
    if !res.diagnostics.is_empty() {
        return Err(format!(
            "Mermaid validation failed: {:?}",
            res.diagnostics[0]
        ));
    }
    Ok(body)
}

/// Render a deterministic, Mermaid-lint-validated subsystem layer flowchart (`WIKI-01`, `WIKI-03`).
pub fn render_subsystem_layer_mermaid(
    spec: &SubsystemLayerSpec,
    crates_in_layer: &[LayerCrateEntry],
) -> Result<String, String> {
    let mut body = String::from("flowchart TD\n");
    let hub_id = mermaid_node_id(spec.slug);
    body.push_str(&format!("    {hub_id}[\"{}\"]\n", spec.title));

    for entry in crates_in_layer.iter().take(20) {
        let cid = mermaid_node_id(&entry.name);
        body.push_str(&format!("    {cid}[\"{} ({})\"]\n", entry.name, entry.kind));
        body.push_str(&format!("    {hub_id} --> {cid}\n"));
    }

    let res = validate_and_repair_mermaid(&body);
    if !res.diagnostics.is_empty() {
        return Err(format!(
            "Mermaid validation failed: {:?}",
            res.diagnostics[0]
        ));
    }
    Ok(body)
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
        let mermaid = render_subsystem_layer_mermaid(spec, &layer_crates)?;

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
        md.push_str("## Subsystem Topology\n\n```mermaid\n");
        md.push_str(&mermaid);
        md.push_str("```\n\n## Crates in Layer\n\n");
        md.push_str("| Crate | Kind | Fan-In Cap | LoC Baseline |\n|---|---|---|---|\n");
        for c in &layer_crates {
            let fan = c
                .max_dependents
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into());
            let loc = c
                .max_loc
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into());
            md.push_str(&format!(
                "| [`{}`](../../api/{}.generated.md) | `{}` | {} | {} |\n",
                c.name, c.name, c.kind, fan, loc
            ));
        }

        let rel = format!(
            "docs/src/architecture/subsystems/{}.generated.md",
            spec.slug
        );
        if write {
            let full = repo_root.join(&rel);
            fs::write(&full, md).map_err(|e| format!("failed to write {}: {e}", full.display()))?;
        }
        generated_paths.push(rel);
    }

    Ok(generated_paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_layers_toml_and_validates_mermaid() {
        let sample = r#"
[crates]
vox-ast = { layer = 0, kind = "library", max_dependents = 20, max_loc = 5_000 }
vox-crypto = { layer = 1, kind = "library" }
"#;
        let parsed = parse_layers_toml(sample);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "vox-ast");
        assert_eq!(parsed[0].layer, 0);

        let m = render_subsystem_layer_mermaid(&SUBSYSTEM_LAYERS[0], &parsed[..1]).unwrap();
        assert!(m.contains("flowchart TD"));
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
        let chart = render_crate_dependency_mermaid(
            "vox-search",
            &["vox-db".to_string()],
            rev.get("vox-search").unwrap(),
        )
        .unwrap();
        assert!(chart.contains("flowchart LR"));
    }
}
