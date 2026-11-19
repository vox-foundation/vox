//! Repo-agnostic 5-layer Polyglot OpenWiki Synthesis and non-destructive OKF auto-fence merger
//! (`WIKI-01`, `WIKI-02`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};

use crate::claims::hasher::hash_normalized_code;
use crate::pipeline::openwiki::{extract_deep_crate_symbols, parse_layers_toml};

const MAX_SCAN_FILES: usize = 6_000;
const MAX_ENTRIES_PER_LAYER: usize = 60;

const LAYER_PAGE_ORDER: &[&str] = &[
    "README.md",
    "architecture-topology.md",
    "interfaces-and-apis.md",
    "data-models-and-schemas.md",
    "gui-surfaces-and-flows.md",
    "workflows-and-runbooks.md",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolyglotModuleRecord {
    pub name: String,
    pub stack: String,
    pub manifest_rel: String,
    pub summary: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolyglotSymbolAnchor {
    pub layer: u8,
    pub kind: String,
    pub stack: String,
    pub name: String,
    pub signature: String,
    pub rel_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub doc_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PolyglotWikiSnapshot {
    pub repo_name: String,
    pub detected_stacks: Vec<String>,
    pub modules: Vec<PolyglotModuleRecord>,
    pub interfaces: Vec<PolyglotSymbolAnchor>,
    pub schemas: Vec<PolyglotSymbolAnchor>,
    pub gui_surfaces: Vec<PolyglotSymbolAnchor>,
    pub workflows: Vec<PolyglotSymbolAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenWikiPageSpec {
    pub rel_path: String,
    pub full_markdown: String,
    pub regions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenWikiManifestPageEntry {
    pub rel_path: String,
    pub content_hash: String,
    pub region_hashes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenWikiManifest {
    pub schema_version: u32,
    pub repo_name: String,
    pub detected_stacks: Vec<String>,
    pub manifest_hash: String,
    pub pages: Vec<OpenWikiManifestPageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenWikiBuildReport {
    pub repo_root: String,
    pub output_dir: String,
    pub manifest_path: String,
    pub detected_stacks: Vec<String>,
    pub generated_pages: Vec<String>,
    pub updated_pages: Vec<String>,
    pub unchanged_pages: Vec<String>,
    pub updated_regions: Vec<(String, Vec<String>)>,
    pub manifest_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenWikiPageStatus {
    pub rel_path: String,
    pub exists: bool,
    pub region_ids: Vec<String>,
    pub stale: bool,
    pub stale_regions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenWikiStatusReport {
    pub initialized: bool,
    pub manifest_present: bool,
    pub wiki_dir: String,
    pub manifest_path: String,
    pub detected_stacks: Vec<String>,
    pub pages: Vec<OpenWikiPageStatus>,
    pub total_pages: usize,
    pub stale_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenWikiPageDiff {
    pub rel_path: String,
    pub changed: bool,
    pub updated_regions: Vec<String>,
    pub legacy_mermaid_blocks: usize,
    pub existing_bytes: usize,
    pub rendered_bytes: usize,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenWikiDiffReport {
    pub wiki_dir: String,
    pub detected_stacks: Vec<String>,
    pub diffs: Vec<OpenWikiPageDiff>,
    pub has_changes: bool,
}

/// Extract deterministic `<!-- openwiki:auto:start id="..." --> ... <!-- openwiki:auto:end -->`
/// blocks keyed by region `id`.
pub fn extract_openwiki_auto_regions(markdown: &str) -> BTreeMap<String, String> {
    let mut regions = BTreeMap::new();
    let mut active: Option<(String, Vec<String>)> = None;

    for raw_line in markdown.lines() {
        let trimmed = raw_line.trim();
        if let Some(id) = parse_auto_start_id(trimmed) {
            if let Some((prev_id, buf)) = active.take() {
                regions.insert(prev_id, buf.join("\n").trim().to_string());
            }
            active = Some((id, Vec::new()));
            continue;
        }
        if is_auto_end_marker(trimmed) {
            if let Some((id, buf)) = active.take() {
                regions.insert(id, buf.join("\n").trim().to_string());
            }
            continue;
        }
        if let Some((_, ref mut buf)) = active {
            buf.push(raw_line.to_string());
        }
    }
    regions
}

/// Merge generated deterministic regions into `existing_markdown`, preserving 100% of human prose,
/// frontmatter, custom headings, and comments outside `openwiki:auto` fences.
pub fn merge_openwiki_auto_regions(
    existing_markdown: &str,
    generated_regions: &BTreeMap<String, String>,
) -> Result<(String, Vec<String>), String> {
    let norm = existing_markdown.replace("\r\n", "\n");
    let mut out_lines: Vec<String> = Vec::new();
    let mut updated_ids: Vec<String> = Vec::new();
    let mut seen_ids: BTreeSet<String> = BTreeSet::new();
    let mut active_id: Option<String> = None;
    let mut existing_buf: Vec<String> = Vec::new();

    for line in norm.lines() {
        let trimmed = line.trim();
        if let Some(ref cur_id) = active_id {
            if is_auto_end_marker(trimmed) {
                let old_body = existing_buf.join("\n").trim().to_string();
                let replacement = generated_regions
                    .get(cur_id)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| old_body.clone());
                if replacement != old_body {
                    updated_ids.push(cur_id.clone());
                }
                if !replacement.is_empty() {
                    for rline in replacement.lines() {
                        out_lines.push(rline.to_string());
                    }
                }
                out_lines.push(line.to_string());
                active_id = None;
                existing_buf.clear();
            } else {
                existing_buf.push(line.to_string());
            }
            continue;
        }

        if let Some(id) = parse_auto_start_id(trimmed) {
            seen_ids.insert(id.clone());
            out_lines.push(line.to_string());
            active_id = Some(id);
            existing_buf.clear();
        } else {
            out_lines.push(line.to_string());
        }
    }

    if let Some(unclosed) = active_id {
        return Err(format!(
            "unclosed openwiki:auto:start fence for id `{unclosed}`"
        ));
    }

    for (id, body) in generated_regions {
        if !seen_ids.contains(id) {
            if !out_lines.is_empty() && !out_lines.last().is_some_and(String::is_empty) {
                out_lines.push(String::new());
            }
            out_lines.push(format!("## Deterministic Section: `{id}`"));
            out_lines.push(String::new());
            out_lines.push(format!("<!-- openwiki:auto:start id=\"{id}\" -->"));
            let body_trimmed = body.trim();
            if !body_trimmed.is_empty() {
                for b in body_trimmed.lines() {
                    out_lines.push(b.to_string());
                }
            }
            out_lines.push(format!("<!-- openwiki:auto:end id=\"{id}\" -->"));
            updated_ids.push(id.clone());
        }
    }

    let mut merged = out_lines.join("\n");
    if !merged.ends_with('\n') {
        merged.push('\n');
    }
    Ok((merged, updated_ids))
}

fn parse_auto_start_id(trimmed_line: &str) -> Option<String> {
    if !trimmed_line.starts_with("<!--") || !trimmed_line.ends_with("-->") {
        return None;
    }
    let inner = trimmed_line
        .trim_start_matches("<!--")
        .trim_end_matches("-->")
        .trim();
    let rest = inner.strip_prefix("openwiki:auto:start")?.trim();
    let after_id = rest.strip_prefix("id=")?.trim();
    let quote = after_id.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let end = after_id[1..].find(quote)?;
    let id = after_id[1..1 + end].trim();
    (!id.is_empty()).then(|| id.to_string())
}

fn is_auto_end_marker(trimmed_line: &str) -> bool {
    if !trimmed_line.starts_with("<!--") || !trimmed_line.ends_with("-->") {
        return false;
    }
    let inner = trimmed_line
        .trim_start_matches("<!--")
        .trim_end_matches("-->")
        .trim();
    inner == "openwiki:auto:end" || inner.starts_with("openwiki:auto:end ")
}

fn is_ignored_dir(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() || entry.depth() == 0 {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    matches!(
        name.as_ref(),
        ".git"
            | ".vox"
            | "target"
            | "node_modules"
            | "dist"
            | "build"
            | ".venv"
            | "__pycache__"
            | ".next"
            | "coverage"
    )
}

fn escape_cell(s: &str) -> String {
    s.replace('\r', "")
        .replace('\n', " ")
        .replace('|', "\\|")
        .trim()
        .to_string()
}

fn anchor_markdown(rel_path: &str, start_line: usize, end_line: usize) -> String {
    let norm = rel_path.replace('\\', "/");
    if end_line > start_line {
        format!("[`{norm}#L{start_line}-L{end_line}`](../../{norm}#L{start_line}-L{end_line})")
    } else {
        format!("[`{norm}#L{start_line}`](../../{norm}#L{start_line})")
    }
}

fn diagram_id(raw: &str) -> String {
    let mut out: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if out.is_empty() || out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, 'n');
    }
    out
}

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

/// Extract polyglot 5-layer architectural facts from any repository root deterministically.
pub fn extract_polyglot_wiki_snapshot(repo_root: &Path) -> PolyglotWikiSnapshot {
    let repo_name = repo_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "repository".to_string());

    let mut stacks: BTreeSet<String> = BTreeSet::new();
    let mut modules: Vec<PolyglotModuleRecord> = Vec::new();
    let mut interfaces: Vec<PolyglotSymbolAnchor> = Vec::new();
    let mut schemas: Vec<PolyglotSymbolAnchor> = Vec::new();
    let mut gui_surfaces: Vec<PolyglotSymbolAnchor> = Vec::new();
    let mut workflows: Vec<PolyglotSymbolAnchor> = Vec::new();

    if let Ok(raw_layers) = fs::read_to_string(repo_root.join("docs/src/architecture/layers.toml"))
    {
        stacks.insert("Rust/Cargo".to_string());
        for entry in parse_layers_toml(&raw_layers).into_iter().take(40) {
            if entry.name == "workspace-hack" {
                continue;
            }
            modules.push(PolyglotModuleRecord {
                name: entry.name,
                stack: "Rust/Cargo".to_string(),
                manifest_rel: "docs/src/architecture/layers.toml".to_string(),
                summary: format!("Layer {} ({})", entry.layer, entry.kind),
                start_line: 1,
                end_line: 1,
            });
        }
    }

    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for entry in WalkDir::new(repo_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored_dir(e))
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if rel.starts_with("docs/wiki/") || rel.starts_with("docs/src/archive/") {
            continue;
        }
        files.push((rel, path));
        if files.len() >= MAX_SCAN_FILES {
            break;
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let re_sql =
        Regex::new(r"(?i)^\s*create\s+table\s+(?:if\s+not\s+exists\s+)?([a-zA-Z0-9_`\.]+)").ok();
    let re_ts_iface = Regex::new(r"^\s*export\s+(?:interface|type)\s+([A-Za-z0-9_]+)").ok();
    let re_ts_fn = Regex::new(r"^\s*export\s+(?:async\s+)?function\s+([A-Za-z0-9_]+)").ok();
    let re_ts_comp =
        Regex::new(r"^\s*export\s+(?:default\s+)?(?:function|const)\s+([A-Z][A-Za-z0-9_]+)").ok();
    let re_py_fn = Regex::new(r"^\s*(?:async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)").ok();
    let re_py_cls = Regex::new(r"^\s*class\s+([A-Z][a-zA-Z0-9_]*)").ok();
    let re_go_fn = Regex::new(r"^\s*func\s+(?:\([^)]+\)\s*)?([A-Z][A-Za-z0-9_]*)\s*\(").ok();
    let re_go_struct = Regex::new(r"^\s*type\s+([A-Z][A-Za-z0-9_]*)\s+(?:struct|interface)\b").ok();
    let re_http_route = Regex::new(r#"(?i)(?:\.route|app\.(?:get|post|put|delete)|router\.(?:get|post)|http\.HandleFunc)\s*\(\s*["']([^"']+)["']"#).ok();
    let re_vox_wf =
        Regex::new(r"^\s*(workflow|actor|activity|fn|type|@table)\s+([A-Za-z_][A-Za-z0-9_]*)").ok();

    for (rel, path) in files {
        let Ok(content) = vox_bounded_fs::read_utf8_path_capped(&path) else {
            continue;
        };
        let file_name = Path::new(&rel)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let ext = Path::new(&rel)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // Manifest detection (Layer 1)
        if file_name == "Cargo.toml" && !rel.contains("/target/") {
            stacks.insert("Rust/Cargo".to_string());
            if let Some(pkg) = parse_toml_name(&content)
                && !modules
                    .iter()
                    .any(|m| m.name == pkg && m.stack == "Rust/Cargo")
            {
                modules.push(PolyglotModuleRecord {
                    name: pkg,
                    stack: "Rust/Cargo".to_string(),
                    manifest_rel: rel.clone(),
                    summary: "Cargo crate/package".to_string(),
                    start_line: 1,
                    end_line: content.lines().count().min(12),
                });
            }
        } else if file_name == "package.json" {
            stacks.insert("TypeScript/Node".to_string());
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let pkg = val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("node-package")
                    .to_string();
                modules.push(PolyglotModuleRecord {
                    name: pkg,
                    stack: "TypeScript/Node".to_string(),
                    manifest_rel: rel.clone(),
                    summary: "Node/TypeScript package".to_string(),
                    start_line: 1,
                    end_line: content.lines().count().min(20),
                });
            }
        } else if file_name == "pyproject.toml" {
            stacks.insert("Python".to_string());
            let pkg = parse_toml_name(&content).unwrap_or_else(|| "python-package".to_string());
            modules.push(PolyglotModuleRecord {
                name: pkg,
                stack: "Python".to_string(),
                manifest_rel: rel.clone(),
                summary: "Python pyproject package".to_string(),
                start_line: 1,
                end_line: content.lines().count().min(15),
            });
        } else if file_name == "go.mod" {
            stacks.insert("Go".to_string());
            let mod_name = content
                .lines()
                .find_map(|l| {
                    l.trim()
                        .strip_prefix("module ")
                        .map(|s| s.trim().to_string())
                })
                .unwrap_or_else(|| "go-module".to_string());
            modules.push(PolyglotModuleRecord {
                name: mod_name,
                stack: "Go".to_string(),
                manifest_rel: rel.clone(),
                summary: "Go module".to_string(),
                start_line: 1,
                end_line: content.lines().count().min(10),
            });
        } else if file_name == "Vox.toml" {
            stacks.insert("Vox".to_string());
            let pkg = parse_toml_name(&content).unwrap_or_else(|| "vox-project".to_string());
            modules.push(PolyglotModuleRecord {
                name: pkg,
                stack: "Vox".to_string(),
                manifest_rel: rel.clone(),
                summary: "Vox workspace package".to_string(),
                start_line: 1,
                end_line: content.lines().count().min(15),
            });
        }

        // Layer 5 CI workflows & runbooks
        if (rel.starts_with(".github/workflows/") && (ext == "yml" || ext == "yaml"))
            || (rel.starts_with("scripts/") && matches!(ext, "vox" | "sh" | "ps1"))
        {
            let lines_len = content.lines().count().max(1);
            let title = content
                .lines()
                .find_map(|l| {
                    l.trim()
                        .strip_prefix("name:")
                        .map(|s| s.trim().trim_matches('"').to_string())
                })
                .unwrap_or_else(|| file_name.to_string());
            workflows.push(PolyglotSymbolAnchor {
                layer: 5,
                kind: if rel.starts_with("scripts/") {
                    "runbook"
                } else {
                    "ci_workflow"
                }
                .to_string(),
                stack: "Automation".to_string(),
                name: title,
                signature: rel.clone(),
                rel_path: rel.clone(),
                start_line: 1,
                end_line: lines_len.min(40),
                doc_summary: "Repository automation or runbook definition".to_string(),
            });
        }

        match ext {
            "rs" => {
                stacks.insert("Rust/Cargo".to_string());
                if !rel.contains("/tests/") {
                    for sym in extract_deep_crate_symbols(&content, &rel) {
                        let anchor = PolyglotSymbolAnchor {
                            layer: if sym.kind == "struct" || sym.kind == "enum" {
                                3
                            } else {
                                2
                            },
                            kind: sym.kind.clone(),
                            stack: "Rust/Cargo".to_string(),
                            name: sym.name,
                            signature: sym.signature,
                            rel_path: rel.clone(),
                            start_line: sym.start_line,
                            end_line: sym.end_line,
                            doc_summary: sym.doc_summary,
                        };
                        if anchor.layer == 3 {
                            schemas.push(anchor);
                        } else {
                            interfaces.push(anchor);
                        }
                    }
                }
                scan_http_routes(
                    &content,
                    &rel,
                    "Rust/Cargo",
                    &re_http_route,
                    &mut interfaces,
                );
            }
            "ts" | "tsx" | "js" | "jsx" => {
                stacks.insert("TypeScript/Node".to_string());
                scan_ts_file(
                    &content,
                    &rel,
                    ext,
                    &re_ts_iface,
                    &re_ts_fn,
                    &re_ts_comp,
                    &re_http_route,
                    &mut interfaces,
                    &mut schemas,
                    &mut gui_surfaces,
                );
            }
            "py" => {
                stacks.insert("Python".to_string());
                scan_py_file(
                    &content,
                    &rel,
                    &re_py_fn,
                    &re_py_cls,
                    &re_http_route,
                    &mut interfaces,
                    &mut schemas,
                );
            }
            "go" => {
                stacks.insert("Go".to_string());
                scan_go_file(
                    &content,
                    &rel,
                    &re_go_fn,
                    &re_go_struct,
                    &re_http_route,
                    &mut interfaces,
                    &mut schemas,
                );
            }
            "sql" => {
                stacks.insert("SQL".to_string());
                scan_sql_file(&content, &rel, &re_sql, &mut schemas);
            }
            "vox" => {
                stacks.insert("Vox".to_string());
                scan_vox_file(
                    &content,
                    &rel,
                    &re_vox_wf,
                    &mut interfaces,
                    &mut schemas,
                    &mut gui_surfaces,
                    &mut workflows,
                );
            }
            _ => {}
        }
    }

    modules.sort_by(|a, b| a.stack.cmp(&b.stack).then_with(|| a.name.cmp(&b.name)));
    modules.truncate(MAX_ENTRIES_PER_LAYER);
    interfaces.truncate(MAX_ENTRIES_PER_LAYER);
    schemas.truncate(MAX_ENTRIES_PER_LAYER);
    gui_surfaces.truncate(MAX_ENTRIES_PER_LAYER);
    workflows.truncate(MAX_ENTRIES_PER_LAYER);

    PolyglotWikiSnapshot {
        repo_name,
        detected_stacks: stacks.into_iter().collect(),
        modules,
        interfaces,
        schemas,
        gui_surfaces,
        workflows,
    }
}

fn parse_toml_name(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let trimmed = line.trim();
        let rest = trimmed.strip_prefix("name")?.trim();
        let val = rest
            .strip_prefix('=')?
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        (!val.is_empty()).then(|| val.to_string())
    })
}

fn scan_http_routes(
    content: &str,
    rel: &str,
    stack: &str,
    re_route: &Option<Regex>,
    interfaces: &mut Vec<PolyglotSymbolAnchor>,
) {
    let Some(re) = re_route else {
        return;
    };
    for (idx, line) in content.lines().enumerate() {
        if let Some(cap) = re.captures(line) {
            let endpoint = cap[1].to_string();
            interfaces.push(PolyglotSymbolAnchor {
                layer: 2,
                kind: "endpoint".to_string(),
                stack: stack.to_string(),
                name: endpoint.clone(),
                signature: line.trim().to_string(),
                rel_path: rel.to_string(),
                start_line: idx + 1,
                end_line: idx + 1,
                doc_summary: format!("Route handler `{endpoint}`"),
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_ts_file(
    content: &str,
    rel: &str,
    ext: &str,
    re_iface: &Option<Regex>,
    re_fn: &Option<Regex>,
    re_comp: &Option<Regex>,
    re_route: &Option<Regex>,
    interfaces: &mut Vec<PolyglotSymbolAnchor>,
    schemas: &mut Vec<PolyglotSymbolAnchor>,
    gui_surfaces: &mut Vec<PolyglotSymbolAnchor>,
) {
    let is_ui =
        ext == "tsx" || ext == "jsx" || rel.contains("/components/") || rel.contains("/pages/");
    for (idx, line) in content.lines().enumerate() {
        let ln = idx + 1;
        if let Some(re) = re_iface
            && let Some(cap) = re.captures(line)
        {
            schemas.push(PolyglotSymbolAnchor {
                layer: 3,
                kind: "type".to_string(),
                stack: "TypeScript/Node".to_string(),
                name: cap[1].to_string(),
                signature: line.trim().trim_end_matches('{').trim().to_string(),
                rel_path: rel.to_string(),
                start_line: ln,
                end_line: ln,
                doc_summary: String::new(),
            });
        }
        if is_ui
            && let Some(re) = re_comp
            && let Some(cap) = re.captures(line)
        {
            gui_surfaces.push(PolyglotSymbolAnchor {
                layer: 4,
                kind: "component".to_string(),
                stack: "TypeScript/Node".to_string(),
                name: cap[1].to_string(),
                signature: line.trim().trim_end_matches('{').trim().to_string(),
                rel_path: rel.to_string(),
                start_line: ln,
                end_line: ln,
                doc_summary: "UI surface or React component".to_string(),
            });
        } else if let Some(re) = re_fn
            && let Some(cap) = re.captures(line)
        {
            interfaces.push(PolyglotSymbolAnchor {
                layer: 2,
                kind: "function".to_string(),
                stack: "TypeScript/Node".to_string(),
                name: cap[1].to_string(),
                signature: line.trim().trim_end_matches('{').trim().to_string(),
                rel_path: rel.to_string(),
                start_line: ln,
                end_line: ln,
                doc_summary: String::new(),
            });
        }
    }
    scan_http_routes(content, rel, "TypeScript/Node", re_route, interfaces);
}

fn scan_py_file(
    content: &str,
    rel: &str,
    re_fn: &Option<Regex>,
    re_cls: &Option<Regex>,
    re_route: &Option<Regex>,
    interfaces: &mut Vec<PolyglotSymbolAnchor>,
    schemas: &mut Vec<PolyglotSymbolAnchor>,
) {
    for (idx, line) in content.lines().enumerate() {
        let ln = idx + 1;
        if let Some(re) = re_cls
            && let Some(cap) = re.captures(line)
        {
            let name = cap[1].to_string();
            let is_model = line.contains("BaseModel")
                || line.contains("TypedDict")
                || line.contains("SQLModel")
                || idx.checked_sub(1).is_some_and(|prev| {
                    content
                        .lines()
                        .nth(prev)
                        .is_some_and(|p| p.contains("@dataclass"))
                });
            let anchor = PolyglotSymbolAnchor {
                layer: if is_model { 3 } else { 2 },
                kind: if is_model { "model" } else { "class" }.to_string(),
                stack: "Python".to_string(),
                name,
                signature: line.trim().trim_end_matches(':').to_string(),
                rel_path: rel.to_string(),
                start_line: ln,
                end_line: ln,
                doc_summary: String::new(),
            };
            if is_model {
                schemas.push(anchor);
            } else {
                interfaces.push(anchor);
            }
        } else if let Some(re) = re_fn
            && let Some(cap) = re.captures(line)
        {
            let name = cap[1].to_string();
            if !name.starts_with('_') {
                interfaces.push(PolyglotSymbolAnchor {
                    layer: 2,
                    kind: "function".to_string(),
                    stack: "Python".to_string(),
                    name,
                    signature: line.trim().trim_end_matches(':').to_string(),
                    rel_path: rel.to_string(),
                    start_line: ln,
                    end_line: ln,
                    doc_summary: String::new(),
                });
            }
        }
    }
    scan_http_routes(content, rel, "Python", re_route, interfaces);
}

fn scan_go_file(
    content: &str,
    rel: &str,
    re_fn: &Option<Regex>,
    re_struct: &Option<Regex>,
    re_route: &Option<Regex>,
    interfaces: &mut Vec<PolyglotSymbolAnchor>,
    schemas: &mut Vec<PolyglotSymbolAnchor>,
) {
    for (idx, line) in content.lines().enumerate() {
        let ln = idx + 1;
        if let Some(re) = re_struct
            && let Some(cap) = re.captures(line)
        {
            schemas.push(PolyglotSymbolAnchor {
                layer: 3,
                kind: "struct".to_string(),
                stack: "Go".to_string(),
                name: cap[1].to_string(),
                signature: line.trim().trim_end_matches('{').trim().to_string(),
                rel_path: rel.to_string(),
                start_line: ln,
                end_line: ln,
                doc_summary: String::new(),
            });
        } else if let Some(re) = re_fn
            && let Some(cap) = re.captures(line)
        {
            interfaces.push(PolyglotSymbolAnchor {
                layer: 2,
                kind: "func".to_string(),
                stack: "Go".to_string(),
                name: cap[1].to_string(),
                signature: line.trim().trim_end_matches('{').trim().to_string(),
                rel_path: rel.to_string(),
                start_line: ln,
                end_line: ln,
                doc_summary: String::new(),
            });
        }
    }
    scan_http_routes(content, rel, "Go", re_route, interfaces);
}

fn scan_sql_file(
    content: &str,
    rel: &str,
    re_sql: &Option<Regex>,
    schemas: &mut Vec<PolyglotSymbolAnchor>,
) {
    let Some(re) = re_sql else {
        return;
    };
    for (idx, line) in content.lines().enumerate() {
        if let Some(cap) = re.captures(line) {
            schemas.push(PolyglotSymbolAnchor {
                layer: 3,
                kind: "table".to_string(),
                stack: "SQL".to_string(),
                name: cap[1].trim_matches('`').to_string(),
                signature: line.trim().trim_end_matches('(').trim().to_string(),
                rel_path: rel.to_string(),
                start_line: idx + 1,
                end_line: idx + 1,
                doc_summary: "SQL table DDL schema".to_string(),
            });
        }
    }
}

fn scan_vox_file(
    content: &str,
    rel: &str,
    re_vox: &Option<Regex>,
    interfaces: &mut Vec<PolyglotSymbolAnchor>,
    schemas: &mut Vec<PolyglotSymbolAnchor>,
    gui_surfaces: &mut Vec<PolyglotSymbolAnchor>,
    workflows: &mut Vec<PolyglotSymbolAnchor>,
) {
    for (idx, line) in content.lines().enumerate() {
        let ln = idx + 1;
        let trimmed = line.trim();
        if trimmed.starts_with("@island")
            || trimmed.starts_with("component ")
            || trimmed.starts_with("page ")
        {
            let name = trimmed
                .split_whitespace()
                .nth(1)
                .unwrap_or("surface")
                .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_');
            gui_surfaces.push(PolyglotSymbolAnchor {
                layer: 4,
                kind: "vox_surface".to_string(),
                stack: "Vox".to_string(),
                name: name.to_string(),
                signature: trimmed.trim_end_matches('{').trim().to_string(),
                rel_path: rel.to_string(),
                start_line: ln,
                end_line: ln,
                doc_summary: "Vox reactive UI component or route".to_string(),
            });
            continue;
        }
        if let Some(re) = re_vox
            && let Some(cap) = re.captures(line)
        {
            let kw = &cap[1];
            let name = cap[2].to_string();
            let anchor = PolyglotSymbolAnchor {
                layer: match kw {
                    "@table" | "type" => 3,
                    "workflow" | "actor" | "activity" => 5,
                    _ => 2,
                },
                kind: kw.to_string(),
                stack: "Vox".to_string(),
                name,
                signature: trimmed.trim_end_matches('{').trim().to_string(),
                rel_path: rel.to_string(),
                start_line: ln,
                end_line: ln,
                doc_summary: String::new(),
            };
            match anchor.layer {
                3 => schemas.push(anchor),
                5 => workflows.push(anchor),
                _ => interfaces.push(anchor),
            }
        }
    }
}

fn render_okf_frontmatter(title: &str, description: &str, topic: &str, domain: &str) -> String {
    format!(
        "---\n\
         title: \"{title}\"\n\
         description: \"{description}\"\n\
         category: \"Architecture SSOTs\"\n\
         status: \"current\"\n\
         topic: \"{topic}\"\n\
         domain: \"{domain}\"\n\
         authorship: \"llm-generated\"\n\
         authorship_model: \"vox-openwiki-v7.9\"\n\
         training_eligible: false\n\
         ---\n"
    )
}

fn render_symbol_table(items: &[PolyglotSymbolAnchor], empty_note: &str) -> String {
    if items.is_empty() {
        return format!("{empty_note}\n");
    }
    let mut out =
        String::from("| Stack | Kind | Symbol | Signature | Source |\n|---|---|---|---|---|\n");
    for item in items.iter().take(35) {
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | {} |\n",
            escape_cell(&item.stack),
            escape_cell(&item.kind),
            escape_cell(&item.name),
            escape_cell(&item.signature),
            anchor_markdown(&item.rel_path, item.start_line, item.end_line)
        ));
    }
    out
}

fn render_layer_diagram(hub_label: &str, labels: &[String]) -> Result<String, String> {
    let mut body = String::from("graph {\n    direction: down\n");
    body.push_str(&format!(
        "    node hub: \"{}\"\n",
        hub_label.replace('"', "'")
    ));
    let mut seen = BTreeSet::from(["hub".to_string()]);
    for label in labels.iter().take(12) {
        let id = diagram_id(label);
        if seen.insert(id.clone()) {
            let safe = label.replace('"', "'");
            body.push_str(&format!("    node {id}: \"{safe}\"\n"));
            body.push_str(&format!("    hub -> {id}\n"));
        }
    }
    finish_diagram(body)
}

/// Render all 6 OKF-compliant OpenWiki pages (`README.md` + 5 system layers).
pub fn render_polyglot_openwiki_pages(
    snapshot: &PolyglotWikiSnapshot,
) -> Result<Vec<OpenWikiPageSpec>, String> {
    let stacks_text = if snapshot.detected_stacks.is_empty() {
        "Polyglot".to_string()
    } else {
        snapshot.detected_stacks.join(", ")
    };

    let hub_diagram = render_layer_diagram(
        &format!("{} (5-Layer OpenWiki)", snapshot.repo_name),
        &[
            "Layer 1: Architecture & Modules".to_string(),
            "Layer 2: CLI, HTTP & API Interfaces".to_string(),
            "Layer 3: Data Models & Schemas".to_string(),
            "Layer 4: GUI & Frontend Surfaces".to_string(),
            "Layer 5: Workflows & Runbooks".to_string(),
        ],
    )?;

    let overview_region = format!(
        "- **Repository:** `{}`\n- **Detected Stacks:** {}\n- **Modules:** {} · **Interfaces:** {} · **Schemas:** {} · **GUI Surfaces:** {} · **Workflows:** {}",
        snapshot.repo_name,
        stacks_text,
        snapshot.modules.len(),
        snapshot.interfaces.len(),
        snapshot.schemas.len(),
        snapshot.gui_surfaces.len(),
        snapshot.workflows.len()
    );
    let diagram_region = format!("```vox-diagram\n{hub_diagram}```");
    let layers_index_region = "\
| Layer | Wiki Page | Coverage |\n\
|---|---|---|\n\
| Layer 1 | [Architecture & Module Topology](./architecture-topology.md) | Packages, workspaces, crates, modules |\n\
| Layer 2 | [CLI, HTTP, MCP & API Interfaces](./interfaces-and-apis.md) | Endpoints, commands, exported functions |\n\
| Layer 3 | [Data Models & SQL/VoxDb Schemas](./data-models-and-schemas.md) | SQL tables, structs, typed models |\n\
| Layer 4 | [GUI / Frontend Surfaces & Flows](./gui-surfaces-and-flows.md) | Components, views, reactive surfaces |\n\
| Layer 5 | [Workflows, Actors & Runbooks](./workflows-and-runbooks.md) | CI pipelines, actors, automation scripts |"
        .to_string();

    let mut pages = Vec::new();
    pages.push(build_page(
        "README.md",
        &format!("{} OpenWiki Hub", snapshot.repo_name),
        &format!("Five-layer OpenWiki documentation hub and module map for {}.", snapshot.repo_name),
        "doc-pipeline",
        "documentation",
        &format!(
            "This OpenWiki hub indexes the deterministic architecture, interface, schema, frontend surface, and workflow documentation for {}. Automatically synchronized machine-readable regions reside inside explicit auto-fences so verified code locations, source line anchors, package manifests, cross-layer relationships, and VoxDiagram topologies stay current across repository updates while human engineering rationale remains completely preserved.",
            snapshot.repo_name
        ),
        vec![
            ("repo-overview", overview_region),
            ("wiki-overview-diagram", diagram_region),
            ("wiki-layers-index", layers_index_region),
        ],
    ));

    let mod_labels: Vec<String> = snapshot.modules.iter().map(|m| m.name.clone()).collect();
    let l1_diag = format!(
        "```vox-diagram\n{}```",
        render_layer_diagram(&format!("{} Modules", snapshot.repo_name), &mod_labels)?
    );
    let mut mod_table = if snapshot.modules.is_empty() {
        "No package manifests detected yet.\n".to_string()
    } else {
        let mut t = String::from("| Module | Stack | Role | Manifest |\n|---|---|---|---|\n");
        for m in snapshot.modules.iter().take(40) {
            t.push_str(&format!(
                "| `{}` | `{}` | {} | {} |\n",
                escape_cell(&m.name),
                escape_cell(&m.stack),
                escape_cell(&m.summary),
                anchor_markdown(&m.manifest_rel, m.start_line, m.end_line)
            ));
        }
        t
    };
    mod_table = mod_table.trim_end().to_string();
    pages.push(build_page(
        "architecture-topology.md",
        &format!("{} — Layer 1: Architecture & Module Topology", snapshot.repo_name),
        &format!("Detected stacks, workspace manifests, and package topology for {}.", snapshot.repo_name),
        "search-retrieval",
        "knowledge",
        &format!(
            "Layer 1 documents the top-level structural topology of {} spanning {} across workspaces, packages, and system modules. Every detected Rust crate, Node package, Python module, Go module, or Vox workspace manifest is anchored directly to its authoritative manifest file and rendered as a validated VoxDiagram topology graph so contributors can inspect module boundaries before diving into source files.",
            snapshot.repo_name, stacks_text
        ),
        vec![
            ("layer1-topology-diagram", l1_diag),
            ("layer1-modules-table", mod_table),
        ],
    ));

    let l2_labels: Vec<String> = snapshot.interfaces.iter().map(|s| s.name.clone()).collect();
    let l2_diag = format!(
        "```vox-diagram\n{}```",
        render_layer_diagram("Layer 2 Interfaces & Endpoints", &l2_labels)?
    );
    let l2_tbl = render_symbol_table(
        &snapshot.interfaces,
        "No public API endpoints or exported interface functions extracted.",
    )
    .trim_end()
    .to_string();
    pages.push(build_page(
        "interfaces-and-apis.md",
        &format!("{} — Layer 2: CLI, HTTP, MCP & API Interfaces", snapshot.repo_name),
        &format!("Source-anchored public functions, CLI commands, and HTTP/API routes in {}.", snapshot.repo_name),
        "cli-surface",
        "platform",
        &format!(
            "Layer 2 catalogs the exported programmatic functions, CLI commands, trait contracts, and HTTP or API endpoints discovered across {}. Each documented interface entry records its implementation language stack, canonical type signature, and exact line-range source anchor so human developers and autonomous coding agents verify behavior against live repository definitions instead of stale assumptions.",
            snapshot.repo_name
        ),
        vec![
            ("layer2-interface-diagram", l2_diag),
            ("layer2-endpoints-and-symbols", l2_tbl),
        ],
    ));

    let l3_labels: Vec<String> = snapshot.schemas.iter().map(|s| s.name.clone()).collect();
    let l3_diag = format!(
        "```vox-diagram\n{}```",
        render_layer_diagram("Layer 3 Data Models & Schemas", &l3_labels)?
    );
    let l3_tbl = render_symbol_table(
        &snapshot.schemas,
        "No SQL tables or typed data model definitions extracted.",
    )
    .trim_end()
    .to_string();
    pages.push(build_page(
        "data-models-and-schemas.md",
        &format!("{} — Layer 3: Data Models & SQL/VoxDb Schemas", snapshot.repo_name),
        &format!("Database tables, typed structs, interfaces, and data schemas in {}.", snapshot.repo_name),
        "database",
        "data",
        &format!(
            "Layer 3 summarizes persistent SQL DDL tables alongside core domain data structures in {} across Rust structs and enums, TypeScript interfaces and types, Python data models, Go structs, and Vox table definitions. Tracking schema sources and field contracts inside deterministic auto-fences highlights structural drift immediately whenever migrations or serialized payload definitions evolve.",
            snapshot.repo_name
        ),
        vec![
            ("layer3-schema-diagram", l3_diag),
            ("layer3-models-and-tables", l3_tbl),
        ],
    ));

    let l4_labels: Vec<String> = snapshot
        .gui_surfaces
        .iter()
        .map(|s| s.name.clone())
        .collect();
    let l4_diag = format!(
        "```vox-diagram\n{}```",
        render_layer_diagram("Layer 4 GUI & Frontend Surfaces", &l4_labels)?
    );
    let l4_tbl = render_symbol_table(
        &snapshot.gui_surfaces,
        "No frontend UI components or interactive views detected.",
    )
    .trim_end()
    .to_string();
    pages.push(build_page(
        "gui-surfaces-and-flows.md",
        &format!("{} — Layer 4: GUI & Frontend Surfaces", snapshot.repo_name),
        &format!("Extracted UI components, pages, views, and frontend surfaces in {}.", snapshot.repo_name),
        "desktop-app",
        "gui",
        &format!(
            "Layer 4 maps user-facing presentation components, React or TSX views, dashboard panels, and Vox reactive UI islands across {}. Maintaining a deterministic, source-anchored inventory of rendered surfaces and component coordinates enables frontend contributors, design reviewers, and visual auditing tools to trace rendered application behavior directly back to exact implementation modules.",
            snapshot.repo_name
        ),
        vec![
            ("layer4-surface-diagram", l4_diag),
            ("layer4-surfaces-table", l4_tbl),
        ],
    ));

    let l5_labels: Vec<String> = snapshot.workflows.iter().map(|s| s.name.clone()).collect();
    let l5_diag = format!(
        "```vox-diagram\n{}```",
        render_layer_diagram("Layer 5 Workflows & Runbooks", &l5_labels)?
    );
    let l5_tbl = render_symbol_table(
        &snapshot.workflows,
        "No durable workflows, CI pipelines, or operational runbooks detected.",
    )
    .trim_end()
    .to_string();
    pages.push(build_page(
        "workflows-and-runbooks.md",
        &format!("{} — Layer 5: Workflows, Actors & Runbooks", snapshot.repo_name),
        &format!("CI workflows, durable actor pipelines, and operational runbooks in {}.", snapshot.repo_name),
        "durable-workflows",
        "runtime",
        &format!(
            "Layer 5 inventories durable workflows, actor definitions, continuous integration pipelines, and repository operational runbooks within {}. Keeping workflow entrypoints, job manifests, and automation helpers source-anchored inside deterministic fences ensures operators and coding agents always locate executable pipeline definitions without relying on unverified manual links or stale instructions.",
            snapshot.repo_name
        ),
        vec![
            ("layer5-workflow-diagram", l5_diag),
            ("layer5-workflows-table", l5_tbl),
        ],
    ));

    Ok(pages)
}

fn build_page(
    rel_path: &str,
    title: &str,
    description: &str,
    topic: &str,
    domain: &str,
    summary_text: &str,
    auto_regions: Vec<(&str, String)>,
) -> OpenWikiPageSpec {
    let mut md = render_okf_frontmatter(title, description, topic, domain);
    md.push_str(&format!(
        "\n# {title}\n\n<!-- kb:summary -->\n{summary_text}\n<!-- /kb:summary -->\n"
    ));

    let mut regions_map = BTreeMap::new();
    for (id, body) in auto_regions {
        let heading = id.replace('-', " ");
        md.push_str(&format!("\n## {heading}\n\n<!-- openwiki:auto:start id=\"{id}\" -->\n{body}\n<!-- openwiki:auto:end id=\"{id}\" -->\n"));
        regions_map.insert(id.to_string(), body);
    }
    md.push_str("\n## Human Architecture Notes\n\nDocument design rationale, invariants, and operational caveats here; OpenWiki preserves prose outside `openwiki:auto` fences.\n");

    OpenWikiPageSpec {
        rel_path: rel_path.to_string(),
        full_markdown: md,
        regions: regions_map,
    }
}

fn resolve_paths(repo_root: &Path, output_override: Option<&Path>) -> (PathBuf, PathBuf) {
    match output_override {
        Some(dir) => (dir.join("wiki"), dir.join("manifest.v1.json")),
        None => (
            repo_root.join("docs/wiki"),
            repo_root
                .join(".vox")
                .join("cache")
                .join("openwiki")
                .join("manifest.v1.json"),
        ),
    }
}

/// Initialize OpenWiki (`docs/wiki/` + manifest if writing).
pub fn openwiki_init(
    repo_root: &Path,
    output_override: Option<&Path>,
) -> Result<OpenWikiBuildReport, String> {
    openwiki_build(repo_root, true, output_override)
}

/// Build or check 5-layer Polyglot OpenWiki pages using non-destructive OKF auto-fence merging.
pub fn openwiki_build(
    repo_root: &Path,
    write: bool,
    output_override: Option<&Path>,
) -> Result<OpenWikiBuildReport, String> {
    let (wiki_dir, manifest_path) = resolve_paths(repo_root, output_override);
    let snapshot = extract_polyglot_wiki_snapshot(repo_root);
    let pages = render_polyglot_openwiki_pages(&snapshot)?;

    if write {
        fs::create_dir_all(&wiki_dir)
            .map_err(|e| format!("failed to create {}: {e}", wiki_dir.display()))?;
        if let Some(parent) = manifest_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        }
    }

    let mut generated_pages = Vec::new();
    let mut updated_pages = Vec::new();
    let mut unchanged_pages = Vec::new();
    let mut updated_regions = Vec::new();
    let mut manifest_entries = Vec::new();

    for spec in &pages {
        let target = wiki_dir.join(&spec.rel_path);
        let (final_md, changed_regs, is_changed) = if target.is_file() {
            let existing = fs::read_to_string(&target)
                .map_err(|e| format!("failed to read {}: {e}", target.display()))?;
            let (merged, regs) = merge_openwiki_auto_regions(&existing, &spec.regions)?;
            let changed = merged != existing.replace("\r\n", "\n");
            (merged, regs, changed)
        } else {
            let regs: Vec<String> = spec.regions.keys().cloned().collect();
            (spec.full_markdown.clone(), regs, true)
        };

        if write && is_changed {
            fs::write(&target, &final_md)
                .map_err(|e| format!("failed to write {}: {e}", target.display()))?;
        }
        let rel_display = format!("docs/wiki/{}", spec.rel_path);
        generated_pages.push(rel_display.clone());
        if is_changed {
            updated_pages.push(rel_display.clone());
            updated_regions.push((rel_display.clone(), changed_regs));
        } else {
            unchanged_pages.push(rel_display.clone());
        }

        let region_hashes = spec
            .regions
            .iter()
            .map(|(k, v)| (k.clone(), hash_normalized_code(v)))
            .collect();
        manifest_entries.push(OpenWikiManifestPageEntry {
            rel_path: spec.rel_path.clone(),
            content_hash: hash_normalized_code(&final_md),
            region_hashes,
        });
    }

    let manifest_payload = serde_json::to_string_pretty(&manifest_entries).unwrap_or_default();
    let manifest_hash = hash_normalized_code(&manifest_payload);
    if write {
        let manifest = OpenWikiManifest {
            schema_version: 1,
            repo_name: snapshot.repo_name.clone(),
            detected_stacks: snapshot.detected_stacks.clone(),
            manifest_hash: manifest_hash.clone(),
            pages: manifest_entries,
        };
        let yaml_or_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("failed to serialize OpenWiki manifest: {e}"))?;
        fs::write(&manifest_path, yaml_or_json)
            .map_err(|e| format!("failed to write {}: {e}", manifest_path.display()))?;
    }

    Ok(OpenWikiBuildReport {
        repo_root: repo_root.display().to_string(),
        output_dir: wiki_dir.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        detected_stacks: snapshot.detected_stacks,
        generated_pages,
        updated_pages,
        unchanged_pages,
        updated_regions,
        manifest_hash,
    })
}

/// Check live OpenWiki status without mutating disk files.
pub fn openwiki_status(
    repo_root: &Path,
    output_override: Option<&Path>,
) -> Result<OpenWikiStatusReport, String> {
    let (wiki_dir, manifest_path) = resolve_paths(repo_root, output_override);
    let snapshot = extract_polyglot_wiki_snapshot(repo_root);
    let specs = render_polyglot_openwiki_pages(&snapshot)?;
    let mut pages = Vec::new();
    let mut stale_pages = 0usize;

    for spec in &specs {
        let target = wiki_dir.join(&spec.rel_path);
        if !target.is_file() {
            stale_pages += 1;
            pages.push(OpenWikiPageStatus {
                rel_path: spec.rel_path.clone(),
                exists: false,
                region_ids: spec.regions.keys().cloned().collect(),
                stale: true,
                stale_regions: spec.regions.keys().cloned().collect(),
            });
            continue;
        }
        let existing = fs::read_to_string(&target)
            .map_err(|e| format!("failed to read {}: {e}", target.display()))?;
        let (_, stale_regions) = merge_openwiki_auto_regions(&existing, &spec.regions)?;
        let stale = !stale_regions.is_empty();
        if stale {
            stale_pages += 1;
        }
        pages.push(OpenWikiPageStatus {
            rel_path: spec.rel_path.clone(),
            exists: true,
            region_ids: extract_openwiki_auto_regions(&existing)
                .into_keys()
                .collect(),
            stale,
            stale_regions,
        });
    }

    let initialized = pages.iter().all(|p| p.exists);
    Ok(OpenWikiStatusReport {
        initialized,
        manifest_present: manifest_path.is_file(),
        wiki_dir: wiki_dir.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        detected_stacks: snapshot.detected_stacks,
        total_pages: LAYER_PAGE_ORDER.len(),
        pages,
        stale_pages,
    })
}

/// Preview deterministic OpenWiki updates without writing files.
pub fn openwiki_diff(
    repo_root: &Path,
    output_override: Option<&Path>,
) -> Result<OpenWikiDiffReport, String> {
    let (wiki_dir, _) = resolve_paths(repo_root, output_override);
    let snapshot = extract_polyglot_wiki_snapshot(repo_root);
    let specs = render_polyglot_openwiki_pages(&snapshot)?;
    let mut diffs = Vec::new();
    let mut has_changes = false;

    for spec in &specs {
        let target = wiki_dir.join(&spec.rel_path);
        let (existing, updated_regions, merged) = if target.is_file() {
            let ex = fs::read_to_string(&target)
                .map_err(|e| format!("failed to read {}: {e}", target.display()))?;
            let (m, regs) = merge_openwiki_auto_regions(&ex, &spec.regions)?;
            (ex, regs, m)
        } else {
            (
                String::new(),
                spec.regions.keys().cloned().collect(),
                spec.full_markdown.clone(),
            )
        };
        let changed = merged != existing.replace("\r\n", "\n");
        if changed {
            has_changes = true;
        }
        let legacy_mermaid_blocks = vox_diagram::fence::extract(&merged, "mermaid").len();
        let preview: String = merged.lines().take(24).collect::<Vec<_>>().join("\n");
        diffs.push(OpenWikiPageDiff {
            rel_path: spec.rel_path.clone(),
            changed,
            updated_regions,
            legacy_mermaid_blocks,
            existing_bytes: existing.len(),
            rendered_bytes: merged.len(),
            preview,
        });
    }

    Ok(OpenWikiDiffReport {
        wiki_dir: wiki_dir.display().to_string(),
        detected_stacks: snapshot.detected_stacks,
        diffs,
        has_changes,
    })
}

#[cfg(test)]
#[path = "openwiki_polyglot_tests.rs"]
mod tests;
