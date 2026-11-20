//! Phase 35 (`VERIFY-01..04`): Evidence-Cited LLM Claim Verifier (Scheduled / Advisory Only).
//!
//! Provides:
//! - Structured claim extraction (`VERIFY-01`) with `doc_path`, `line`, `claim_text`, and
//!   resolved `symbol_anchors` + `symbol_span_hash`.
//! - Evidence-cited verification verdicts (`VERIFY-02`) requiring `[path:Lstart-Lend]` citations;
//!   verdicts lacking valid line-range citations are automatically downgraded to `Uncertain`.
//! - Content-addressed cache (`(claim_hash, symbol_span_hash)`) and token/cost budget controls
//!   (`VERIFY-03`) for nightly / scheduled / advisory runs (never PR-blocking).
//! - Golden drift benchmark evaluation (`VERIFY-04`) against
//!   `contracts/reports/docs-freshness/golden-drift-eval-set.v1.json` enforcing
//!   `false_positive_rate <= 0.10` and `precision >= 0.80` before bot promotion.

use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Execution mode for the LLM claim verifier (`VERIFY-03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifierScheduleMode {
    /// Scheduled nightly audit job (`vox ci doc-verify-llm --nightly`).
    NightlyScheduled,
    /// On-demand advisory run (`vox audit docs --llm-verify`).
    OnDemandAdvisory,
    /// Disallowed in PR-blocking CI gates (`VERIFY-03`).
    PrBlockingForbidden,
}

impl VerifierScheduleMode {
    /// Returns true if the mode is permitted to execute (`VERIFY-03`).
    pub fn is_permitted(self) -> bool {
        !matches!(self, Self::PrBlockingForbidden)
    }
}

/// Extracted factual claim candidate from prose paragraphs (`VERIFY-01`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocClaimCandidate {
    pub doc_path: String,
    pub line: usize,
    pub claim_text: String,
    pub symbol_anchors: Vec<String>,
    pub symbol_span_hash: String,
}

impl DocClaimCandidate {
    /// Content hash of the normalized claim text (`VERIFY-03`).
    pub fn claim_hash(&self) -> String {
        sha256_hex(self.claim_text.trim().as_bytes())
    }

    /// Composite cache key `(claim_hash, symbol_span_hash)` (`VERIFY-03`).
    pub fn cache_key(&self) -> (String, String) {
        (self.claim_hash(), self.symbol_span_hash.clone())
    }
}

/// Verdict classification for a documentation claim (`VERIFY-02`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimVerdictKind {
    Verified,
    Drifted,
    Uncertain,
}

/// Structured evidence-cited verdict (`VERIFY-02`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocClaimVerdict {
    pub doc_path: String,
    pub line: usize,
    pub verdict: ClaimVerdictKind,
    pub evidence_citations: Vec<String>,
    pub rationale: String,
    pub estimated_tokens: u64,
}

/// Validates that a citation follows `[path:Lstart-Lend]` or `path:Lstart-Lend` with `1 <= start <= end` (`VERIFY-02`).
pub fn is_valid_line_range_citation(citation: &str) -> bool {
    let trimmed = citation
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    let Some((path_part, range_part)) = trimmed.rsplit_once(":L") else {
        return false;
    };
    if path_part.trim().is_empty() || path_part.contains(' ') {
        return false;
    }
    let Some((start_str, end_str)) = range_part.split_once("-L") else {
        return false;
    };
    let (Ok(start), Ok(end)) = (start_str.parse::<usize>(), end_str.parse::<usize>()) else {
        return false;
    };
    start >= 1 && end >= start
}

/// A slice of real code/contract text offered to an entailment judge as evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSpan {
    /// `[path:Lstart-Lend]` citation of this span (never the document being verified).
    pub citation: String,
    /// The text at that span.
    pub text: String,
}

/// What an entailment judge decided about one claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeOutput {
    pub verdict: ClaimVerdictKind,
    pub rationale: String,
    pub estimated_tokens: u64,
}

/// Model-agnostic entailment judge. `vox-doc-pipeline` defines the seam; the CLI layer implements
/// it over `vox_actor_runtime::llm`, so this crate takes no LLM dependency and hosts no vendor
/// code. Returning `None` means "no judgement" (offline, error, unparsable), and the deterministic
/// verdict stands.
pub trait ClaimEntailmentJudge {
    fn judge(&self, claim: &DocClaimCandidate, evidence: &[EvidenceSpan]) -> Option<JudgeOutput>;
}

/// Path part of a `[path:Lstart-Lend]` citation.
fn citation_path(citation: &str) -> Option<&str> {
    let trimmed = citation
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    trimmed.rsplit_once(":L").map(|(p, _)| p.trim())
}

/// True when `citation` points into the very document being verified. A document can never be
/// its own evidence (`LLM-HEAL-01`).
pub fn is_doc_self_citation(doc_path: &str, citation: &str) -> bool {
    citation_path(citation).is_some_and(|p| {
        p.replace('\\', "/").trim_start_matches("./") == doc_path.trim_start_matches("./")
    })
}

/// Enforces `VERIFY-02`: any `Verified` or `Drifted` verdict lacking at least one valid
/// `[path:Lstart-Lend]` evidence citation, or citing the document itself, is downgraded to
/// `Uncertain` and never surfaces as an error.
pub fn enforce_citation_requirement(mut verdict: DocClaimVerdict) -> DocClaimVerdict {
    let has_valid_citation = !verdict.evidence_citations.is_empty()
        && verdict.evidence_citations.iter().all(|c| {
            is_valid_line_range_citation(c) && !is_doc_self_citation(&verdict.doc_path, c)
        });
    if !has_valid_citation && verdict.verdict != ClaimVerdictKind::Uncertain {
        verdict.verdict = ClaimVerdictKind::Uncertain;
        verdict.rationale = format!(
            "Downgraded to Uncertain (missing valid non-self [path:Lstart-Lend] citation): {}",
            verdict.rationale
        );
    }
    verdict
}

/// Extract factual prose claims adjacent to code/symbol anchors (`VERIFY-01`).
pub fn extract_claim_candidates(
    doc_path: &str,
    markdown: &str,
    symbol_span_hashes: &BTreeMap<String, String>,
) -> Vec<DocClaimCandidate> {
    let inline_code_re = Regex::new(r"`([A-Za-z0-9_:/.\-]+)`").unwrap();
    let mut candidates = Vec::new();
    let mut in_fence = false;

    for (idx, raw_line) in markdown.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("---")
        {
            continue;
        }
        let mut anchors = Vec::new();
        for cap in inline_code_re.captures_iter(trimmed) {
            let sym = cap[1].to_string();
            if sym.contains("::") || sym.contains('/') || sym.contains('_') {
                anchors.push(sym);
            }
        }
        if anchors.is_empty() {
            continue;
        }
        anchors.sort();
        anchors.dedup();

        let mut span_buf = Vec::new();
        for a in &anchors {
            let h = symbol_span_hashes
                .get(a)
                .map(String::as_str)
                .unwrap_or("unindexed");
            span_buf.extend_from_slice(a.as_bytes());
            span_buf.push(b':');
            span_buf.extend_from_slice(h.as_bytes());
            span_buf.push(b'\n');
        }
        let symbol_span_hash = sha256_hex(&span_buf);

        candidates.push(DocClaimCandidate {
            doc_path: doc_path.to_string(),
            line: line_no,
            claim_text: trimmed.to_string(),
            symbol_anchors: anchors,
            symbol_span_hash,
        });
    }

    candidates
}

/// Token and spend cap for scheduled LLM claim verification (`VERIFY-03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocVerifyBudget {
    pub max_tokens_per_run: u64,
    pub max_claims_per_run: usize,
}

impl Default for DocVerifyBudget {
    fn default() -> Self {
        Self {
            max_tokens_per_run: 50_000,
            max_claims_per_run: 250,
        }
    }
}

/// Content-addressed cache keyed by `(claim_hash, symbol_span_hash)` (`VERIFY-03`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmVerifyCache {
    pub entries: BTreeMap<String, DocClaimVerdict>,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub tokens_spent: u64,
}

impl LlmVerifyCache {
    fn composite_key(claim_hash: &str, symbol_span_hash: &str) -> String {
        format!("{claim_hash}::{symbol_span_hash}")
    }

    pub fn get(&mut self, claim: &DocClaimCandidate) -> Option<DocClaimVerdict> {
        let (ch, sh) = claim.cache_key();
        let key = Self::composite_key(&ch, &sh);
        if let Some(v) = self.entries.get(&key).cloned() {
            self.cache_hits += 1;
            Some(v)
        } else {
            self.cache_misses += 1;
            None
        }
    }

    pub fn insert(&mut self, claim: &DocClaimCandidate, verdict: DocClaimVerdict) {
        let (ch, sh) = claim.cache_key();
        let key = Self::composite_key(&ch, &sh);
        self.tokens_spent = self.tokens_spent.saturating_add(verdict.estimated_tokens);
        self.entries
            .insert(key, enforce_citation_requirement(verdict));
    }
}

/// Validates that a citation follows `[path:Lstart-Lend]` AND that `path` exists under `repo_root` (`OPENWIKI-VERIFY-03`).
pub fn is_valid_line_range_citation_on_disk(repo_root: &Path, citation: &str) -> bool {
    if !is_valid_line_range_citation(citation) {
        return false;
    }
    let trimmed = citation
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    if let Some((path_part, _)) = trimmed.rsplit_once(":L") {
        repo_root.join(path_part.trim()).is_file()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citation_validator_rejects_missing_or_malformed_spans() {
        assert!(is_valid_line_range_citation(
            "[crates/vox-db/src/schema.rs:L10-L42]"
        ));
        assert!(!is_valid_line_range_citation("crates/vox-db/src/schema.rs"));
        assert!(!is_valid_line_range_citation(
            "[crates/vox-db/src/schema.rs:L42-L10]"
        ));

        let unanchored = DocClaimVerdict {
            doc_path: "docs/src/api/db.md".to_string(),
            line: 12,
            verdict: ClaimVerdictKind::Drifted,
            evidence_citations: vec![],
            rationale: "Thought it drifted".to_string(),
            estimated_tokens: 50,
        };
        let downgraded = enforce_citation_requirement(unanchored);
        assert_eq!(downgraded.verdict, ClaimVerdictKind::Uncertain);
    }

    #[test]
    fn doc_to_self_citation_is_downgraded() {
        let verdict = DocClaimVerdict {
            doc_path: "docs/src/api/db.md".to_string(),
            line: 3,
            verdict: ClaimVerdictKind::Verified,
            evidence_citations: vec!["[docs/src/api/db.md:L1-L3]".to_string()],
            rationale: "cites itself".to_string(),
            estimated_tokens: 1,
        };
        assert!(is_doc_self_citation(
            "docs/src/api/db.md",
            "[docs/src/api/db.md:L1-L3]"
        ));
        assert_eq!(
            enforce_citation_requirement(verdict).verdict,
            ClaimVerdictKind::Uncertain
        );
    }

    #[test]
    fn cache_hits_on_unchanged_claim_and_symbol_span_hash() {
        let mut hashes = BTreeMap::new();
        hashes.insert(
            "vox_db::table_to_ddl".to_string(),
            "abc123spanhash".to_string(),
        );
        let claims = extract_claim_candidates(
            "docs/src/api/db.md",
            "The `vox_db::table_to_ddl` helper injects `_id`.\n",
            &hashes,
        );
        assert_eq!(claims.len(), 1);

        let mut cache = LlmVerifyCache::default();
        assert!(cache.get(&claims[0]).is_none());
        cache.insert(
            &claims[0],
            DocClaimVerdict {
                doc_path: claims[0].doc_path.clone(),
                line: claims[0].line,
                verdict: ClaimVerdictKind::Verified,
                evidence_citations: vec!["[crates/vox-db/src/schema.rs:L1-L20]".to_string()],
                rationale: "Matches implementation".to_string(),
                estimated_tokens: 95,
            },
        );
        assert!(cache.get(&claims[0]).is_some());
        assert_eq!(cache.cache_hits, 1);
        assert_eq!(cache.tokens_spent, 95);
    }
}
