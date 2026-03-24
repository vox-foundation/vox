use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use crate::types::{TaskId, TaskStatus};
use crate::orchestrator::Orchestrator;
use crate::workflow_runtime::WorkflowTracker;
use tracing::info;

/// A bridge between the interpreted workflow runtime and the Orchestrator's task dispatch system.
pub struct OrchestratorWorkflowBridge {
    /// Shared orchestrator handle.
    pub orchestrator: Arc<Orchestrator>,
    /// Current session ID.
    pub session_id: String,
    /// Completion tracking for idempotency.
    pub activity_completions: HashMap<String, Value>,
    /// Active task mapping (activity_id -> TaskId).
    pub active_tasks: HashMap<String, TaskId>,
}

impl OrchestratorWorkflowBridge {
    /// Create a new bridge for a specific session.
    pub fn new(orchestrator: Arc<Orchestrator>, session_id: String) -> Self {
        Self {
            orchestrator,
            session_id,
            activity_completions: HashMap::new(),
            active_tasks: HashMap::new(),
        }
    }
}

impl WorkflowTracker for OrchestratorWorkflowBridge {
    fn is_activity_completed(
        &self,
        _workflow_name: &str,
        activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<bool>> + Send {
        let is_completed = self.activity_completions.contains_key(activity_id);
        async move { Ok(is_completed) }
    }

    fn on_workflow_started(
        &mut self,
        _workflow_name: &str,
        plan_len: usize,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        info!("Workflow started ({} steps)", plan_len);
        async { Ok(()) }
    }

    fn on_activity_started(
        &mut self,
        _workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        let a_name = activity_name.to_string();
        let a_id = activity_id.to_string();
        let s_id = self.session_id.clone();
        let orch = Arc::clone(&self.orchestrator);
        
        async move {
            info!("Workflow activity starting: {} ({})", a_name, a_id);
            
            // Submit task to orchestrator - it is now thread-safe (&self)
            let task_id = orch.submit_task(
                format!("Workflow Activity: {}", a_name),
                vec![], // Empty manifest
                None,   // Priority
                Some(s_id),
            ).await.map_err(|e| anyhow::anyhow!("Failed to submit workflow task: {}", e))?;
            
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                
                let status = orch.task_status(task_id);
                
                match status {
                    Some(TaskStatus::Completed) => break,
                    Some(TaskStatus::Failed(err)) => return Err(anyhow::anyhow!("Workflow activity failed: {}", err)),
                    Some(TaskStatus::Cancelled) => return Err(anyhow::anyhow!("Workflow activity cancelled")),
                    None => {
                        return Err(anyhow::anyhow!("Workflow activity task disappeared (TaskId: {})", task_id));
                    }
                    _ => { /* Still running or pending */ }
                }
            }
            Ok(())
        }
    }

    fn on_activity_completed(
        &mut self,
        _workflow_name: &str,
        _activity_name: &str,
        activity_id: &str,
        result: &Value,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        self.activity_completions.insert(activity_id.to_string(), result.clone());
        self.active_tasks.remove(activity_id);
        async { Ok(()) }
    }

    fn on_workflow_completed(
        &mut self,
        workflow_name: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        let w_name = workflow_name.to_string();
        async move {
            info!("Workflow completed: {}", w_name);
            Ok(())
        }
    }
}
