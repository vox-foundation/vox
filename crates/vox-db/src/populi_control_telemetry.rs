//! Mens control-plane audit rows in `research_metrics` (session `mens:<repository_id>`).

use serde::Serialize;
use serde_json::Value;

use crate::{StoreError, VoxDb};

#[derive(Debug, Clone, Serialize)]
struct MeshControlEventMeta {
    /// Event name (`local_registry_publish`, `orchestrator_status_snapshot`, …).
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl VoxDb {
    /// Append one mens observation under `mens:<repository_id>`, metric type `populi_control_event`.
    pub async fn record_populi_control_event(
        &self,
        repository_id: &str,
        name: &str,
        details: Option<Value>,
    ) -> Result<i64, StoreError> {
        let meta = MeshControlEventMeta {
            name: name.to_string(),
            repository_id: Some(repository_id.to_string()),
            details,
        };
        let json =
            serde_json::to_string(&meta).map_err(|e| StoreError::Serialization(e.to_string()))?;
        let session = format!("mens:{repository_id}");
        self.append_research_metric(&session, "populi_control_event", None, Some(&json))
            .await
    }
}
