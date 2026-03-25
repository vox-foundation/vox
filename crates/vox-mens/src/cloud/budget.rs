//! Arca-backed budget ledger — cost tracking and dispatch gating for cloud GPU jobs.
//!
//! Cap is provided by `CloudProviderConfig.max_budget_usd`, not independently from env,
//! preventing disagreement between the ledger and the watchdog.

use std::sync::Arc;

use vox_db::VoxDb;

use super::{CloudProviderConfig, JobHandle, TerminationReason};

/// Arca-backed ledger: tracks accrued spend and gates dispatch against the global cap.
pub struct BudgetLedger {
    db: Option<Arc<VoxDb>>,
    /// Global spend cap in USD — passed directly from `CloudProviderConfig.max_budget_usd`.
    pub global_cap_usd: f64,
}

impl BudgetLedger {
    /// Construct from a VoxDb handle and config.
    ///
    /// Cap is taken from `config.max_budget_usd` — **not** re-read from env.
    pub fn new(db: Option<Arc<VoxDb>>, config: &CloudProviderConfig) -> Self {
        Self { db, global_cap_usd: config.max_budget_usd }
    }

    /// Current accrued cost from all running jobs (pro-rated by elapsed time).
    pub async fn current_accrued_usd(&self) -> f64 {
        if let Some(ref db) = self.db {
            db.cloud_accrued_cost_usd().await.unwrap_or(0.0)
        } else {
            0.0
        }
    }

    /// Remaining budget before the global cap is hit.
    pub async fn remaining_usd(&self) -> f64 {
        (self.global_cap_usd - self.current_accrued_usd().await).max(0.0)
    }

    /// Gate dispatch: error if `new_job_cost` would push total over the cap.
    pub async fn check_capacity(&self, new_job_cost: f64) -> anyhow::Result<()> {
        let accrued = self.current_accrued_usd().await;
        if accrued + new_job_cost > self.global_cap_usd {
            anyhow::bail!(
                "Budget cap ${:.2} would be exceeded:\n  \
                 current accrued: ${accrued:.2}\n  \
                 new job estimate: ${new_job_cost:.2}\n  \
                 total: ${:.2} > cap ${:.2}\n\n\
                 Use --max-budget=N to raise the cap, or wait for running jobs to complete.\n\
                 Check spend: vox mens status --cloud",
                self.global_cap_usd,
                accrued + new_job_cost,
                self.global_cap_usd,
            );
        }
        Ok(())
    }

    /// Record a newly dispatched job in Arca `cloud_dispatch_log`.
    #[allow(clippy::too_many_arguments)]
    pub async fn open_job(
        &self,
        handle: &JobHandle,
        offer_id: &str,
        gpu_name: &str,
        vram_mb: u64,
        est_cost: f64,
        job_kind: &str,
    ) -> anyhow::Result<()> {
        if let Some(ref db) = self.db {
            db.cloud_open_job(
                &handle.job_id,
                handle.provider.display_name(),
                offer_id,
                gpu_name,
                vram_mb,
                handle.price_per_hour_usd,
                est_cost,
                job_kind,
            ).await.map_err(anyhow::Error::from)
        } else {
            Ok(())
        }
    }

    /// Mark a job complete, record actual cost and why it ended.
    pub async fn close_job(
        &self,
        job_id: &str,
        actual_cost: f64,
        reason: TerminationReason,
    ) -> anyhow::Result<()> {
        if let Some(ref db) = self.db {
            db.cloud_close_job(job_id, actual_cost, reason.as_str())
                .await.map_err(anyhow::Error::from)
        } else {
            Ok(())
        }
    }
}
