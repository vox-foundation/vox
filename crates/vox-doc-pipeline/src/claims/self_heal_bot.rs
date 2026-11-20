//! Phase 36 (`BOT-01..04`): Guarded Self-Healing Doc Draft-PR Bot.
//!
//! Provides:
//! - Human-reviewed draft-PR generation (`BOT-01`) with `[path:Lstart-Lend]` evidence citations;
//!   never auto-merges or commits directly to `main`.
//! - Strict path-scope enforcement (`BOT-02`): only `docs/src/**` (excluding `*.generated.md`
//!   and `docs/src/archive/**`), rejecting `.rs`, `.vox`, `Cargo.toml`, and `contracts/**`.
//! - Pre-PR verification gates (`BOT-03`): frontmatter check, Mermaid structural validation,
//!   balanced code fences, and valid `[path:Lstart-Lend]` citations before opening a draft PR.
//! - Concurrency (`<= 3` open draft PRs), in-place branch updates, and acceptance/rejection
//!   metrics ledger with auto-pause (`BOT-04`).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::extractor::extract_mentions;
use super::llm_verifier::{
    ClaimEntailmentJudge, ClaimVerdictKind, DocClaimVerdict, DocVerifyBudget, EvidenceSpan,
    LlmVerifyCache, enforce_citation_requirement, extract_claim_candidates,
    is_valid_line_range_citation,
};
use super::resolver::GroundTruthRegistry;
use super::types::{MentionKind, MentionRole, ResolutionStatus};
use crate::pipeline::fence_issues;
use crate::pipeline::mermaid::validate_and_repair_markdown_mermaid;
use crate::pipeline::types::DiagramIssue;

/// Maximum concurrent open draft PRs permitted across the repository (`BOT-04`).
pub const MAX_CONCURRENT_BOT_DRAFT_PRS: usize = 3;

/// Minimum rolling acceptance precision before the self-healing bot auto-pauses (`BOT-04`).
pub const MIN_BOT_ACCEPTANCE_PRECISION: f64 = 0.80;

/// Candidate documentation patch proposed for a `drifting` or `broken` page (`BOT-01..03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateDocPatch {
    pub subsystem: String,
    pub target_doc_path: String,
    pub updated_markdown: String,
    pub evidence_citations: Vec<String>,
    pub summary: String,
}

/// Reason a candidate bot patch was rejected before opening a draft PR (`BOT-02`, `BOT-03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotPatchRejection {
    OutOfScopePath(String),
    GeneratedDocForbidden(String),
    ArchiveDocForbidden(String),
    MissingEvidenceCitations,
    InvalidEvidenceCitation(String),
    MissingFrontmatter,
    UnbalancedCodeFence,
    MermaidValidationFailed(String),
    /// A `vox-diagram` fence fails `vox_diagram::check` (warnings do not block).
    DiagramValidationFailed(String),
}

/// Validates strict path scope (`BOT-02`): only `docs/src/**` (excluding `*.generated.md`
/// and `docs/src/archive/**`). Never permits `.rs`, `.vox`, `Cargo.toml`, or `contracts/**`.
pub fn validate_bot_target_path(rel_path: &str) -> Result<(), BotPatchRejection> {
    let norm = rel_path.trim().replace('\\', "/");
    if !norm.starts_with("docs/src/") || !(norm.ends_with(".md") || norm.ends_with(".mdx")) {
        return Err(BotPatchRejection::OutOfScopePath(norm));
    }
    if norm.ends_with(".generated.md") {
        return Err(BotPatchRejection::GeneratedDocForbidden(norm));
    }
    if norm.starts_with("docs/src/archive/") || norm.contains("/archive/") {
        return Err(BotPatchRejection::ArchiveDocForbidden(norm));
    }
    Ok(())
}

/// Validates `BOT-02` path scope and `BOT-03` pre-PR verification gates (frontmatter, balanced
/// code fences, Mermaid structural validation, and `[path:Lstart-Lend]` evidence citations).
pub fn validate_bot_patch_scope_and_gates(
    patch: &CandidateDocPatch,
) -> Result<(), BotPatchRejection> {
    validate_bot_target_path(&patch.target_doc_path)?;

    if patch.evidence_citations.is_empty() {
        return Err(BotPatchRejection::MissingEvidenceCitations);
    }
    for c in &patch.evidence_citations {
        if !is_valid_line_range_citation(c) {
            return Err(BotPatchRejection::InvalidEvidenceCitation(c.clone()));
        }
    }

    let md = &patch.updated_markdown;
    if !md.starts_with("---\n") || !md[4..].contains("\n---") {
        return Err(BotPatchRejection::MissingFrontmatter);
    }

    let fence_count = md
        .lines()
        .filter(|line| line.trim_start().starts_with("```"))
        .count();
    if fence_count % 2 != 0 {
        return Err(BotPatchRejection::UnbalancedCodeFence);
    }

    let (_, mermaid_diags, _) = validate_and_repair_markdown_mermaid(md);
    if let Some(first) = mermaid_diags.first() {
        return Err(BotPatchRejection::MermaidValidationFailed(format!(
            "{} at line {}: {}",
            first.code, first.line, first.message
        )));
    }

    if let Some(issue) = fence_issues(md)
        .into_iter()
        .find(|i| !matches!(i, DiagramIssue::Check { error: false, .. }))
    {
        return Err(BotPatchRejection::DiagramValidationFailed(format!(
            "{issue:?}"
        )));
    }

    Ok(())
}

/// Draft PR specification produced by `vox ci doc-self-heal --open-draft-pr` (`BOT-01`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BotDraftPrSpec {
    pub branch_name: String,
    pub target_base: String,
    pub is_draft: bool,
    pub auto_merge: bool,
    pub updated_in_place: bool,
    pub title: String,
    pub body: String,
    pub modified_paths: Vec<String>,
}

/// Metrics ledger tracking open draft PRs, in-place updates, and acceptance/rejection rate (`BOT-04`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BotMetricsLedger {
    pub schema_version: u32,
    pub open_draft_branches: BTreeMap<String, String>,
    pub accepted_prs: usize,
    pub rejected_prs: usize,
    pub discarded_candidates: usize,
}

impl Default for BotMetricsLedger {
    fn default() -> Self {
        Self {
            schema_version: 1,
            open_draft_branches: BTreeMap::new(),
            accepted_prs: 0,
            rejected_prs: 0,
            discarded_candidates: 0,
        }
    }
}

impl BotMetricsLedger {
    /// Current rolling precision (`accepted / (accepted + rejected)`), defaulting to `1.0` until
    /// at least 5 PRs have been reviewed (`BOT-04`).
    pub fn acceptance_precision(&self) -> f64 {
        let total = self.accepted_prs + self.rejected_prs;
        if total == 0 {
            1.0
        } else {
            self.accepted_prs as f64 / total as f64
        }
    }

    /// Returns true if the bot should auto-pause due to low precision (`BOT-04`).
    pub fn is_auto_paused(&self) -> bool {
        let total = self.accepted_prs + self.rejected_prs;
        total >= 5 && self.acceptance_precision() < MIN_BOT_ACCEPTANCE_PRECISION
    }

    /// Attempts to stage or update a human-reviewed draft PR (`BOT-01..04`).
    ///
    /// - Rejects if auto-paused (`BOT-04`).
    /// - Validates scope and pre-PR gates (`BOT-02`, `BOT-03`), incrementing `discarded_candidates` on failure.
    /// - Updates in place if a draft PR for the subsystem is already open (`BOT-04`).
    /// - Enforces `<= 3` concurrent open draft PRs (`BOT-04`).
    pub fn plan_or_update_draft_pr(
        &mut self,
        patch: &CandidateDocPatch,
    ) -> Result<Option<BotDraftPrSpec>, BotPatchRejection> {
        if self.is_auto_paused() {
            return Ok(None);
        }

        if let Err(rej) = validate_bot_patch_scope_and_gates(patch) {
            self.discarded_candidates += 1;
            return Err(rej);
        }

        let branch_name = format!("bot/doc-self-heal-{}", patch.subsystem);
        let already_open = self.open_draft_branches.contains_key(&branch_name);

        if !already_open && self.open_draft_branches.len() >= MAX_CONCURRENT_BOT_DRAFT_PRS {
            return Ok(None);
        }

        self.open_draft_branches
            .insert(branch_name.clone(), patch.target_doc_path.clone());

        let citations_list = patch
            .evidence_citations
            .iter()
            .map(|c| format!("- `{c}`"))
            .collect::<Vec<_>>()
            .join("\n");

        let body = format!(
            "## Automated Documentation Self-Heal (Draft PR)\n\n\
             **Subsystem:** `{}`\n\
             **Target Page:** `{}`\n\n\
             ### Summary\n{}\n\n\
             ### Ground-Truth Evidence Citations\n{}\n\n\
             > Human review required (`BOT-01`). Auto-merge is disabled.",
            patch.subsystem, patch.target_doc_path, patch.summary, citations_list
        );

        Ok(Some(BotDraftPrSpec {
            branch_name,
            target_base: "main".to_string(),
            is_draft: true,
            auto_merge: false,
            updated_in_place: already_open,
            title: format!(
                "docs({}): self-heal drift in {}",
                patch.subsystem, patch.target_doc_path
            ),
            body,
            modified_paths: vec![patch.target_doc_path.clone()],
        }))
    }

    pub fn load_or_default(path: &Path) -> Self {
        if let Ok(raw) = fs::read_to_string(path)
            && let Ok(parsed) = serde_json::from_str::<Self>(&raw)
        {
            return parsed;
        }
        Self::default()
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

/// Inline-code tokens (`` `like/this` ``) of a Markdown document, deduplicated.
fn inline_code_tokens(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = markdown;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        let tok = &after[..close];
        if !tok.is_empty() && !tok.contains('\n') && tok.len() < 200 {
            out.push(tok.to_string());
        }
        rest = &after[close + 1..];
    }
    out.sort();
    out.dedup();
    out
}

/// Head of a real file as an evidence span (`[path:L1-Lk]`, at most 25 lines).
fn file_head(repo_root: &Path, rel: &str) -> Option<EvidenceSpan> {
    let full = repo_root.join(rel);
    if !full.is_file() {
        return None;
    }
    let text = fs::read_to_string(&full).ok()?;
    let head: Vec<&str> = text.lines().take(25).collect();
    let end = head.len().max(1);
    Some(EvidenceSpan {
        citation: format!("[{rel}:L1-L{end}]"),
        text: head.join("\n"),
    })
}

/// Line of `needle` (case-insensitive) in a repo file, as a one-line citation.
pub(crate) fn contract_line_citation(repo_root: &Path, rel: &str, needle: &str) -> Option<String> {
    let text = fs::read_to_string(repo_root.join(rel)).ok()?;
    let needle = needle.to_ascii_lowercase();
    let idx = text
        .lines()
        .position(|l| l.to_ascii_lowercase().contains(&needle))?;
    Some(format!("[{rel}:L{0}-L{0}]", idx + 1))
}

/// Deterministic, offline claim verification grounded in repository ground truth
/// (`LLM-HEAL-01`). Evidence always points into code or contracts, never into the document
/// being checked.
///
/// - A prescriptive mention of a retired symbol is `Drifted`, cited to the retired-symbols
///   contract line that retires it.
/// - Mentions that all resolve to real files/crates are `Uncertain`: the anchors exist, but
///   whether the prose is *entailed* by the code needs an NLI judge, which this offline path
///   does not run, so it never claims `Verified`.
/// - Anything missing or ambiguous is `Uncertain` and left to the reality audit.
pub fn verify_claim_grounded(
    repo_root: &Path,
    registry: &GroundTruthRegistry,
    claim: &super::llm_verifier::DocClaimCandidate,
) -> DocClaimVerdict {
    verify_claim_grounded_with(repo_root, registry, claim, None)
}

/// [`verify_claim_grounded`] with an optional entailment judge. The judge only runs when every
/// anchor resolved and there is real evidence; its `Verified`/`Drifted` verdicts still pass
/// through [`enforce_citation_requirement`], so they cannot stand without a non-self citation.
pub fn verify_claim_grounded_with(
    repo_root: &Path,
    registry: &GroundTruthRegistry,
    claim: &super::llm_verifier::DocClaimCandidate,
    judge: Option<&dyn ClaimEntailmentJudge>,
) -> DocClaimVerdict {
    let mentions: Vec<_> = extract_mentions(Path::new(&claim.doc_path), &claim.claim_text)
        .into_iter()
        .map(|m| registry.resolve_mention(&m))
        .collect();

    let mut spans: Vec<EvidenceSpan> = Vec::new();
    let mut evidence: Vec<String> = Vec::new();
    let mut unresolved: Vec<String> = Vec::new();
    for rm in &mentions {
        match rm.status {
            ResolutionStatus::Retired if rm.extracted.role == MentionRole::Prescribes => {
                if let Some(c) = contract_line_citation(
                    repo_root,
                    "contracts/documentation/retired-symbols.v1.yaml",
                    &rm.extracted.normalized_target,
                ) {
                    return enforce_citation_requirement(DocClaimVerdict {
                        doc_path: claim.doc_path.clone(),
                        line: claim.line,
                        verdict: ClaimVerdictKind::Drifted,
                        evidence_citations: vec![c],
                        rationale: format!(
                            "`{}` is retired{}",
                            rm.extracted.normalized_target,
                            rm.replacement
                                .as_ref()
                                .map(|r| format!("; use `{r}`"))
                                .unwrap_or_default()
                        ),
                        estimated_tokens: 0,
                    });
                }
                unresolved.push(rm.extracted.raw_text.clone());
            }
            ResolutionStatus::Resolved => {
                let span = match rm.extracted.kind {
                    MentionKind::FilePath => file_head(repo_root, &rm.extracted.normalized_target),
                    MentionKind::CrateName => file_head(
                        repo_root,
                        &format!("crates/{}/Cargo.toml", rm.extracted.normalized_target),
                    ),
                    _ => None,
                };
                if let Some(s) = span {
                    evidence.push(s.citation.clone());
                    spans.push(s);
                }
            }
            _ => unresolved.push(rm.extracted.raw_text.clone()),
        }
    }
    evidence.sort();
    evidence.dedup();
    spans.sort_by(|a, b| a.citation.cmp(&b.citation));
    spans.dedup_by(|a, b| a.citation == b.citation);

    if unresolved.is_empty()
        && !spans.is_empty()
        && let Some(j) = judge
        && let Some(out) = j.judge(claim, &spans)
    {
        return enforce_citation_requirement(DocClaimVerdict {
            doc_path: claim.doc_path.clone(),
            line: claim.line,
            verdict: out.verdict,
            evidence_citations: evidence,
            rationale: out.rationale,
            estimated_tokens: out.estimated_tokens,
        });
    }

    let rationale = if !unresolved.is_empty() {
        format!("anchors not resolvable to evidence: {unresolved:?}; see reality audit")
    } else if evidence.is_empty() {
        "no file/crate anchors to ground; semantic entailment not evaluated".to_string()
    } else if judge.is_some() {
        "anchors resolve to real files/crates; entailment judge gave no usable verdict".to_string()
    } else {
        "anchors resolve to real files/crates; semantic entailment not evaluated (no NLI judge)"
            .to_string()
    };
    DocClaimVerdict {
        doc_path: claim.doc_path.clone(),
        line: claim.line,
        verdict: ClaimVerdictKind::Uncertain,
        evidence_citations: evidence,
        rationale,
        estimated_tokens: 0,
    }
}

/// Run an advisory, budget-capped LLM/anchor claim verification pass over `docs/src/` (`OPENWIKI-VERIFY-01..03`).
pub fn run_advisory_claim_verification(
    repo_root: &Path,
    cache_path: &Path,
    budget: DocVerifyBudget,
    filter_paths: Option<&[PathBuf]>,
) -> Result<LlmVerifyCache> {
    run_advisory_claim_verification_with_judge(repo_root, cache_path, budget, filter_paths, None)
}

/// [`run_advisory_claim_verification`] with an optional entailment judge (`LLM-HEAL-01`).
pub fn run_advisory_claim_verification_with_judge(
    repo_root: &Path,
    cache_path: &Path,
    budget: DocVerifyBudget,
    filter_paths: Option<&[PathBuf]>,
    judge: Option<&dyn ClaimEntailmentJudge>,
) -> Result<LlmVerifyCache> {
    let mut cache = if let Ok(raw) = fs::read_to_string(cache_path) {
        serde_json::from_str::<LlmVerifyCache>(&raw).unwrap_or_default()
    } else {
        LlmVerifyCache::default()
    };
    let registry = GroundTruthRegistry::load(repo_root)?;
    let docs_dir = repo_root.join("docs/src");
    let mut md_paths = Vec::new();
    if let Some(paths) = filter_paths {
        for p in paths {
            let full = if p.is_absolute() {
                p.clone()
            } else {
                repo_root.join(p)
            };
            if full.is_file() {
                md_paths.push(full);
            }
        }
    } else if docs_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&docs_dir)
            .follow_links(false)
            .into_iter()
            .flatten()
        {
            let p = entry.path();
            if p.is_file() && p.extension().is_some_and(|e| e == "md" || e == "mdx") {
                let rel = p.strip_prefix(repo_root).unwrap_or(p).to_string_lossy();
                if !rel.contains("/archive/") && !rel.ends_with(".generated.md") {
                    md_paths.push(p.to_path_buf());
                }
            }
        }
    }
    md_paths.sort();

    let mut evaluated = 0usize;
    'outer: for full_path in md_paths {
        let Ok(content) = fs::read_to_string(&full_path) else {
            continue;
        };
        let rel = full_path
            .strip_prefix(repo_root)
            .unwrap_or(&full_path)
            .to_string_lossy()
            .replace('\\', "/");
        let mut symbol_hashes: BTreeMap<String, String> = BTreeMap::new();
        for sym in inline_code_tokens(&content) {
            if let Ok(h) = super::hasher::hash_anchor(repo_root, &sym) {
                symbol_hashes.insert(sym, h);
            }
        }
        for claim in extract_claim_candidates(&rel, &content, &symbol_hashes) {
            if evaluated >= budget.max_claims_per_run
                || cache.tokens_spent >= budget.max_tokens_per_run
            {
                break 'outer;
            }
            evaluated += 1;
            if cache.get(&claim).is_some() {
                continue;
            }
            let verdict = verify_claim_grounded_with(repo_root, &registry, &claim, judge);
            cache.insert(&claim, verdict);
        }
    }
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(cache_path, serde_json::to_string_pretty(&cache)?)?;
    Ok(cache)
}

/// Gate the bot on the v2 golden drift benchmark and persist advisory bot metrics
/// (`LLM-HEAL-02`, `OPENWIKI-BOT-01..04`). The bot stays off until the benchmark holds at least
/// [`super::golden_mine::MIN_HUMAN_REVIEWED_CASES`] human-reviewed real cases that clear the
/// precision and false-positive thresholds.
pub fn run_self_heal_bot_check(repo_root: &Path, write_metrics: bool) -> Result<BotMetricsLedger> {
    let eval_path =
        repo_root.join("contracts/reports/docs-freshness/golden-drift-eval-set.v2.json");
    let registry = GroundTruthRegistry::load(repo_root)?;
    let metrics = super::golden_mine::evaluate_golden_set_v2(&eval_path, &registry)?;
    if !metrics.eligible_for_bot_promotion {
        anyhow::bail!(
            "Golden drift benchmark does not license the bot: {} (human-reviewed={}, agent-reviewed={}, pending={}, precision={:.2}%, fp_rate={:.2}%)",
            metrics.ineligible_reason.as_deref().unwrap_or("unknown"),
            metrics.human_reviewed,
            metrics.agent_reviewed,
            metrics.pending,
            metrics.precision * 100.0,
            metrics.false_positive_rate * 100.0
        );
    }
    let bot_path = repo_root.join("contracts/reports/docs-freshness/bot-metrics.v1.json");
    let ledger = BotMetricsLedger::load_or_default(&bot_path);
    if write_metrics || !bot_path.is_file() {
        ledger.save_to_file(&bot_path)?;
    }
    Ok(ledger)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_patch(subsystem: &str, path: &str) -> CandidateDocPatch {
        CandidateDocPatch {
            subsystem: subsystem.to_string(),
            target_doc_path: path.to_string(),
            updated_markdown: "---\ntitle: \"Guide\"\ndescription: \"Updated guide.\"\ncategory: \"How-To Guides\"\n---\n\n# Guide\n\n```mermaid\nflowchart TD\n  A[\"Valid Node\"] --> B[\"Target\"]\n```\n".to_string(),
            evidence_citations: vec!["[crates/vox-db/src/schema.rs:L10-L25]".to_string()],
            summary: "Synchronize table primary key documentation with schema.rs.".to_string(),
        }
    }

    fn repo_root_for_tests() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
    }

    fn claim_for(doc: &str, text: &str) -> super::super::llm_verifier::DocClaimCandidate {
        extract_claim_candidates(doc, text, &BTreeMap::new())
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("no claim extracted from {text:?}"))
    }

    #[test]
    fn grounded_verifier_cites_code_never_the_doc_and_never_claims_verified_offline() {
        let root = repo_root_for_tests();
        let registry = GroundTruthRegistry::load(root).expect("registry");
        let doc = "docs/src/architecture/example.md";
        let claim = claim_for(
            doc,
            "The entry point lives in `crates/vox-doc-pipeline/src/lib.rs` today.\n",
        );
        let v = verify_claim_grounded(root, &registry, &claim);
        assert_eq!(v.verdict, ClaimVerdictKind::Uncertain, "{v:?}");
        assert!(
            v.evidence_citations
                .iter()
                .any(|c| c.starts_with("[crates/vox-doc-pipeline/src/lib.rs:L1-L")),
            "{v:?}"
        );
        assert!(
            v.evidence_citations
                .iter()
                .all(|c| !super::super::llm_verifier::is_doc_self_citation(doc, c)),
            "{v:?}"
        );
        assert_eq!(v.estimated_tokens, 0);
    }

    #[test]
    fn grounded_verifier_gives_no_evidence_for_a_missing_anchor() {
        let root = repo_root_for_tests();
        let registry = GroundTruthRegistry::load(root).expect("registry");
        let claim = claim_for(
            "docs/src/architecture/example.md",
            "Configured by `crates/vox-nonexistent-zzz/src/lib.rs` always.\n",
        );
        let v = verify_claim_grounded(root, &registry, &claim);
        assert_eq!(v.verdict, ClaimVerdictKind::Uncertain, "{v:?}");
        assert!(v.evidence_citations.is_empty(), "{v:?}");
    }

    struct FixedJudge(ClaimVerdictKind);
    impl ClaimEntailmentJudge for FixedJudge {
        fn judge(
            &self,
            _claim: &super::super::llm_verifier::DocClaimCandidate,
            evidence: &[EvidenceSpan],
        ) -> Option<super::super::llm_verifier::JudgeOutput> {
            assert!(!evidence.is_empty(), "judge must only see real evidence");
            Some(super::super::llm_verifier::JudgeOutput {
                verdict: self.0,
                rationale: "fake".to_string(),
                estimated_tokens: 7,
            })
        }
    }

    #[test]
    fn judge_verdict_is_accepted_only_with_real_non_self_evidence() {
        let root = repo_root_for_tests();
        let registry = GroundTruthRegistry::load(root).expect("registry");
        let doc = "docs/src/architecture/example.md";

        let grounded = claim_for(
            doc,
            "The entry point lives in `crates/vox-doc-pipeline/src/lib.rs` today.\n",
        );
        let judge = FixedJudge(ClaimVerdictKind::Verified);
        let v = verify_claim_grounded_with(root, &registry, &grounded, Some(&judge));
        assert_eq!(v.verdict, ClaimVerdictKind::Verified, "{v:?}");
        assert_eq!(v.estimated_tokens, 7);
        assert!(
            v.evidence_citations
                .iter()
                .all(|c| !super::super::llm_verifier::is_doc_self_citation(doc, c))
        );

        // Missing anchor: the judge must never be consulted (FixedJudge asserts non-empty evidence).
        let missing = claim_for(
            doc,
            "Configured by `crates/vox-nonexistent-zzz/src/lib.rs` always.\n",
        );
        let v = verify_claim_grounded_with(root, &registry, &missing, Some(&judge));
        assert_eq!(v.verdict, ClaimVerdictKind::Uncertain, "{v:?}");
    }

    #[test]
    fn inline_code_tokens_are_deduplicated_and_skip_unclosed_ticks() {
        let toks = inline_code_tokens("a `x/y` b `x/y` c `z` d `open");
        assert_eq!(toks, vec!["x/y".to_string(), "z".to_string()]);
    }

    #[test]
    fn bot_rejects_code_contracts_generated_and_archive_paths() {
        assert_eq!(
            validate_bot_target_path("crates/vox-db/src/lib.rs"),
            Err(BotPatchRejection::OutOfScopePath(
                "crates/vox-db/src/lib.rs".to_string()
            ))
        );
        assert_eq!(
            validate_bot_target_path("contracts/index.yaml"),
            Err(BotPatchRejection::OutOfScopePath(
                "contracts/index.yaml".to_string()
            ))
        );
        assert_eq!(
            validate_bot_target_path("docs/src/reference/cli.generated.md"),
            Err(BotPatchRejection::GeneratedDocForbidden(
                "docs/src/reference/cli.generated.md".to_string()
            ))
        );
        assert_eq!(
            validate_bot_target_path("docs/src/archive/old.md"),
            Err(BotPatchRejection::ArchiveDocForbidden(
                "docs/src/archive/old.md".to_string()
            ))
        );
        assert!(validate_bot_target_path("docs/src/architecture/db.md").is_ok());
    }

    #[test]
    fn bot_enforces_pre_pr_gates_concurrency_cap_and_in_place_updates() {
        let mut ledger = BotMetricsLedger::default();
        let p1 = valid_patch("db", "docs/src/architecture/db.md");
        let pr1 = ledger
            .plan_or_update_draft_pr(&p1)
            .expect("valid")
            .expect("opened");
        assert!(pr1.is_draft);
        assert!(!pr1.auto_merge);
        assert!(!pr1.updated_in_place);

        // Updating same subsystem updates in place rather than opening a duplicate PR.
        let pr1_update = ledger
            .plan_or_update_draft_pr(&p1)
            .expect("valid")
            .expect("updated");
        assert!(pr1_update.updated_in_place);
        assert_eq!(ledger.open_draft_branches.len(), 1);

        // Fill up to MAX_CONCURRENT_BOT_DRAFT_PRS (3).
        let p2 = valid_patch("mcp", "docs/src/architecture/mcp.md");
        let p3 = valid_patch("search", "docs/src/architecture/search.md");
        let p4 = valid_patch("cli", "docs/src/architecture/cli.md");
        assert!(ledger.plan_or_update_draft_pr(&p2).unwrap().is_some());
        assert!(ledger.plan_or_update_draft_pr(&p3).unwrap().is_some());
        assert!(ledger.plan_or_update_draft_pr(&p4).unwrap().is_none());
    }

    #[test]
    fn bot_rejects_a_patch_with_an_invalid_vox_diagram_fence() {
        let mut p = valid_patch("db", "docs/src/architecture/db.md");
        p.updated_markdown.push_str(
            "\n```vox-diagram\ngraph {\n  node a: \"A\"\n  node b: \"B\"\n  a -> b\n}\n```\n",
        );
        assert_eq!(validate_bot_patch_scope_and_gates(&p), Ok(()));

        p.updated_markdown
            .push_str("\n```vox-diagram\ngraph {\n  a -> \n}\n```\n");
        assert!(matches!(
            validate_bot_patch_scope_and_gates(&p),
            Err(BotPatchRejection::DiagramValidationFailed(_))
        ));
    }

    #[test]
    fn bot_auto_pauses_when_precision_drops_below_threshold() {
        let mut ledger = BotMetricsLedger {
            accepted_prs: 2,
            rejected_prs: 4,
            ..Default::default()
        };
        assert!(ledger.is_auto_paused());
        let p = valid_patch("db", "docs/src/architecture/db.md");
        assert!(ledger.plan_or_update_draft_pr(&p).unwrap().is_none());
    }
}
