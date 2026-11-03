//! Research Trial Tier-A atomic persistence (`STORE-01`, `STORE-02`).
//!
//! Enforces:
//! - Controlled vocabularies (`TRIAL_CAMPAIGN_STATUSES`, `TRIAL_STOPPING_RULE_KINDS`,
//!   `TRIAL_CAMPAIGN_LOG_EVENTS`, `TRIAL_RUN_STATUSES`, `TRIAL_ADAPTER_TAGS`,
//!   `TRIAL_MEMORY_ABLATION_FLAGS`, `TRIAL_SAMPLE_VERDICTS`) before any SQL write.
//! - Legal status transitions (`draft -> sealed -> running -> completed|aborted|voided` for
//!   campaigns; `running -> completed|failed|voided` for runs).
//! - Single-transaction multi-table writes via [`crate::GuardedConnection::transaction`]
//!   with deterministic [`PersistFailpoint`] fault injection (`STORE-02`).
//! - Benchmark-vs-organic spend partitioning on `llm_interactions.trial_run_label`.

use crate::{StoreError, VoxDb};
use vox_db_types::store_types::trials::{
    PersistFailpoint, TRIAL_ADAPTER_TAGS, TRIAL_CAMPAIGN_LOG_EVENTS, TRIAL_CAMPAIGN_STATUSES,
    TRIAL_MEMORY_ABLATION_FLAGS, TRIAL_RUN_STATUSES, TRIAL_SAMPLE_VERDICTS,
    TRIAL_STOPPING_RULE_KINDS, TrialCampaignLogRow, TrialCampaignRecord, TrialCampaignRow,
    TrialRunBatch, TrialRunRow, TrialSampleRow, TrialSpendPartition, is_valid_campaign_transition,
    is_valid_run_transition,
};

fn ensure_vocab(
    field: &'static str,
    value: &str,
    allowed_slice: &[&str],
    allowed_desc: &'static str,
) -> Result<(), StoreError> {
    if allowed_slice.contains(&value) {
        Ok(())
    } else {
        Err(StoreError::InvalidTrialValue {
            field,
            value: value.to_string(),
            allowed: allowed_desc,
        })
    }
}

impl VoxDb {
    /// Register a new `trial_campaigns` row in `draft` status and append a `created` log event
    /// in a single transaction (`STORE-01`, `STORE-02`).
    pub async fn register_campaign(&self, rec: &TrialCampaignRecord) -> Result<(), StoreError> {
        ensure_vocab(
            "trial_campaigns.stopping_rule_kind",
            &rec.stopping_rule_kind,
            TRIAL_STOPPING_RULE_KINDS,
            "fixed_n | group_sequential_obrien_fleming | group_sequential_pocock | bayesian_rope",
        )?;
        if !rec.reserved_budget_usd.is_finite() || rec.reserved_budget_usd < 0.0 {
            return Err(StoreError::InvalidTrialValue {
                field: "trial_campaigns.reserved_budget_usd",
                value: rec.reserved_budget_usd.to_string(),
                allowed: "finite >= 0.0",
            });
        }

        let rec = rec.clone();
        self.conn
            .transaction(|conn| async move {
                conn.execute(
                    "INSERT INTO trial_campaigns (
                        campaign_id, trusty_uri, prereg_json, prereg_signature, signer_pubkey,
                        status, reserved_budget_usd, spent_budget_usd, stopping_rule_kind,
                        stopping_rule_params_json, kb_snapshot_manifest_digest,
                        kb_snapshot_row_count, git_commit, cargo_lock_blake3, uv_lock_blake3,
                        rustc_version, host_triple, created_at_ms
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5,
                        'draft', ?6, 0.0, ?7,
                        ?8, ?9,
                        ?10, ?11, ?12, ?13,
                        ?14, ?15, ?16
                    )",
                    turso::params![
                        rec.campaign_id.as_str(),
                        rec.trusty_uri.as_str(),
                        rec.prereg_json.as_str(),
                        rec.prereg_signature.as_str(),
                        rec.signer_pubkey.as_str(),
                        rec.reserved_budget_usd,
                        rec.stopping_rule_kind.as_str(),
                        rec.stopping_rule_params_json.as_str(),
                        rec.kb_snapshot_manifest_digest.as_deref(),
                        rec.kb_snapshot_row_count,
                        rec.git_commit.as_str(),
                        rec.cargo_lock_blake3.as_str(),
                        rec.uv_lock_blake3.as_deref(),
                        rec.rustc_version.as_str(),
                        rec.host_triple.as_str(),
                        rec.created_at_ms,
                    ],
                )
                .await?;

                conn.execute(
                    "INSERT INTO trial_campaigns_log (
                        campaign_id, event_kind, actor, reason, from_status, to_status,
                        payload_json, logged_at_ms
                    ) VALUES (?1, 'created', ?2, NULL, NULL, 'draft', '{}', ?3)",
                    turso::params![
                        rec.campaign_id.as_str(),
                        rec.actor.as_str(),
                        rec.created_at_ms
                    ],
                )
                .await?;

                Ok(())
            })
            .await
    }

    /// Seal a `draft` campaign (`draft -> sealed`), setting `sealed_at_ms` and logging `'sealed'`.
    pub async fn seal_campaign(
        &self,
        campaign_id: &str,
        actor: &str,
        sealed_at_ms: i64,
    ) -> Result<(), StoreError> {
        self.transition_campaign(
            campaign_id,
            "sealed",
            actor,
            None,
            sealed_at_ms,
            PersistFailpoint::None,
        )
        .await
    }

    /// Transition a `trial_campaigns` row to `to_status` with state-machine validation,
    /// audit log insertion, and optional [`PersistFailpoint`] fault injection.
    pub async fn transition_campaign(
        &self,
        campaign_id: &str,
        to_status: &str,
        actor: &str,
        reason: Option<&str>,
        logged_at_ms: i64,
        failpoint: PersistFailpoint,
    ) -> Result<(), StoreError> {
        ensure_vocab(
            "trial_campaigns.status",
            to_status,
            TRIAL_CAMPAIGN_STATUSES,
            "draft | sealed | running | completed | aborted | voided",
        )?;

        let event_kind = match to_status {
            "sealed" => "sealed",
            "running" => "started",
            "completed" => "completed",
            "aborted" => "aborted",
            "voided" => "voided",
            _ => "note",
        };
        ensure_vocab(
            "trial_campaigns_log.event_kind",
            event_kind,
            TRIAL_CAMPAIGN_LOG_EVENTS,
            "created | sealed | started | arm_completed | stopped_early | completed | aborted | voided | note",
        )?;

        let campaign_id_owned = campaign_id.to_string();
        let to_status_owned = to_status.to_string();
        let actor_owned = actor.to_string();
        let reason_owned = reason.map(std::borrow::ToOwned::to_owned);

        self.conn
            .transaction(|conn| async move {
                let mut rows = conn
                    .query(
                        "SELECT status FROM trial_campaigns WHERE campaign_id = ?1",
                        turso::params![campaign_id_owned.as_str()],
                    )
                    .await?;
                let Some(row) = rows.next().await? else {
                    return Err(StoreError::NotFound(format!(
                        "trial_campaign '{}'",
                        campaign_id_owned
                    )));
                };
                let from_status: String = row.get(0)?;
                drop(rows);

                if !is_valid_campaign_transition(&from_status, &to_status_owned) {
                    return Err(StoreError::IllegalTrialTransition {
                        entity: "campaign",
                        id: campaign_id_owned,
                        from: from_status,
                        to: to_status_owned,
                    });
                }

                conn.execute(
                    "INSERT INTO trial_campaigns_log (
                        campaign_id, event_kind, actor, reason, from_status, to_status,
                        payload_json, logged_at_ms
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}', ?7)",
                    turso::params![
                        campaign_id_owned.as_str(),
                        event_kind,
                        actor_owned.as_str(),
                        reason_owned.as_deref(),
                        from_status.as_str(),
                        to_status_owned.as_str(),
                        logged_at_ms,
                    ],
                )
                .await?;

                if matches!(failpoint, PersistFailpoint::AfterCampaignLog) {
                    return Err(StoreError::FailpointTriggered("AfterCampaignLog"));
                }

                match to_status_owned.as_str() {
                    "sealed" => {
                        conn.execute(
                            "UPDATE trial_campaigns
                             SET status = 'sealed', sealed_at_ms = ?2
                             WHERE campaign_id = ?1",
                            turso::params![campaign_id_owned.as_str(), logged_at_ms],
                        )
                        .await?;
                    }
                    "completed" | "aborted" | "voided" => {
                        conn.execute(
                            "UPDATE trial_campaigns
                             SET status = ?2, completed_at_ms = ?3
                             WHERE campaign_id = ?1",
                            turso::params![
                                campaign_id_owned.as_str(),
                                to_status_owned.as_str(),
                                logged_at_ms
                            ],
                        )
                        .await?;
                    }
                    _ => {
                        conn.execute(
                            "UPDATE trial_campaigns
                             SET status = ?2
                             WHERE campaign_id = ?1",
                            turso::params![campaign_id_owned.as_str(), to_status_owned.as_str()],
                        )
                        .await?;
                    }
                }

                Ok(())
            })
            .await
    }

    /// Atomically persist a [`TrialRunBatch`]:
    /// 1. Validates vocabularies on `run`, `outcome`, and every sample before touching SQLite.
    /// 2. Verifies the parent campaign is in `'running'` status.
    /// 3. Upserts the `trial_runs` row from `'running'` to `outcome.status`.
    /// 4. Inserts all `trial_samples` rows.
    /// 5. Increments `trial_campaigns.spent_budget_usd` by `outcome.cost_usd` and appends an
    ///    `arm_completed` log entry.
    ///
    /// Any error or [`PersistFailpoint`] trigger rolls back all changes (`STORE-02`).
    pub async fn record_trial_run_batch(
        &self,
        batch: &TrialRunBatch,
        failpoint: PersistFailpoint,
    ) -> Result<(), StoreError> {
        ensure_vocab(
            "trial_runs.adapter_tag",
            &batch.run.adapter_tag,
            TRIAL_ADAPTER_TAGS,
            "research_shim | code_bench | oratio_bench",
        )?;
        ensure_vocab(
            "trial_runs.memory_ablation_flag",
            &batch.run.memory_ablation_flag,
            TRIAL_MEMORY_ABLATION_FLAGS,
            "off | session_only | persistent_unverified | full_rag_hybrid",
        )?;
        ensure_vocab(
            "trial_runs.status",
            &batch.outcome.status,
            TRIAL_RUN_STATUSES,
            "running | completed | failed | voided",
        )?;
        if !is_valid_run_transition("running", &batch.outcome.status) {
            return Err(StoreError::IllegalTrialTransition {
                entity: "run",
                id: batch.run.run_id.clone(),
                from: "running".to_string(),
                to: batch.outcome.status.clone(),
            });
        }
        if !batch.outcome.cost_usd.is_finite() || batch.outcome.cost_usd < 0.0 {
            return Err(StoreError::InvalidTrialValue {
                field: "trial_runs.cost_usd",
                value: batch.outcome.cost_usd.to_string(),
                allowed: "finite >= 0.0",
            });
        }
        for sample in &batch.samples {
            ensure_vocab(
                "trial_samples.verdict",
                &sample.verdict,
                TRIAL_SAMPLE_VERDICTS,
                "pass | fail | abstain | error",
            )?;
            if sample.run_id != batch.run.run_id || sample.campaign_id != batch.run.campaign_id {
                return Err(StoreError::InvalidTrialValue {
                    field: "trial_samples.run_id/campaign_id",
                    value: format!("{}/{}", sample.run_id, sample.campaign_id),
                    allowed: "must match parent TrialRunRecord",
                });
            }
        }

        let batch = batch.clone();
        self.conn
            .transaction(|conn| async move {
                let mut rows = conn
                    .query(
                        "SELECT status FROM trial_campaigns WHERE campaign_id = ?1",
                        turso::params![batch.run.campaign_id.as_str()],
                    )
                    .await?;
                let Some(row) = rows.next().await? else {
                    return Err(StoreError::NotFound(format!(
                        "trial_campaign '{}'",
                        batch.run.campaign_id
                    )));
                };
                let camp_status: String = row.get(0)?;
                drop(rows);

                if camp_status != "running" {
                    return Err(StoreError::IllegalTrialTransition {
                        entity: "campaign_run_admission",
                        id: batch.run.campaign_id.clone(),
                        from: camp_status,
                        to: "record_trial_run_batch (requires running)".to_string(),
                    });
                }

                conn.execute(
                    "INSERT INTO trial_runs (
                        run_id, campaign_id, arm_id, seed, paired_group_id, status,
                        adapter_tag, model_id, memory_ablation_flag, started_at_ms,
                        ended_at_ms, wall_time_ms, input_tokens, output_tokens, cost_usd,
                        primary_metric_name, primary_metric_value, secondary_metrics_json,
                        error_class, error_message
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6,
                        ?7, ?8, ?9, ?10,
                        ?11, ?12, ?13, ?14, ?15,
                        ?16, ?17, ?18,
                        ?19, ?20
                    )",
                    turso::params![
                        batch.run.run_id.as_str(),
                        batch.run.campaign_id.as_str(),
                        batch.run.arm_id.as_str(),
                        batch.run.seed,
                        batch.run.paired_group_id.as_deref(),
                        batch.outcome.status.as_str(),
                        batch.run.adapter_tag.as_str(),
                        batch.run.model_id.as_str(),
                        batch.run.memory_ablation_flag.as_str(),
                        batch.run.started_at_ms,
                        batch.outcome.ended_at_ms,
                        batch.outcome.wall_time_ms,
                        batch.outcome.input_tokens,
                        batch.outcome.output_tokens,
                        batch.outcome.cost_usd,
                        batch.run.primary_metric_name.as_str(),
                        batch.outcome.primary_metric_value,
                        batch.outcome.secondary_metrics_json.as_str(),
                        batch.outcome.error_class.as_deref(),
                        batch.outcome.error_message.as_deref(),
                    ],
                )
                .await?;

                if matches!(failpoint, PersistFailpoint::AfterRunUpdate) {
                    return Err(StoreError::FailpointTriggered("AfterRunUpdate"));
                }

                for (idx, sample) in batch.samples.iter().enumerate() {
                    conn.execute(
                        "INSERT INTO trial_samples (
                            run_id, campaign_id, item_id, prompt_blake3, output_blake3,
                            verdict, score, latency_ms, input_tokens, output_tokens,
                            cost_usd, judge_model_id, judge_verdict_json, recorded_at_ms
                        ) VALUES (
                            ?1, ?2, ?3, ?4, ?5,
                            ?6, ?7, ?8, ?9, ?10,
                            ?11, ?12, ?13, ?14
                        )",
                        turso::params![
                            sample.run_id.as_str(),
                            sample.campaign_id.as_str(),
                            sample.item_id.as_str(),
                            sample.prompt_blake3.as_str(),
                            sample.output_blake3.as_deref(),
                            sample.verdict.as_str(),
                            sample.score,
                            sample.latency_ms,
                            sample.input_tokens,
                            sample.output_tokens,
                            sample.cost_usd,
                            sample.judge_model_id.as_deref(),
                            sample.judge_verdict_json.as_deref(),
                            sample.recorded_at_ms,
                        ],
                    )
                    .await?;

                    if matches!(failpoint, PersistFailpoint::AfterSampleAt(k) if k == idx) {
                        return Err(StoreError::FailpointTriggered("AfterSampleAt"));
                    }
                }

                if matches!(failpoint, PersistFailpoint::AfterAllSamples) {
                    return Err(StoreError::FailpointTriggered("AfterAllSamples"));
                }

                conn.execute(
                    "UPDATE trial_campaigns
                     SET spent_budget_usd = spent_budget_usd + ?2
                     WHERE campaign_id = ?1",
                    turso::params![batch.run.campaign_id.as_str(), batch.outcome.cost_usd],
                )
                .await?;

                conn.execute(
                    "INSERT INTO trial_campaigns_log (
                        campaign_id, event_kind, actor, reason, from_status, to_status,
                        payload_json, logged_at_ms
                    ) VALUES (?1, 'arm_completed', ?2, ?3, 'running', 'running', '{}', ?4)",
                    turso::params![
                        batch.run.campaign_id.as_str(),
                        batch.actor.as_str(),
                        batch.run.run_id.as_str(),
                        batch.outcome.ended_at_ms,
                    ],
                )
                .await?;

                if matches!(failpoint, PersistFailpoint::AfterCampaignLog) {
                    return Err(StoreError::FailpointTriggered("AfterCampaignLog"));
                }

                Ok(())
            })
            .await
    }

    /// Fetch a single `trial_campaigns` row by primary key.
    pub async fn get_campaign(
        &self,
        campaign_id: &str,
    ) -> Result<Option<TrialCampaignRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT
                    campaign_id, trusty_uri, prereg_json, prereg_signature, signer_pubkey,
                    status, reserved_budget_usd, spent_budget_usd, stopping_rule_kind,
                    stopping_rule_params_json, kb_snapshot_manifest_digest,
                    kb_snapshot_row_count, git_commit, cargo_lock_blake3, uv_lock_blake3,
                    rustc_version, host_triple, created_at_ms, sealed_at_ms, completed_at_ms
                 FROM trial_campaigns
                 WHERE campaign_id = ?1",
                turso::params![campaign_id],
            )
            .await?;
        let Some(row) = rows.next().await? else {
            return Ok(None);
        };
        Ok(Some(TrialCampaignRow {
            campaign_id: row.get(0)?,
            trusty_uri: row.get(1)?,
            prereg_json: row.get(2)?,
            prereg_signature: row.get(3)?,
            signer_pubkey: row.get(4)?,
            status: row.get(5)?,
            reserved_budget_usd: row.get(6)?,
            spent_budget_usd: row.get(7)?,
            stopping_rule_kind: row.get(8)?,
            stopping_rule_params_json: row.get(9)?,
            kb_snapshot_manifest_digest: row.get(10)?,
            kb_snapshot_row_count: row.get(11)?,
            git_commit: row.get(12)?,
            cargo_lock_blake3: row.get(13)?,
            uv_lock_blake3: row.get(14)?,
            rustc_version: row.get(15)?,
            host_triple: row.get(16)?,
            created_at_ms: row.get(17)?,
            sealed_at_ms: row.get(18)?,
            completed_at_ms: row.get(19)?,
        }))
    }

    /// List all audit log rows for a campaign in chronological order.
    pub async fn list_campaign_logs(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<TrialCampaignLogRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT
                    id, campaign_id, event_kind, actor, reason, from_status, to_status,
                    payload_json, logged_at_ms
                 FROM trial_campaigns_log
                 WHERE campaign_id = ?1
                 ORDER BY logged_at_ms ASC, id ASC",
                turso::params![campaign_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(TrialCampaignLogRow {
                id: row.get(0)?,
                campaign_id: row.get(1)?,
                event_kind: row.get(2)?,
                actor: row.get(3)?,
                reason: row.get(4)?,
                from_status: row.get(5)?,
                to_status: row.get(6)?,
                payload_json: row.get(7)?,
                logged_at_ms: row.get(8)?,
            });
        }
        Ok(out)
    }

    /// List all `trial_runs` rows for a campaign.
    pub async fn list_campaign_runs(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<TrialRunRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT
                    run_id, campaign_id, arm_id, seed, paired_group_id, status,
                    adapter_tag, model_id, memory_ablation_flag, started_at_ms,
                    ended_at_ms, wall_time_ms, input_tokens, output_tokens, cost_usd,
                    primary_metric_name, primary_metric_value, secondary_metrics_json,
                    error_class, error_message
                 FROM trial_runs
                 WHERE campaign_id = ?1
                 ORDER BY started_at_ms ASC, run_id ASC",
                turso::params![campaign_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(TrialRunRow {
                run_id: row.get(0)?,
                campaign_id: row.get(1)?,
                arm_id: row.get(2)?,
                seed: row.get(3)?,
                paired_group_id: row.get(4)?,
                status: row.get(5)?,
                adapter_tag: row.get(6)?,
                model_id: row.get(7)?,
                memory_ablation_flag: row.get(8)?,
                started_at_ms: row.get(9)?,
                ended_at_ms: row.get(10)?,
                wall_time_ms: row.get(11)?,
                input_tokens: row.get(12)?,
                output_tokens: row.get(13)?,
                cost_usd: row.get(14)?,
                primary_metric_name: row.get(15)?,
                primary_metric_value: row.get(16)?,
                secondary_metrics_json: row.get(17)?,
                error_class: row.get(18)?,
                error_message: row.get(19)?,
            });
        }
        Ok(out)
    }

    /// List all `trial_samples` rows for a run.
    pub async fn list_run_samples(&self, run_id: &str) -> Result<Vec<TrialSampleRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT
                    id, run_id, campaign_id, item_id, prompt_blake3, output_blake3,
                    verdict, score, latency_ms, input_tokens, output_tokens,
                    cost_usd, judge_model_id, judge_verdict_json, recorded_at_ms
                 FROM trial_samples
                 WHERE run_id = ?1
                 ORDER BY id ASC",
                turso::params![run_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(TrialSampleRow {
                id: row.get(0)?,
                run_id: row.get(1)?,
                campaign_id: row.get(2)?,
                item_id: row.get(3)?,
                prompt_blake3: row.get(4)?,
                output_blake3: row.get(5)?,
                verdict: row.get(6)?,
                score: row.get(7)?,
                latency_ms: row.get(8)?,
                input_tokens: row.get(9)?,
                output_tokens: row.get(10)?,
                cost_usd: row.get(11)?,
                judge_model_id: row.get(12)?,
                judge_verdict_json: row.get(13)?,
                recorded_at_ms: row.get(14)?,
            });
        }
        Ok(out)
    }

    /// Partition `llm_interactions` spend into benchmark (`trial_run_label IS NOT NULL`) vs
    /// organic (`trial_run_label IS NULL`) buckets (`TRIAL-01`).
    pub async fn partition_llm_spend(&self) -> Result<TrialSpendPartition, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT
                    COALESCE(SUM(CASE WHEN trial_run_label IS NOT NULL THEN cost_usd ELSE 0.0 END), 0.0),
                    COALESCE(SUM(CASE WHEN trial_run_label IS NULL THEN cost_usd ELSE 0.0 END), 0.0),
                    COALESCE(SUM(CASE WHEN trial_run_label IS NOT NULL THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN trial_run_label IS NULL THEN 1 ELSE 0 END), 0)
                 FROM llm_interactions",
                (),
            )
            .await?;
        let Some(row) = rows.next().await? else {
            return Ok(TrialSpendPartition::default());
        };
        Ok(TrialSpendPartition {
            benchmark_spend_usd: row.get(0)?,
            organic_spend_usd: row.get(1)?,
            benchmark_calls: row.get(2)?,
            organic_calls: row.get(3)?,
        })
    }
}

#[cfg(all(test, feature = "local"))]
mod tests {
    use super::*;
    use vox_db_types::store_types::trials::{TrialRunOutcome, TrialRunRecord, TrialSampleRecord};

    fn fixture_campaign(campaign_id: &str) -> TrialCampaignRecord {
        TrialCampaignRecord {
            campaign_id: campaign_id.to_string(),
            trusty_uri: format!("vox:prereg:RA-test-{campaign_id}"),
            prereg_json: r#"{"schema":"vox.trial_prereg.v1"}"#.to_string(),
            prereg_signature: "deadbeef".repeat(16),
            signer_pubkey: "cafebabe".repeat(8),
            reserved_budget_usd: 15.0,
            stopping_rule_kind: "group_sequential_obrien_fleming".to_string(),
            stopping_rule_params_json: r#"{"max_looks":3,"alpha":0.05}"#.to_string(),
            kb_snapshot_manifest_digest: Some("b3_manifest_digest".to_string()),
            kb_snapshot_row_count: Some(42),
            git_commit: "0123456789abcdef0123456789abcdef01234567".to_string(),
            cargo_lock_blake3: "blake3_cargo_lock".to_string(),
            uv_lock_blake3: None,
            rustc_version: "rustc 1.88.0".to_string(),
            host_triple: "aarch64-apple-darwin".to_string(),
            created_at_ms: 1_700_000_000_000,
            actor: "researcher@vox".to_string(),
        }
    }

    fn fixture_batch(campaign_id: &str, run_id: &str) -> TrialRunBatch {
        TrialRunBatch {
            run: TrialRunRecord {
                run_id: run_id.to_string(),
                campaign_id: campaign_id.to_string(),
                arm_id: "arm_hybrid".to_string(),
                seed: 42,
                paired_group_id: Some("pair-001".to_string()),
                adapter_tag: "research_shim".to_string(),
                model_id: "claude-sonnet-4-5".to_string(),
                memory_ablation_flag: "full_rag_hybrid".to_string(),
                started_at_ms: 1_700_000_001_000,
                primary_metric_name: "ndcg_at_5".to_string(),
            },
            outcome: TrialRunOutcome {
                status: "completed".to_string(),
                ended_at_ms: 1_700_000_002_500,
                wall_time_ms: 1_500,
                input_tokens: 1_200,
                output_tokens: 350,
                cost_usd: 0.25,
                primary_metric_value: Some(0.84),
                secondary_metrics_json: r#"{"recall_at_10":0.92}"#.to_string(),
                error_class: None,
                error_message: None,
            },
            samples: (0..4)
                .map(|idx| TrialSampleRecord {
                    run_id: run_id.to_string(),
                    campaign_id: campaign_id.to_string(),
                    item_id: format!("item-{idx}"),
                    prompt_blake3: format!("prompt-hash-{idx}"),
                    output_blake3: Some(format!("output-hash-{idx}")),
                    verdict: "pass".to_string(),
                    score: Some(0.8 + (idx as f64) * 0.02),
                    latency_ms: Some(300 + idx as i64 * 25),
                    input_tokens: 300,
                    output_tokens: 85,
                    cost_usd: 0.0625,
                    judge_model_id: Some("judge-v1".to_string()),
                    judge_verdict_json: Some(r#"{"pass":true}"#.to_string()),
                    recorded_at_ms: 1_700_000_002_000 + idx as i64,
                })
                .collect(),
            actor: "harness@vox".to_string(),
        }
    }

    #[tokio::test]
    async fn register_seal_transition_happy_path() {
        let db = VoxDb::open_memory().await.expect("open memory");
        let rec = fixture_campaign("camp-01");
        db.register_campaign(&rec).await.expect("register");

        let draft = db
            .get_campaign("camp-01")
            .await
            .expect("get")
            .expect("present");
        assert_eq!(draft.status, "draft");
        assert_eq!(draft.sealed_at_ms, None);

        db.seal_campaign("camp-01", "researcher@vox", 1_700_000_000_500)
            .await
            .expect("seal");
        let sealed = db
            .get_campaign("camp-01")
            .await
            .expect("get")
            .expect("present");
        assert_eq!(sealed.status, "sealed");
        assert_eq!(sealed.sealed_at_ms, Some(1_700_000_000_500));

        db.transition_campaign(
            "camp-01",
            "running",
            "harness@vox",
            None,
            1_700_000_000_900,
            PersistFailpoint::None,
        )
        .await
        .expect("start running");

        db.transition_campaign(
            "camp-01",
            "completed",
            "harness@vox",
            Some("fixed_n reached"),
            1_700_000_005_000,
            PersistFailpoint::None,
        )
        .await
        .expect("complete");

        let completed = db
            .get_campaign("camp-01")
            .await
            .expect("get")
            .expect("present");
        assert_eq!(completed.status, "completed");
        assert_eq!(completed.completed_at_ms, Some(1_700_000_005_000));

        let logs = db.list_campaign_logs("camp-01").await.expect("logs");
        let kinds: Vec<&str> = logs.iter().map(|l| l.event_kind.as_str()).collect();
        assert_eq!(kinds, vec!["created", "sealed", "started", "completed"]);
    }

    #[tokio::test]
    async fn illegal_transition_rejected() {
        let db = VoxDb::open_memory().await.expect("open memory");
        let rec = fixture_campaign("camp-02");
        db.register_campaign(&rec).await.expect("register");

        // Cannot jump straight from draft to running without sealing first.
        let err = db
            .transition_campaign(
                "camp-02",
                "running",
                "harness@vox",
                None,
                1_700_000_000_500,
                PersistFailpoint::None,
            )
            .await
            .expect_err("draft -> running must fail");
        assert!(matches!(err, StoreError::IllegalTrialTransition { .. }));

        // Campaign remains in draft and only has the 'created' log entry.
        let row = db
            .get_campaign("camp-02")
            .await
            .expect("get")
            .expect("present");
        assert_eq!(row.status, "draft");
        let logs = db.list_campaign_logs("camp-02").await.expect("logs");
        assert_eq!(logs.len(), 1);
    }

    #[tokio::test]
    async fn batch_commit_persists_run_samples_and_spend() {
        let db = VoxDb::open_memory().await.expect("open memory");
        let rec = fixture_campaign("camp-03");
        db.register_campaign(&rec).await.expect("register");
        db.seal_campaign("camp-03", "researcher@vox", 1_700_000_000_500)
            .await
            .expect("seal");
        db.transition_campaign(
            "camp-03",
            "running",
            "harness@vox",
            None,
            1_700_000_000_800,
            PersistFailpoint::None,
        )
        .await
        .expect("running");

        let batch = fixture_batch("camp-03", "run-01");
        db.record_trial_run_batch(&batch, PersistFailpoint::None)
            .await
            .expect("record batch");

        let runs = db.list_campaign_runs("camp-03").await.expect("runs");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, "completed");
        assert!((runs[0].cost_usd - 0.25).abs() < 1e-9);

        let samples = db.list_run_samples("run-01").await.expect("samples");
        assert_eq!(samples.len(), 4);

        let camp = db
            .get_campaign("camp-03")
            .await
            .expect("get")
            .expect("present");
        assert!((camp.spent_budget_usd - 0.25).abs() < 1e-9);
    }

    #[tokio::test]
    async fn failpoint_after_run_update_rolls_back_entire_batch() {
        let db = VoxDb::open_memory().await.expect("open memory");
        let rec = fixture_campaign("camp-04");
        db.register_campaign(&rec).await.expect("register");
        db.seal_campaign("camp-04", "researcher@vox", 1_700_000_000_500)
            .await
            .expect("seal");
        db.transition_campaign(
            "camp-04",
            "running",
            "harness@vox",
            None,
            1_700_000_000_800,
            PersistFailpoint::None,
        )
        .await
        .expect("running");

        let batch = fixture_batch("camp-04", "run-rollback-1");
        let err = db
            .record_trial_run_batch(&batch, PersistFailpoint::AfterRunUpdate)
            .await
            .expect_err("failpoint must trigger");
        assert!(matches!(
            err,
            StoreError::FailpointTriggered("AfterRunUpdate")
        ));

        assert!(
            db.list_campaign_runs("camp-04")
                .await
                .expect("runs")
                .is_empty()
        );
        assert!(
            db.list_run_samples("run-rollback-1")
                .await
                .expect("samples")
                .is_empty()
        );
        let camp = db
            .get_campaign("camp-04")
            .await
            .expect("get")
            .expect("present");
        assert_eq!(camp.spent_budget_usd, 0.0);
    }

    #[tokio::test]
    async fn failpoint_mid_samples_leaves_zero_samples_and_zero_spend_delta() {
        let db = VoxDb::open_memory().await.expect("open memory");
        let rec = fixture_campaign("camp-05");
        db.register_campaign(&rec).await.expect("register");
        db.seal_campaign("camp-05", "researcher@vox", 1_700_000_000_500)
            .await
            .expect("seal");
        db.transition_campaign(
            "camp-05",
            "running",
            "harness@vox",
            None,
            1_700_000_000_800,
            PersistFailpoint::None,
        )
        .await
        .expect("running");

        let batch = fixture_batch("camp-05", "run-rollback-2");
        let err = db
            .record_trial_run_batch(&batch, PersistFailpoint::AfterSampleAt(2))
            .await
            .expect_err("mid-sample failpoint must trigger");
        assert!(matches!(
            err,
            StoreError::FailpointTriggered("AfterSampleAt")
        ));

        assert!(
            db.list_campaign_runs("camp-05")
                .await
                .expect("runs")
                .is_empty()
        );
        assert!(
            db.list_run_samples("run-rollback-2")
                .await
                .expect("samples")
                .is_empty()
        );
        let camp = db
            .get_campaign("camp-05")
            .await
            .expect("get")
            .expect("present");
        assert_eq!(camp.spent_budget_usd, 0.0);
    }

    #[tokio::test]
    async fn invalid_vocabulary_rejected_before_write() {
        let db = VoxDb::open_memory().await.expect("open memory");
        let mut bad_rec = fixture_campaign("camp-06");
        bad_rec.stopping_rule_kind = "ad_hoc_peek".to_string();
        let err = db
            .register_campaign(&bad_rec)
            .await
            .expect_err("invalid stopping_rule_kind must fail");
        assert!(matches!(err, StoreError::InvalidTrialValue { .. }));
        assert!(db.get_campaign("camp-06").await.expect("get").is_none());
    }

    #[tokio::test]
    async fn spend_partition_separates_benchmark_from_organic() {
        let db = VoxDb::open_memory().await.expect("open memory");
        db.connection()
            .execute(
                "INSERT INTO llm_interactions (
                    session_id, prompt, response, model_version, cost_usd, trial_run_label
                ) VALUES ('s1', 'p', 'r', 'm1', 1.25, 'trial:camp-01:run-01')",
                (),
            )
            .await
            .expect("insert benchmark interaction");
        db.connection()
            .execute(
                "INSERT INTO llm_interactions (
                    session_id, prompt, response, model_version, cost_usd, trial_run_label
                ) VALUES ('s2', 'p', 'r', 'm1', 0.50, NULL)",
                (),
            )
            .await
            .expect("insert organic interaction");

        let part = db.partition_llm_spend().await.expect("partition");
        assert!((part.benchmark_spend_usd - 1.25).abs() < 1e-9);
        assert!((part.organic_spend_usd - 0.50).abs() < 1e-9);
        assert_eq!(part.benchmark_calls, 1);
        assert_eq!(part.organic_calls, 1);
    }
}
