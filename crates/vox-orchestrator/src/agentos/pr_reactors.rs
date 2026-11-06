//! Pull Request sync, exponential-backoff CI watch, and structured review guide services (`T3-WORKTREE-VCS-04`).

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Base interval for PR check watching (`60_000 ms = 60s`).
pub const PR_WATCH_BASE_DELAY_MS: u64 = 60_000;
/// Maximum interval cap for PR check watching (`900_000 ms = 15m`).
pub const PR_WATCH_MAX_DELAY_MS: u64 = 900_000;

/// Computes exponential backoff `min(60s * 2^(attempt-1), 15m)` with deterministic jitter in `[0.8, 1.2]`
/// parameterized by `jitter_unit ∈ [0.0, 1.0]` (`T3-WORKTREE-VCS-04`).
#[must_use]
pub fn compute_pr_watch_backoff_ms(attempt: u32, jitter_unit: f64) -> u64 {
    let shift = attempt.saturating_sub(1).min(10);
    let raw = PR_WATCH_BASE_DELAY_MS.saturating_mul(1u64 << shift);
    let capped = raw.min(PR_WATCH_MAX_DELAY_MS);
    let clamped_unit = jitter_unit.clamp(0.0, 1.0);
    let factor = 0.8 + 0.4 * clamped_unit;
    ((capped as f64) * factor).round() as u64
}

/// Tracks synced Pull Request metadata per thread (`T3-WORKTREE-VCS-04`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestBinding {
    pub thread_id: String,
    pub pr_number: u64,
    pub head_branch: String,
    pub base_branch: String,
    pub head_sha: String,
}

#[derive(Debug, Clone, Default)]
pub struct PullRequestSyncReactor {
    bindings: HashMap<String, PullRequestBinding>,
}

impl PullRequestSyncReactor {
    pub fn upsert_binding(&mut self, binding: PullRequestBinding) {
        self.bindings.insert(binding.thread_id.clone(), binding);
    }

    #[must_use]
    pub fn binding(&self, thread_id: &str) -> Option<&PullRequestBinding> {
        self.bindings.get(thread_id)
    }
}

/// Deduplicating CI check failure watcher (`T3-WORKTREE-VCS-04`).
#[derive(Debug, Clone, Default)]
pub struct PullRequestWatchReactor {
    reported_failure_keys: HashSet<String>,
}

impl PullRequestWatchReactor {
    /// Returns `true` if `(pr_number, head_sha, check_name)` has not yet been reported as failed,
    /// recording it so duplicate failure notifications are suppressed.
    pub fn should_dispatch_failure_followup(
        &mut self,
        pr_number: u64,
        head_sha: &str,
        check_name: &str,
    ) -> bool {
        let key = format!("{pr_number}:{head_sha}:{check_name}");
        self.reported_failure_keys.insert(key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewGuideSection {
    pub title: String,
    pub files: Vec<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestReviewGuide {
    pub pr_number: u64,
    pub head_sha: String,
    pub summary: String,
    pub sections: Vec<ReviewGuideSection>,
}

/// Generates and caches structured PR review guides keyed by `(pr_number, head_sha)` (`T3-WORKTREE-VCS-04`).
#[derive(Debug, Clone, Default)]
pub struct PullRequestReviewGuideService {
    cache: HashMap<(u64, String), PullRequestReviewGuide>,
}

impl PullRequestReviewGuideService {
    pub fn get_or_generate(
        &mut self,
        pr_number: u64,
        head_sha: &str,
        changed_files: &[String],
    ) -> PullRequestReviewGuide {
        let key = (pr_number, head_sha.to_string());
        if let Some(existing) = self.cache.get(&key) {
            return existing.clone();
        }
        let guide = PullRequestReviewGuide {
            pr_number,
            head_sha: head_sha.to_string(),
            summary: format!(
                "PR #{pr_number} ({head_sha}) touches {} file(s).",
                changed_files.len()
            ),
            sections: vec![ReviewGuideSection {
                title: "Primary Changes".to_string(),
                files: changed_files.to_vec(),
                rationale: "Ordered for top-down architectural review.".to_string(),
            }],
        };
        self.cache.insert(key, guide.clone());
        guide
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pr_watch_backoff_follows_60s_exponential_curve_capped_at_15m_with_jitter() {
        // Attempt 1 at midpoint jitter (0.5 -> factor 1.0) = 60_000ms
        assert_eq!(compute_pr_watch_backoff_ms(1, 0.5), 60_000);
        // Attempt 2 at midpoint = 120_000ms
        assert_eq!(compute_pr_watch_backoff_ms(2, 0.5), 120_000);
        // Attempt 5 (60s * 16 = 960s -> capped at 900_000ms)
        assert_eq!(compute_pr_watch_backoff_ms(5, 0.5), 900_000);
        // Jitter bounds [0.8, 1.2]
        assert_eq!(compute_pr_watch_backoff_ms(1, 0.0), 48_000);
        assert_eq!(compute_pr_watch_backoff_ms(1, 1.0), 72_000);
    }

    #[test]
    fn pr_watch_deduplicates_identical_check_failures() {
        let mut watch = PullRequestWatchReactor::default();
        assert!(watch.should_dispatch_failure_followup(10, "sha1", "cargo-test"));
        assert!(!watch.should_dispatch_failure_followup(10, "sha1", "cargo-test"));
        assert!(watch.should_dispatch_failure_followup(10, "sha2", "cargo-test"));
    }
}
