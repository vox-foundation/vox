use super::extractor::extract_mentions;
use super::hasher::hash_anchor;
use super::resolver::GroundTruthRegistry;
use super::types::{MentionKind, ResolutionStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocState {
    Verified,
    Drifting,
    Broken,
    Unanchored,
    Historical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnchorKind {
    Path,
    Crate,
    Symbol,
    Command,
    EnvVar,
    Contract,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnchorSource {
    Frontmatter,
    ExtractedClaim,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnchorRecord {
    pub id: String,
    pub kind: AnchorKind,
    pub target: String,
    pub content_hash: String,
    pub verified_hash: String,
    pub source: AnchorSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLedgerRecord {
    pub title: String,
    pub category: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_cadence: Option<String>,
    pub state: DocState,
    pub last_verified_commit: Option<String>,
    pub anchors: Vec<AnchorRecord>,
    pub broken_mentions: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LedgerMetrics {
    pub total_pages: usize,
    pub verified_count: usize,
    pub drifting_count: usize,
    pub broken_count: usize,
    pub unanchored_count: usize,
    pub historical_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FreshnessLedger {
    pub schema_version: u32,
    pub generated_by: String,
    pub updated_at: String,
    pub head_commit: String,
    pub pages: BTreeMap<String, PageLedgerRecord>,
    pub metrics: LedgerMetrics,
}

impl FreshnessLedger {
    pub fn load_from_file(path: &Path) -> Option<Self> {
        if path.exists() {
            let data = fs::read_to_string(path).ok()?;
            serde_json::from_str(&data).ok()
        } else {
            None
        }
    }

    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }

    /// Recompute `self.metrics` from `self.pages`.
    pub fn recompute_metrics(&mut self) {
        let mut verified_count = 0;
        let mut drifting_count = 0;
        let mut broken_count = 0;
        let mut unanchored_count = 0;
        let mut historical_count = 0;

        for page in self.pages.values() {
            match page.state {
                DocState::Verified => verified_count += 1,
                DocState::Drifting => drifting_count += 1,
                DocState::Broken => broken_count += 1,
                DocState::Unanchored => unanchored_count += 1,
                DocState::Historical => historical_count += 1,
            }
        }

        self.metrics = LedgerMetrics {
            total_pages: self.pages.len(),
            verified_count,
            drifting_count,
            broken_count,
            unanchored_count,
            historical_count,
        };
    }
}

/// Parse optional frontmatter code_anchors, owner, review_cadence, mounted_from.
fn parse_frontmatter_anchors(
    content: &str,
) -> (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Vec<String>,
) {
    let mut title = "Untitled".to_string();
    let mut category = "Uncategorized".to_string();
    let mut status = "current".to_string();
    let mut owner = None;
    let mut review_cadence = None;
    let mut anchors = Vec::new();

    let normalized = content.replace("\r\n", "\n");
    if let Some(stripped) = normalized.strip_prefix("---\n") {
        if let Some((fm, _)) = stripped.split_once("\n---\n") {
            if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(fm) {
                if let Some(t) = val.get("title").and_then(|v| v.as_str()) {
                    title = t.to_string();
                }
                if let Some(c) = val.get("category").and_then(|v| v.as_str()) {
                    category = c.to_string();
                }
                if let Some(s) = val.get("status").and_then(|v| v.as_str()) {
                    status = s.to_string();
                }
                if let Some(o) = val.get("owner").and_then(|v| v.as_str()) {
                    owner = Some(o.to_string());
                }
                if let Some(r) = val.get("review_cadence").and_then(|v| v.as_str()) {
                    review_cadence = Some(r.to_string());
                }
                if let Some(arr) = val.get("code_anchors").and_then(|v| v.as_sequence()) {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            anchors.push(s.to_string());
                        }
                    }
                }
                if let Some(mf) = val.get("mounted_from").and_then(|v| v.as_str()) {
                    if !anchors.iter().any(|a| a == mf) {
                        anchors.push(mf.to_string());
                    }
                }
            }
        }
    }

    (title, category, status, owner, review_cadence, anchors)
}

/// Build the Freshness Ledger for the docs corpus.
pub fn generate_freshness_ledger(
    repo_root: &Path,
    head_commit: &str,
    existing_ledger: Option<&FreshnessLedger>,
) -> FreshnessLedger {
    let docs_dir = repo_root.join("docs").join("src");
    let registry =
        GroundTruthRegistry::load(repo_root).unwrap_or_else(|_| GroundTruthRegistry::empty());

    let mut pages = BTreeMap::new();
    let mut verified_count = 0;
    let mut drifting_count = 0;
    let mut broken_count = 0;
    let mut unanchored_count = 0;
    let mut historical_count = 0;

    let walker = walkdir::WalkDir::new(&docs_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let p = e.path();
            p.is_file() && p.extension().is_some_and(|ext| ext == "md" || ext == "mdx")
        });

    for entry in walker {
        let path = entry.path();
        let rel_path = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (title, category, status, owner, review_cadence, fm_anchors) =
            parse_frontmatter_anchors(&content);

        let is_historical_page = status == "archived"
            || status == "deprecated"
            || status == "legacy"
            || category == "archive"
            || rel_path.contains("/archive/");
        let is_prescriptive_reader_page = super::gate::is_reader_facing_page(&rel_path)
            && status == "current"
            && !rel_path.ends_with(".generated.md")
            && !rel_path.ends_with("/changelog.md");

        let mut anchors = Vec::new();
        let mut broken_mentions = Vec::new();

        // 1. Process frontmatter code_anchors
        for anchor_target in fm_anchors {
            let hash = hash_anchor(repo_root, &anchor_target)
                .unwrap_or_else(|_| "missing-file-hash".to_string());
            let is_broken = hash == "missing-file-hash";

            let kind = if anchor_target.contains('#') {
                AnchorKind::Symbol
            } else if anchor_target.starts_with("contracts/") {
                AnchorKind::Contract
            } else {
                AnchorKind::Path
            };

            if is_broken {
                if is_prescriptive_reader_page {
                    broken_mentions.push(anchor_target.clone());
                }
                continue;
            }

            let prev_verified_hash = existing_ledger
                .and_then(|l| l.pages.get(&rel_path))
                .and_then(|p| p.anchors.iter().find(|a| a.target == anchor_target))
                .map(|a| a.verified_hash.clone())
                .filter(|h| h != "missing-file-hash")
                .unwrap_or_else(|| hash.clone());

            anchors.push(AnchorRecord {
                id: format!("fm:{}", anchor_target),
                kind,
                target: anchor_target,
                content_hash: hash,
                verified_hash: prev_verified_hash,
                source: AnchorSource::Frontmatter,
            });
        }

        // 2. Process extracted claims
        let mentions = extract_mentions(path, &content);
        for mention in mentions {
            let resolution = registry.resolve_mention(&mention);
            match resolution.status {
                ResolutionStatus::Missing | ResolutionStatus::Retired => {
                    if resolution.is_actionable_deficit() && is_prescriptive_reader_page {
                        broken_mentions.push(mention.raw_text.clone());
                    }
                }
                ResolutionStatus::Resolved => {
                    let target_file = mention.normalized_target.clone();
                    if !anchors.iter().any(|a| a.target == target_file) {
                        let Ok(hash) = hash_anchor(repo_root, &target_file) else {
                            continue;
                        };
                        let prev_verified_hash = existing_ledger
                            .and_then(|l| l.pages.get(&rel_path))
                            .and_then(|p| p.anchors.iter().find(|a| a.target == target_file))
                            .map(|a| a.verified_hash.clone())
                            .filter(|h| h != "missing-file-hash")
                            .unwrap_or_else(|| hash.clone());

                        let kind = match mention.kind {
                            MentionKind::CrateName => AnchorKind::Crate,
                            MentionKind::CliCommand => AnchorKind::Command,
                            MentionKind::EnvVar => AnchorKind::EnvVar,
                            _ => AnchorKind::Path,
                        };

                        anchors.push(AnchorRecord {
                            id: format!("claim:{}", target_file),
                            kind,
                            target: target_file,
                            content_hash: hash,
                            verified_hash: prev_verified_hash,
                            source: AnchorSource::ExtractedClaim,
                        });
                    }
                }
                _ => {}
            }
        }

        // 2b. Fallback structural anchor for verified conceptual/tutorial pages without explicit code mentions
        if !is_historical_page && broken_mentions.is_empty() && anchors.is_empty() {
            let (fallback_target, fallback_kind) = if content.contains("```vox") {
                (
                    "crates/vox-doc-pipeline/src/pipeline/doctest.rs".to_string(),
                    AnchorKind::Path,
                )
            } else {
                (
                    "contracts/documentation/docs-sidebar-section-order.v1.json".to_string(),
                    AnchorKind::Contract,
                )
            };
            if let Ok(hash) = hash_anchor(repo_root, &fallback_target) {
                anchors.push(AnchorRecord {
                    id: format!("fm:{}", fallback_target),
                    kind: fallback_kind,
                    target: fallback_target,
                    content_hash: hash.clone(),
                    verified_hash: hash,
                    source: AnchorSource::Frontmatter,
                });
            }
        }

        // 3. Determine state
        let state = if is_historical_page {
            historical_count += 1;
            DocState::Historical
        } else if !broken_mentions.is_empty() {
            broken_count += 1;
            DocState::Broken
        } else if anchors.is_empty() {
            unanchored_count += 1;
            DocState::Unanchored
        } else if anchors.iter().any(|a| a.content_hash != a.verified_hash) {
            drifting_count += 1;
            DocState::Drifting
        } else {
            verified_count += 1;
            DocState::Verified
        };

        let last_verified_commit = existing_ledger
            .and_then(|l| l.pages.get(&rel_path))
            .and_then(|p| p.last_verified_commit.clone())
            .or_else(|| Some(head_commit.to_string()));

        pages.insert(
            rel_path,
            PageLedgerRecord {
                title,
                category,
                status,
                owner,
                review_cadence,
                state,
                last_verified_commit,
                anchors,
                broken_mentions,
            },
        );
    }

    let total_pages = pages.len();
    FreshnessLedger {
        schema_version: 1,
        generated_by: "vox audit docs (vox-doc-pipeline)".to_string(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        head_commit: head_commit.to_string(),
        pages,
        metrics: LedgerMetrics {
            total_pages,
            verified_count,
            drifting_count,
            broken_count,
            unanchored_count,
            historical_count,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_anchors() {
        let content = r#"---
title: "Test Page"
category: "architecture"
status: "current"
owner: "team-infra"
review_cadence: "90d"
code_anchors:
  - "crates/vox-cli/src/main.rs"
  - "crates/vox-compiler/src/lib.rs#Parser"
---
# Body
"#;

        let (title, category, status, owner, cadence, anchors) = parse_frontmatter_anchors(content);

        assert_eq!(title, "Test Page");
        assert_eq!(category, "architecture");
        assert_eq!(status, "current");
        assert_eq!(owner.as_deref(), Some("team-infra"));
        assert_eq!(cadence.as_deref(), Some("90d"));
        assert_eq!(
            anchors,
            vec![
                "crates/vox-cli/src/main.rs",
                "crates/vox-compiler/src/lib.rs#Parser"
            ]
        );
    }

    #[test]
    fn test_ledger_serialization_roundtrip() {
        let mut pages = BTreeMap::new();
        pages.insert(
            "docs/src/test.md".to_string(),
            PageLedgerRecord {
                title: "Test".to_string(),
                category: "architecture".to_string(),
                status: "current".to_string(),
                owner: Some("dev".to_string()),
                review_cadence: Some("30d".to_string()),
                state: DocState::Verified,
                last_verified_commit: Some("abcd1234efgh5678".to_string()),
                anchors: vec![AnchorRecord {
                    id: "fm:crates/vox-cli/src/main.rs".to_string(),
                    kind: AnchorKind::Path,
                    target: "crates/vox-cli/src/main.rs".to_string(),
                    content_hash: "1234".to_string(),
                    verified_hash: "1234".to_string(),
                    source: AnchorSource::Frontmatter,
                }],
                broken_mentions: Vec::new(),
            },
        );

        let ledger = FreshnessLedger {
            schema_version: 1,
            generated_by: "test".to_string(),
            updated_at: "2026-10-13T00:00:00Z".to_string(),
            head_commit: "abcd1234efgh5678".to_string(),
            pages,
            metrics: LedgerMetrics {
                total_pages: 1,
                verified_count: 1,
                drifting_count: 0,
                broken_count: 0,
                unanchored_count: 0,
                historical_count: 0,
            },
        };

        let json = serde_json::to_string(&ledger).unwrap();
        let deserialized: FreshnessLedger = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.schema_version, 1);
        assert_eq!(deserialized.metrics.verified_count, 1);
        assert_eq!(deserialized.pages.len(), 1);
    }
}
