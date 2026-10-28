//! Phase 40 (`REWR-01..05`): KB Page-One Summary Rewrite Guard, Batch Validator,
//! Usage Prioritizer, Summary-Distinctiveness Ratchet, and Authorship Source-Bias Metrics.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Canonical retired surfaces that must never reappear in rewritten summaries (`REWR-01`).
pub const DEFAULT_RETIRED_SURFACES: &[&str] = &[
    "KbStore",
    "KbRouter",
    "TantivyDocsIndex",
    "signal_research",
    "research-index.md",
    "architecture-index.md",
];

/// Canonical negation tokens whose presence must not be dropped by a summary rewrite (`REWR-01`).
const NEGATION_TOKENS: &[&str] = &["not", "never", "cannot", "without", "forbid", "prohibit"];

/// Result of running `verify_page_one_rewrite` (`REWR-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RewriteGuardReport {
    pub passed: bool,
    pub code_fences_untouched: bool,
    pub body_outside_summary_untouched: bool,
    pub retired_surface_violations: Vec<String>,
    pub dropped_numbers: Vec<String>,
    pub dropped_negations: Vec<String>,
    pub unresolved_backtick_tokens: Vec<String>,
}

fn extract_fenced_blocks(md: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut in_fence = false;
    let mut current = String::new();
    for line in md.lines() {
        if line.trim_start().starts_with("```") {
            current.push_str(line);
            current.push('\n');
            if in_fence {
                blocks.push(std::mem::take(&mut current));
                in_fence = false;
            } else {
                in_fence = true;
            }
        } else if in_fence {
            current.push_str(line);
            current.push('\n');
        }
    }
    blocks
}

/// Split markdown into `(before_summary_body, summary_block, after_summary_body)`,
/// ignoring frontmatter so frontmatter `authorship` updates are permitted (`REWR-02`).
fn split_summary_and_body(md: &str) -> (String, String, String) {
    let body = if let Some(rest) = md.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            &rest[end + 4..]
        } else {
            md
        }
    } else {
        md
    };

    if let Some(start) = body.find("<!-- kb:summary -->") {
        let after_start = &body[start + "<!-- kb:summary -->".len()..];
        if let Some(end) = after_start.find("<!-- /kb:summary -->") {
            let before = body[..start].trim().to_string();
            let summary = after_start[..end].trim().to_string();
            let after = after_start[end + "<!-- /kb:summary -->".len()..]
                .trim()
                .to_string();
            return (before, summary, after);
        }
    }
    (body.trim().to_string(), String::new(), String::new())
}

fn extract_numeric_tokens(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_ascii_digit() && c != '.')
        .map(|s| s.trim_matches('.'))
        .filter(|s| !s.is_empty() && s.chars().any(|c| c.is_ascii_digit()))
        .map(str::to_string)
        .collect()
}

fn extract_negation_tokens(text: &str) -> BTreeSet<String> {
    let lower = text.to_ascii_lowercase();
    let words: BTreeSet<&str> = lower
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|w| !w.is_empty())
        .collect();
    NEGATION_TOKENS
        .iter()
        .filter(|&&n| words.contains(n))
        .map(|&s| s.to_string())
        .collect()
}

fn extract_backtick_spans(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut parts = text.split('`');
    let _ = parts.next();
    while let Some(code) = parts.next() {
        let trimmed = code.trim();
        if !trimmed.is_empty() {
            out.insert(trimmed.to_string());
        }
        let _ = parts.next();
    }
    out
}

/// Verify a page-one summary rewrite (`REWR-01`):
/// - Code fences must be 100% identical to original.
/// - Body text outside `<!-- kb:summary --> ... <!-- /kb:summary -->` must be untouched.
/// - Retired surfaces must not appear in the rewritten summary.
/// - Numeric literals and negation tokens present in the original summary must be preserved.
/// - All backticked identifiers/paths/CLI commands in the rewritten summary must resolve in
///   `allowed_references` or already appear in the original document.
pub fn verify_page_one_rewrite(
    original_md: &str,
    rewritten_md: &str,
    allowed_references: &BTreeSet<String>,
) -> RewriteGuardReport {
    let orig_fences = extract_fenced_blocks(original_md);
    let new_fences = extract_fenced_blocks(rewritten_md);
    let code_fences_untouched = orig_fences == new_fences;

    let (orig_before, orig_summary, orig_after) = split_summary_and_body(original_md);
    let (new_before, new_summary, new_after) = split_summary_and_body(rewritten_md);
    let body_outside_summary_untouched = orig_before == new_before && orig_after == new_after;

    let retired_surface_violations: Vec<String> = DEFAULT_RETIRED_SURFACES
        .iter()
        .filter(|&&surf| new_summary.contains(surf))
        .map(|&s| s.to_string())
        .collect();

    let orig_nums = extract_numeric_tokens(&orig_summary);
    let new_nums = extract_numeric_tokens(&new_summary);
    let dropped_numbers: Vec<String> = orig_nums.difference(&new_nums).cloned().collect();

    let orig_negs = extract_negation_tokens(&orig_summary);
    let new_negs = extract_negation_tokens(&new_summary);
    let dropped_negations: Vec<String> = orig_negs.difference(&new_negs).cloned().collect();

    let orig_backticks = extract_backtick_spans(original_md);
    let new_backticks = extract_backtick_spans(&new_summary);
    let unresolved_backtick_tokens: Vec<String> = new_backticks
        .into_iter()
        .filter(|tok| !orig_backticks.contains(tok) && !allowed_references.contains(tok))
        .collect();

    let passed = code_fences_untouched
        && body_outside_summary_untouched
        && !new_summary.is_empty()
        && retired_surface_violations.is_empty()
        && dropped_numbers.is_empty()
        && dropped_negations.is_empty()
        && unresolved_backtick_tokens.is_empty();

    RewriteGuardReport {
        passed,
        code_fences_untouched,
        body_outside_summary_untouched,
        retired_surface_violations,
        dropped_numbers,
        dropped_negations,
        unresolved_backtick_tokens,
    }
}

/// Page entry in a rewrite batch (`REWR-02`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RewriteBatchEntry {
    pub doc_path: String,
    pub category: String,
    pub authorship: String,
    pub authorship_model: Option<String>,
    pub training_eligible: bool,
    pub human_attested_by: Option<String>,
}

/// Validate a rewrite batch (`REWR-02`):
/// - At most 25 pages per batch.
/// - Grouped by a single `category`.
/// - Every LLM-assisted/generated rewrite must set `authorship_model` and force
///   `training_eligible == false` unless `human_attested_by` is present.
pub fn validate_rewrite_batch(entries: &[RewriteBatchEntry]) -> Result<(), String> {
    if entries.is_empty() {
        return Err("rewrite batch must not be empty".into());
    }
    if entries.len() > 25 {
        return Err(format!(
            "rewrite batch size {} exceeds maximum of 25 pages",
            entries.len()
        ));
    }
    let first_cat = &entries[0].category;
    for entry in entries {
        if &entry.category != first_cat {
            return Err(format!(
                "rewrite batch mixes categories '{}' and '{}'",
                first_cat, entry.category
            ));
        }
        let is_llm = matches!(entry.authorship.as_str(), "llm-assisted" | "llm-generated");
        if is_llm {
            if entry
                .authorship_model
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                return Err(format!(
                    "page '{}' has authorship '{}' without authorship_model",
                    entry.doc_path, entry.authorship
                ));
            }
            if entry.training_eligible && entry.human_attested_by.is_none() {
                return Err(format!(
                    "page '{}' has LLM authorship and training_eligible=true without human_attested_by",
                    entry.doc_path
                ));
            }
        }
    }
    Ok(())
}

/// Candidate for usage-prioritized rewrite ordering (`REWR-03`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RewritePriorityCandidate {
    pub doc_path: String,
    pub retrieval_usage_count: u64,
    pub is_landing_or_hub: bool,
}

/// Prioritize rewrite candidates (`REWR-03`, `REWR-04`):
/// Landing and hub pages come first (`REWR-04`), followed by top-N docs by retrieval usage (`REWR-03`),
/// then the long tail in deterministic path order.
pub fn prioritize_rewrite_candidates(
    mut candidates: Vec<RewritePriorityCandidate>,
) -> Vec<RewritePriorityCandidate> {
    candidates.sort_by(|a, b| {
        b.is_landing_or_hub
            .cmp(&a.is_landing_or_hub)
            .then_with(|| b.retrieval_usage_count.cmp(&a.retrieval_usage_count))
            .then_with(|| a.doc_path.cmp(&b.doc_path))
    });
    candidates
}

/// Summary distinctiveness & authorship source-bias report (`REWR-05`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SummaryDistinctivenessAndBiasReport {
    /// Mean (1.0 - Jaccard) across summary pairs (`1.0` = completely distinct, `0.0` = identical).
    pub mean_distinctiveness: f64,
    /// Maximum pairwise Jaccard similarity across summaries.
    pub max_pairwise_similarity: f64,
    /// Retrieval share `[0.0, 1.0]` stratified by authorship (`human`, `llm-assisted`, `llm-generated`).
    pub retrieval_share_by_authorship: BTreeMap<String, f64>,
    /// True if LLM-written pages exceed `max_llm_share` (`REWR-05`).
    pub llm_crowding_alert: bool,
}

/// Compute summary distinctiveness and retrieval share stratified by authorship (`REWR-05`).
pub fn evaluate_distinctiveness_and_source_bias(
    summaries: &[String],
    retrieval_hits_authorship: &[String],
    max_llm_share: f64,
) -> SummaryDistinctivenessAndBiasReport {
    let token_sets: Vec<BTreeSet<String>> = summaries
        .iter()
        .map(|s| {
            s.split(|c: char| !c.is_alphanumeric())
                .filter(|w| w.len() >= 3)
                .map(|w| w.to_ascii_lowercase())
                .collect()
        })
        .collect();

    let mut total_sim = 0.0f64;
    let mut max_sim = 0.0f64;
    let mut pairs = 0usize;
    for i in 0..token_sets.len() {
        for j in (i + 1)..token_sets.len() {
            let u = token_sets[i].union(&token_sets[j]).count() as f64;
            let sim = if u > 0.0 {
                (token_sets[i].intersection(&token_sets[j]).count() as f64) / u
            } else {
                0.0
            };
            total_sim += sim;
            if sim > max_sim {
                max_sim = sim;
            }
            pairs += 1;
        }
    }
    let mean_distinctiveness = if pairs > 0 {
        (1.0 - (total_sim / pairs as f64)).clamp(0.0, 1.0)
    } else {
        1.0
    };

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for auth in retrieval_hits_authorship {
        *counts.entry(auth.clone()).or_insert(0) += 1;
    }
    let total_hits = retrieval_hits_authorship.len().max(1) as f64;
    let mut shares = BTreeMap::new();
    let mut llm_share = 0.0f64;
    for (k, v) in counts {
        let share = (v as f64) / total_hits;
        if k.starts_with("llm") {
            llm_share += share;
        }
        shares.insert(k, share);
    }

    SummaryDistinctivenessAndBiasReport {
        mean_distinctiveness,
        max_pairwise_similarity: max_sim,
        retrieval_share_by_authorship: shares,
        llm_crowding_alert: llm_share > max_llm_share,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_guard_preserves_fences_numbers_negations_and_blocks_retired() {
        let orig = "---\ntitle: \"Guide\"\n---\n# Guide\n<!-- kb:summary -->\nDo not exceed 50 items using `vox check`.\n<!-- /kb:summary -->\n\nBody paragraph.\n```vox\nfn main() -> Int { ret 0 }\n```";
        let good = "---\ntitle: \"Guide\"\nauthorship: \"llm-assisted\"\n---\n# Guide\n<!-- kb:summary -->\nYou must not exceed 50 items when running `vox check`.\n<!-- /kb:summary -->\n\nBody paragraph.\n```vox\nfn main() -> Int { ret 0 }\n```";
        let allowed = BTreeSet::new();
        let rep = verify_page_one_rewrite(orig, good, &allowed);
        assert!(rep.passed, "{rep:?}");

        // Dropping negation "not" or number "50" or introducing retired "KbStore" fails
        let bad = "---\ntitle: \"Guide\"\n---\n# Guide\n<!-- kb:summary -->\nExceed items with `KbStore`.\n<!-- /kb:summary -->\n\nBody paragraph.\n```vox\nfn main() -> Int { ret 0 }\n```";
        let bad_rep = verify_page_one_rewrite(orig, bad, &allowed);
        assert!(!bad_rep.passed);
        assert!(!bad_rep.retired_surface_violations.is_empty());
        assert!(!bad_rep.dropped_numbers.is_empty());
        assert!(!bad_rep.dropped_negations.is_empty());
    }

    #[test]
    fn batch_validator_enforces_cap_and_training_eligible_gate() {
        let ok_batch = vec![RewriteBatchEntry {
            doc_path: "docs/src/a.md".into(),
            category: "reference".into(),
            authorship: "llm-assisted".into(),
            authorship_model: Some("gemini-3-flash".into()),
            training_eligible: false,
            human_attested_by: None,
        }];
        assert!(validate_rewrite_batch(&ok_batch).is_ok());

        let bad_batch = vec![RewriteBatchEntry {
            doc_path: "docs/src/a.md".into(),
            category: "reference".into(),
            authorship: "llm-assisted".into(),
            authorship_model: Some("gemini-3-flash".into()),
            training_eligible: true,
            human_attested_by: None,
        }];
        assert!(validate_rewrite_batch(&bad_batch).is_err());
    }

    #[test]
    fn distinctiveness_and_source_bias_detect_crowding() {
        let summaries = vec![
            "Vox actors communicate via typed message mailboxes and state isolation.".to_string(),
            "Clavis resolves secrets through keyring, environment, and zero-trust brokers."
                .to_string(),
        ];
        let hits = vec![
            "human".to_string(),
            "llm-assisted".to_string(),
            "llm-assisted".to_string(),
            "llm-generated".to_string(),
        ];
        let rep = evaluate_distinctiveness_and_source_bias(&summaries, &hits, 0.60);
        assert!(rep.mean_distinctiveness > 0.80);
        assert!(rep.llm_crowding_alert);
    }
}
