//! Persisted record shapes for `vox harness eval --live` runs (chat harness continuous eval
//! design, 2026-08-02). Mirrors `research.rs`'s `ResearchEvalRunRecord`/`ResearchEvalSampleRecord`
//! split: one row per eval invocation, N child rows per golden task result, plus a per-model-
//! selection-decision child table for cost-tier drift tracking.

use serde::{Deserialize, Serialize};

/// One row per `vox harness eval --live` invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessEvalRunRecord {
    pub run_id: String,
    pub triggered_by: String,
    pub git_sha: String,
    pub git_branch: String,
    pub changed_files: Vec<String>,
    pub config_version: Option<String>,
    pub samples_per_task: i64,
    pub task_count: i64,
    pub pass_count: i64,
    pub fail_count: i64,
    pub skip_count: i64,
    pub total_cost_usd: f64,
    pub started_at_ms: i64,
    pub finished_at_ms: i64,
}

/// One row per golden task per run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessEvalTaskResultRecord {
    pub run_id: String,
    pub task_id: String,
    pub category: String,
    pub checker_kind: String,
    pub status: String,
    pub pass_samples: i64,
    pub total_samples: i64,
    pub latency_p50_ms: Option<i64>,
    pub cost_usd: Option<f64>,
    pub failure_detail: Option<String>,
    pub recorded_at_ms: i64,
}

/// One row per model-selection decision observed during a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelectionEventRecord {
    pub run_id: String,
    pub task_id: String,
    pub model_id: String,
    pub cost_tier: String,
    pub selection_reason: String,
    pub was_privacy_gated: bool,
    pub recorded_at_ms: i64,
}

// ── Regression detection (vox harness report + the Harness Health surface) ──────────────────

/// A detected regression between two consecutive runs. `flipped_task_ids` is populated only for
/// `RegressionKind::TaskFlippedToFail` (design spec §10.3's "specific task/selection rows that
/// changed" requirement) — empty for the aggregate-threshold kinds.
#[derive(Debug, Clone, PartialEq)]
pub struct RegressionFlag {
    pub kind: RegressionKind,
    pub previous_run_id: String,
    pub current_run_id: String,
    pub previous_git_sha: String,
    pub current_git_sha: String,
    pub changed_files: Vec<String>,
    pub flipped_task_ids: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionKind {
    /// A single task went from pass in the previous run to fail in the current run — flagged
    /// independent of the aggregate pass-rate threshold below, since an aggregate percentage can
    /// mask one task flipping fail while a different task happens to flip pass in the same run.
    TaskFlippedToFail,
    PassRateDrop,
    CostTierRatioDrop,
}

const PASS_RATE_DROP_THRESHOLD_PP: f64 = 10.0;
const COST_TIER_RATIO_DROP_THRESHOLD_PP: f64 = 15.0;

/// Pass rate (percent) over graded tasks; a run with nothing graded counts as 100%.
pub fn pass_rate(run: &HarnessEvalRunRecord) -> f64 {
    let graded = run.task_count - run.skip_count;
    if graded <= 0 {
        return 100.0;
    }
    (run.pass_count as f64 / graded as f64) * 100.0
}

/// Percent of non-privacy-gated selections that picked a free or cheap tier (100% when none).
pub fn free_cheap_ratio(events: &[ModelSelectionEventRecord]) -> f64 {
    let non_privacy_forced: Vec<_> = events.iter().filter(|e| !e.was_privacy_gated).collect();
    if non_privacy_forced.is_empty() {
        return 100.0;
    }
    let free_or_cheap = non_privacy_forced
        .iter()
        .filter(|e| e.cost_tier == "free" || e.cost_tier == "cheap")
        .count();
    (free_or_cheap as f64 / non_privacy_forced.len() as f64) * 100.0
}

/// Compare two consecutive runs, their selection events, AND their per-task result lists,
/// returning any regressions detected. Pure function — no DB access — so it's fully unit-testable
/// against fixture data (design spec §12). Aggregate-only comparison (the two runs' pass_count/
/// task_count alone) cannot answer "which task regressed" — that's why `previous_task_results`/
/// `current_task_results` are required inputs, not optional ones.
pub fn detect_regressions(
    previous: &HarnessEvalRunRecord,
    current: &HarnessEvalRunRecord,
    previous_task_results: &[HarnessEvalTaskResultRecord],
    current_task_results: &[HarnessEvalTaskResultRecord],
    previous_events: &[ModelSelectionEventRecord],
    current_events: &[ModelSelectionEventRecord],
    changed_files: &[String],
) -> Vec<RegressionFlag> {
    let mut flags = Vec::new();

    let prev_status_by_task: std::collections::HashMap<&str, &str> = previous_task_results
        .iter()
        .map(|t| (t.task_id.as_str(), t.status.as_str()))
        .collect();
    let flipped_task_ids: Vec<String> = current_task_results
        .iter()
        .filter(|t| {
            t.status == "fail" && prev_status_by_task.get(t.task_id.as_str()) == Some(&"pass")
        })
        .map(|t| t.task_id.clone())
        .collect();
    if !flipped_task_ids.is_empty() {
        flags.push(RegressionFlag {
            kind: RegressionKind::TaskFlippedToFail,
            previous_run_id: previous.run_id.clone(),
            current_run_id: current.run_id.clone(),
            previous_git_sha: previous.git_sha.clone(),
            current_git_sha: current.git_sha.clone(),
            changed_files: changed_files.to_vec(),
            detail: format!(
                "{} task(s) flipped from pass to fail: {}",
                flipped_task_ids.len(),
                flipped_task_ids.join(", ")
            ),
            flipped_task_ids,
        });
    }

    let prev_pass_rate = pass_rate(previous);
    let cur_pass_rate = pass_rate(current);
    if prev_pass_rate - cur_pass_rate > PASS_RATE_DROP_THRESHOLD_PP {
        flags.push(RegressionFlag {
            kind: RegressionKind::PassRateDrop,
            previous_run_id: previous.run_id.clone(),
            current_run_id: current.run_id.clone(),
            previous_git_sha: previous.git_sha.clone(),
            current_git_sha: current.git_sha.clone(),
            changed_files: changed_files.to_vec(),
            flipped_task_ids: vec![],
            detail: format!("pass rate dropped from {prev_pass_rate:.1}% to {cur_pass_rate:.1}%"),
        });
    }

    let prev_ratio = free_cheap_ratio(previous_events);
    let cur_ratio = free_cheap_ratio(current_events);
    if prev_ratio - cur_ratio > COST_TIER_RATIO_DROP_THRESHOLD_PP {
        flags.push(RegressionFlag {
            kind: RegressionKind::CostTierRatioDrop,
            previous_run_id: previous.run_id.clone(),
            current_run_id: current.run_id.clone(),
            previous_git_sha: previous.git_sha.clone(),
            current_git_sha: current.git_sha.clone(),
            changed_files: changed_files.to_vec(),
            flipped_task_ids: vec![],
            detail: format!(
                "free/cheap model-selection ratio dropped from {prev_ratio:.1}% to {cur_ratio:.1}%"
            ),
        });
    }

    flags
}

#[cfg(test)]
mod regression_tests {
    use super::*;

    fn run(run_id: &str, pass_count: i64, task_count: i64) -> HarnessEvalRunRecord {
        HarnessEvalRunRecord {
            run_id: run_id.to_string(),
            triggered_by: "ci".to_string(),
            git_sha: format!("sha-{run_id}"),
            git_branch: "main".to_string(),
            changed_files: vec![],
            config_version: None,
            samples_per_task: 1,
            task_count,
            pass_count,
            fail_count: task_count - pass_count,
            skip_count: 0,
            total_cost_usd: 0.0,
            started_at_ms: 0,
            finished_at_ms: 0,
        }
    }

    fn event(tier: &str, gated: bool) -> ModelSelectionEventRecord {
        ModelSelectionEventRecord {
            run_id: "r".into(),
            task_id: "t".into(),
            model_id: "m".into(),
            cost_tier: tier.into(),
            selection_reason: "x".into(),
            was_privacy_gated: gated,
            recorded_at_ms: 0,
        }
    }

    #[test]
    fn pass_rate_ignores_skips_and_defaults_to_full() {
        assert_eq!(pass_rate(&run("a", 9, 10)), 90.0);
        assert_eq!(pass_rate(&run("a", 0, 0)), 100.0);
    }

    #[test]
    fn free_cheap_ratio_excludes_privacy_gated_selections() {
        let events = [
            event("free", false),
            event("premium", false),
            event("premium", true),
        ];
        assert_eq!(free_cheap_ratio(&events), 50.0);
        assert_eq!(free_cheap_ratio(&[]), 100.0);
    }

    #[test]
    fn pass_rate_drop_beyond_threshold_is_flagged_with_changed_files() {
        let files = ["src/foo.rs".to_string()];
        let flags = detect_regressions(
            &run("p", 10, 10),
            &run("c", 8, 10),
            &[],
            &[],
            &[],
            &[],
            &files,
        );
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].kind, RegressionKind::PassRateDrop);
        assert_eq!(flags[0].changed_files, files);
        assert!(
            detect_regressions(
                &run("p", 10, 10),
                &run("c", 10, 10),
                &[],
                &[],
                &[],
                &[],
                &[]
            )
            .is_empty()
        );
    }
}
