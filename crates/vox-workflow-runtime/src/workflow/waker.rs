//! Durable workflow timer wake-up polling.

use std::sync::Arc;

use vox_db::VoxDb;

/// Polls persisted workflow timer deadlines and makes due runs runnable.
pub struct WorkflowWaker {
    db: Arc<VoxDb>,
}

impl WorkflowWaker {
    /// Create a timer waker backed by the workflow database.
    pub fn new(db: Arc<VoxDb>) -> Self {
        Self { db }
    }

    /// Mark all currently due timer-parked runs as running.
    ///
    /// This deliberately does not claim a lease. The resumed workflow tracker
    /// claims it when execution starts.
    pub async fn tick(&self) -> anyhow::Result<Vec<String>> {
        let now_ms = wall_now_ms();
        let due = self.db.due_workflow_timer_run_ids(now_ms).await?;
        let mut resumed = Vec::with_capacity(due.len());
        for run_id in due {
            if self.db.wake_workflow_timer_run(&run_id, now_ms).await? {
                resumed.push(run_id);
            }
        }
        Ok(resumed)
    }
}

fn wall_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
