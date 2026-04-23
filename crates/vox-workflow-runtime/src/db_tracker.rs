//! SQL-backed [`WorkflowTracker`] implementation using `vox_db::VoxDb`.
//!
//! Persists every activity start/complete to the `workflow_activity_log` table,
//! enabling crash-safe resume: if a workflow is interrupted mid-execution, the
//! next `interpret_workflow_durable` call will skip already-completed steps.

use crate::WorkflowTracker;
use anyhow::Result;
use std::future::Future;
use std::sync::Arc;
use vox_db::VoxDb;

/// A durable workflow tracker that persists step completions to VoxDb.
pub struct VoxDbTracker {
    db: Arc<VoxDb>,
    run_id: String,
    owner_id: String,
    lease_ttl_ms: u64,
    plan_context: Option<(String, String, i64)>,
}

impl VoxDbTracker {
    /// Create a tracker for a specific workflow run.
    pub fn new(db: Arc<VoxDb>, run_id: impl Into<String>) -> Self {
        let pid = std::process::id();
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self {
            db,
            run_id: run_id.into(),
            owner_id: format!("workflow-runner-{pid}-{now_ns}"),
            lease_ttl_ms: 30_000,
            plan_context: None,
        }
    }

    /// Create a tracker with explicit lease owner and TTL (tests and advanced runners).
    pub fn with_owner(
        db: Arc<VoxDb>,
        run_id: impl Into<String>,
        owner_id: impl Into<String>,
        lease_ttl_ms: u64,
    ) -> Self {
        Self {
            db,
            run_id: run_id.into(),
            owner_id: owner_id.into(),
            lease_ttl_ms: lease_ttl_ms.max(1),
            plan_context: None,
        }
    }

    /// Attach optional orchestration planning context for this run tracker.
    pub fn with_plan_context(
        mut self,
        plan_session_id: impl Into<String>,
        plan_node_id: impl Into<String>,
        plan_version: i64,
    ) -> Self {
        self.plan_context = Some((
            plan_session_id.into(),
            plan_node_id.into(),
            plan_version.max(1),
        ));
        self
    }
}

impl WorkflowTracker for VoxDbTracker {
    fn on_workflow_started(
        &mut self,
        workflow_name: &str,
        plan_len: usize,
    ) -> impl Future<Output = Result<()>> + Send {
        let db = self.db.clone();
        let run_id = self.run_id.clone();
        let owner_id = self.owner_id.clone();
        let lease_ttl_ms = self.lease_ttl_ms;
        let plan_context = self.plan_context.clone();
        let workflow_name = workflow_name.to_string();
        async move {
            if db
                .is_workflow_run_cancelled(&run_id)
                .await
                .map_err(|e| anyhow::anyhow!("DB error: {}", e))?
            {
                anyhow::bail!("workflow run `{}` is cancelled", run_id);
            }
            db.record_workflow_run_started(&run_id, &workflow_name, plan_len)
                .await
                .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            if let Some((plan_session_id, plan_node_id, plan_version)) = plan_context {
                db.record_workflow_run_plan_context(
                    &run_id,
                    &plan_session_id,
                    &plan_node_id,
                    plan_version,
                )
                .await
                .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            }
            let lease_claimed = db
                .try_claim_workflow_run_lease(&run_id, &owner_id, lease_ttl_ms)
                .await
                .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            if !lease_claimed {
                anyhow::bail!(
                    "workflow run lease conflict for run_id `{}`: owner `{}` is active",
                    run_id,
                    owner_id
                );
            }
            Ok(())
        }
    }

    fn is_activity_completed(
        &self,
        workflow_name: &str,
        activity_id: &str,
    ) -> impl Future<Output = Result<bool>> + Send {
        let db = self.db.clone();
        let run_id = self.run_id.clone();
        let workflow_name = workflow_name.to_string();
        let activity_id = activity_id.to_string();
        async move {
            let res: bool = db
                .is_workflow_activity_completed(&run_id, &workflow_name, &activity_id)
                .await
                .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            Ok(res)
        }
    }

    fn on_activity_started(
        &mut self,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
    ) -> impl Future<Output = Result<()>> + Send {
        let db = self.db.clone();
        let run_id = self.run_id.clone();
        let owner_id = self.owner_id.clone();
        let lease_ttl_ms = self.lease_ttl_ms;
        let workflow_name = workflow_name.to_string();
        let activity_name = activity_name.to_string();
        let activity_id = activity_id.to_string();
        async move {
            if db
                .is_workflow_run_cancelled(&run_id)
                .await
                .map_err(|e| anyhow::anyhow!("DB error: {}", e))?
            {
                anyhow::bail!(
                    "workflow run `{}` is cancelled before activity `{}`",
                    run_id,
                    activity_id
                );
            }
            let lease_claimed = db
                .try_claim_workflow_run_lease(&run_id, &owner_id, lease_ttl_ms)
                .await
                .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            if !lease_claimed {
                anyhow::bail!(
                    "workflow run lease lost for run_id `{}` before activity `{}`",
                    run_id,
                    activity_id
                );
            }
            if let Some(signal_key) = activity_name.strip_prefix("__durable_signal_wait:") {
                let consumed = db
                    .consume_workflow_signal(&run_id, signal_key)
                    .await
                    .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
                if !consumed {
                    anyhow::bail!(
                        "workflow run `{}` is waiting for signal `{}`",
                        run_id,
                        signal_key
                    );
                }
            }
            db.record_workflow_activity_started(
                &run_id,
                &workflow_name,
                &activity_name,
                &activity_id,
            )
            .await
            .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            Ok(())
        }
    }

    fn on_activity_completed(
        &mut self,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
        result: &serde_json::Value,
    ) -> impl Future<Output = Result<()>> + Send {
        let db = self.db.clone();
        let run_id = self.run_id.clone();
        let workflow_name = workflow_name.to_string();
        let activity_name = activity_name.to_string();
        let activity_id = activity_id.to_string();
        let result = result.clone();
        async move {
            db.record_workflow_activity_completed(
                &run_id,
                &workflow_name,
                &activity_name,
                &activity_id,
                &result,
            )
            .await
            .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            Ok(())
        }
    }

    fn on_activity_attempt_started(
        &mut self,
        workflow_name: &str,
        _activity_name: &str,
        activity_id: &str,
        attempt: u32,
    ) -> impl Future<Output = Result<()>> + Send {
        let db = self.db.clone();
        let run_id = self.run_id.clone();
        let owner_id = self.owner_id.clone();
        let lease_ttl_ms = self.lease_ttl_ms;
        let workflow_name = workflow_name.to_string();
        let activity_id = activity_id.to_string();
        async move {
            db.record_workflow_activity_attempt_started(
                &run_id,
                &workflow_name,
                &activity_id,
                attempt,
                &owner_id,
                lease_ttl_ms,
            )
            .await
            .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            Ok(())
        }
    }

    fn on_activity_attempt_failed(
        &mut self,
        workflow_name: &str,
        _activity_name: &str,
        activity_id: &str,
        attempt: u32,
        error: &str,
    ) -> impl Future<Output = Result<()>> + Send {
        let db = self.db.clone();
        let run_id = self.run_id.clone();
        let owner_id = self.owner_id.clone();
        let workflow_name = workflow_name.to_string();
        let activity_id = activity_id.to_string();
        let error = error.to_string();
        async move {
            db.record_workflow_activity_attempt_failed(
                &run_id,
                &workflow_name,
                &activity_id,
                attempt,
                &owner_id,
                &error,
            )
            .await
            .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            Ok(())
        }
    }

    fn on_activity_attempt_completed(
        &mut self,
        workflow_name: &str,
        _activity_name: &str,
        activity_id: &str,
        attempt: u32,
    ) -> impl Future<Output = Result<()>> + Send {
        let db = self.db.clone();
        let run_id = self.run_id.clone();
        let owner_id = self.owner_id.clone();
        let workflow_name = workflow_name.to_string();
        let activity_id = activity_id.to_string();
        async move {
            db.record_workflow_activity_attempt_completed(
                &run_id,
                &workflow_name,
                &activity_id,
                attempt,
                &owner_id,
            )
            .await
            .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            Ok(())
        }
    }

    fn next_activity_attempt_start(
        &self,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
    ) -> impl Future<Output = Result<u32>> + Send {
        let db = self.db.clone();
        let run_id = self.run_id.clone();
        let owner_id = self.owner_id.clone();
        let workflow_name = workflow_name.to_string();
        let activity_name = activity_name.to_string();
        let activity_id = activity_id.to_string();
        async move {
            let latest = db
                .load_latest_workflow_activity_attempt(&run_id, &workflow_name, &activity_id)
                .await
                .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            let Some((attempt_no, status, worker_owner, lease_until_ms)) = latest else {
                return Ok(1);
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            if status == "started"
                && let (Some(owner), Some(lease_until)) = (worker_owner.as_deref(), lease_until_ms)
                && lease_until >= now
                && owner != owner_id
            {
                anyhow::bail!(
                    "activity attempt lease active for run `{}` activity `{}` owner `{}`",
                    run_id,
                    activity_name,
                    owner
                );
            }
            Ok(attempt_no.saturating_add(1))
        }
    }

    fn load_activity_result(
        &self,
        workflow_name: &str,
        activity_id: &str,
    ) -> impl Future<Output = Result<Option<serde_json::Value>>> + Send {
        let db = self.db.clone();
        let run_id = self.run_id.clone();
        let workflow_name = workflow_name.to_string();
        let activity_id = activity_id.to_string();
        async move {
            let result = db
                .load_workflow_activity_result(&run_id, &workflow_name, &activity_id)
                .await
                .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            Ok(result)
        }
    }

    fn on_workflow_completed(
        &mut self,
        workflow_name: &str,
    ) -> impl Future<Output = Result<()>> + Send {
        let db = self.db.clone();
        let run_id = self.run_id.clone();
        let workflow_name = workflow_name.to_string();
        async move {
            db.record_workflow_run_completed(&run_id, &workflow_name)
                .await
                .map_err(|e| anyhow::anyhow!("DB error: {}", e))?;
            Ok(())
        }
    }
}
