//! Tauri commands for GUI-driven edits to a live plan DAG. Served by the daemon
//! (`GET /api/v2/plan/*` and the `vox_plan_node_upsert` / `vox_plan_nodes_approve` ops), which
//! write through the same `upsert_plan_node` primitive the orchestrator's plan synthesis uses —
//! so edits take effect for any node the scheduler hasn't dispatched yet.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::orchestrator::{
    InsertPlanNodeInput, LatestPlanSessionForChat, PlanNodeDto, UpdatePlanNodeInput,
};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, encode_component, path_with_query, tool_data};

const UPSERT_TOOL: &str = "vox_plan_node_upsert";
const APPROVE_TOOL: &str = "vox_plan_nodes_approve";

pub(crate) async fn update_impl<B: BackendCalls>(
    b: &B,
    input: UpdatePlanNodeInput,
) -> Result<(), String> {
    let args = json!({ "mode": "update", "update": input });
    tool_data(UPSERT_TOOL, b.call_tool(UPSERT_TOOL, args).await?)
}

pub(crate) async fn insert_impl<B: BackendCalls>(
    b: &B,
    input: InsertPlanNodeInput,
) -> Result<(), String> {
    let args = json!({ "mode": "insert", "insert": input });
    tool_data(UPSERT_TOOL, b.call_tool(UPSERT_TOOL, args).await?)
}

pub(crate) async fn list_nodes_impl<B: BackendCalls>(
    b: &B,
    plan_session_id: &str,
    plan_version: i64,
) -> Result<Vec<PlanNodeDto>, String> {
    let path = path_with_query(
        &format!("/plan/sessions/{}/nodes", encode_component(plan_session_id)),
        &[("version", Some(plan_version.to_string()))],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn open_task_counts_impl<B: BackendCalls>(
    b: &B,
    session_ids: &[String],
) -> Result<HashMap<String, i64>, String> {
    if session_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let path = path_with_query(
        "/plan/open-task-counts",
        &[("session_ids", Some(session_ids.join(",")))],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn approve_impl<B: BackendCalls>(
    b: &B,
    plan_session_id: String,
) -> Result<u64, String> {
    let args = json!({ "plan_session_id": plan_session_id });
    tool_data(APPROVE_TOOL, b.call_tool(APPROVE_TOOL, args).await?)
}

pub(crate) async fn latest_for_chat_impl<B: BackendCalls>(
    b: &B,
    session_id: &str,
) -> Result<Option<LatestPlanSessionForChat>, String> {
    let path = format!("/plan/latest-for-chat/{}", encode_component(session_id));
    api_data(b.api_v2_get(&path).await?)
}

/// Edit a not-yet-dispatched plan node's description.
#[tauri::command]
pub async fn update_plan_node(
    backend: State<'_, Arc<BackendState>>,
    input: UpdatePlanNodeInput,
) -> Result<(), String> {
    update_impl(&backend.current().await, input).await
}

/// Insert a new intermediate step into the live plan; inserted `blocked_on_approval` while the
/// plan is still unapproved.
#[tauri::command]
pub async fn insert_plan_node(
    backend: State<'_, Arc<BackendState>>,
    input: InsertPlanNodeInput,
) -> Result<(), String> {
    insert_impl(&backend.current().await, input).await
}

/// The current plan-node rows for a session/version, for the plan panel's live DAG.
#[tauri::command]
pub async fn list_plan_nodes(
    backend: State<'_, Arc<BackendState>>,
    plan_session_id: String,
    plan_version: i64,
) -> Result<Vec<PlanNodeDto>, String> {
    list_nodes_impl(&backend.current().await, &plan_session_id, plan_version).await
}

/// Batched open-task counts for the sidebar's task-count badges, keyed by chat session id; a
/// session absent from the map has zero open tasks.
#[tauri::command]
pub async fn plan_open_task_counts(
    backend: State<'_, Arc<BackendState>>,
    session_ids: Vec<String>,
) -> Result<HashMap<String, i64>, String> {
    open_task_counts_impl(&backend.current().await, &session_ids).await
}

/// Approve every `blocked_on_approval` node in a plan session — the `PlanPanel` footer's
/// "Approve" button.
#[tauri::command]
pub async fn approve_plan_nodes(
    backend: State<'_, Arc<BackendState>>,
    plan_session_id: String,
) -> Result<u64, String> {
    approve_impl(&backend.current().await, plan_session_id).await
}

/// The most recently updated plan session linked to a chat session, if any.
#[tauri::command]
pub async fn latest_plan_session_for_chat(
    backend: State<'_, Arc<BackendState>>,
    session_id: String,
) -> Result<Option<LatestPlanSessionForChat>, String> {
    latest_for_chat_impl(&backend.current().await, &session_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    #[tokio::test]
    async fn reads_go_to_the_plan_routes() {
        let b = FakeBackend::new()
            .on_get(
                "/plan/sessions/ps%2F1/nodes?version=2",
                Ok(json!({ "v": 1, "data": [{
                    "plan_session_id": "ps/1", "version": 2, "node_id": "n1",
                    "description": "d", "dependencies_json": "[]",
                    "execution_policy_json": "{}", "status": "pending",
                    "workflow_invocation": null
                }] })),
            )
            .on_get(
                "/plan/open-task-counts?session_ids=a%2Cb",
                Ok(json!({ "v": 1, "data": { "a": 3 } })),
            )
            .on_get(
                "/plan/latest-for-chat/chat-x",
                Ok(json!({ "v": 1, "data": null })),
            );
        let nodes = list_nodes_impl(&b, "ps/1", 2).await.unwrap();
        assert_eq!(nodes[0].node_id, "n1");
        let counts = open_task_counts_impl(&b, &["a".into(), "b".into()])
            .await
            .unwrap();
        assert_eq!(counts.get("a"), Some(&3));
        assert_eq!(latest_for_chat_impl(&b, "chat-x").await.unwrap(), None);
    }

    #[tokio::test]
    async fn empty_session_list_skips_the_round_trip() {
        let b = FakeBackend::new();
        assert!(open_task_counts_impl(&b, &[]).await.unwrap().is_empty());
        assert!(b.calls().is_empty());
    }

    #[tokio::test]
    async fn writes_go_to_the_upsert_and_approve_ops() {
        let b = FakeBackend::new()
            .on_tool(UPSERT_TOOL, Ok(json!({ "success": true, "data": null })))
            .on_tool(APPROVE_TOOL, Ok(json!({ "success": true, "data": 2 })));
        update_impl(
            &b,
            UpdatePlanNodeInput {
                plan_session_id: "ps1".into(),
                plan_version: 1,
                node_id: "n1".into(),
                description: "edited".into(),
            },
        )
        .await
        .unwrap();
        insert_impl(
            &b,
            InsertPlanNodeInput {
                plan_session_id: "ps1".into(),
                plan_version: 1,
                node_id: "n2".into(),
                description: "new".into(),
                depends_on: vec!["n1".into()],
            },
        )
        .await
        .unwrap();
        assert_eq!(approve_impl(&b, "ps1".into()).await.unwrap(), 2);
        let calls = b.calls();
        assert_eq!(calls[0].1["mode"], "update");
        assert_eq!(calls[0].1["update"]["description"], "edited");
        assert_eq!(calls[1].1["mode"], "insert");
        assert_eq!(calls[1].1["insert"]["depends_on"], json!(["n1"]));
        assert_eq!(calls[2].1, json!({ "plan_session_id": "ps1" }));
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("plan_panel.rs");
        assert!(!src.contains(concat!("vox", "_db")));
        assert!(!src.contains(concat!("Gui", "DbPool")));
    }
}
