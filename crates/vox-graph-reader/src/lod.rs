//! Budgeted Level-of-Detail (LOD) subgraph slicer and derived `crate -> file -> symbol`
//! containment hierarchy for the Code & Architecture Atlas (Phase 25).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ast::EXTRACTOR_VERSION;
use crate::crate_model::build_crate_map;

pub const DEFAULT_ATLAS_LIMIT: usize = 400;
pub const MAX_ATLAS_LIMIT: usize = 1000;

fn default_atlas_limit() -> usize {
    DEFAULT_ATLAS_LIMIT
}

fn default_extractor_version() -> String {
    EXTRACTOR_VERSION.to_string()
}

/// Hierarchy level for progressive drill-down in the Code Atlas.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AtlasLevel {
    Crate,
    Module,
    File,
    Symbol,
}

/// Request payload for `vox_search_atlas` / `query_atlas_subgraph`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AtlasQuery {
    pub level: AtlasLevel,
    #[serde(default)]
    pub focus: Option<String>,
    #[serde(default)]
    pub branch_tip: Option<String>,
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default = "default_atlas_limit")]
    pub limit: usize,
}

/// Provenance and freshness header attached to every `AtlasSubgraph` response.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AtlasIdentity {
    pub corpus_id: String,
    #[serde(default)]
    pub git_sha: Option<String>,
    #[serde(default)]
    pub ref_name: Option<String>,
    #[serde(default)]
    pub worktree_id: Option<String>,
    #[serde(default)]
    pub built_at: Option<String>,
    #[serde(default)]
    pub lens: Option<String>,
    #[serde(default = "default_extractor_version")]
    pub extractor_version: String,
}

impl Default for AtlasIdentity {
    fn default() -> Self {
        Self {
            corpus_id: "default".to_string(),
            git_sha: None,
            ref_name: None,
            worktree_id: None,
            built_at: None,
            lens: Some("structural".to_string()),
            extractor_version: EXTRACTOR_VERSION.to_string(),
        }
    }
}

/// Budgeted subgraph slice returned to the GUI / MCP consumer.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AtlasSubgraph {
    pub nodes: Vec<Value>,
    pub links: Vec<Value>,
    pub parent: Option<String>,
    pub truncated: bool,
    pub identity: AtlasIdentity,
}

/// Extract the file path portion of a `<path>::<container>*::<name>` symbol ID.
pub fn file_of_symbol_id(id: &str) -> &str {
    id.split("::").next().unwrap_or(id)
}

/// Derive the owning crate/package name from a repo-relative file path or symbol ID.
///
/// Examples:
/// - `"crates/vox-cli/src/main.rs::main"` -> `Some("vox-cli")`
/// - `"apps/vox-cloud/src/App.tsx"` -> `Some("vox-cloud")`
pub fn crate_of_id(id: &str) -> Option<String> {
    let path = file_of_symbol_id(id).trim_start_matches("./");
    let mut segs = path.split('/');
    match (segs.next(), segs.next()) {
        (Some("crates" | "apps" | "packages"), Some(name)) if !name.is_empty() => {
            Some(name.to_string())
        }
        _ => None,
    }
}

fn normalize_crate_focus(focus: &str) -> String {
    let trimmed = focus.trim().trim_matches('/');
    if let Some(rest) = trimmed
        .strip_prefix("crates/")
        .or_else(|| trimmed.strip_prefix("apps/"))
        .or_else(|| trimmed.strip_prefix("packages/"))
    {
        rest.split('/').next().unwrap_or(rest).to_string()
    } else {
        trimmed.to_string()
    }
}

/// Slice a structural graph (and optional `crate-graph.v1.json` + `crate_audit.json`) into a
/// budgeted `AtlasSubgraph` at the requested `AtlasLevel`.
pub fn query_atlas_subgraph(
    graph: &Value,
    crate_graph: Option<&Value>,
    crate_audit: Option<&Value>,
    query: &AtlasQuery,
    identity: AtlasIdentity,
) -> Result<AtlasSubgraph, String> {
    if identity.extractor_version != EXTRACTOR_VERSION {
        return Err(format!(
            "stale graph extractor_version '{}' (expected '{}'); rebuild required",
            identity.extractor_version, EXTRACTOR_VERSION
        ));
    }

    let cap = if query.limit == 0 {
        DEFAULT_ATLAS_LIMIT
    } else {
        query.limit.min(MAX_ATLAS_LIMIT)
    };

    let empty_vec = Vec::new();
    let raw_nodes = graph
        .get("nodes")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_vec);
    let raw_links = graph
        .get("links")
        .or_else(|| graph.get("edges"))
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_vec);

    let (mut nodes, mut links, parent) = match query.level {
        AtlasLevel::Crate => build_crate_level(
            raw_nodes,
            raw_links,
            crate_graph,
            crate_audit,
            query.focus.as_deref(),
        ),
        AtlasLevel::Module | AtlasLevel::File => {
            build_file_level(raw_nodes, raw_links, query.focus.as_deref())
        }
        AtlasLevel::Symbol => build_symbol_level(raw_nodes, raw_links, query.focus.as_deref()),
    };

    // Optional edge-kind filter (`query.kinds`): when non-empty, keep edges whose `kind` is
    // in `query.kinds` (plus `contains` hierarchy edges when drilling in).
    if !query.kinds.is_empty() {
        let allowed: HashSet<&str> = query.kinds.iter().map(String::as_str).collect();
        links.retain(|l| {
            let k = l.get("kind").and_then(|v| v.as_str()).unwrap_or("calls");
            allowed.contains(k)
        });
    }

    // Deterministic ordering before truncation: container/focused nodes first, then by degree/blast_s desc, then id asc.
    let mut degree: HashMap<String, usize> = HashMap::new();
    for l in &links {
        if let Some(s) = l.get("source").and_then(|v| v.as_str()) {
            *degree.entry(s.to_string()).or_insert(0) += 1;
        }
        if let Some(t) = l.get("target").and_then(|v| v.as_str()) {
            *degree.entry(t.to_string()).or_insert(0) += 1;
        }
    }
    let focus_norm = query.focus.as_deref().map(normalize_crate_focus);
    nodes.sort_by(|a, b| {
        let id_a = a.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let id_b = b.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let pin_a = query.focus.as_deref() == Some(id_a)
            || focus_norm.as_deref() == Some(id_a)
            || parent.as_deref() == Some(id_a);
        let pin_b = query.focus.as_deref() == Some(id_b)
            || focus_norm.as_deref() == Some(id_b)
            || parent.as_deref() == Some(id_b);
        pin_b
            .cmp(&pin_a)
            .then_with(|| {
                let deg_a = degree.get(id_a).copied().unwrap_or(0);
                let deg_b = degree.get(id_b).copied().unwrap_or(0);
                deg_b.cmp(&deg_a)
            })
            .then_with(|| id_a.cmp(id_b))
    });

    let truncated = nodes.len() > cap;
    if truncated {
        nodes.truncate(cap);
    }

    let kept_ids: HashSet<&str> = nodes
        .iter()
        .filter_map(|n| n.get("id").and_then(|v| v.as_str()))
        .collect();
    links.retain(|l| {
        let s = l.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let t = l.get("target").and_then(|v| v.as_str()).unwrap_or("");
        kept_ids.contains(s) && kept_ids.contains(t)
    });

    Ok(AtlasSubgraph {
        nodes,
        links,
        parent,
        truncated,
        identity,
    })
}

fn build_crate_level(
    raw_nodes: &[Value],
    raw_links: &[Value],
    crate_graph: Option<&Value>,
    crate_audit: Option<&Value>,
    focus: Option<&str>,
) -> (Vec<Value>, Vec<Value>, Option<String>) {
    let mut crate_nodes_by_id: HashMap<String, Value> = HashMap::new();
    let mut crate_edges: Vec<(String, String)> = Vec::new();

    if let Some(cg) = crate_graph {
        let empty_audit = json!([]);
        let cmap = build_crate_map(cg, crate_audit.unwrap_or(&empty_audit));
        if let Some(ns) = cmap.get("nodes").and_then(|v| v.as_array()) {
            for n in ns {
                if let Some(id) = n.get("id").and_then(|v| v.as_str()) {
                    let mut obj = n.clone();
                    obj["kind"] = json!("crate");
                    crate_nodes_by_id.insert(id.to_string(), obj);
                }
            }
        }
        if let Some(ls) = cmap.get("links").and_then(|v| v.as_array()) {
            for l in ls {
                if let (Some(s), Some(t)) = (
                    l.get("source").and_then(|v| v.as_str()),
                    l.get("target").and_then(|v| v.as_str()),
                ) {
                    crate_edges.push((s.to_string(), t.to_string()));
                }
            }
        }
    }

    // Also discover crates from symbol node paths so corpora without crate-graph.v1.json still
    // render a crate-level overview.
    let mut symbol_counts: HashMap<String, usize> = HashMap::new();
    for n in raw_nodes {
        if let Some(id) = n.get("id").and_then(|v| v.as_str()) {
            if let Some(c) = crate_of_id(id) {
                *symbol_counts.entry(c.clone()).or_insert(0) += 1;
                crate_nodes_by_id.entry(c.clone()).or_insert_with(|| {
                    json!({
                        "id": c,
                        "label": c,
                        "kind": "crate",
                        "community": "c_0"
                    })
                });
            }
        }
    }
    for (c, count) in &symbol_counts {
        if let Some(node) = crate_nodes_by_id.get_mut(c) {
            node["symbol_count"] = json!(count);
        }
    }

    if crate_edges.is_empty() {
        let mut seen = HashSet::new();
        for l in raw_links {
            if let (Some(s), Some(t)) = (
                l.get("source").and_then(|v| v.as_str()),
                l.get("target").and_then(|v| v.as_str()),
            ) {
                if let (Some(cs), Some(ct)) = (crate_of_id(s), crate_of_id(t)) {
                    if cs != ct && seen.insert((cs.clone(), ct.clone())) {
                        crate_edges.push((cs, ct));
                    }
                }
            }
        }
    }

    crate_edges.sort();
    crate_edges.dedup();

    if let Some(raw_focus) = focus.filter(|s| !s.trim().is_empty()) {
        let focused = normalize_crate_focus(raw_focus);
        let mut keep_crates: HashSet<String> = HashSet::from([focused.clone()]);
        let mut filtered_links = Vec::new();
        for (s, t) in crate_edges {
            if s == focused || t == focused {
                keep_crates.insert(s.clone());
                keep_crates.insert(t.clone());
                filtered_links.push(json!({
                    "source": s,
                    "target": t,
                    "kind": "crate_dep",
                    "confidence": "declared"
                }));
            }
        }
        let nodes = crate_nodes_by_id
            .into_iter()
            .filter(|(k, _)| keep_crates.contains(k))
            .map(|(_, v)| v)
            .collect();
        (nodes, filtered_links, None)
    } else {
        let links = crate_edges
            .into_iter()
            .map(|(s, t)| {
                json!({
                    "source": s,
                    "target": t,
                    "kind": "crate_dep",
                    "confidence": "declared"
                })
            })
            .collect();
        let nodes = crate_nodes_by_id.into_values().collect();
        (nodes, links, None)
    }
}

fn build_file_level(
    raw_nodes: &[Value],
    raw_links: &[Value],
    focus: Option<&str>,
) -> (Vec<Value>, Vec<Value>, Option<String>) {
    let focused_crate = focus
        .filter(|s| !s.trim().is_empty())
        .map(normalize_crate_focus);

    let mut file_symbol_counts: HashMap<String, usize> = HashMap::new();
    for n in raw_nodes {
        let Some(id) = n.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        // Skip synthetic command/tool/surface registry nodes when enumerating source files.
        if !id.contains("::") && id.contains(':') {
            continue;
        }
        let file = file_of_symbol_id(id);
        if let Some(ref fc) = focused_crate {
            let matches_crate = crate_of_id(file).as_deref() == Some(fc.as_str())
                || file.starts_with(focus.unwrap_or(""));
            if !matches_crate {
                continue;
            }
        }
        *file_symbol_counts.entry(file.to_string()).or_insert(0) += 1;
    }

    let mut nodes: Vec<Value> = Vec::new();
    let mut links: Vec<Value> = Vec::new();

    if let Some(ref fc) = focused_crate {
        nodes.push(json!({
            "id": fc,
            "label": fc,
            "kind": "crate",
            "community": "c_0"
        }));
    }

    let mut files: Vec<String> = file_symbol_counts.keys().cloned().collect();
    files.sort();
    for f in &files {
        let label = f.rsplit('/').next().unwrap_or(f);
        let sym_count = file_symbol_counts.get(f).copied().unwrap_or(0);
        nodes.push(json!({
            "id": f,
            "label": label,
            "kind": "file",
            "symbol_count": sym_count,
            "community": crate_of_id(f).unwrap_or_else(|| "c_0".to_string())
        }));
        if let Some(ref fc) = focused_crate {
            links.push(json!({
                "source": fc,
                "target": f,
                "kind": "contains",
                "confidence": "resolved"
            }));
        }
    }

    let file_set: HashSet<&str> = files.iter().map(String::as_str).collect();
    let mut inter_file_weights: HashMap<(String, String), usize> = HashMap::new();
    for l in raw_links {
        if let (Some(s), Some(t)) = (
            l.get("source").and_then(|v| v.as_str()),
            l.get("target").and_then(|v| v.as_str()),
        ) {
            let fs = file_of_symbol_id(s);
            let ft = file_of_symbol_id(t);
            if fs != ft && file_set.contains(fs) && file_set.contains(ft) {
                *inter_file_weights
                    .entry((fs.to_string(), ft.to_string()))
                    .or_insert(0) += 1;
            }
        }
    }
    let mut weighted_edges: Vec<((String, String), usize)> =
        inter_file_weights.into_iter().collect();
    weighted_edges.sort();
    for ((s, t), w) in weighted_edges {
        links.push(json!({
            "source": s,
            "target": t,
            "kind": "calls",
            "confidence": "resolved",
            "weight": w
        }));
    }

    (nodes, links, focused_crate)
}

fn build_symbol_level(
    raw_nodes: &[Value],
    raw_links: &[Value],
    focus: Option<&str>,
) -> (Vec<Value>, Vec<Value>, Option<String>) {
    let nodes_by_id: HashMap<&str, &Value> = raw_nodes
        .iter()
        .filter_map(|n| n.get("id").and_then(|v| v.as_str()).map(|id| (id, n)))
        .collect();

    let Some(focus_str) = focus.filter(|s| !s.trim().is_empty()) else {
        let nodes = raw_nodes.to_vec();
        let links = raw_links
            .iter()
            .map(|l| {
                let mut edge = l.clone();
                if edge.get("kind").is_none() {
                    let conf = edge
                        .get("confidence")
                        .and_then(|v| v.as_str())
                        .unwrap_or("resolved");
                    edge["kind"] = json!(if conf == "dangling" {
                        "mentions"
                    } else {
                        "calls"
                    });
                }
                edge
            })
            .collect();
        return (nodes, links, None);
    };

    // Determine whether `focus_str` is a specific symbol ID or a file path.
    let is_symbol_focus = nodes_by_id.contains_key(focus_str) || focus_str.contains("::");

    if is_symbol_focus {
        let mut selected_ids: HashSet<String> = HashSet::from([focus_str.to_string()]);
        let mut links: Vec<Value> = Vec::new();
        for l in raw_links {
            let s = l.get("source").and_then(|v| v.as_str()).unwrap_or("");
            let t = l.get("target").and_then(|v| v.as_str()).unwrap_or("");
            if s == focus_str || t == focus_str {
                selected_ids.insert(s.to_string());
                selected_ids.insert(t.to_string());
                let mut edge = l.clone();
                if edge.get("kind").is_none() {
                    let conf = edge
                        .get("confidence")
                        .and_then(|v| v.as_str())
                        .unwrap_or("resolved");
                    edge["kind"] = json!(if conf == "dangling" {
                        "mentions"
                    } else {
                        "calls"
                    });
                }
                links.push(edge);
            }
        }
        let parent_file = file_of_symbol_id(focus_str).to_string();
        let mut nodes: Vec<Value> = Vec::new();
        if parent_file != focus_str {
            let file_label = parent_file.rsplit('/').next().unwrap_or(&parent_file);
            nodes.push(json!({
                "id": parent_file,
                "label": file_label,
                "kind": "file",
                "community": crate_of_id(&parent_file).unwrap_or_else(|| "c_0".to_string())
            }));
            links.push(json!({
                "source": parent_file,
                "target": focus_str,
                "kind": "contains",
                "confidence": "resolved"
            }));
        }
        for id in &selected_ids {
            if let Some(n) = nodes_by_id.get(id.as_str()) {
                nodes.push((*n).clone());
            }
        }
        let parent = (parent_file != focus_str).then_some(parent_file);
        (nodes, links, parent)
    } else {
        // File focus -> all symbols in that file + 1-hop cross-file call targets + file `contains` edges.
        let file_path = focus_str;
        let file_label = file_path.rsplit('/').next().unwrap_or(file_path);
        let mut nodes: Vec<Value> = vec![json!({
            "id": file_path,
            "label": file_label,
            "kind": "file",
            "community": crate_of_id(file_path).unwrap_or_else(|| "c_0".to_string())
        })];
        let mut links: Vec<Value> = Vec::new();
        let mut in_file_ids: HashSet<String> = HashSet::new();

        for n in raw_nodes {
            if let Some(id) = n.get("id").and_then(|v| v.as_str()) {
                if file_of_symbol_id(id) == file_path {
                    in_file_ids.insert(id.to_string());
                    nodes.push(n.clone());
                    links.push(json!({
                        "source": file_path,
                        "target": id,
                        "kind": "contains",
                        "confidence": "resolved"
                    }));
                }
            }
        }

        let mut neighbor_ids: HashSet<String> = HashSet::new();
        for l in raw_links {
            let s = l.get("source").and_then(|v| v.as_str()).unwrap_or("");
            let t = l.get("target").and_then(|v| v.as_str()).unwrap_or("");
            if in_file_ids.contains(s) || in_file_ids.contains(t) {
                if !in_file_ids.contains(s) {
                    neighbor_ids.insert(s.to_string());
                }
                if !in_file_ids.contains(t) {
                    neighbor_ids.insert(t.to_string());
                }
                let mut edge = l.clone();
                if edge.get("kind").is_none() {
                    let conf = edge
                        .get("confidence")
                        .and_then(|v| v.as_str())
                        .unwrap_or("resolved");
                    edge["kind"] = json!(if conf == "dangling" {
                        "mentions"
                    } else {
                        "calls"
                    });
                }
                links.push(edge);
            }
        }

        for nid in neighbor_ids {
            if let Some(n) = nodes_by_id.get(nid.as_str()) {
                nodes.push((*n).clone());
            }
        }

        let parent = crate_of_id(file_path);
        (nodes, links, parent)
    }
}
