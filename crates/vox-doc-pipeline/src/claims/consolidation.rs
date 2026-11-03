//! Phase 34 (`DEBT-01..03`): Docs Corpus Duplicate Cluster Consolidation,
//! Starlight 301 Redirect Synthesis, and Shrink-Only KB Facet Backlog Ratchet.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A duplicate documentation cluster from `prune-report.v1.json` (`DEBT-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DuplicateCluster {
    pub cluster_id: String,
    pub paths: Vec<String>,
}

/// Consolidation action for a duplicate cluster (`DEBT-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterConsolidationPlan {
    pub cluster_id: String,
    pub canonical_path: String,
    pub retired_paths: Vec<String>,
    pub redirects: Vec<(String, String, u16)>,
}

fn doc_path_to_public_url(doc_path: &str) -> String {
    let rel = doc_path
        .strip_prefix("docs/src/")
        .unwrap_or(doc_path)
        .trim_end_matches(".mdx")
        .trim_end_matches(".md")
        .trim_end_matches(".generated");
    if rel == "index" {
        "/".to_string()
    } else if let Some(prefix) = rel.strip_suffix("/index") {
        format!("/{prefix}/")
    } else {
        format!("/{rel}/")
    }
}

fn canonical_rank(path: &str) -> (u8, usize) {
    // Lower tuple wins: prefer `.generated.md`, then `*-ssot*.md`, then shorter path.
    let tier = if path.ends_with(".generated.md") {
        0
    } else if path.contains("-ssot") {
        1
    } else if path.starts_with("docs/src/reference/") || path.starts_with("docs/src/architecture/")
    {
        2
    } else {
        3
    };
    (tier, path.len())
}

/// Select the canonical SSOT page for each duplicate cluster and synthesize 301 redirects (`DEBT-01`).
pub fn plan_duplicate_cluster_consolidation(
    clusters: &[DuplicateCluster],
) -> Vec<ClusterConsolidationPlan> {
    let mut plans = Vec::with_capacity(clusters.len());
    for cluster in clusters {
        if cluster.paths.len() < 2 {
            continue;
        }
        let mut sorted = cluster.paths.clone();
        sorted.sort_by(|a, b| {
            canonical_rank(a)
                .cmp(&canonical_rank(b))
                .then_with(|| a.cmp(b))
        });
        let canonical = sorted[0].clone();
        let canonical_url = doc_path_to_public_url(&canonical);
        let retired: Vec<String> = sorted.into_iter().skip(1).collect();
        let redirects = retired
            .iter()
            .filter_map(|ret| {
                let from_url = doc_path_to_public_url(ret);
                if from_url != canonical_url {
                    Some((from_url, canonical_url.clone(), 301u16))
                } else {
                    None
                }
            })
            .collect();
        plans.push(ClusterConsolidationPlan {
            cluster_id: cluster.cluster_id.clone(),
            canonical_path: canonical,
            retired_paths: retired,
            redirects,
        });
    }
    plans
}

/// Render Cloudflare Pages / Starlight `_redirects` lines from consolidation plans (`DEBT-01`).
pub fn render_redirects_block(plans: &[ClusterConsolidationPlan]) -> String {
    let mut lines = Vec::new();
    for plan in plans {
        for (from, to, code) in &plan.redirects {
            lines.push(format!("{from} {to} {code}"));
        }
    }
    lines.sort();
    lines.dedup();
    lines.join("\n")
}

/// Shrink-only ratchet for KB facet backlog paths (`DEBT-03`):
/// Removes `graduated_or_retired_paths` from `current_backlog_paths` and enforces
/// that no new paths are ever added.
pub fn ratchet_kb_facet_backlog(
    current_backlog_paths: &[String],
    proposed_backlog_paths: &[String],
) -> Result<usize, String> {
    let current_set: BTreeSet<&str> = current_backlog_paths.iter().map(String::as_str).collect();
    let proposed_set: BTreeSet<&str> = proposed_backlog_paths.iter().map(String::as_str).collect();

    let added: Vec<&str> = proposed_set.difference(&current_set).copied().collect();
    if !added.is_empty() {
        return Err(format!(
            "kb-facet-backlog ratchet violation: {} new path(s) added (first: {})",
            added.len(),
            added[0]
        ));
    }
    Ok(current_set.len().saturating_sub(proposed_set.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_cluster_picks_generated_or_ssot_and_emits_301_redirects() {
        let clusters = vec![DuplicateCluster {
            cluster_id: "env-vars".into(),
            paths: vec![
                "docs/src/ref/env-vars.md".into(),
                "docs/src/reference/env-vars.generated.md".into(),
            ],
        }];
        let plans = plan_duplicate_cluster_consolidation(&clusters);
        assert_eq!(plans.len(), 1);
        assert_eq!(
            plans[0].canonical_path,
            "docs/src/reference/env-vars.generated.md"
        );
        let redirects = render_redirects_block(&plans);
        assert_eq!(redirects, "/ref/env-vars/ /reference/env-vars/ 301");
    }

    #[test]
    fn facet_backlog_ratchet_only_shrinks() {
        let cur = vec!["a.md".to_string(), "b.md".to_string(), "c.md".to_string()];
        let shrunk = vec!["a.md".to_string()];
        assert_eq!(ratchet_kb_facet_backlog(&cur, &shrunk).unwrap(), 2);

        let expanded = vec!["a.md".to_string(), "d.md".to_string()];
        assert!(ratchet_kb_facet_backlog(&cur, &expanded).is_err());
    }
}
