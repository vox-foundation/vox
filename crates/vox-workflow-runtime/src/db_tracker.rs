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
}

impl VoxDbTracker {
    /// Create a tracker for a specific workflow run.
    pub fn new(db: Arc<VoxDb>, run_id: impl Into<String>) -> Self {
        Self {
            db,
            run_id: run_id.into(),
        }
    }
}

impl WorkflowTracker for VoxDbTracker {
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
        let workflow_name = workflow_name.to_string();
        let activity_name = activity_name.to_string();
        let activity_id = activity_id.to_string();
        async move {
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
        _result: &serde_json::Value,
    ) -> impl Future<Output = Result<()>> + Send {
        let db = self.db.clone();
        let run_id = self.run_id.clone();
        let workflow_name = workflow_name.to_string();
        let activity_name = activity_name.to_string();
        let activity_id = activity_id.to_string();
        async move {
            db.record_workflow_activity_completed(
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

    fn on_workflow_completed(
        &mut self,
        _workflow_name: &str,
    ) -> impl Future<Output = Result<()>> + Send {
        async move { Ok(()) }
    }
}
