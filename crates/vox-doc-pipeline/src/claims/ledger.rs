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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
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
    /// Compare semantic ledger content while ignoring run-specific `updated_at` and `head_commit`.
    ///
    /// Scheduled refreshes run on every commit to `main`; ignoring those two metadata fields keeps
    /// ledger files byte-identical when no tracked documentation or anchored code has changed.
    pub fn same_content(&self, other: &FreshnessLedger) -> bool {
        self.schema_version == other.schema_version
            && self.generated_by == other.generated_by
            && self.pages == other.pages
            && self.metrics == other.metrics
    }

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
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
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

/// Write the freshness ledger to `path` only if it does not already match in semantic content.
///
/// Returns `Ok(true)` when a write occurred and `Ok(false)` when the on-disk ledger had
/// identical content (`same_content`).
pub fn write_ledger_if_changed(path: &Path, ledger: &FreshnessLedger) -> std::io::Result<bool> {
    if let Some(existing) = FreshnessLedger::load_from_file(path)
        && existing.same_content(ledger)
    {
        return Ok(false);
    }
    ledger.save_to_file(path)?;
    Ok(true)
}

/// Parse optional frontmatter code_anchors, owner, review_cadence, mounted_from.
/// Reports this pipeline itself writes (ledger, findings, metrics, corpus ratchet).
/// They change on every run, so anchoring a page to one re-drifts it right after it
/// is attested (the governance page names the ledger path and could never verify).
fn is_pipeline_output(target: &str) -> bool {
    target.starts_with("contracts/reports/docs-freshness/")
}

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
    if let Some(stripped) = normalized.strip_prefix("---\n")
        && let Some((fm, _)) = stripped.split_once("\n---\n")
        && let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(fm)
    {
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
        if let Some(mf) = val.get("mounted_from").and_then(|v| v.as_str())
            && !anchors.iter().any(|a| a == mf)
        {
            anchors.push(mf.to_string());
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

        let is_historical_page = matches!(
            status.as_str(),
            "archived" | "deprecated" | "legacy" | "superseded" | "research" | "roadmap"
        ) || category == "archive"
            || rel_path.contains("/archive/");
        let is_active_current_page = !is_historical_page
            && status == "current"
            && super::gate::is_active_corpus_page(&rel_path);

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
                if is_active_current_page {
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
                    if resolution.is_actionable_deficit() && is_active_current_page {
                        broken_mentions.push(mention.raw_text.clone());
                    }
                }
                ResolutionStatus::Resolved => {
                    let target_file = mention.normalized_target.clone();
                    if is_pipeline_output(&target_file) {
                        continue;
                    }
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
    fn pipeline_outputs_are_never_anchors() {
        // The ledger and its sibling reports are rewritten by every audit run, so a
        // page anchored to one would drift again right after being attested.
        assert!(is_pipeline_output(
            "contracts/reports/docs-freshness/ledger.v1.json"
        ));
        assert!(is_pipeline_output(
            "contracts/reports/docs-freshness/corpus-ratchet.v1.json"
        ));
        assert!(!is_pipeline_output(
            "contracts/reports/gui-surface-registry.v1.json"
        ));
        assert!(!is_pipeline_output(
            "crates/vox-doc-pipeline/src/claims/ledger.rs"
        ));
    }

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

    struct ScopedTempDir(std::path::PathBuf);

    impl ScopedTempDir {
        fn new(name: &str) -> Self {
            let unique = format!(
                "vox-ledger-test-{}-{}-{}",
                name,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            );
            let path = std::env::temp_dir().join(unique);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for ScopedTempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sample_ledger(
        updated_at: &str,
        head_commit: &str,
        state: DocState,
        hash: &str,
    ) -> FreshnessLedger {
        let mut pages = BTreeMap::new();
        pages.insert(
            "docs/src/sample.md".to_string(),
            PageLedgerRecord {
                title: "Sample".to_string(),
                category: "architecture".to_string(),
                status: "current".to_string(),
                owner: None,
                review_cadence: None,
                state: state.clone(),
                last_verified_commit: Some("verified0".to_string()),
                anchors: vec![AnchorRecord {
                    id: "fm:src/lib.rs".to_string(),
                    kind: AnchorKind::Path,
                    target: "src/lib.rs".to_string(),
                    content_hash: hash.to_string(),
                    verified_hash: "old".to_string(),
                    source: AnchorSource::Frontmatter,
                }],
                broken_mentions: Vec::new(),
            },
        );
        let mut ledger = FreshnessLedger {
            schema_version: 1,
            generated_by: "test".to_string(),
            updated_at: updated_at.to_string(),
            head_commit: head_commit.to_string(),
            pages,
            metrics: LedgerMetrics::default(),
        };
        ledger.recompute_metrics();
        ledger
    }

    #[test]
    fn same_content_ignores_updated_at_and_head_commit() {
        let a = sample_ledger("2026-11-01T00:00:00Z", "aaa111", DocState::Verified, "h1");
        let b = sample_ledger("2026-11-12T00:00:00Z", "bbb222", DocState::Verified, "h1");
        assert!(a.same_content(&b));

        let c = sample_ledger("2026-11-12T00:00:00Z", "bbb222", DocState::Drifting, "h1");
        assert!(!a.same_content(&c));

        let d = sample_ledger("2026-11-12T00:00:00Z", "bbb222", DocState::Verified, "h2");
        assert!(!a.same_content(&d));
    }

    #[test]
    fn write_ledger_if_changed_skips_same_content_and_writes_diff() {
        let dir = ScopedTempDir::new("write-if-changed");
        let path = dir.0.join("ledger.v1.json");

        let l1 = sample_ledger("2026-11-01T00:00:00Z", "aaa111", DocState::Verified, "h1");
        assert!(write_ledger_if_changed(&path, &l1).unwrap());
        let bytes1 = fs::read(&path).unwrap();

        let l2 = sample_ledger("2026-11-12T00:00:00Z", "bbb222", DocState::Verified, "h1");
        assert!(!write_ledger_if_changed(&path, &l2).unwrap());
        assert_eq!(fs::read(&path).unwrap(), bytes1);

        let l3 = sample_ledger("2026-11-12T00:00:00Z", "bbb222", DocState::Drifting, "h2");
        assert!(write_ledger_if_changed(&path, &l3).unwrap());
        assert_ne!(fs::read(&path).unwrap(), bytes1);
    }

    #[test]
    fn generate_freshness_ledger_twice_is_idempotent_and_preserves_verified_hash() {
        let dir = ScopedTempDir::new("generate-twice");
        let root = &dir.0;
        fs::create_dir_all(root.join("docs/src/architecture")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/sample.rs"), "pub fn demo() {}\n").unwrap();
        fs::write(
            root.join("docs/src/architecture/sample.md"),
            "---\ntitle: \"Sample\"\ncategory: \"architecture\"\nstatus: \"current\"\ncode_anchors:\n  - \"src/sample.rs\"\n---\n# Sample\n",
        )
        .unwrap();

        let first = generate_freshness_ledger(root, "head1", None);
        let second = generate_freshness_ledger(root, "head2", Some(&first));
        assert!(first.same_content(&second));

        // Carry verified_hash "old" when target content changes
        let mut stale = second.clone();
        if let Some(page) = stale.pages.get_mut("docs/src/architecture/sample.md")
            && let Some(anchor) = page.anchors.first_mut()
        {
            anchor.verified_hash = "old".to_string();
        }
        fs::write(root.join("src/sample.rs"), "pub fn demo() { let _ = 1; }\n").unwrap();
        let drifted = generate_freshness_ledger(root, "head3", Some(&stale));
        let drifted_page = &drifted.pages["docs/src/architecture/sample.md"];
        assert_eq!(drifted_page.state, DocState::Drifting);
        assert_eq!(drifted_page.anchors[0].verified_hash, "old");
        assert_ne!(drifted_page.anchors[0].content_hash, "old");
    }
}
