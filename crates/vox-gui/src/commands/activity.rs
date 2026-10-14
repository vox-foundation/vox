//! Tauri command for querying the activity log. Served by the daemon (`GET /api/v2/activity`).

use std::sync::Arc;

use serde::Deserialize;
use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::orchestrator::ActivityRowDto;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query};

/// Filter options for querying the activity log.
#[derive(Debug, Clone, Deserialize)]
pub struct ActivityFilter {
    pub agent_id: Option<String>,
    pub kind: Option<String>,
    pub session_id: Option<String>,
    pub limit: u32,
    pub before_id: Option<i64>,
}

fn activity_path(filter: ActivityFilter) -> String {
    path_with_query(
        "/activity",
        &[
            ("agent_id", filter.agent_id),
            ("kind", filter.kind),
            ("session_id", filter.session_id),
            ("limit", Some(filter.limit.to_string())),
            ("before_id", filter.before_id.map(|id| id.to_string())),
        ],
    )
}

pub(crate) async fn query_impl<B: BackendCalls>(
    b: &B,
    filter: ActivityFilter,
) -> Result<Vec<ActivityRowDto>, String> {
    api_data(b.api_v2_get(&activity_path(filter)).await?)
}

/// Query the activity log, newest first.
#[tauri::command]
pub async fn activity_query(
    backend: State<'_, Arc<BackendState>>,
    filter: ActivityFilter,
) -> Result<Vec<ActivityRowDto>, String> {
    query_impl(&backend.current().await, filter).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;
    use serde_json::json;

    fn filter() -> ActivityFilter {
        ActivityFilter {
            agent_id: None,
            kind: None,
            session_id: None,
            limit: 50,
            before_id: None,
        }
    }

    #[test]
    fn every_filter_field_becomes_a_query_param() {
        let path = activity_path(ActivityFilter {
            agent_id: Some("A1".into()),
            kind: Some("TaskCompleted".into()),
            session_id: Some("chat:s1".into()),
            before_id: Some(100),
            ..filter()
        });
        assert_eq!(
            path,
            "/activity?agent_id=A1&kind=TaskCompleted&session_id=chat%3As1&limit=50&before_id=100"
        );
        assert_eq!(activity_path(filter()), "/activity?limit=50");
    }

    #[tokio::test]
    async fn query_reads_the_activity_route() {
        let b = FakeBackend::new().on_get(
            "/activity?limit=50",
            Ok(json!({ "v": 1, "data": [{
                "id": 1, "ts_ms": 2, "agent_id": null, "session_id": null,
                "kind": "k", "summary": "s", "detail_json": "{}"
            }] })),
        );
        let rows = query_impl(&b, filter()).await.unwrap();
        assert_eq!(rows[0].kind, "k");
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("activity.rs");
        assert!(!src.contains(concat!("vox", "_db")));
        assert!(!src.contains(concat!("tur", "so")));
    }
}
