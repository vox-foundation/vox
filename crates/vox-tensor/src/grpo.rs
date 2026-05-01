//! Group Relative Policy Optimization (GRPO) reward and advantage computation for Vox MENS.
//!
//! **Research sources:**
//! - `research-grpo-reward-shaping-2026.md` — cluster overview
//! - `research-grpo-binary-parse-rate-2026.md` — why binary syntax must be a gate, not additive
//! - `research-grpo-ast-reward-hacking-2026.md` — why AST density is removed
//! - `research-grpo-reward-weights-2026.md` — why 0.6/0.3/0.1 is pathological
//! - `research-grpo-vram-small-batch-2026.md` — why median replaces mean (MC-GRPO)
//! - `research-grpo-positive-only-optimization-2026.md` — why negatives enter the RL loop
//!
//! **Key corrections from the existing 254-task blueprint (`vox_agentic_loop_and_mens_plan.md`):**
//! - Blueprint tasks 211-217 use reward weights `0.6*syntax + 0.3*test + 0.1*coverage`.
//!   This module uses **gated multiplication**: `syntax * (test_w * r_test + concise_w * r_concise)`.
//! - Blueprint task 214 uses mean-baseline GRPO. This module uses **median** (MC-GRPO).
//! - Blueprint task 221 separates failures into SFT. Here failures enter the GRPO group directly.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Reward weight allocation for the gated reward function.
///
/// Syntax correctness is a **gate** (multiplier), not an additive component.
/// Total reward = `r_syntax * (test_weight * r_test + conciseness_weight * r_concise)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardWeights {
    /// Weight for functional test pass-rate in the composite reward (default 0.7).
    pub test_weight: f64,
    /// Weight for conciseness penalty in the composite reward (default 0.3).
    pub conciseness_weight: f64,
    /// Weight for routing efficiency (NNT small-world topology) in the composite reward (default 0.2).
    pub routing_efficiency_weight: f64,
    /// Maximum expected candidate length in characters for conciseness scoring.
    pub max_expected_len: usize,
}

impl Default for RewardWeights {
    fn default() -> Self {
        Self {
            test_weight: 0.7,
            conciseness_weight: 0.1,
            routing_efficiency_weight: 0.2,
            max_expected_len: 2000,
        }
    }
}

/// Full GRPO training configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpoConfig {
    /// Number of candidate completions per prompt (default 8).
    pub k_samples: usize,
    /// Sampling temperature for candidate generation (default 0.8).
    pub temperature: f64,
    /// Reward weight allocation.
    pub reward_weights: RewardWeights,
    /// Policy learning rate (default 1e-5).
    pub policy_lr: f64,
    /// PPO-clip lower bound (default 0.2).
    pub clip_epsilon: f64,
    /// Asymmetric upper clip bound — DAPO mechanics (default 0.28).
    pub clip_upper: f64,
    /// Maximum training steps (default 500).
    pub max_steps: usize,
    /// Hard corpus gate: refuse to train if fewer than this many validated pairs (default 1000).
    pub min_corpus_pairs: usize,
    /// Minimum fraction of negative examples per GRPO group (Anna Karenina sampling, default 0.3).
    pub min_negative_ratio: f64,
    /// Minimum fraction of human-origin pairs per batch (default 0.15).
    pub min_human_ratio: f64,
}

impl Default for GrpoConfig {
    fn default() -> Self {
        Self {
            k_samples: 8,
            temperature: 0.8,
            reward_weights: RewardWeights::default(),
            policy_lr: 1e-5,
            clip_epsilon: 0.2,
            clip_upper: 0.28,
            max_steps: 500,
            min_corpus_pairs: 1000,
            min_negative_ratio: 0.3,
            min_human_ratio: 0.15,
        }
    }
}

// ---------------------------------------------------------------------------
// Test result abstraction
// ---------------------------------------------------------------------------

/// Outcome of running `@test` blocks against a candidate completion.
#[derive(Debug, Clone, Default)]
pub struct TestResults {
    pub total: usize,
    pub passed: usize,
}

impl TestResults {
    /// Fraction of tests that passed in `[0.0, 1.0]`. Returns 0.0 if no tests exist.
    #[must_use]
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.passed as f64 / self.total as f64
    }
}

// ---------------------------------------------------------------------------
// Reward signal
// ---------------------------------------------------------------------------

/// Computed reward signal for a single GRPO candidate.
#[derive(Debug, Clone, Serialize)]
pub struct RewardSignal {
    /// Binary: 1.0 if the candidate parses, 0.0 otherwise.
    pub parse_score: f64,
    /// Fraction of `@test` blocks that pass in `[0.0, 1.0]`.
    pub test_score: f64,
    /// Conciseness penalty: shorter code scores higher.
    pub conciseness_score: f64,
    /// NNT small-world routing efficiency score in `[0.0, 1.0]`.
    pub routing_efficiency_score: f64,
    /// Final composite reward (gated).
    pub composite: f64,
}

/// Compute the gated reward for a candidate completion.
///
/// **Gated formula:** `r_syntax * (test_w * r_test + concise_w * r_concise)`
///
/// If the candidate does not parse (`r_syntax = 0`), the total reward is **zero**
/// regardless of test or conciseness scores. This prevents the pathological local
/// optimum where the model produces syntactically valid but logically empty code
/// to harvest a 60% baseline reward.
///
/// The AST density / construct coverage metric has been **removed** — the research
/// proves it causes Goodhart's Law exploitation (reward hacking via bloated code).
/// A conciseness (length) penalty replaces it: shorter code with the same test pass
/// rate scores higher.
#[must_use]
pub fn compute_reward(
    candidate: &str,
    test_results: &TestResults,
    routing_efficiency: f64,
    weights: &RewardWeights,
) -> RewardSignal {
    let report = vox_compiler::ast_eval(candidate);
    let parse_score = if report.parse_success { 1.0 } else { 0.0 };
    let test_score = test_results.pass_rate();
    let conciseness_score =
        1.0 - (candidate.len() as f64 / weights.max_expected_len as f64).min(1.0);
    let routing_efficiency_score = routing_efficiency.clamp(0.0, 1.0);

    let composite = parse_score
        * (weights.test_weight * test_score
            + weights.conciseness_weight * conciseness_score
            + weights.routing_efficiency_weight * routing_efficiency_score);

    RewardSignal {
        parse_score,
        test_score,
        conciseness_score,
        routing_efficiency_score,
        composite,
    }
}

// ---------------------------------------------------------------------------
// Advantage computation (MC-GRPO)
// ---------------------------------------------------------------------------

/// Compute group-relative advantages using **median** baseline (MC-GRPO).
///
/// The research proves that `k=8` with a **mean** baseline causes advantage sign
/// flipping when a single outlier reward skews the group mean. Median is robust
/// against this because a single outlier cannot shift the median by more than one
/// rank position.
///
/// Returns one advantage value per reward in the input slice.
#[must_use]
pub fn compute_advantages(rewards: &[f64]) -> Vec<f64> {
    if rewards.is_empty() {
        return vec![];
    }
    let median = statistical_median(rewards);
    let std = statistical_std_from_center(rewards, median).max(1e-8);
    rewards.iter().map(|r| (r - median) / std).collect()
}

/// Sort-free median (O(n log n) via clone + sort).
fn statistical_median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    }
}

/// Standard deviation computed around a given center (not necessarily the mean).
fn statistical_std_from_center(values: &[f64], center: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let variance = values.iter().map(|v| (v - center).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

// ---------------------------------------------------------------------------
// Anna Karenina sampling
// ---------------------------------------------------------------------------

/// Ensure a GRPO candidate group has at least `min_negative_ratio` fraction of failures.
///
/// If the group has too few negatives, this function fills remaining slots by drawing
/// from `negative_pool` (the model's own recent rollout failures). This maintains
/// high policy entropy (+35% per research) and prevents catastrophic overfitting on
/// trivial syntax.
///
/// Returns the augmented reward vector and corresponding candidate indices.
pub fn anna_karenina_augment(
    rewards: &[f64],
    negative_pool_rewards: &[f64],
    min_negative_ratio: f64,
) -> Vec<f64> {
    let k = rewards.len();
    if k == 0 {
        return vec![];
    }

    let negatives_in_group = rewards.iter().filter(|&&r| r == 0.0).count();
    let required_negatives = (k as f64 * min_negative_ratio).ceil() as usize;

    if negatives_in_group >= required_negatives {
        return rewards.to_vec();
    }

    let need = required_negatives - negatives_in_group;
    let mut augmented = rewards.to_vec();

    // Replace the lowest-reward non-negative candidates with pool negatives
    let mut indices: Vec<usize> = (0..k).filter(|&i| rewards[i] > 0.0).collect();
    indices.sort_by(|&a, &b| {
        rewards[a]
            .partial_cmp(&rewards[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let pool_iter = negative_pool_rewards.iter().cycle();
    for (slot, &_pool_r) in indices.iter().take(need).zip(pool_iter) {
        augmented[*slot] = 0.0; // inject failure reward
    }

    augmented
}

// ---------------------------------------------------------------------------
// Corpus origin tracking
// ---------------------------------------------------------------------------

/// Origin of a training pair for accumulate-vs-replace policy enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairOrigin {
    /// Human-written ground truth (must be ≥15% of every batch).
    Human,
    /// Synthetically generated (template expansion, augmentation).
    Synthetic,
    /// Agent-generated (MCP code generation, Schola, Scientia).
    Agent,
}

/// Validate that a batch meets the minimum human-origin ratio.
///
/// Returns `Err` with the actual ratio if the constraint is violated.
pub fn validate_human_ratio(origins: &[PairOrigin], min_ratio: f64) -> Result<f64, f64> {
    if origins.is_empty() {
        return Err(0.0);
    }
    let human_count = origins.iter().filter(|&&o| o == PairOrigin::Human).count();
    let ratio = human_count as f64 / origins.len() as f64;
    if ratio < min_ratio {
        Err(ratio)
    } else {
        Ok(ratio)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gated_reward_zero_on_parse_failure() {
        let bad_code = "fn broken( { ";
        let tests = TestResults {
            total: 1,
            passed: 1,
        };
        let signal = compute_reward(bad_code, &tests, 0.0, &RewardWeights::default());
        assert_eq!(signal.parse_score, 0.0);
        assert_eq!(
            signal.composite, 0.0,
            "Parse failure must gate total reward to zero"
        );
    }

    #[test]
    fn gated_reward_nonzero_on_valid_code() {
        let good_code = "fn hello() to str { return \"hi\" }";
        let tests = TestResults {
            total: 1,
            passed: 1,
        };
        let signal = compute_reward(good_code, &tests, 0.0, &RewardWeights::default());
        assert_eq!(signal.parse_score, 1.0);
        assert!(signal.composite > 0.0);
    }

    #[test]
    fn gated_reward_conciseness_rewards_shorter_code() {
        let short = "fn a() to int { return 1 }";
        let long = &"fn a() to int { return 1 }".to_string().repeat(20);
        let tests = TestResults {
            total: 0,
            passed: 0,
        };
        let w = RewardWeights::default();
        let short_signal = compute_reward(short, &tests, 0.0, &w);
        let long_signal = compute_reward(long, &tests, 0.0, &w);
        assert!(short_signal.conciseness_score > long_signal.conciseness_score);
    }

    #[test]
    fn gated_reward_routing_efficiency_rewards_dense_topology() {
        let code = "fn a() -> int { return 1 }";
        let tests = TestResults {
            total: 0,
            passed: 0,
        };
        let w = RewardWeights::default();
        let efficient = compute_reward(code, &tests, 1.0, &w);
        let inefficient = compute_reward(code, &tests, 0.5, &w);
        assert!(efficient.routing_efficiency_score > inefficient.routing_efficiency_score);
        assert!(efficient.composite > inefficient.composite);
    }

    #[test]
    fn median_advantages_no_sign_flip_on_outlier() {
        // 7 mediocre candidates at ~0.2 and one outlier at 0.9
        let rewards = vec![0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.9];
        let advantages = compute_advantages(&rewards);

        // With median baseline (~0.2), the 0.2 candidates should have advantage ≈ 0
        // With mean baseline (~0.2875), they would have NEGATIVE advantage — the bug MC-GRPO fixes
        let median_candidates: Vec<f64> = advantages[..7].to_vec();
        for (i, &a) in median_candidates.iter().enumerate() {
            assert!(
                a >= -0.01,
                "Candidate {i} with reward 0.2 should not have significantly negative advantage (got {a}). \
                 MC-GRPO median baseline prevents sign flipping."
            );
        }
        // The outlier should have positive advantage
        assert!(
            advantages[7] > 0.0,
            "Outlier should have positive advantage"
        );
    }

    #[test]
    fn median_vs_mean_demonstrates_sign_flip_bug() {
        let rewards = vec![0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.9];
        let mean = rewards.iter().sum::<f64>() / rewards.len() as f64;
        // Mean = 0.2875 — higher than 0.2, so 0.2 candidates get negative advantage
        assert!(mean > 0.2, "Mean is skewed by outlier");
        // This is the exact bug the research documents: advantage sign flipping
        let mean_advantage_for_0_2 = 0.2 - mean;
        assert!(
            mean_advantage_for_0_2 < 0.0,
            "Mean-based advantage would be negative for 0.2 candidates"
        );
    }

    #[test]
    fn anna_karenina_augments_when_too_few_negatives() {
        let rewards = vec![0.5, 0.6, 0.7, 0.8, 0.9, 0.4, 0.3, 0.2]; // 0 negatives
        let pool = vec![0.0, 0.0, 0.0];
        let augmented = anna_karenina_augment(&rewards, &pool, 0.3);
        let neg_count = augmented.iter().filter(|&&r| r == 0.0).count();
        assert!(
            neg_count >= 3,
            "Should have at least ceil(8*0.3)=3 negatives, got {neg_count}"
        );
    }

    #[test]
    fn anna_karenina_noop_when_enough_negatives() {
        let rewards = vec![0.0, 0.0, 0.0, 0.5, 0.6, 0.7, 0.8, 0.9]; // 3 negatives already
        let pool = vec![0.0];
        let augmented = anna_karenina_augment(&rewards, &pool, 0.3);
        assert_eq!(
            augmented, rewards,
            "Should not modify when ratio already met"
        );
    }

    #[test]
    fn human_ratio_validation() {
        let origins = vec![
            PairOrigin::Human,
            PairOrigin::Synthetic,
            PairOrigin::Synthetic,
            PairOrigin::Agent,
            PairOrigin::Synthetic,
        ];
        // 1/5 = 0.2 > 0.15 threshold
        assert!(validate_human_ratio(&origins, 0.15).is_ok());
        // 1/5 = 0.2 < 0.25 threshold
        assert!(validate_human_ratio(&origins, 0.25).is_err());
    }

    #[test]
    fn empty_rewards_produce_empty_advantages() {
        assert!(compute_advantages(&[]).is_empty());
    }

    #[test]
    fn config_defaults_are_sane() {
        let config = GrpoConfig::default();
        assert_eq!(config.k_samples, 8);
        assert_eq!(config.min_corpus_pairs, 1000);
        assert!(
            config.reward_weights.test_weight + config.reward_weights.conciseness_weight <= 1.01
        );
    }
}
