//! Cloud GPU watchdog — kills instances that exceed time, cost, or idle thresholds.
//!
//! Spawned as `tokio::spawn(watchdog.run())` immediately after every dispatch.
//! Persists independently of the CLI process for the job lifetime.
//!
//! # Kill conditions (in evaluation order)
//!
//! 1. **Absolute hard cap** — elapsed > `absolute_max_runtime_secs` (always enforced)
//! 2. **Budget exhaustion** — `accrued >= global_cap_usd` (was dead code; now fixed)
//! 3. **Time overage** — elapsed > `estimated_secs × time_factor`
//! 4. **Idle GPU** — GPU util < `idle_pct` for `idle_grace_secs` after startup grace
//! 5. **Orphaned** — provider API unreachable for `max_poll_failures` polls
//!
//! # Normal completion
//!
//! When `poll_status` returns `Completed`, watchdog records the termination reason
//! and closes the Arca ledger entry.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{
    BudgetLedger, CloudProvider, CloudProviderConfig, JobHandle, JobStatus, TerminationReason,
};

/// Watchdog daemon for a single cloud GPU job.
pub struct CloudWatchdog {
    /// Provider client used for polling and termination.
    pub provider: Arc<dyn CloudProvider>,
    /// Handle to the running job.
    pub handle: JobHandle,
    /// Budget ledger for cost checks and closing the job record.
    pub budget: Arc<BudgetLedger>,
    /// Configuration shared with the resolver.
    pub config: Arc<CloudProviderConfig>,
}

impl CloudWatchdog {
    /// Spawn this watchdog as a background Tokio task.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(self.run())
    }

    /// Main watchdog loop.
    pub async fn run(self) {
        let started = Instant::now();

        // Derive deadline using Instant (monotonic) — not SystemTime
        let time_limit_secs = (self.handle.estimated_seconds * self.config.watchdog_time_factor)
            .max(self.config.min_deadline_secs as f64);
        let time_deadline = started + Duration::from_secs_f64(time_limit_secs);

        // Absolute hard cap — always enforced regardless of estimate
        let abs_deadline = if self.config.absolute_max_runtime_secs > 0 {
            Some(started + Duration::from_secs(self.config.absolute_max_runtime_secs))
        } else {
            None
        };

        let startup_grace_end =
            started + Duration::from_secs(self.config.watchdog_startup_grace_secs);
        let poll_interval = Duration::from_secs(self.config.watchdog_poll_secs);
        let mut idle_since: Option<Instant> = None;
        let mut consecutive_failures: u32 = 0;

        loop {
            tokio::time::sleep(poll_interval).await;
            let now = Instant::now();
            let accrued = self.handle.accrued_cost_usd();

            // ── Kill: absolute hard cap ───────────────────────────────────────
            if abs_deadline.map_or(false, |d| now >= d) {
                tracing::warn!(
                    "[watchdog:{}] Absolute hard cap {}s reached (${accrued:.3}). Terminating.",
                    self.handle.job_id,
                    self.config.absolute_max_runtime_secs
                );
                self.terminate_and_close(accrued, TerminationReason::WatchdogAbsoluteCap)
                    .await;
                return;
            }

            // ── Kill: budget exhausted ────────────────────────────────────────
            // Fixed: was `accrued >= remaining + accrued` (always false). Now compares
            // against the actual global cap.
            if accrued >= self.config.max_budget_usd {
                tracing::warn!(
                    "[watchdog:{}] Budget cap ${:.2} reached (accrued ${accrued:.3}). Terminating.",
                    self.handle.job_id,
                    self.config.max_budget_usd
                );
                self.terminate_and_close(accrued, TerminationReason::WatchdogBudget)
                    .await;
                return;
            }

            // ── Kill: time estimate × factor exceeded ─────────────────────────
            if !self.handle.is_persistent && now >= time_deadline {
                tracing::warn!(
                    "[watchdog:{}] Time limit {time_limit_secs:.0}s exceeded. Terminating.",
                    self.handle.job_id
                );
                self.terminate_and_close(accrued, TerminationReason::WatchdogTime)
                    .await;
                return;
            }

            // ── Poll provider status ──────────────────────────────────────────
            match self.provider.poll_status(&self.handle).await {
                Ok(status) => {
                    consecutive_failures = 0; // reset on successful poll
                    match status {
                        JobStatus::Completed { .. } => {
                            tracing::info!(
                                "[watchdog:{}] Completed normally (accrued ${accrued:.3}).",
                                self.handle.job_id
                            );
                            let _ = self
                                .budget
                                .close_job(
                                    &self.handle.job_id,
                                    accrued,
                                    TerminationReason::Completed,
                                )
                                .await;
                            return;
                        }
                        JobStatus::Terminated => {
                            tracing::info!(
                                "[watchdog:{}] Terminated by provider/user.",
                                self.handle.job_id
                            );
                            let _ = self
                                .budget
                                .close_job(
                                    &self.handle.job_id,
                                    accrued,
                                    TerminationReason::Completed,
                                )
                                .await;
                            return;
                        }
                        JobStatus::Failed(e) => {
                            tracing::error!("[watchdog:{}] Failed: {e}", self.handle.job_id);
                            let _ = self
                                .budget
                                .close_job(&self.handle.job_id, accrued, TerminationReason::Failed)
                                .await;
                            return;
                        }
                        JobStatus::Running {
                            gpu_util_pct: Some(util),
                            ..
                        } if !self.handle.is_persistent
                            && now >= startup_grace_end
                            && util < self.config.watchdog_idle_pct =>
                        {
                            let since = idle_since.get_or_insert(now);
                            if since.elapsed().as_secs() >= self.config.watchdog_idle_grace_secs {
                                tracing::warn!(
                                    "[watchdog:{}] GPU idle {util:.0}% for {}s. Terminating.",
                                    self.handle.job_id,
                                    since.elapsed().as_secs()
                                );
                                self.terminate_and_close(accrued, TerminationReason::WatchdogIdle)
                                    .await;
                                return;
                            }
                        }
                        JobStatus::Running { gpu_util_pct, .. } => {
                            idle_since = None;
                            tracing::debug!(
                                "[watchdog:{}] Running — util={} accrued=${accrued:.3}",
                                self.handle.job_id,
                                gpu_util_pct.map_or("?".into(), |u| format!("{u:.0}%")),
                            );
                        }
                        JobStatus::Pending => {
                            tracing::debug!("[watchdog:{}] Still pending.", self.handle.job_id);
                        }
                    }
                }
                Err(e) => {
                    consecutive_failures += 1;
                    tracing::warn!(
                        "[watchdog:{}] Poll failure {}/{}: {e}",
                        self.handle.job_id,
                        consecutive_failures,
                        self.config.watchdog_max_poll_failures
                    );
                    if consecutive_failures >= self.config.watchdog_max_poll_failures {
                        tracing::error!(
                            "[watchdog:{}] Orphaned — {} consecutive poll failures. Recording and exiting.",
                            self.handle.job_id,
                            consecutive_failures
                        );
                        let _ = self
                            .budget
                            .close_job(&self.handle.job_id, accrued, TerminationReason::Orphaned)
                            .await;
                        return;
                    }
                }
            }
        }
    }

    async fn terminate_and_close(&self, accrued_cost: f64, reason: TerminationReason) {
        if let Err(e) = self.provider.terminate(&self.handle).await {
            tracing::error!(
                "[watchdog:{}] Terminate request failed: {e}",
                self.handle.job_id
            );
        }
        if let Err(e) = self
            .budget
            .close_job(&self.handle.job_id, accrued_cost, reason)
            .await
        {
            tracing::error!(
                "[watchdog:{}] Failed to close budget record: {e}",
                self.handle.job_id
            );
        }
    }
}
