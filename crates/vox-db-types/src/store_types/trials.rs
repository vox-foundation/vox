//! Research Trial Tier-A parameter, row, batch, and failpoint types (`STORE-01`, `STORE-02`).

use serde::{Deserialize, Serialize};

/// Controlled vocabulary for `trial_campaigns.status`.
pub const TRIAL_CAMPAIGN_STATUSES: &[&str] = &[
    "draft",
    "sealed",
    "running",
    "completed",
    "aborted",
    "voided",
];

/// Controlled vocabulary for `trial_campaigns.stopping_rule_kind`.
pub const TRIAL_STOPPING_RULE_KINDS: &[&str] = &[
    "fixed_n",
    "group_sequential_obrien_fleming",
    "group_sequential_pocock",
    "bayesian_rope",
];

/// Controlled vocabulary for `trial_campaigns_log.event_kind`.
pub const TRIAL_CAMPAIGN_LOG_EVENTS: &[&str] = &[
    "created",
    "sealed",
    "started",
    "arm_completed",
    "stopped_early",
    "completed",
    "aborted",
    "voided",
    "note",
];

/// Controlled vocabulary for `trial_runs.status`.
pub const TRIAL_RUN_STATUSES: &[&str] = &["running", "completed", "failed", "voided"];

/// Controlled vocabulary for `trial_runs.adapter_tag`.
pub const TRIAL_ADAPTER_TAGS: &[&str] = &["research_shim", "code_bench", "oratio_bench"];

/// Controlled vocabulary for `trial_runs.memory_ablation_flag`.
pub const TRIAL_MEMORY_ABLATION_FLAGS: &[&str] = &[
    "off",
    "session_only",
    "persistent_unverified",
    "full_rag_hybrid",
];

/// Controlled vocabulary for `trial_samples.verdict`.
pub const TRIAL_SAMPLE_VERDICTS: &[&str] = &["pass", "fail", "abstain", "error"];

/// Check whether `(from, to)` is a legal `trial_campaigns.status` transition.
#[must_use]
pub fn is_valid_campaign_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("draft", "sealed")
            | ("draft", "voided")
            | ("sealed", "running")
            | ("sealed", "voided")
            | ("running", "completed")
            | ("running", "aborted")
            | ("running", "voided")
    )
}

/// Check whether `(from, to)` is a legal `trial_runs.status` transition.
#[must_use]
pub fn is_valid_run_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("running", "completed") | ("running", "failed") | ("running", "voided")
    )
}

/// Deterministic fault-injection point for atomic trial persistence tests (`STORE-02`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PersistFailpoint {
    /// Normal execution (no fault injection).
    #[default]
    None,
    /// Abort immediately after inserting a `trial_campaigns_log` row.
    AfterCampaignLog,
    /// Abort immediately after inserting/updating `trial_runs`.
    AfterRunUpdate,
    /// Abort immediately after inserting the `k`-th sample (0-indexed).
    AfterSampleAt(usize),
    /// Abort after all samples are inserted, before updating `spent_budget_usd`.
    AfterAllSamples,
}

/// Parameters to register a new `trial_campaigns` row (`status = 'draft'`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrialCampaignRecord {
    pub campaign_id: String,
    pub trusty_uri: String,
    pub prereg_json: String,
    pub prereg_signature: String,
    pub signer_pubkey: String,
    pub reserved_budget_usd: f64,
    pub stopping_rule_kind: String,
    pub stopping_rule_params_json: String,
    pub kb_snapshot_manifest_digest: Option<String>,
    pub kb_snapshot_row_count: Option<i64>,
    pub git_commit: String,
    pub cargo_lock_blake3: String,
    pub uv_lock_blake3: Option<String>,
    pub rustc_version: String,
    pub host_triple: String,
    pub created_at_ms: i64,
    pub actor: String,
}

/// Materialized row from `trial_campaigns`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrialCampaignRow {
    pub campaign_id: String,
    pub trusty_uri: String,
    pub prereg_json: String,
    pub prereg_signature: String,
    pub signer_pubkey: String,
    pub status: String,
    pub reserved_budget_usd: f64,
    pub spent_budget_usd: f64,
    pub stopping_rule_kind: String,
    pub stopping_rule_params_json: String,
    pub kb_snapshot_manifest_digest: Option<String>,
    pub kb_snapshot_row_count: Option<i64>,
    pub git_commit: String,
    pub cargo_lock_blake3: String,
    pub uv_lock_blake3: Option<String>,
    pub rustc_version: String,
    pub host_triple: String,
    pub created_at_ms: i64,
    pub sealed_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
}

/// Materialized row from `trial_campaigns_log`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrialCampaignLogRow {
    pub id: i64,
    pub campaign_id: String,
    pub event_kind: String,
    pub actor: String,
    pub reason: Option<String>,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub payload_json: String,
    pub logged_at_ms: i64,
}

/// Parameters to start/record a `trial_runs` row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrialRunRecord {
    pub run_id: String,
    pub campaign_id: String,
    pub arm_id: String,
    pub seed: i64,
    pub paired_group_id: Option<String>,
    pub adapter_tag: String,
    pub model_id: String,
    pub memory_ablation_flag: String,
    pub started_at_ms: i64,
    pub primary_metric_name: String,
}

/// Terminal outcome for a `trial_runs` row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrialRunOutcome {
    pub status: String,
    pub ended_at_ms: i64,
    pub wall_time_ms: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f64,
    pub primary_metric_value: Option<f64>,
    pub secondary_metrics_json: String,
    pub error_class: Option<String>,
    pub error_message: Option<String>,
}

/// Materialized row from `trial_runs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrialRunRow {
    pub run_id: String,
    pub campaign_id: String,
    pub arm_id: String,
    pub seed: i64,
    pub paired_group_id: Option<String>,
    pub status: String,
    pub adapter_tag: String,
    pub model_id: String,
    pub memory_ablation_flag: String,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub wall_time_ms: Option<i64>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f64,
    pub primary_metric_name: String,
    pub primary_metric_value: Option<f64>,
    pub secondary_metrics_json: String,
    pub error_class: Option<String>,
    pub error_message: Option<String>,
}

/// Parameters to insert a `trial_samples` row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrialSampleRecord {
    pub run_id: String,
    pub campaign_id: String,
    pub item_id: String,
    pub prompt_blake3: String,
    pub output_blake3: Option<String>,
    pub verdict: String,
    pub score: Option<f64>,
    pub latency_ms: Option<i64>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f64,
    pub judge_model_id: Option<String>,
    pub judge_verdict_json: Option<String>,
    pub recorded_at_ms: i64,
}

/// Materialized row from `trial_samples`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrialSampleRow {
    pub id: i64,
    pub run_id: String,
    pub campaign_id: String,
    pub item_id: String,
    pub prompt_blake3: String,
    pub output_blake3: Option<String>,
    pub verdict: String,
    pub score: Option<f64>,
    pub latency_ms: Option<i64>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f64,
    pub judge_model_id: Option<String>,
    pub judge_verdict_json: Option<String>,
    pub recorded_at_ms: i64,
}

/// Atomic batch written by [`vox_db::VoxDb::record_trial_run_batch`] (`STORE-02`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrialRunBatch {
    pub run: TrialRunRecord,
    pub outcome: TrialRunOutcome,
    pub samples: Vec<TrialSampleRecord>,
    pub actor: String,
}

/// Spend partition between benchmark-labeled (`trial_run_label IS NOT NULL`) and organic
/// (`trial_run_label IS NULL`) `llm_interactions` rows (`TRIAL-01`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub struct TrialSpendPartition {
    pub benchmark_spend_usd: f64,
    pub organic_spend_usd: f64,
    pub benchmark_calls: i64,
    pub organic_calls: i64,
}
