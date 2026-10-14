//! Claims and mention extraction engine for documentation reality auditing.

pub mod dod;
pub mod eval;
pub mod extractor;
pub mod findings;
pub mod gate;
pub mod hasher;
pub mod ledger;
pub mod prune;
pub mod resolver;
pub mod reverse_index;
pub mod suppressions;
pub mod types;

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
            if full_path.is_file() {
                if let Ok(content) = fs::read_to_string(&full_path) {
                    let rel_path = full_path.strip_prefix(repo_root).unwrap_or(&full_path);
                    resolved.extend(scan_file(rel_path, &content, &registry));
                }
            }
        }
    } else if docs_dir.is_dir() {
        for entry in WalkDir::new(&docs_dir)
            .follow_links(false)
            .into_iter()
            .flatten()
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if ext == "md" || ext == "mdx" {
                        if let Ok(content) = fs::read_to_string(path) {
                            let rel_path = path.strip_prefix(repo_root).unwrap_or(path);
                            resolved.extend(scan_file(rel_path, &content, &registry));
                        }
                    }
                }
            }
        }
    }

    Ok(resolved)
}
