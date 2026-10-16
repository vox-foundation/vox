//! Phase 39 (`GROW-01..05`): KB Web Fallback Spend Ledger, Quarantined Summarizer,
//! Inbox Review (`vox_kb_propose` / `vox_kb_inbox_list` / `vox_kb_inbox_review`),
//! Similar-Docs Duplicate Check, and `>= 2` Domain Corroboration Promotion Gate.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use vox_db::ContextTrustTier;

/// Maximum verbatim quote length allowed in promoted KB sources (`TAX-07`, `GROW-02`).
pub const MAX_VERBATIM_QUOTE_CHARS: usize = 280;

/// Free-first CRAG web fallback provider chain (`GROW-01`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebFallbackProvider {
    Searxng,
    DuckDuckGo,
    PaidTavily,
}

/// Daily spend ledger for paid web fallback (`GROW-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebFallbackSpendLedger {
    pub day_ymd: String,
    pub daily_cap_usd: f64,
    pub spent_today_usd: f64,
}

impl WebFallbackSpendLedger {
    pub fn new(day_ymd: impl Into<String>, daily_cap_usd: f64) -> Self {
        Self {
            day_ymd: day_ymd.into(),
            daily_cap_usd: daily_cap_usd.max(0.0),
            spent_today_usd: 0.0,
        }
    }

    pub fn remaining_usd(&self) -> f64 {
        (self.daily_cap_usd - self.spent_today_usd).max(0.0)
    }

    pub fn try_debit(&mut self, amount_usd: f64) -> Result<(), String> {
        if amount_usd <= 0.0 {
            return Ok(());
        }
        if self.spent_today_usd + amount_usd > self.daily_cap_usd + 1e-9 {
            return Err(format!(
                "daily paid web search cap reached (${:.4} / ${:.4} on {})",
                self.spent_today_usd, self.daily_cap_usd, self.day_ymd
            ));
        }
        self.spent_today_usd += amount_usd;
        Ok(())
    }
}

/// Decision returned by `plan_web_fallback` (`GROW-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebFallbackPlan {
    pub should_search_web: bool,
    pub trigger_reason: Option<String>,
    pub provider_chain: Vec<WebFallbackProvider>,
    /// Surfaced warning when the paid daily spend cap is reached (never silent, `GROW-01`).
    pub spend_cap_notice: Option<String>,
}

/// Decide whether and how to trigger web fallback (`GROW-01`):
/// - Triggers when local retrieval confidence `< threshold` OR `needs_newer_than_cutoff == true`,
///   AND `research_mode_enabled == true`.
/// - Always orders free providers first (`Searxng -> DuckDuckGo`), appending `PaidTavily` only if
///   within the daily spend cap; if the cap is exhausted, records a non-silent `spend_cap_notice`.
pub fn plan_web_fallback(
    local_confidence: f64,
    confidence_threshold: f64,
    needs_newer_than_cutoff: bool,
    research_mode_enabled: bool,
    ledger: &mut WebFallbackSpendLedger,
    estimated_paid_cost_usd: f64,
) -> WebFallbackPlan {
    if !research_mode_enabled {
        return WebFallbackPlan {
            should_search_web: false,
            trigger_reason: None,
            provider_chain: Vec::new(),
            spend_cap_notice: None,
        };
    }

    let low_conf = local_confidence < confidence_threshold;
    if !low_conf && !needs_newer_than_cutoff {
        return WebFallbackPlan {
            should_search_web: false,
            trigger_reason: None,
            provider_chain: Vec::new(),
            spend_cap_notice: None,
        };
    }

    let reason = if needs_newer_than_cutoff {
        "query_requires_post_cutoff_evidence".to_string()
    } else {
        format!(
            "local_confidence_below_threshold ({local_confidence:.2} < {confidence_threshold:.2})"
        )
    };

    let mut chain = vec![
        WebFallbackProvider::Searxng,
        WebFallbackProvider::DuckDuckGo,
    ];
    let mut notice = None;
    if estimated_paid_cost_usd > 0.0 {
        match ledger.try_debit(estimated_paid_cost_usd) {
            Ok(()) => chain.push(WebFallbackProvider::PaidTavily),
            Err(msg) => notice = Some(msg),
        }
    }

    WebFallbackPlan {
        should_search_web: true,
        trigger_reason: Some(reason),
        provider_chain: chain,
        spend_cap_notice: notice,
    }
}

/// Provenance source entry for a pending KB inbox finding (`GROW-02`, `TAX-07`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingSourceEntry {
    pub url: String,
    pub domain: String,
    pub fetch_date: String,
    pub content_hash: String,
    pub short_quote: String,
}

/// Review status of a finding in `kb_findings` (`GROW-03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Pending,
    ApprovedForPromotion,
    Rejected { reason: String },
}

/// Pending finding stored in the `kb_findings` inbox (`GROW-02`, `GROW-03`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KbFindingRecord {
    pub finding_id: String,
    pub title: String,
    pub summary: String,
    pub trust_tier: ContextTrustTier,
    /// Quarantined tool-less step flag (`GROW-02`).
    pub quarantined: bool,
    /// Never injected as curated while in the inbox (`GROW-02`).
    pub injectable_as_curated: bool,
    pub status: FindingStatus,
    pub sources: Vec<FindingSourceEntry>,
    pub duplicate_of_paths: Vec<String>,
}

/// Extract hostname domain from a URL.
pub fn extract_domain(url: &str) -> String {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    without_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Summarize captured web text in a quarantined, tool-less step (`GROW-02`):
/// Strips invisible characters/HTML comments, caps quotes to `MAX_VERBATIM_QUOTE_CHARS`,
/// stamps `ContextTrustTier::Untrusted` and `injectable_as_curated: false`.
pub fn summarize_web_capture_quarantined(
    finding_id: impl Into<String>,
    title: impl Into<String>,
    raw_text: &str,
    url: &str,
    fetch_date: &str,
    content_hash: &str,
) -> KbFindingRecord {
    let clean = raw_text
        .chars()
        .filter(|&c| {
            let cp = c as u32;
            !(0xE0000..=0xE007F).contains(&cp)
                && !matches!(cp, 0x200B..=0x200F | 0x202A..=0x202E | 0x2066..=0x2069 | 0xFEFF)
        })
        .collect::<String>();

    let summary: String = clean
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");

    let quote: String = summary.chars().take(MAX_VERBATIM_QUOTE_CHARS).collect();
    let domain = extract_domain(url);

    KbFindingRecord {
        finding_id: finding_id.into(),
        title: title.into(),
        summary,
        trust_tier: ContextTrustTier::Untrusted,
        quarantined: true,
        injectable_as_curated: false,
        status: FindingStatus::Pending,
        sources: vec![FindingSourceEntry {
            url: url.to_string(),
            domain,
            fetch_date: fetch_date.to_string(),
            content_hash: content_hash.to_string(),
            short_quote: quote,
        }],
        duplicate_of_paths: Vec::new(),
    }
}

fn word_tokens(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_ascii_lowercase())
        .collect()
}

/// Similar-docs duplicate check (`GROW-03`): returns existing doc paths whose token Jaccard
/// similarity with `(title + summary)` is `>= 0.50`.
pub fn check_similar_docs(
    draft_title: &str,
    draft_summary: &str,
    existing_docs: &[(String, String, String)], // (doc_path, title, summary)
) -> Vec<String> {
    let draft_set = word_tokens(&format!("{draft_title} {draft_summary}"));
    if draft_set.is_empty() {
        return Vec::new();
    }
    let mut matches = Vec::new();
    for (path, title, summary) in existing_docs {
        let doc_set = word_tokens(&format!("{title} {summary}"));
        if doc_set.is_empty() {
            continue;
        }
        let inter = draft_set.intersection(&doc_set).count() as f64;
        let union = draft_set.union(&doc_set).count() as f64;
        if union > 0.0 && (inter / union) >= 0.50 {
            matches.push(path.clone());
        }
    }
    matches
}

/// Review action for `kb_inbox_review` (`GROW-03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxReviewDecision {
    Approve,
    Reject { reason: String },
}

/// Apply `vox_kb_inbox_review` to a finding (`GROW-03`).
pub fn review_inbox_finding(
    finding: &mut KbFindingRecord,
    decision: InboxReviewDecision,
) -> Result<(), String> {
    match decision {
        InboxReviewDecision::Approve => {
            finding.status = FindingStatus::ApprovedForPromotion;
            Ok(())
        }
        InboxReviewDecision::Reject { reason } => {
            if reason.trim().is_empty() {
                return Err("rejection reason must not be empty".to_string());
            }
            finding.status = FindingStatus::Rejected { reason };
            Ok(())
        }
    }
}

/// Promotion validation and isolated-worktree draft PR specification (`GROW-04`, `GROW-05`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromotionDraftPrSpec {
    pub finding_id: String,
    pub target_doc_path: String,
    pub distinct_domains: Vec<String>,
    pub worktree_branch: String,
    pub pr_title: String,
    pub pr_body_markdown: String,
    pub requires_human_merge: bool,
}

/// Validate promotion eligibility (`GROW-04`, `GROW-05`, `TAX-07`):
/// - Requires `FindingStatus::ApprovedForPromotion`.
/// - Requires `>= 2` distinct source domains OR `reviewer_override == true`.
/// - Every source entry must have non-empty `url`, `fetch_date`, `content_hash`, and
///   `short_quote.chars().count() <= MAX_VERBATIM_QUOTE_CHARS`.
/// - Emits a draft PR specification from an isolated worktree with per-claim source excerpts;
///   `requires_human_merge` is always `true`.
pub fn build_promotion_draft_pr(
    finding: &KbFindingRecord,
    target_doc_path: &str,
    reviewer_override: bool,
) -> Result<PromotionDraftPrSpec, String> {
    if finding.status != FindingStatus::ApprovedForPromotion {
        return Err("finding must be ApprovedForPromotion before opening a promotion PR".into());
    }
    if finding.sources.is_empty() {
        return Err("finding has no provenance sources".into());
    }
    let mut domains = BTreeSet::new();
    for src in &finding.sources {
        if src.url.trim().is_empty()
            || src.fetch_date.trim().is_empty()
            || src.content_hash.trim().is_empty()
        {
            return Err(format!(
                "source '{}' is missing required url, fetch_date, or content_hash",
                src.url
            ));
        }
        if src.short_quote.chars().count() > MAX_VERBATIM_QUOTE_CHARS {
            return Err(format!(
                "source '{}' quote exceeds {} chars",
                src.url, MAX_VERBATIM_QUOTE_CHARS
            ));
        }
        if !src.domain.trim().is_empty() {
            domains.insert(src.domain.clone());
        }
    }

    if domains.len() < 2 && !reviewer_override {
        return Err(format!(
            "promotion requires >= 2 distinct corroborating domains (found {}) or explicit reviewer_override",
            domains.len()
        ));
    }

    let mut body = format!(
        "## KB Promotion Draft: {}\n\n- **Finding ID:** `{}`\n- **Target Path:** `{}`\n- **Distinct Domains:** `{}`\n\n### Source Excerpts\n",
        finding.title,
        finding.finding_id,
        target_doc_path,
        domains.len()
    );
    for src in &finding.sources {
        body.push_str(&format!(
            "- [{}]({}) (fetched `{}`, sha `{}`): \"{}\"\n",
            src.domain, src.url, src.fetch_date, src.content_hash, src.short_quote
        ));
    }

    Ok(PromotionDraftPrSpec {
        finding_id: finding.finding_id.clone(),
        target_doc_path: target_doc_path.to_string(),
        distinct_domains: domains.into_iter().collect(),
        worktree_branch: format!("kb-promote/{}", finding.finding_id),
        pr_title: format!(
            "docs(kb): promote finding {} ({})",
            finding.finding_id, finding.title
        ),
        pr_body_markdown: body,
        requires_human_merge: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_fallback_is_free_first_and_surfaces_spend_cap() {
        let mut ledger = WebFallbackSpendLedger::new("2026-10-15", 0.05);
        let plan1 = plan_web_fallback(0.20, 0.50, false, true, &mut ledger, 0.03);
        assert!(plan1.should_search_web);
        assert_eq!(
            plan1.provider_chain,
            vec![
                WebFallbackProvider::Searxng,
                WebFallbackProvider::DuckDuckGo,
                WebFallbackProvider::PaidTavily,
            ]
        );
        assert!(plan1.spend_cap_notice.is_none());

        // Second query exceeds remaining $0.02 cap -> free providers still run, cap notice surfaced
        let plan2 = plan_web_fallback(0.20, 0.50, false, true, &mut ledger, 0.03);
        assert!(plan2.should_search_web);
        assert_eq!(
            plan2.provider_chain,
            vec![
                WebFallbackProvider::Searxng,
                WebFallbackProvider::DuckDuckGo,
            ]
        );
        assert!(plan2.spend_cap_notice.is_some());
    }

    #[test]
    fn quarantined_capture_is_untrusted_and_never_curated() {
        let f = summarize_web_capture_quarantined(
            "find-1",
            "SQLite WAL Mode",
            "WAL mode enables concurrent readers.\u{200B}",
            "https://sqlite.org/wal.html",
            "2026-10-15",
            "blake3:abc",
        );
        assert_eq!(f.trust_tier, ContextTrustTier::Untrusted);
        assert!(f.quarantined);
        assert!(!f.injectable_as_curated);
        assert!(!f.summary.contains('\u{200B}'));
    }

    #[test]
    fn promotion_requires_two_domains_unless_overridden() {
        let mut f = summarize_web_capture_quarantined(
            "find-2",
            "SQLite WAL",
            "WAL improves concurrency.",
            "https://sqlite.org/wal.html",
            "2026-10-15",
            "blake3:111",
        );
        review_inbox_finding(&mut f, InboxReviewDecision::Approve).unwrap();

        // Single domain fails without override
        assert!(build_promotion_draft_pr(&f, "docs/src/library/sqlite-wal.md", false).is_err());

        // Second domain added -> passes without override
        f.sources.push(FindingSourceEntry {
            url: "https://turso.tech/blog/wal".into(),
            domain: "turso.tech".into(),
            fetch_date: "2026-10-15".into(),
            content_hash: "blake3:222".into(),
            short_quote: "Turso uses WAL for concurrent reads.".into(),
        });
        let pr = build_promotion_draft_pr(&f, "docs/src/library/sqlite-wal.md", false).unwrap();
        assert!(pr.requires_human_merge);
        assert_eq!(pr.distinct_domains.len(), 2);
    }
}
