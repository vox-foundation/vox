//! Interpreted workflow journal rows in `research_metrics` (session `workflow:<repository_id>`).

use serde::Serialize;
use serde_json::Value;

use crate::{StoreError, VoxDb};

#[derive(Debug, Clone, Serialize)]
struct WorkflowJournalMeta {
    /// Workflow name executed.
    pub workflow: String,
    /// One journal object (e.g. `WorkflowStarted`, `LocalActivity`, `MeshActivity`).
    pub entry: Value,
}

impl VoxDb {
    /// Append one interpreted-workflow journal row under `workflow:<repository_id>`, metric `workflow_journal_entry`.
    pub async fn record_workflow_journal_entry(
        &self,
        repository_id: &str,
        workflow_name: &str,
        entry: &Value,
    ) -> Result<i64, StoreError> {
        let meta = WorkflowJournalMeta {
            workflow: workflow_name.to_string(),
            entry: entry.clone(),
        };
        let json =
            serde_json::to_string(&meta).map_err(|e| StoreError::Serialization(e.to_string()))?;
        let session = format!("workflow:{repository_id}");
        self.append_research_metric(&session, "workflow_journal_entry", None, Some(&json))
            .await
    }
}
