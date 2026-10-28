//! Research Trial Tier-A schema fragment (ADR 025).
//!
//! Source of truth: `docs/src/architecture/research-trial-tier-a-schema-2026.md`.
//! Tables: `trial_campaigns`, `trial_campaigns_log`, `trial_runs`, `trial_samples`.
//! Also adds the nullable `trial_run_label` column on `llm_interactions` for
//! benchmark-vs-organic spend attribution (`STORE-01` / `TRIAL-01`).

/// DDL fragment registered in `crate::schema::manifest::BASELINE_SQL_SLICES` as `"trials"`.
pub const SCHEMA_TRIALS: &str = "
CREATE TABLE IF NOT EXISTS trial_campaigns (
    campaign_id TEXT PRIMARY KEY,
    trusty_uri TEXT NOT NULL UNIQUE,
    prereg_json TEXT NOT NULL,
    prereg_signature TEXT NOT NULL,
    signer_pubkey TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    reserved_budget_usd REAL NOT NULL DEFAULT 0.0,
    spent_budget_usd REAL NOT NULL DEFAULT 0.0,
    stopping_rule_kind TEXT NOT NULL,
    stopping_rule_params_json TEXT NOT NULL,
    kb_snapshot_manifest_digest TEXT,
    kb_snapshot_row_count INTEGER,
    git_commit TEXT NOT NULL,
    cargo_lock_blake3 TEXT NOT NULL,
    uv_lock_blake3 TEXT,
    rustc_version TEXT NOT NULL,
    host_triple TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    sealed_at_ms INTEGER,
    completed_at_ms INTEGER
);
CREATE INDEX IF NOT EXISTS idx_trial_campaigns_status ON trial_campaigns(status, created_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_trial_campaigns_trusty ON trial_campaigns(trusty_uri);

CREATE TABLE IF NOT EXISTS trial_campaigns_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    campaign_id TEXT NOT NULL REFERENCES trial_campaigns(campaign_id) ON DELETE CASCADE,
    event_kind TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT,
    from_status TEXT,
    to_status TEXT,
    payload_json TEXT NOT NULL DEFAULT '{}',
    logged_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_trial_campaigns_log_c ON trial_campaigns_log(campaign_id, logged_at_ms);

CREATE TABLE IF NOT EXISTS trial_runs (
    run_id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL REFERENCES trial_campaigns(campaign_id) ON DELETE CASCADE,
    arm_id TEXT NOT NULL,
    seed INTEGER NOT NULL,
    paired_group_id TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    adapter_tag TEXT NOT NULL,
    model_id TEXT NOT NULL,
    memory_ablation_flag TEXT NOT NULL DEFAULT 'full_rag_hybrid',
    started_at_ms INTEGER NOT NULL,
    ended_at_ms INTEGER,
    wall_time_ms INTEGER,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    primary_metric_name TEXT NOT NULL,
    primary_metric_value REAL,
    secondary_metrics_json TEXT NOT NULL DEFAULT '{}',
    error_class TEXT,
    error_message TEXT,
    UNIQUE(campaign_id, arm_id, seed, paired_group_id)
);
CREATE INDEX IF NOT EXISTS idx_trial_runs_campaign_arm ON trial_runs(campaign_id, arm_id, status);
CREATE INDEX IF NOT EXISTS idx_trial_runs_paired ON trial_runs(campaign_id, paired_group_id);

CREATE TABLE IF NOT EXISTS trial_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL REFERENCES trial_runs(run_id) ON DELETE CASCADE,
    campaign_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    prompt_blake3 TEXT NOT NULL,
    output_blake3 TEXT,
    verdict TEXT NOT NULL,
    score REAL,
    latency_ms INTEGER,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    judge_model_id TEXT,
    judge_verdict_json TEXT,
    recorded_at_ms INTEGER NOT NULL,
    UNIQUE(run_id, item_id)
);
CREATE INDEX IF NOT EXISTS idx_trial_samples_run ON trial_samples(run_id);
CREATE INDEX IF NOT EXISTS idx_trial_samples_campaign_item ON trial_samples(campaign_id, item_id);
";

/// Required table names that `store/open.rs` asserts exist after applying the baseline.
/// Keep in alphabetical order.
pub const TRIAL_REQUIRED_TABLES: &[&str] = &[
    "trial_campaigns",
    "trial_campaigns_log",
    "trial_runs",
    "trial_samples",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn trials_tables_exist_after_open() {
        let db = crate::VoxDb::connect(crate::DbConfig::Memory)
            .await
            .expect("open in-memory VoxDb");
        for tbl in TRIAL_REQUIRED_TABLES {
            let mut rows = db
                .connection()
                .query(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name = ?1",
                    turso::params![*tbl],
                )
                .await
                .expect("query sqlite_master");
            let found = rows.next().await.expect("step").is_some();
            assert!(found, "table {tbl} must exist after open()");
        }
    }
}
