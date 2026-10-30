//! Claims and mention extraction engine for documentation reality auditing.

pub mod consolidation;
pub mod dod;
pub mod eval;
pub mod extractor;
pub mod findings;
pub mod gate;
pub mod hasher;
pub mod ledger;
pub mod llm_verifier;
pub mod prune;
pub mod resolver;
pub mod reverse_index;
pub mod self_heal_bot;
pub mod suppressions;
pub mod types;

pub use consolidation::{
    ClusterConsolidationPlan, DuplicateCluster, plan_duplicate_cluster_consolidation,
    ratchet_kb_facet_backlog, render_redirects_block,
};
pub use dod::{DodWarning, check_definition_of_done};
pub use eval::{EvalReport, EvaluationDataset, EvaluationSample, evaluate_sample_dataset};
pub use extractor::extract_mentions;
pub use findings::{
    FindingsFile, MetricsFile, RealityAuditFinding, compute_fingerprint, compute_metrics,
    generate_findings,
};
pub use gate::{
    BaselineFile, DriftGateResult, GrandfatheredDeficit, evaluate_drift_gate, is_reader_facing_page,
};
pub use hasher::{
    extract_symbol_span, hash_anchor, hash_normalized_code, hash_normalized_contract,
};
pub use ledger::{
    AnchorKind, AnchorRecord, AnchorSource, DocState, FreshnessLedger, LedgerMetrics,
    PageLedgerRecord, generate_freshness_ledger,
};
pub use prune::{PagePruneRecord, PruneDisposition, PruneReport, generate_prune_report};
pub use resolver::GroundTruthRegistry;
pub use reverse_index::{DocPageReference, PR_IMPACT_COMMENT_MARKER, PrImpactReport, ReverseIndex};
pub use suppressions::{DocSuppression, SuppressionsFile};
pub use types::{ExtractedMention, MentionKind, MentionRole, ResolutionStatus, ResolvedMention};

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Scan a single documentation file and resolve all extracted mentions.
pub fn scan_file(
    source_file: &Path,
    content: &str,
    registry: &GroundTruthRegistry,
) -> Vec<ResolvedMention> {
    let extracted = extract_mentions(source_file, content);
    extracted
        .into_iter()
        .map(|m| registry.resolve_mention(&m))
        .collect()
}

/// Scan the entire documentation corpus (or filtered paths) and resolve all mentions.
pub fn scan_corpus(
    repo_root: &Path,
    filter_paths: Option<&[PathBuf]>,
) -> Result<Vec<ResolvedMention>> {
    let registry =
        GroundTruthRegistry::load(repo_root).context("Failed to load ground truth registry")?;

    let docs_dir = repo_root.join("docs/src");
    let mut resolved = Vec::new();

    if let Some(paths) = filter_paths {
        for p in paths {
            let full_path = if p.is_absolute() {
                p.clone()
            } else {
                repo_root.join(p)
            };
            if full_path.is_file()
                && let Ok(content) = fs::read_to_string(&full_path)
            {
                let rel_path = full_path.strip_prefix(repo_root).unwrap_or(&full_path);
                resolved.extend(scan_file(rel_path, &content, &registry));
            }
        }
    } else if docs_dir.is_dir() {
        for entry in WalkDir::new(&docs_dir)
            .follow_links(false)
            .into_iter()
            .flatten()
        {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|s| s.to_str())
                && (ext == "md" || ext == "mdx")
                && let Ok(content) = fs::read_to_string(path)
            {
                let rel_path = path.strip_prefix(repo_root).unwrap_or(path);
                resolved.extend(scan_file(rel_path, &content, &registry));
            }
        }
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_file_resolves_each_extracted_mention() {
        let doc = "Set `VOX_DB_URL` and see crate `vox-orchestrator`.\n";
        let path = Path::new("docs/src/test.md");
        let resolved = scan_file(path, doc, &GroundTruthRegistry::empty());
        assert_eq!(resolved.len(), extract_mentions(path, doc).len());
        assert!(!resolved.is_empty());
    }

    #[test]
    fn scan_corpus_reads_only_markdown_under_docs_src() {
        let root = std::env::temp_dir().join(format!("vox-scan-corpus-{}", std::process::id()));
        let docs = root.join("docs/src");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("a.md"), "Set `VOX_DB_URL`.\n").unwrap();
        fs::write(docs.join("b.txt"), "Set `VOX_DB_URL`.\n").unwrap();

        let all = scan_corpus(&root, None).unwrap();
        let filtered = scan_corpus(&root, Some(&[PathBuf::from("docs/src/b.txt")])).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert!(!all.is_empty());
        assert!(
            all.iter()
                .all(|m| m.extracted.source_file.ends_with("a.md"))
        );
        assert_eq!(
            filtered.len(),
            all.len(),
            "explicit paths bypass the extension filter"
        );
    }
}
