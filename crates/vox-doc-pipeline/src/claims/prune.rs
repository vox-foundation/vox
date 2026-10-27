//! Documentation prune report engine: computes inbound links, substantive edit age,
//! dead mention density, topic clusters, and recommends page dispositions.

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

use super::types::ResolvedMention;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PruneDisposition {
    Keep,
    Merge,
    MoveToInternals,
    Archive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagePruneRecord {
    pub relative_path: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub dead_mentions_count: usize,
    pub total_mentions_count: usize,
    pub inbound_links_count: usize,
    pub inbound_sources: Vec<String>,
    pub days_since_substantive_edit: u64,
    pub cluster_id: Option<String>,
    pub recommended_disposition: PruneDisposition,
    pub disposition_rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicCluster {
    pub cluster_id: String,
    pub pages: Vec<String>,
    pub common_tokens: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PruneReport {
    pub schema_version: u32,
    pub generated_at: String,
    pub total_pages_scanned: usize,
    pub count_by_disposition: HashMap<String, usize>,
    pub clusters: Vec<TopicCluster>,
    pub pages: Vec<PagePruneRecord>,
}

/// Build inbound link graph: target_rel_path -> set of source_rel_paths.
pub fn build_inbound_link_graph(
    repo_root: &Path,
    docs_dir: &Path,
) -> HashMap<String, HashSet<String>> {
    let mut graph: HashMap<String, HashSet<String>> = HashMap::new();
    let link_re = Regex::new(r"\[[^\]]*\]\(([^)]+)\)").unwrap();

    for entry in WalkDir::new(docs_dir)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "md" && ext != "mdx" {
            continue;
        }

        let source_rel = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        // Ensure entry exists
        graph.entry(source_rel.clone()).or_default();

        if let Ok(content) = fs::read_to_string(path) {
            let parent_dir = path.parent().unwrap_or(docs_dir);
            for cap in link_re.captures_iter(&content) {
                let target = &cap[1];
                if target.starts_with("http://")
                    || target.starts_with("https://")
                    || target.starts_with('#')
                    || target.starts_with("mailto:")
                {
                    continue;
                }

                let clean_target = target
                    .split('#')
                    .next()
                    .unwrap_or(target)
                    .split('?')
                    .next()
                    .unwrap_or(target);
                if clean_target.is_empty() {
                    continue;
                }

                let resolved_target = if clean_target.starts_with('/') {
                    docs_dir.join(clean_target.trim_start_matches('/'))
                } else {
                    parent_dir.join(clean_target)
                };

                let canonical = if resolved_target.is_file() {
                    Some(resolved_target)
                } else if resolved_target.with_extension("md").is_file() {
                    Some(resolved_target.with_extension("md"))
                } else if resolved_target.with_extension("mdx").is_file() {
                    Some(resolved_target.with_extension("mdx"))
                } else if resolved_target.join("index.md").is_file() {
                    Some(resolved_target.join("index.md"))
                } else {
                    None
                };

                if let Some(c) = canonical {
                    let target_rel = c
                        .strip_prefix(repo_root)
                        .unwrap_or(&c)
                        .to_string_lossy()
                        .to_string();
                    graph
                        .entry(target_rel)
                        .or_default()
                        .insert(source_rel.clone());
                }
            }
        }
    }

    graph
}

/// Extract frontmatter title, status, and category.
fn extract_frontmatter_metadata(content: &str) -> (String, String, String) {
    let mut title = "Untitled".to_string();
    let mut status = "current".to_string();
    let mut category = "General".to_string();

    if content.starts_with("---")
        && let Some(end) = content[3..].find("---")
    {
        let fm_slice = &content[3..3 + end];
        for line in fm_slice.lines() {
            let trimmed = line.trim();
            if let Some(t) = trimmed.strip_prefix("title:") {
                title = t.trim().trim_matches('"').trim_matches('\'').to_string();
            } else if let Some(s) = trimmed.strip_prefix("status:") {
                status = s.trim().trim_matches('"').trim_matches('\'').to_string();
            } else if let Some(c) = trimmed.strip_prefix("category:") {
                category = c.trim().trim_matches('"').trim_matches('\'').to_string();
            }
        }
    }

    (title, status, category)
}

/// Fetch last substantive commit date per doc file using git log.
pub fn get_substantive_git_dates(repo_root: &Path) -> HashMap<String, u64> {
    let mut dates = HashMap::new();

    // // vox-arch-check: allow git-exec
    // Rationale: Required to batch-fetch commit dates to compute days_since_substantive_edit for documentation freshness.
    let output = Command::new("git")
        .current_dir(repo_root)
        .args([
            "log",
            "--pretty=format:COMMIT:%ct%x09%s",
            "--name-only",
            "--no-renames",
            "--",
            "docs/src",
        ])
        .output();

    if let Ok(out) = output
        && out.status.success()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut current_ts: Option<u64> = None;
        let mut is_substantive = false;

        for line in stdout.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("COMMIT:") {
                let mut parts = rest.split('\t');
                let ts_str = parts.next().unwrap_or("0");
                let subject = parts.next().unwrap_or("").to_lowercase();

                is_substantive = !subject.contains("chore(docs): regenerate")
                    && !subject.contains("ssot-autoregen")
                    && !subject.contains("style: fmt")
                    && !subject.contains("ci: sync")
                    && !subject.contains("fix(#");

                current_ts = ts_str.parse::<u64>().ok();
            } else if !trimmed.is_empty()
                && is_substantive
                && let Some(ts) = current_ts
            {
                dates.entry(trimmed.to_string()).or_insert(ts);
            }
        }
    }

    dates
}

/// Tokenize a title or slug for clustering.
fn tokenize(text: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();
    let re = Regex::new(r"[a-z0-9]+").unwrap();
    let stop_words: HashSet<&str> = [
        "a", "an", "the", "in", "on", "and", "or", "for", "with", "to", "of", "vox", "guide",
        "overview", "plan",
    ]
    .into_iter()
    .collect();

    for mat in re.find_iter(&text.to_lowercase()) {
        let w = mat.as_str();
        if w.len() >= 3 && !stop_words.contains(w) {
            tokens.insert(w.to_string());
        }
    }
    tokens
}

/// Jaccard similarity between two token sets.
fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Find duplicate or overlapping topic clusters.
pub fn find_topic_clusters(pages: &[(String, String)]) -> Vec<TopicCluster> {
    let mut token_sets: Vec<(String, HashSet<String>)> = Vec::new();
    for (path, title) in pages {
        let slug = Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let mut tokens = tokenize(title);
        tokens.extend(tokenize(slug));
        if !tokens.is_empty() {
            token_sets.push((path.clone(), tokens));
        }
    }

    let mut clusters: Vec<TopicCluster> = Vec::new();
    let mut clustered_paths = HashSet::new();

    for i in 0..token_sets.len() {
        let (path_a, tokens_a) = &token_sets[i];
        if clustered_paths.contains(path_a) {
            continue;
        }

        let mut cluster_members = vec![path_a.clone()];
        for (path_b, tokens_b) in &token_sets[(i + 1)..] {
            if clustered_paths.contains(path_b) {
                continue;
            }

            if jaccard_similarity(tokens_a, tokens_b) >= 0.6 {
                cluster_members.push(path_b.clone());
            }
        }

        if cluster_members.len() > 1 {
            for m in &cluster_members {
                clustered_paths.insert(m.clone());
            }
            let common_tokens: Vec<String> = tokens_a.iter().take(5).cloned().collect();
            clusters.push(TopicCluster {
                cluster_id: format!("cluster-{:03}", clusters.len() + 1),
                pages: cluster_members,
                common_tokens,
            });
        }
    }

    clusters
}

/// Generate a full prune report across the repository documentation corpus.
pub fn generate_prune_report(
    repo_root: &Path,
    resolved_mentions: &[ResolvedMention],
) -> Result<PruneReport> {
    let docs_dir = repo_root.join("docs/src");
    let link_graph = build_inbound_link_graph(repo_root, &docs_dir);
    let git_dates = get_substantive_git_dates(repo_root);

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Map resolved mentions by source file
    let mut dead_by_file: HashMap<String, usize> = HashMap::new();
    let mut total_by_file: HashMap<String, usize> = HashMap::new();

    for m in resolved_mentions {
        let file = &m.extracted.source_file;
        *total_by_file.entry(file.clone()).or_insert(0) += 1;
        if m.is_actionable_deficit() {
            *dead_by_file.entry(file.clone()).or_insert(0) += 1;
        }
    }

    // Gather live pages
    let mut page_metas = Vec::new();
    for entry in WalkDir::new(&docs_dir)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "md" && ext != "mdx" {
            continue;
        }

        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        if let Ok(content) = fs::read_to_string(path) {
            let (title, status, category) = extract_frontmatter_metadata(&content);
            page_metas.push((rel, title, status, category));
        }
    }

    // Clusters
    let title_pairs: Vec<(String, String)> = page_metas
        .iter()
        .map(|(p, t, _, _)| (p.clone(), t.clone()))
        .collect();
    let clusters = find_topic_clusters(&title_pairs);

    let mut path_to_cluster: HashMap<String, String> = HashMap::new();
    for c in &clusters {
        for p in &c.pages {
            path_to_cluster.insert(p.clone(), c.cluster_id.clone());
        }
    }

    let mut records = Vec::new();
    let mut disposition_counts: HashMap<String, usize> = HashMap::new();

    for (rel, title, status, category) in page_metas {
        let dead_count = dead_by_file.get(&rel).copied().unwrap_or(0);
        let total_count = total_by_file.get(&rel).copied().unwrap_or(0);
        let inbound_sources: Vec<String> = link_graph
            .get(&rel)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default();
        let inbound_count = inbound_sources.len();

        let commit_ts = git_dates.get(&rel).copied().unwrap_or(now_ts);
        let age_days = (now_ts.saturating_sub(commit_ts)) / 86400;

        let cluster_id = path_to_cluster.get(&rel).cloned();

        // Disposition rules
        let (disposition, rationale) = if rel.starts_with("docs/src/archive/") {
            (
                PruneDisposition::Archive,
                "Already located under docs/src/archive/".to_string(),
            )
        } else if status == "research" || status == "roadmap" {
            (
                PruneDisposition::MoveToInternals,
                format!(
                    "Frontmatter status is '{}', grouped into Internals sidebar per MEASURE-06",
                    status
                ),
            )
        } else if category.contains("ADR")
            || category.contains("Architecture Decisions")
            || rel.starts_with("docs/src/adr/")
        {
            (
                PruneDisposition::MoveToInternals,
                "ADR / architecture decision record belongs in Internals group".to_string(),
            )
        } else if status == "deprecated" || status == "legacy" {
            (
                PruneDisposition::Archive,
                format!("Frontmatter status is '{}'", status),
            )
        } else if inbound_count == 0 && (dead_count >= 3 || age_days > 180) {
            (
                PruneDisposition::Archive,
                format!(
                    "Orphaned page (0 inbound links) with {} dead mentions, age {} days",
                    dead_count, age_days
                ),
            )
        } else if let Some(ref cid) = cluster_id {
            (
                PruneDisposition::Merge,
                format!("Member of topic cluster {} with overlapping content", cid),
            )
        } else {
            (
                PruneDisposition::Keep,
                "Active public reader documentation with valid references".to_string(),
            )
        };

        let disp_str = format!("{:?}", disposition).to_lowercase();
        *disposition_counts.entry(disp_str).or_insert(0) += 1;

        records.push(PagePruneRecord {
            relative_path: rel,
            title,
            category,
            status,
            dead_mentions_count: dead_count,
            total_mentions_count: total_count,
            inbound_links_count: inbound_count,
            inbound_sources,
            days_since_substantive_edit: age_days,
            cluster_id,
            recommended_disposition: disposition,
            disposition_rationale: rationale,
        });
    }

    Ok(PruneReport {
        schema_version: 1,
        generated_at: "2026-10-13T00:00:00Z".to_string(),
        total_pages_scanned: records.len(),
        count_by_disposition: disposition_counts,
        clusters,
        pages: records,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_and_tokenization() {
        let a = tokenize("Architecture Overview and Design");
        let b = tokenize("Architecture Design and Structure");
        assert!(jaccard_similarity(&a, &b) >= 0.5);
    }
}
