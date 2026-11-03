//! Reverse code-to-docs index and PR impact report generator (DRIFT-01).

use super::gate::is_reader_facing_page;
use super::ledger::FreshnessLedger;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PR_IMPACT_COMMENT_MARKER: &str = "<!-- vox-docs-freshness-pr-impact -->";

/// Reference to a reader-facing documentation page affected by code changes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocPageReference {
    pub page_path: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub matched_anchors: Vec<String>,
}

/// PR impact report detailing which documentation pages reference changed code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrImpactReport {
    pub changed_files: Vec<String>,
    pub affected_pages: Vec<DocPageReference>,
    pub markdown_comment: String,
}

/// Reverse index mapping code entities (files, crates, commands, symbols) to documentation pages.
#[derive(Debug, Default)]
pub struct ReverseIndex {
    /// Exact file or symbol target -> set of doc page paths
    pub target_to_pages: BTreeMap<String, BTreeSet<String>>,
    /// Crate name -> set of doc page paths
    pub crate_to_pages: BTreeMap<String, BTreeSet<String>>,
}

impl ReverseIndex {
    /// Build reverse index from an existing Freshness Ledger.
    pub fn build(ledger: &FreshnessLedger) -> Self {
        let mut index = Self::default();

        for (page_path, page_rec) in &ledger.pages {
            // Only index reader-facing pages with status: current
            if !is_reader_facing_page(page_path) || page_rec.status != "current" {
                continue;
            }

            for anchor in &page_rec.anchors {
                let target = anchor.target.trim().replace('\\', "/");
                if target.is_empty() {
                    continue;
                }

                // Check crate references
                if target.starts_with("crates/") {
                    let parts: Vec<&str> = target.split('/').collect();
                    if parts.len() >= 2 {
                        let crate_name = parts[1];
                        index
                            .crate_to_pages
                            .entry(crate_name.to_string())
                            .or_default()
                            .insert(page_path.clone());
                    }
                }

                index
                    .target_to_pages
                    .entry(target)
                    .or_default()
                    .insert(page_path.clone());
            }

            // Also index broken mentions targets
            for claim in &page_rec.broken_mentions {
                let target = claim.trim().replace('\\', "/");
                if !target.is_empty() {
                    index
                        .target_to_pages
                        .entry(target)
                        .or_default()
                        .insert(page_path.clone());
                }
            }
        }

        index
    }

    /// Query the reverse index for documentation pages affected by the given changed files.
    pub fn query_impact(
        &self,
        changed_files: &[String],
        ledger: &FreshnessLedger,
    ) -> PrImpactReport {
        let mut affected_map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for file in changed_files {
            let norm_file = file.trim().replace('\\', "/");

            // 1. Exact match on target
            if let Some(pages) = self.target_to_pages.get(&norm_file) {
                for page in pages {
                    affected_map
                        .entry(page.clone())
                        .or_default()
                        .insert(norm_file.clone());
                }
            }

            // 2. Check if file is inside an anchored crate
            if norm_file.starts_with("crates/") {
                let parts: Vec<&str> = norm_file.split('/').collect();
                if parts.len() >= 2 {
                    let crate_name = parts[1];
                    if let Some(pages) = self.crate_to_pages.get(crate_name) {
                        for page in pages {
                            affected_map
                                .entry(page.clone())
                                .or_default()
                                .insert(format!("crate:{}", crate_name));
                        }
                    }
                }
            }

            // 3. Command registry changes affect any page anchoring CLI commands
            if norm_file == "contracts/cli/command-registry.yaml" {
                for (target, pages) in &self.target_to_pages {
                    if target.starts_with("vox ") || target.contains("--") {
                        for page in pages {
                            affected_map
                                .entry(page.clone())
                                .or_default()
                                .insert("contracts/cli/command-registry.yaml".to_string());
                        }
                    }
                }
            }

            // 4. Env vars registry changes affect pages referencing env vars
            if norm_file == "contracts/config/env-vars.v1.yaml" {
                for (target, pages) in &self.target_to_pages {
                    if target.starts_with("VOX_") {
                        for page in pages {
                            affected_map
                                .entry(page.clone())
                                .or_default()
                                .insert("contracts/config/env-vars.v1.yaml".to_string());
                        }
                    }
                }
            }
        }

        let mut affected_pages = Vec::new();
        for (page_path, matched) in affected_map {
            if let Some(page_rec) = ledger.pages.get(&page_path) {
                affected_pages.push(DocPageReference {
                    page_path: page_path.clone(),
                    title: page_rec.title.clone(),
                    category: page_rec.category.clone(),
                    status: page_rec.status.clone(),
                    matched_anchors: matched.into_iter().collect(),
                });
            }
        }

        // Sort by page path for determinism
        affected_pages.sort_by(|a, b| a.page_path.cmp(&b.page_path));

        let markdown_comment = format_markdown_comment(changed_files, &affected_pages);

        PrImpactReport {
            changed_files: changed_files.to_vec(),
            affected_pages,
            markdown_comment,
        }
    }
}

/// Format the PR impact comment with idempotency marker.
fn format_markdown_comment(
    _changed_files: &[String],
    affected_pages: &[DocPageReference],
) -> String {
    let mut out = String::new();
    out.push_str(PR_IMPACT_COMMENT_MARKER);
    out.push_str("\n## 📚 Documentation Impact Report\n\n");

    if affected_pages.is_empty() {
        out.push_str(
            "No reader-facing documentation pages are affected by the code changes in this PR.\n",
        );
        return out;
    }

    out.push_str(&format!(
        "This pull request modifies files that are referenced by **{}** reader-facing documentation page(s):\n\n",
        affected_pages.len()
    ));

    out.push_str("| Documentation Page | Category | Referenced In PR |\n");
    out.push_str("| :--- | :--- | :--- |\n");

    for p in affected_pages {
        let refs = p.matched_anchors.join(", ");
        out.push_str(&format!(
            "| [`{}`]({}) | {} | `{}` |\n",
            p.title, p.page_path, p.category, refs
        ));
    }

    out.push_str("\n### 🔍 Re-attestation Instructions\n\n");
    out.push_str("If your changes update the behavior described on these pages, verify their accuracy and re-attest:\n");
    out.push_str("```bash\n");
    for p in affected_pages.iter().take(3) {
        out.push_str(&format!("vox audit docs --attest {}\n", p.page_path));
    }
    if affected_pages.len() > 3 {
        out.push_str("# ... (see above table for remaining pages)\n");
    }
    out.push_str("```\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claims::ledger::{
        AnchorKind, AnchorRecord, AnchorSource, DocState, PageLedgerRecord,
    };

    #[test]
    fn test_reverse_index_query_impact() {
        let mut ledger = FreshnessLedger::default();

        let page = PageLedgerRecord {
            title: "Quickstart".to_string(),
            category: "tutorials".to_string(),
            status: "current".to_string(),
            owner: None,
            review_cadence: None,
            state: DocState::Verified,
            last_verified_commit: None,
            anchors: vec![
                AnchorRecord {
                    id: "anchor:crates/vox-cli/src/main.rs".to_string(),
                    kind: AnchorKind::Path,
                    target: "crates/vox-cli/src/main.rs".to_string(),
                    content_hash: "hash1".to_string(),
                    verified_hash: "hash1".to_string(),
                    source: AnchorSource::Frontmatter,
                },
                AnchorRecord {
                    id: "anchor:vox run".to_string(),
                    kind: AnchorKind::Command,
                    target: "vox run".to_string(),
                    content_hash: "hash2".to_string(),
                    verified_hash: "hash2".to_string(),
                    source: AnchorSource::ExtractedClaim,
                },
            ],
            broken_mentions: Vec::new(),
        };

        ledger
            .pages
            .insert("docs/src/tutorials/quickstart.md".to_string(), page);

        let index = ReverseIndex::build(&ledger);

        // 1. Matching exact file
        let report = index.query_impact(&["crates/vox-cli/src/main.rs".to_string()], &ledger);
        assert_eq!(report.affected_pages.len(), 1);
        assert_eq!(
            report.affected_pages[0].page_path,
            "docs/src/tutorials/quickstart.md"
        );
        assert!(report.markdown_comment.contains(PR_IMPACT_COMMENT_MARKER));
        assert!(report.markdown_comment.contains("Quickstart"));

        // 2. Unrelated file
        let report_unrelated =
            index.query_impact(&["crates/vox-unrelated/foo.rs".to_string()], &ledger);
        assert_eq!(report_unrelated.affected_pages.len(), 0);
        assert!(
            report_unrelated
                .markdown_comment
                .contains("No reader-facing documentation pages")
        );

        // 3. Command registry file
        let report_cmd = index.query_impact(
            &["contracts/cli/command-registry.yaml".to_string()],
            &ledger,
        );
        assert_eq!(report_cmd.affected_pages.len(), 1);
    }
}
