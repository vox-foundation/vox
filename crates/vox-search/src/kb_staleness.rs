//! Phase 41 (`STALE-01..04`): KB Staleness Detection, Hash-Jittered `review_by`,
//! Mechanical Re-Verification Gate, and Demote-to-Legacy Lifecycle.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Deterministic FNV-1a 64-bit hash used to jitter `review_by` horizons (`STALE-01`)
/// so docs edited on the same day do not all expire on the exact same date.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Base horizon in days by facet (`kb_tier` / `scope`) before hash jitter (`STALE-01`).
pub fn base_horizon_days(kb_tier: Option<&str>, scope: Option<&str>) -> u32 {
    match (kb_tier.unwrap_or("reference"), scope.unwrap_or("vox")) {
        (_, "external") => 30,
        ("ssot" | "architecture-ssot", _) => 60,
        ("tutorial" | "how-to", _) => 90,
        ("archive" | "historical", _) => 365,
        _ => 120,
    }
}

/// Deterministic jitter in days `[-7, +7]` derived from `doc_path` (`STALE-01`).
pub fn jitter_days_for_path(doc_path: &str) -> i32 {
    let h = fnv1a64(doc_path.as_bytes());
    ((h % 15) as i32) - 7
}

/// Parse `YYYY-MM-DD` to days since Unix epoch (civil calendar arithmetic).
pub fn parse_ymd_to_epoch_days(ymd: &str) -> Option<i64> {
    let parts: Vec<&str> = ymd.trim().split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i64 = parts[0].parse().ok()?;
    let m: i64 = parts[1].parse().ok()?;
    let d: i64 = parts[2].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Howard Hinnant's civil_from_days / days_from_civil algorithm
    let y_adj = y - if m <= 2 { 1 } else { 0 };
    let era = if y_adj >= 0 { y_adj } else { y_adj - 399 } / 400;
    let yoe = y_adj - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

/// Format days since Unix epoch as `YYYY-MM-DD`.
pub fn format_epoch_days_to_ymd(days: i64) -> String {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y_final = y + if m <= 2 { 1 } else { 0 };
    format!("{y_final:04}-{m:02}-{d:02}")
}

/// Compute hash-jittered `review_by` date (`YYYY-MM-DD`) from `verified_at` (`STALE-01`).
pub fn compute_review_by_date(
    doc_path: &str,
    verified_at_ymd: &str,
    kb_tier: Option<&str>,
    scope: Option<&str>,
) -> Option<String> {
    let base_days = parse_ymd_to_epoch_days(verified_at_ymd)?;
    let horizon = base_horizon_days(kb_tier, scope) as i64;
    let jitter = jitter_days_for_path(doc_path) as i64;
    let total = (horizon + jitter).max(7);
    Some(format_epoch_days_to_ymd(base_days + total))
}

/// Returns `Some(days_overdue)` when `today_ymd > review_by_ymd`, or `None` if current (`STALE-01`).
pub fn days_overdue(review_by_ymd: &str, today_ymd: &str) -> Option<u32> {
    let rev = parse_ymd_to_epoch_days(review_by_ymd)?;
    let now = parse_ymd_to_epoch_days(today_ymd)?;
    if now > rev {
        Some((now - rev) as u32)
    } else {
        None
    }
}

/// Down-ranking multiplier for overdue docs (`STALE-01`):
/// Overdue docs show a `stale` badge and are gently down-ranked (`0.75..0.95`), never hidden (`> 0.0`).
pub fn staleness_rank_multiplier(days_overdue: Option<u32>) -> f64 {
    match days_overdue {
        None | Some(0) => 1.0,
        Some(1..=30) => 0.90,
        Some(31..=90) => 0.82,
        Some(_) => 0.75,
    }
}

/// Candidate item for the bounded `@scheduled` verification queue (`STALE-02`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationQueueCandidate {
    pub doc_path: String,
    pub review_by: String,
    pub retrieval_usage_count: u64,
    pub estimated_tokens: usize,
}

/// Prioritized verification queue entry ordered by `usage * days_overdue` (`STALE-02`, `GUI-KB-07`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationQueueItem {
    pub doc_path: String,
    pub days_overdue: u32,
    pub retrieval_usage_count: u64,
    pub priority_score: u64,
    pub estimated_tokens: usize,
}

/// Build the bounded re-verification queue ordered by `(usage.max(1)) * days_overdue` descending (`STALE-02`).
pub fn build_verification_queue(
    candidates: &[VerificationQueueCandidate],
    today_ymd: &str,
    max_pages: usize,
    max_token_budget: usize,
) -> Vec<VerificationQueueItem> {
    let mut overdue_items: Vec<VerificationQueueItem> = candidates
        .iter()
        .filter_map(|c| {
            let overdue = days_overdue(&c.review_by, today_ymd)?;
            let usage_weight = c.retrieval_usage_count.max(1);
            let priority_score = usage_weight.saturating_mul(overdue as u64);
            Some(VerificationQueueItem {
                doc_path: c.doc_path.clone(),
                days_overdue: overdue,
                retrieval_usage_count: c.retrieval_usage_count,
                priority_score,
                estimated_tokens: c.estimated_tokens,
            })
        })
        .collect();

    overdue_items.sort_by(|a, b| {
        b.priority_score
            .cmp(&a.priority_score)
            .then_with(|| a.doc_path.cmp(&b.doc_path))
    });

    let mut selected = Vec::new();
    let mut used_tokens = 0usize;
    for item in overdue_items {
        if selected.len() >= max_pages {
            break;
        }
        if used_tokens + item.estimated_tokens > max_token_budget && !selected.is_empty() {
            continue;
        }
        used_tokens += item.estimated_tokens;
        selected.push(item);
    }
    selected
}

/// Input to the mechanical evidence verifier (`STALE-03`).
#[derive(Debug, Clone, Default)]
pub struct MechanicalEvidenceInput {
    pub doc_path: String,
    pub referenced_paths: Vec<String>,
    pub referenced_symbols: Vec<String>,
    pub referenced_cli_commands: Vec<String>,
    pub referenced_links: Vec<String>,
    pub existing_paths: BTreeSet<String>,
    pub resolved_graph_symbols: BTreeSet<String>,
    pub valid_cli_commands: BTreeSet<String>,
    pub valid_links: BTreeSet<String>,
    /// Optional advisory LLM verdict (`true` = claims look valid, `false` = prose drift suspected).
    pub llm_advisory_verdict: Option<bool>,
}

/// Outcome of `verify_doc_mechanically` (`STALE-03`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MechanicalVerificationResult {
    pub doc_path: String,
    pub mechanical_passed: bool,
    pub advance_verified_at: bool,
    pub open_inbox_finding: bool,
    pub missing_paths: Vec<String>,
    pub unresolved_symbols: Vec<String>,
    pub missing_cli_commands: Vec<String>,
    pub broken_links: Vec<String>,
}

/// Verify a document mechanically (`STALE-03`):
/// - `verified_at` advances ONLY when all mechanical checks pass (`missing_paths`, `unresolved_symbols`,
///   `missing_cli_commands`, `broken_links` are all empty).
/// - An LLM verdict alone NEVER advances `verified_at`; if mechanical checks fail or the LLM flags
///   prose drift, `open_inbox_finding` is set to `true`.
pub fn verify_doc_mechanically(
    input: &MechanicalEvidenceInput,
    has_mechanical_anchors: bool,
) -> MechanicalVerificationResult {
    let missing_paths: Vec<String> = input
        .referenced_paths
        .iter()
        .filter(|p| !input.existing_paths.contains(*p))
        .cloned()
        .collect();
    let unresolved_symbols: Vec<String> = input
        .referenced_symbols
        .iter()
        .filter(|s| !input.resolved_graph_symbols.contains(*s))
        .cloned()
        .collect();
    let missing_cli_commands: Vec<String> = input
        .referenced_cli_commands
        .iter()
        .filter(|c| !input.valid_cli_commands.contains(*c))
        .cloned()
        .collect();
    let broken_links: Vec<String> = input
        .referenced_links
        .iter()
        .filter(|l| !input.valid_links.contains(*l))
        .cloned()
        .collect();

    let all_checks_clean = missing_paths.is_empty()
        && unresolved_symbols.is_empty()
        && missing_cli_commands.is_empty()
        && broken_links.is_empty();

    let mechanical_passed = has_mechanical_anchors && all_checks_clean;
    let llm_flagged_drift = input.llm_advisory_verdict == Some(false);
    let advance_verified_at = mechanical_passed && !llm_flagged_drift;
    let open_inbox_finding = !all_checks_clean || llm_flagged_drift;

    MechanicalVerificationResult {
        doc_path: input.doc_path.clone(),
        mechanical_passed,
        advance_verified_at,
        open_inbox_finding,
        missing_paths,
        unresolved_symbols,
        missing_cli_commands,
        broken_links,
    }
}

/// Demote a superseded or failed doc to `status: "legacy"` without deleting it (`STALE-04`).
pub fn demote_to_legacy_markdown(markdown: &str, superseded_by: Option<&str>) -> String {
    let Some(rest) = markdown.strip_prefix("---\n") else {
        return markdown.to_string();
    };
    let Some(end_idx) = rest.find("\n---") else {
        return markdown.to_string();
    };
    let fm = &rest[..end_idx];
    let after_fm = &rest[end_idx + 4..];

    let mut lines: Vec<String> = Vec::new();
    let mut had_status = false;
    let mut had_superseded = false;
    for line in fm.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("status:") {
            lines.push("status: \"legacy\"".to_string());
            had_status = true;
        } else if trimmed.starts_with("superseded_by:") {
            if let Some(target) = superseded_by {
                lines.push(format!("superseded_by: \"{target}\""));
            } else {
                lines.push(line.to_string());
            }
            had_superseded = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !had_status {
        lines.push("status: \"legacy\"".to_string());
    }
    if !had_superseded && let Some(target) = superseded_by {
        lines.push(format!("superseded_by: \"{target}\""));
    }
    format!("---\n{}\n---{}", lines.join("\n"), after_fm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_by_is_deterministic_and_jittered() {
        let d1 = compute_review_by_date(
            "docs/src/architecture/a.md",
            "2026-10-01",
            Some("ssot"),
            Some("vox"),
        )
        .unwrap();
        let d2 = compute_review_by_date(
            "docs/src/architecture/b.md",
            "2026-10-01",
            Some("ssot"),
            Some("vox"),
        )
        .unwrap();
        assert_eq!(
            d1,
            compute_review_by_date(
                "docs/src/architecture/a.md",
                "2026-10-01",
                Some("ssot"),
                Some("vox")
            )
            .unwrap()
        );
        // External docs have a shorter horizon (~30d) than architecture SSOT (~60d).
        let ext = compute_review_by_date(
            "docs/src/library/ext.md",
            "2026-10-01",
            Some("reference"),
            Some("external"),
        )
        .unwrap();
        assert!(ext < d1);
        assert!(ext < d2);
    }

    #[test]
    fn overdue_docs_are_down_ranked_never_zeroed() {
        let overdue = days_overdue("2026-09-01", "2026-10-15");
        assert_eq!(overdue, Some(44));
        let mult = staleness_rank_multiplier(overdue);
        assert!(mult > 0.0 && mult < 1.0);
    }

    #[test]
    fn verification_queue_orders_by_usage_times_days_overdue() {
        let candidates = vec![
            VerificationQueueCandidate {
                doc_path: "docs/low.md".into(),
                review_by: "2026-10-05".into(),
                retrieval_usage_count: 2,
                estimated_tokens: 500,
            },
            VerificationQueueCandidate {
                doc_path: "docs/high.md".into(),
                review_by: "2026-10-10".into(),
                retrieval_usage_count: 50,
                estimated_tokens: 500,
            },
        ];
        let q = build_verification_queue(&candidates, "2026-10-15", 10, 2000);
        assert_eq!(q.len(), 2);
        assert_eq!(q[0].doc_path, "docs/high.md");
        assert_eq!(q[0].priority_score, 50 * 5);
    }

    #[test]
    fn verified_at_requires_mechanical_anchors_not_llm_alone() {
        let mut input = MechanicalEvidenceInput {
            doc_path: "docs/test.md".into(),
            llm_advisory_verdict: Some(true),
            ..Default::default()
        };
        // LLM alone without mechanical anchors cannot advance verified_at
        let r1 = verify_doc_mechanically(&input, false);
        assert!(!r1.advance_verified_at);

        // Mechanical anchor present and resolves -> advances verified_at
        input
            .referenced_paths
            .push("crates/vox-search/src/lib.rs".into());
        input
            .existing_paths
            .insert("crates/vox-search/src/lib.rs".into());
        let r2 = verify_doc_mechanically(&input, true);
        assert!(r2.advance_verified_at);
        assert!(!r2.open_inbox_finding);
    }

    #[test]
    fn demote_to_legacy_updates_status_without_deleting_content() {
        let md = "---\ntitle: \"Old\"\nstatus: \"current\"\n---\n# Old\nBody text.";
        let demoted = demote_to_legacy_markdown(md, Some("docs/src/new.md"));
        assert!(demoted.contains("status: \"legacy\""));
        assert!(demoted.contains("superseded_by: \"docs/src/new.md\""));
        assert!(demoted.contains("Body text."));
    }
}
