//! Polyglot extractors (Go, SQL/DDL, HTTP/CLI endpoints, Markdown/ADR citations) and
//! deterministic Code <-> Doc <-> Decision cross-edge materialization (`GRAPH-02`, `GRAPH-03`).

use std::collections::HashSet;
use std::path::Path;

use crate::ast::{ExtractedEdge, ExtractedGraph, ExtractedNode, IdAlloc, qualify_in};

pub use crate::polyglot_edges::{
    augment_rebuild_with_polyglot_and_cross_edges, extract_markdown_citations,
    materialize_cross_edges, walk_doc_and_schema_files,
};

/// Confidence / honesty tier stamped on extracted and inferred cross-edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HonestyLevel {
    Extracted,
    Inferred,
    Ambiguous,
}

impl HonestyLevel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Extracted => "EXTRACTED",
            Self::Inferred => "INFERRED",
            Self::Ambiguous => "AMBIGUOUS",
        }
    }

    #[must_use]
    pub const fn weight(self) -> f32 {
        match self {
            Self::Extracted => 1.0,
            Self::Inferred => 0.7,
            Self::Ambiguous => 0.35,
        }
    }
}

/// Canonical Code <-> Doc <-> Decision cross-edge relations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CrossEdgeKind {
    DocumentedIn,
    CitesSymbol,
    GovernedByAdr,
    TouchedByDecision,
}

impl CrossEdgeKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DocumentedIn => "documented_in",
            Self::CitesSymbol => "cites_symbol",
            Self::GovernedByAdr => "governed_by_adr",
            Self::TouchedByDecision => "touched_by_decision",
        }
    }
}

pub(crate) fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn sanitize_ident_prefix(s: &str) -> &str {
    let end = s
        .char_indices()
        .find(|&(_, c)| !is_ident_char(c))
        .map_or(s.len(), |(i, _)| i);
    &s[..end]
}

/// Extract Go packages, structs/interfaces, methods, and functions.
pub fn extract_go_in_module(content: &str, module_id: &str) -> ExtractedGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut ids = IdAlloc::default();
    let mut pkg_node_id: Option<String> = None;
    let mut fn_names = HashSet::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if let Some(rest) = line.strip_prefix("package ") {
            let pkg = sanitize_ident_prefix(rest.trim());
            if !pkg.is_empty() && pkg_node_id.is_none() {
                let id = ids.alloc(qualify_in(module_id, &[], pkg));
                pkg_node_id = Some(id.clone());
                nodes.push(ExtractedNode {
                    id,
                    label: pkg.to_string(),
                    kind: "go-package".to_string(),
                });
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("type ") {
            let name = sanitize_ident_prefix(rest.trim());
            let after = rest.trim()[name.len()..].trim_start();
            let kind = if after.starts_with("interface") {
                "trait"
            } else if after.starts_with("struct") {
                "struct"
            } else {
                "type"
            };
            if !name.is_empty() {
                let id = ids.alloc(qualify_in(module_id, &[], name));
                if let Some(ref pkg_id) = pkg_node_id {
                    edges.push(ExtractedEdge {
                        source: pkg_id.clone(),
                        target: id.clone(),
                        confidence: HonestyLevel::Extracted.as_str().to_string(),
                        kind: "defines".to_string(),
                    });
                }
                nodes.push(ExtractedNode {
                    id,
                    label: name.to_string(),
                    kind: kind.to_string(),
                });
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("func ") {
            let trimmed = rest.trim_start();
            if trimmed.starts_with('(') {
                if let Some(close) = trimmed.find(')') {
                    let recv_raw = trimmed[1..close].trim();
                    let recv_ty = recv_raw
                        .split_whitespace()
                        .last()
                        .unwrap_or("")
                        .trim_start_matches('*');
                    let after_recv = trimmed[close + 1..].trim_start();
                    let name = sanitize_ident_prefix(after_recv);
                    if !name.is_empty() {
                        let scope = if recv_ty.is_empty() {
                            Vec::new()
                        } else {
                            vec![recv_ty.to_string()]
                        };
                        let id = ids.alloc(qualify_in(module_id, &scope, name));
                        fn_names.insert((name.to_string(), id.clone()));
                        nodes.push(ExtractedNode {
                            id,
                            label: name.to_string(),
                            kind: "method".to_string(),
                        });
                    }
                }
            } else {
                let name = sanitize_ident_prefix(trimmed);
                if !name.is_empty() {
                    let id = ids.alloc(qualify_in(module_id, &[], name));
                    fn_names.insert((name.to_string(), id.clone()));
                    if let Some(ref pkg_id) = pkg_node_id {
                        edges.push(ExtractedEdge {
                            source: pkg_id.clone(),
                            target: id.clone(),
                            confidence: HonestyLevel::Extracted.as_str().to_string(),
                            kind: "defines".to_string(),
                        });
                    }
                    nodes.push(ExtractedNode {
                        id,
                        label: name.to_string(),
                        kind: "fn".to_string(),
                    });
                }
            }
        }
    }

    // Attach lightweight inline call/table/endpoint relations.
    let mut inline = extract_inline_sql_and_endpoints(content, module_id);
    nodes.append(&mut inline.nodes);
    edges.append(&mut inline.edges);
    ExtractedGraph { nodes, edges }
}

/// Extract SQL tables and foreign-key `REFERENCES` edges from `.sql` DDL text.
pub fn extract_sql_in_module(content: &str, module_id: &str) -> ExtractedGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen_tables = HashSet::new();
    let mut current_table: Option<String> = None;

    for raw_line in content.lines() {
        let lower = raw_line.to_ascii_lowercase();
        let trimmed_lower = lower.trim();
        if let Some(idx) = trimmed_lower.find("create table") {
            let mut rest = raw_line.trim()[idx + "create table".len()..].trim_start();
            let rest_lower = rest.to_ascii_lowercase();
            if rest_lower.starts_with("if not exists") {
                rest = rest["if not exists".len()..].trim_start();
            }
            let tbl = rest
                .trim_start_matches(['"', '`', '['])
                .split(|c: char| !is_ident_char(c) && c != '.')
                .next()
                .unwrap_or("")
                .trim();
            if !tbl.is_empty() {
                let tbl_id = format!("sql:{tbl}");
                current_table = Some(tbl_id.clone());
                if seen_tables.insert(tbl_id.clone()) {
                    nodes.push(ExtractedNode {
                        id: tbl_id.clone(),
                        label: tbl.to_string(),
                        kind: "sql-table".to_string(),
                    });
                }
                if !module_id.is_empty() {
                    edges.push(ExtractedEdge {
                        source: format!("file:{module_id}"),
                        target: tbl_id,
                        confidence: HonestyLevel::Extracted.as_str().to_string(),
                        kind: "defines_table".to_string(),
                    });
                }
            }
        }
        if let Some(ref src_tbl) = current_table
            && let Some(ref_pos) = trimmed_lower.find("references ")
        {
            let after = raw_line.trim()[ref_pos + "references ".len()..].trim_start();
            let target = after
                .trim_start_matches(['"', '`', '['])
                .split(|c: char| !is_ident_char(c) && c != '.')
                .next()
                .unwrap_or("")
                .trim();
            if !target.is_empty() {
                edges.push(ExtractedEdge {
                    source: src_tbl.clone(),
                    target: format!("sql:{target}"),
                    confidence: HonestyLevel::Extracted.as_str().to_string(),
                    kind: "references_table".to_string(),
                });
            }
        }
        if trimmed_lower.contains(';') {
            current_table = None;
        }
    }

    ExtractedGraph { nodes, edges }
}

/// Extract inline HTTP/CLI endpoint routes and inline `CREATE TABLE` strings from `.rs`/`.ts`/`.py`/`.vox`/`.go`.
pub fn extract_inline_sql_and_endpoints(content: &str, module_id: &str) -> ExtractedGraph {
    let mut graph = extract_sql_in_module(content, module_id);
    let mut seen_endpoints = HashSet::new();
    let route_markers = [
        ".route(\"",
        ".get(\"",
        ".post(\"",
        ".put(\"",
        ".delete(\"",
        "@app.get(\"",
        "@app.post(\"",
        "route \"",
    ];
    for line in content.lines() {
        for marker in route_markers {
            if let Some(pos) = line.find(marker) {
                let after = &line[pos + marker.len()..];
                if let Some(end) = after.find('"') {
                    let route = &after[..end];
                    if route.starts_with('/') && seen_endpoints.insert(route.to_string()) {
                        let ep_id = format!("endpoint:{route}");
                        graph.nodes.push(ExtractedNode {
                            id: ep_id.clone(),
                            label: route.to_string(),
                            kind: "endpoint".to_string(),
                        });
                        if !module_id.is_empty() {
                            graph.edges.push(ExtractedEdge {
                                source: format!("file:{module_id}"),
                                target: ep_id,
                                confidence: HonestyLevel::Extracted.as_str().to_string(),
                                kind: "exposes_endpoint".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    graph
}

#[must_use]
pub fn is_adr_path(rel_path: &str) -> bool {
    let lower = rel_path.to_ascii_lowercase();
    lower.contains("/adr/")
        || lower.starts_with("adr/")
        || lower.contains("adr-")
        || lower.contains("decision")
}

fn md_title(content: &str, module_id: &str) -> String {
    for line in content.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("# ")
            && !rest.trim().is_empty()
        {
            return rest.trim().to_string();
        }
    }
    Path::new(module_id)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(module_id)
        .to_string()
}

/// Extract document node + decision ids from a Markdown/ADR document.
pub fn extract_markdown_in_module(content: &str, module_id: &str) -> ExtractedGraph {
    let doc_id = if module_id.is_empty() {
        "doc:untitled".to_string()
    } else {
        format!("doc:{module_id}")
    };
    let kind = if is_adr_path(module_id) { "adr" } else { "doc" };
    let mut nodes = vec![ExtractedNode {
        id: doc_id.clone(),
        label: md_title(content, module_id),
        kind: kind.to_string(),
    }];
    let mut edges = Vec::new();
    let (_, _, _, decisions) = extract_markdown_citations(content);
    for d in decisions {
        let dec_id = format!("decision:{d}");
        nodes.push(ExtractedNode {
            id: dec_id.clone(),
            label: d,
            kind: "decision".to_string(),
        });
        edges.push(ExtractedEdge {
            source: doc_id.clone(),
            target: dec_id,
            confidence: HonestyLevel::Extracted.as_str().to_string(),
            kind: CrossEdgeKind::TouchedByDecision.as_str().to_string(),
        });
    }
    ExtractedGraph { nodes, edges }
}

/// Dispatch non-Rust/TS/Python/Vox file types (`go`, `sql`, `md`, `mdx`).
pub fn extract_polyglot_in_module(ext: &str, content: &str, module_id: &str) -> ExtractedGraph {
    match ext {
        "go" => extract_go_in_module(content, module_id),
        "sql" => extract_sql_in_module(content, module_id),
        "md" | "mdx" => extract_markdown_in_module(content, module_id),
        _ => ExtractedGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
        },
    }
}
