//! Markdown/ADR citation extraction, Code <-> Doc <-> Decision cross-edge materialization,
//! and incremental rebuild integration (`GRAPH-02`, `GRAPH-03`).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{EXTRACTOR_VERSION, ExtractedEdge, ExtractedNode};
use crate::cache::CacheManager;
use crate::polyglot::{
    CrossEdgeKind, HonestyLevel, extract_polyglot_in_module, is_adr_path, is_ident_char,
};

/// Parse markdown citations: `(explicit_paths, qualified_symbols, bare_symbols, decision_ids)`.
#[must_use]
pub fn extract_markdown_citations(
    content: &str,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut paths = BTreeSet::new();
    let mut qualified = BTreeSet::new();
    let mut bare = BTreeSet::new();
    let mut decisions = BTreeSet::new();

    let mut cursor = content;
    while let Some(open_br) = cursor.find("](") {
        let after = &cursor[open_br + 2..];
        let Some(close_par) = after.find(')') else {
            break;
        };
        let raw_target = after[..close_par].trim();
        let clean = raw_target
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_start_matches("./");
        let path_part = clean.split('#').next().unwrap_or("");
        if !path_part.is_empty()
            && !path_part.starts_with("http://")
            && !path_part.starts_with("https://")
            && !path_part.starts_with("mailto:")
            && !path_part.starts_with("kb://")
            && !path_part.starts_with('/')
            && !path_part.contains("..")
        {
            paths.insert(path_part.replace('\\', "/"));
        }
        cursor = &after[close_par + 1..];
    }

    let mut in_fence = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if let Some(d) = trimmed.strip_prefix("decision_id:") {
            let val = d.trim().trim_matches(|c| c == '"' || c == '\'');
            if !val.is_empty() {
                decisions.insert(val.to_string());
            }
        }
        if let Some(pos) = trimmed.find("ADR-") {
            let after = &trimmed[pos..];
            let token: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if token.len() > 4 {
                decisions.insert(token);
            }
        }
        if in_fence {
            continue;
        }
        let mut s = line;
        while let Some(start) = s.find('`') {
            let rest = &s[start + 1..];
            let Some(end) = rest.find('`') else {
                break;
            };
            let span = rest[..end].trim();
            if !span.is_empty() && !span.contains(' ') {
                if span.contains("::") && span.chars().all(|c| is_ident_char(c) || c == ':') {
                    qualified.insert(span.to_string());
                } else if span.len() >= 3
                    && span.chars().all(is_ident_char)
                    && (span.contains('_') || span.chars().any(|c| c.is_ascii_uppercase()))
                {
                    bare.insert(span.to_string());
                }
            }
            s = &rest[end + 1..];
        }
    }

    (
        paths.into_iter().collect(),
        qualified.into_iter().collect(),
        bare.into_iter().collect(),
        decisions.into_iter().collect(),
    )
}

/// Materialize deterministic Code <-> Doc <-> Decision cross-edges (`documented_in`, `cites_symbol`,
/// `governed_by_adr`, `touched_by_decision`) tagged with `EXTRACTED` / `INFERRED` / `AMBIGUOUS`.
#[must_use]
pub fn materialize_cross_edges(
    nodes: &[ExtractedNode],
    doc_files: &[(String, String)],
) -> Vec<ExtractedEdge> {
    let mut by_module: HashMap<String, Vec<&ExtractedNode>> = HashMap::new();
    let mut by_bare: HashMap<String, Vec<&ExtractedNode>> = HashMap::new();
    for n in nodes {
        if matches!(n.kind.as_str(), "doc" | "adr" | "decision") {
            continue;
        }
        if let Some((mod_path, _)) = n.id.split_once("::") {
            by_module.entry(mod_path.to_string()).or_default().push(n);
        }
        let bare = n.id.rsplit("::").next().unwrap_or(&n.label);
        by_bare.entry(bare.to_string()).or_default().push(n);
    }

    let mut seen = HashSet::new();
    let mut edges = Vec::new();
    let mut push_edge = |src: &str, dst: &str, kind: CrossEdgeKind, honesty: HonestyLevel| {
        let key = (src.to_string(), dst.to_string(), kind.as_str());
        if seen.insert(key) {
            edges.push(ExtractedEdge {
                source: src.to_string(),
                target: dst.to_string(),
                confidence: honesty.as_str().to_string(),
                kind: kind.as_str().to_string(),
            });
        }
    };

    for (doc_path, content) in doc_files {
        let doc_id = format!("doc:{doc_path}");
        let adr = is_adr_path(doc_path);
        let (paths, qualified, bare, decisions) = extract_markdown_citations(content);
        let has_decision = adr || !decisions.is_empty();

        for rel in paths {
            if let Some(candidates) = by_module.get(&rel) {
                for n in candidates.iter().take(4) {
                    push_edge(
                        &doc_id,
                        &n.id,
                        CrossEdgeKind::CitesSymbol,
                        HonestyLevel::Extracted,
                    );
                    push_edge(
                        &n.id,
                        &doc_id,
                        CrossEdgeKind::DocumentedIn,
                        HonestyLevel::Extracted,
                    );
                    if adr {
                        push_edge(
                            &n.id,
                            &doc_id,
                            CrossEdgeKind::GovernedByAdr,
                            HonestyLevel::Extracted,
                        );
                    } else if has_decision {
                        push_edge(
                            &n.id,
                            &doc_id,
                            CrossEdgeKind::TouchedByDecision,
                            HonestyLevel::Extracted,
                        );
                    }
                }
            }
        }

        for q in qualified {
            let suffix = format!("::{q}");
            let matches: Vec<&ExtractedNode> = nodes
                .iter()
                .filter(|n| n.id == q || n.id.ends_with(&suffix))
                .collect();
            let honesty = if matches.len() == 1 {
                HonestyLevel::Extracted
            } else {
                HonestyLevel::Ambiguous
            };
            for n in matches.into_iter().take(3) {
                push_edge(&doc_id, &n.id, CrossEdgeKind::CitesSymbol, honesty);
                push_edge(&n.id, &doc_id, CrossEdgeKind::DocumentedIn, honesty);
                if adr {
                    push_edge(&n.id, &doc_id, CrossEdgeKind::GovernedByAdr, honesty);
                } else if has_decision {
                    push_edge(&n.id, &doc_id, CrossEdgeKind::TouchedByDecision, honesty);
                }
            }
        }

        for b in bare {
            if let Some(matches) = by_bare.get(&b) {
                let honesty = if matches.len() == 1 {
                    HonestyLevel::Inferred
                } else {
                    HonestyLevel::Ambiguous
                };
                for n in matches.iter().take(3) {
                    push_edge(&doc_id, &n.id, CrossEdgeKind::CitesSymbol, honesty);
                    push_edge(&n.id, &doc_id, CrossEdgeKind::DocumentedIn, honesty);
                    if adr {
                        push_edge(&n.id, &doc_id, CrossEdgeKind::GovernedByAdr, honesty);
                    } else if has_decision {
                        push_edge(&n.id, &doc_id, CrossEdgeKind::TouchedByDecision, honesty);
                    }
                }
            }
        }
    }

    edges
}

/// Walk `.go`, `.sql`, `.md`, and `.mdx` files respecting `.gitignore` and excluding build dirs.
pub fn walk_doc_and_schema_files(source_dir: &Path) -> Vec<PathBuf> {
    ignore::WalkBuilder::new(source_dir)
        .require_git(false)
        .hidden(true)
        .git_global(false)
        .git_exclude(false)
        .sort_by_file_path(|a, b| a.cmp(b))
        .filter_entry(|e| {
            let n = e.file_name().to_string_lossy();
            n != "target" && n != "node_modules" && n != "web-dist"
        })
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .filter(|e| {
            matches!(
                e.path().extension().and_then(|x| x.to_str()),
                Some("go" | "sql" | "md" | "mdx")
            )
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Fold Go, SQL, Markdown, and Code <-> Doc <-> Decision cross-edges into `rebuild_graph`.
pub fn augment_rebuild_with_polyglot_and_cross_edges(
    source_dir: &Path,
    manager: &CacheManager,
    all_nodes: &mut Vec<ExtractedNode>,
    all_edges: &mut Vec<ExtractedEdge>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc_files: Vec<(String, String)> = Vec::new();
    for path in walk_doc_and_schema_files(source_dir) {
        let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let module_id = path
            .strip_prefix(source_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if matches!(ext, "md" | "mdx") {
            doc_files.push((module_id.clone(), content.clone()));
        }
        let hash = blake3::hash(format!("{EXTRACTOR_VERSION}\u{0}{content}").as_bytes())
            .to_hex()
            .to_string();
        let graph = if manager.get_cached_hash(&path).as_deref() == Some(&hash) {
            manager
                .load_cache(&path)
                .unwrap_or_else(|| extract_polyglot_in_module(ext, &content, &module_id))
        } else {
            let g = extract_polyglot_in_module(ext, &content, &module_id);
            manager.write_cache(&path, &hash, &g);
            g
        };
        all_nodes.extend(graph.nodes);
        all_edges.extend(graph.edges);
    }

    if !doc_files.is_empty() {
        let cross = materialize_cross_edges(all_nodes, &doc_files);
        all_edges.extend(cross);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polyglot::{extract_go_in_module, extract_sql_in_module};

    #[test]
    fn go_and_sql_and_endpoint_extractors_emit_expected_nodes_and_edges() {
        let go_src = r#"
package worker

type JobQueue struct {}
type Scheduler interface {}

func (q *JobQueue) Enqueue() {}
func StartWorker() {
    http.route("/v1/jobs")
}
"#;
        let g = extract_go_in_module(go_src, "cmd/worker.go");
        assert!(
            g.nodes
                .iter()
                .any(|n| n.kind == "go-package" && n.label == "worker")
        );
        assert!(
            g.nodes
                .iter()
                .any(|n| n.kind == "struct" && n.label == "JobQueue")
        );
        assert!(
            g.nodes
                .iter()
                .any(|n| n.kind == "trait" && n.label == "Scheduler")
        );
        assert!(
            g.nodes
                .iter()
                .any(|n| n.kind == "method" && n.id == "cmd/worker.go::JobQueue::Enqueue")
        );
        assert!(
            g.nodes
                .iter()
                .any(|n| n.kind == "endpoint" && n.label == "/v1/jobs")
        );

        let sql_src = "CREATE TABLE users (id INT PRIMARY KEY);\nCREATE TABLE orders (user_id INT REFERENCES users(id));";
        let s = extract_sql_in_module(sql_src, "db/schema.sql");
        assert!(s.nodes.iter().any(|n| n.id == "sql:users"));
        assert!(s.edges.iter().any(|e| {
            e.source == "sql:orders" && e.target == "sql:users" && e.kind == "references_table"
        }));
    }

    #[test]
    fn materialize_cross_edges_sets_extracted_inferred_and_ambiguous_honesty() {
        let nodes = vec![
            ExtractedNode {
                id: "src/auth.rs::TokenVerifier::verify".into(),
                label: "verify".into(),
                kind: "method".into(),
            },
            ExtractedNode {
                id: "src/cache.rs::warm_cache".into(),
                label: "warm_cache".into(),
                kind: "fn".into(),
            },
            ExtractedNode {
                id: "src/a.rs::dup_sym".into(),
                label: "dup_sym".into(),
                kind: "fn".into(),
            },
            ExtractedNode {
                id: "src/b.rs::dup_sym".into(),
                label: "dup_sym".into(),
                kind: "fn".into(),
            },
        ];
        let docs = vec![(
            "docs/adr/001-auth.md".to_string(),
            "# ADR-001 Auth\ndecision_id: dec-auth\nSee [verifier](src/auth.rs#L1-L20), `warm_cache`, and `dup_sym`.\n"
                .to_string(),
        )];
        let edges = materialize_cross_edges(&nodes, &docs);
        assert!(edges.iter().any(|e| {
            e.source == "src/auth.rs::TokenVerifier::verify"
                && e.target == "doc:docs/adr/001-auth.md"
                && e.kind == "governed_by_adr"
                && e.confidence == "EXTRACTED"
        }));
        assert!(edges.iter().any(|e| {
            e.source == "src/cache.rs::warm_cache"
                && e.target == "doc:docs/adr/001-auth.md"
                && e.kind == "documented_in"
                && e.confidence == "INFERRED"
        }));
        assert!(edges.iter().any(|e| {
            e.source == "doc:docs/adr/001-auth.md"
                && e.target == "src/a.rs::dup_sym"
                && e.confidence == "AMBIGUOUS"
        }));
    }
}
