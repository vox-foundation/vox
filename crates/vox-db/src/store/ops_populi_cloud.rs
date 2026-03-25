//! Arca CRUD for Populi cloud GPU dispatch tables.
//!
//! Tables: `cloud_dispatch_log`, `training_throughput_profiles`, `local_train_log`.
//! Schema defined in `crates/vox-db/src/schema/domains/populi_cloud.rs`.

use crate::store::types::StoreError;

impl crate::VoxDb {
    /// Compute the total USD currently accrued by all running cloud jobs.
    ///
    /// Uses `unixepoch` for second-precision elapsed time (not julianday which
    /// accumulates float errors for sub-hour durations).
    pub async fn cloud_accrued_cost_usd(&self) -> Result<f64, StoreError> {
        let mut rows = self.conn.query(
            "SELECT COALESCE(SUM(
                 (CAST(unixepoch('now') - unixepoch(created_at) AS REAL) / 3600.0)
                 * price_per_hr_usd
             ), 0.0)
             FROM cloud_dispatch_log WHERE status = 'running'",
            (),
        ).await?;
        let cost: f64 = rows.next().await?.and_then(|r| r.get(0).ok()).unwrap_or(0.0);
        Ok(cost.max(0.0))
    }

    /// Insert a new running job into `cloud_dispatch_log`.
    #[allow(clippy::too_many_arguments)]
    pub async fn cloud_open_job(
        &self,
        job_id: &str,
        provider: &str,
        offer_id: &str,
        gpu_name: &str,
        vram_mb: u64,
        price_per_hr_usd: f64,
        estimated_cost: f64,
        job_kind: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO cloud_dispatch_log
             (job_id, provider, offer_id, gpu_name, vram_mb, price_per_hr_usd,
              estimated_cost, job_kind, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'running')",
            (job_id, provider, offer_id, gpu_name, vram_mb as i64,
             price_per_hr_usd, estimated_cost, job_kind),
        ).await?;
        Ok(())
    }

    /// Mark a job complete, record actual cost and termination reason for audit.
    pub async fn cloud_close_job(
        &self,
        job_id: &str,
        actual_cost: f64,
        termination_reason: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE cloud_dispatch_log
             SET status = 'completed',
                 actual_cost = ?2,
                 termination_reason = ?3,
                 completed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE job_id = ?1",
            (job_id, actual_cost, termination_reason),
        ).await?;
        Ok(())
    }

    /// Update phase timing metadata for a running job (called by the container via sidecar).
    pub async fn cloud_update_phase(
        &self,
        job_id: &str,
        setup_secs: Option<f64>,
        download_secs: Option<f64>,
        train_secs: Option<f64>,
        upload_secs: Option<f64>,
        total_steps: Option<i64>,
        total_tokens: Option<i64>,
    ) -> Result<(), StoreError> {
        // Compute tokens_per_dollar if we have both tokens and enough cost accrued
        self.conn.execute(
            "UPDATE cloud_dispatch_log SET
                setup_secs    = COALESCE(?2, setup_secs),
                download_secs = COALESCE(?3, download_secs),
                train_secs    = COALESCE(?4, train_secs),
                upload_secs   = COALESCE(?5, upload_secs),
                total_steps   = COALESCE(?6, total_steps),
                total_tokens  = COALESCE(?7, total_tokens),
                tokens_per_dollar = CASE
                    WHEN ?7 IS NOT NULL AND estimated_cost > 0
                    THEN CAST(?7 AS REAL) / estimated_cost
                    ELSE tokens_per_dollar
                END
             WHERE job_id = ?1",
            (job_id, setup_secs, download_secs, train_secs, upload_secs,
             total_steps, total_tokens),
        ).await?;
        Ok(())
    }

    /// Load all measured training throughput profiles.
    ///
    /// Returns `(gpu_name, seq_len, batch_size, ms_per_step)` tuples.
    pub async fn cloud_load_throughput_profiles(
        &self,
    ) -> Result<Vec<(String, usize, usize, f64)>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT gpu_name, seq_len, batch_size, ms_per_step
             FROM training_throughput_profiles
             ORDER BY gpu_name, seq_len, batch_size",
            (),
        ).await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            let gpu: String = r.get(0)?;
            let seq_len: i64 = r.get(1).unwrap_or(512);
            let batch: i64 = r.get(2).unwrap_or(1);
            let ms: f64 = r.get(3).unwrap_or(200.0);
            out.push((gpu, seq_len as usize, batch as usize, ms));
        }
        Ok(out)
    }

    /// Upsert a measured training throughput profile using EMA smoothing (α=0.3).
    pub async fn cloud_upsert_throughput_profile(
        &self,
        gpu_name: &str,
        seq_len: usize,
        batch_size: usize,
        ms_per_step: f64,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO training_throughput_profiles
             (gpu_name, seq_len, batch_size, ms_per_step, sample_count)
             VALUES (?1, ?2, ?3, ?4, 1)
             ON CONFLICT(gpu_name, seq_len, batch_size) DO UPDATE SET
               ms_per_step = 0.3 * excluded.ms_per_step + 0.7 * ms_per_step,
               sample_count = sample_count + 1,
               last_updated = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
            (gpu_name, seq_len as i64, batch_size as i64, ms_per_step),
        ).await?;
        Ok(())
    }

    /// Record a local GPU training run (4080 Super etc.) for cost/efficiency parity.
    pub async fn local_log_train_run(
        &self,
        gpu_name: &str,
        model_id: &str,
        preset: &str,
        wall_secs: f64,
        total_steps: i64,
        total_tokens: i64,
        ms_per_step: Option<f64>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO local_train_log
             (gpu_name, model_id, preset, wall_secs, total_steps, total_tokens, ms_per_step)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (gpu_name, model_id, preset, wall_secs, total_steps, total_tokens, ms_per_step),
        ).await?;
        // Also upsert the throughput profile so the cloud estimator has local data
        if let Some(ms) = ms_per_step {
            // Derive seq_len/batch_size from preset name (stored separately; use defaults)
            let _ = self.cloud_upsert_throughput_profile(gpu_name, 512, 1, ms).await;
        }
        Ok(())
    }

    /// Summarize completed cloud and local jobs for `vox populi status --cloud`.
    pub async fn cloud_cost_summary(&self) -> Result<CloudCostSummary, StoreError> {
        let mut rows = self.conn.query(
            "SELECT
                COUNT(*) FILTER (WHERE status='running')            AS running,
                COUNT(*) FILTER (WHERE status='completed')          AS completed,
                COALESCE(SUM(actual_cost) FILTER (WHERE status='completed'), 0.0) AS total_spent,
                COALESCE(AVG(tokens_per_dollar) FILTER (WHERE tokens_per_dollar IS NOT NULL), 0.0) AS avg_tpd
             FROM cloud_dispatch_log",
            (),
        ).await?;
        if let Some(r) = rows.next().await? {
            Ok(CloudCostSummary {
                running_jobs: r.get::<i64>(0).unwrap_or(0) as u32,
                completed_jobs: r.get::<i64>(1).unwrap_or(0) as u32,
                total_spent_usd: r.get::<f64>(2).unwrap_or(0.0),
                avg_tokens_per_dollar: r.get::<f64>(3).unwrap_or(0.0),
                accrued_usd: self.cloud_accrued_cost_usd().await.unwrap_or(0.0),
            })
        } else {
            Ok(CloudCostSummary::default())
        }
    }
}

/// Aggregated cost and efficiency summary for CLI display.
#[derive(Debug, Default)]
pub struct CloudCostSummary {
    /// Number of currently running cloud jobs.
    pub running_jobs: u32,
    /// Number of completed cloud jobs.
    pub completed_jobs: u32,
    /// Total USD spent on completed jobs.
    pub total_spent_usd: f64,
    /// Average tokens processed per dollar across all completed jobs.
    pub avg_tokens_per_dollar: f64,
    /// Currently accruing cost from running jobs.
    pub accrued_usd: f64,
}
