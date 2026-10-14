//! Tauri commands for the unified task hopper. Served by the daemon (`GET /api/v2/hopper/list`
//! and the `vox_hopper_*` ops), so Axis and the dashboard read and write one persistent queue.

use std::sync::Arc;

use serde_json::json;
use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::orchestrator::HopperTaskDto;

use crate::commands::orchestrator::emit_tasks_changed;
use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, tool_data};

const SUBMIT_TOOL: &str = "vox_hopper_submit";
const REPRIORITIZE_TOOL: &str = "vox_hopper_reprioritize";
const CANCEL_TOOL: &str = "vox_hopper_cancel";
const MARK_DONE_TOOL: &str = "vox_hopper_mark_done";

pub(crate) async fn list_impl<B: BackendCalls>(b: &B) -> Result<Vec<HopperTaskDto>, String> {
    api_data(b.api_v2_get("/hopper/list").await?)
}

pub(crate) async fn submit_impl<B: BackendCalls>(
    b: &B,
    intent: String,
    affinity: Vec<String>,
) -> Result<HopperTaskDto, String> {
    let args = json!({ "intent": intent, "affinity": affinity });
    tool_data(SUBMIT_TOOL, b.call_tool(SUBMIT_TOOL, args).await?)
}

pub(crate) async fn reprioritize_impl<B: BackendCalls>(
    b: &B,
    item_id: String,
    priority: u8,
) -> Result<HopperTaskDto, String> {
    let args = json!({ "item_id": item_id, "priority": priority });
    tool_data(
        REPRIORITIZE_TOOL,
        b.call_tool(REPRIORITIZE_TOOL, args).await?,
    )
}

pub(crate) async fn item_op_impl<B: BackendCalls>(
    b: &B,
    tool: &str,
    item_id: String,
) -> Result<HopperTaskDto, String> {
    tool_data(
        tool,
        b.call_tool(tool, json!({ "item_id": item_id })).await?,
    )
}

/// Open (inbox + assigned) hopper items plus the most recent completed ones.
#[tauri::command]
pub async fn hopper_list(
    backend: State<'_, Arc<BackendState>>,
) -> Result<Vec<HopperTaskDto>, String> {
    list_impl(&backend.current().await).await
}

#[tauri::command]
pub async fn hopper_submit(
    app_handle: tauri::AppHandle,
    backend: State<'_, Arc<BackendState>>,
    intent: String,
    affinity: Vec<String>,
) -> Result<HopperTaskDto, String> {
    let item = submit_impl(&backend.current().await, intent, affinity).await?;
    emit_tasks_changed(&app_handle);
    Ok(item)
}

#[tauri::command]
pub async fn hopper_reprioritize(
    app_handle: tauri::AppHandle,
    backend: State<'_, Arc<BackendState>>,
    item_id: String,
    priority: u8,
) -> Result<HopperTaskDto, String> {
    let item = reprioritize_impl(&backend.current().await, item_id, priority).await?;
    emit_tasks_changed(&app_handle);
    Ok(item)
}

#[tauri::command]
pub async fn hopper_cancel(
    app_handle: tauri::AppHandle,
    backend: State<'_, Arc<BackendState>>,
    item_id: String,
) -> Result<HopperTaskDto, String> {
    let item = item_op_impl(&backend.current().await, CANCEL_TOOL, item_id).await?;
    emit_tasks_changed(&app_handle);
    Ok(item)
}

#[tauri::command]
pub async fn hopper_mark_done(
    app_handle: tauri::AppHandle,
    backend: State<'_, Arc<BackendState>>,
    item_id: String,
) -> Result<HopperTaskDto, String> {
    let item = item_op_impl(&backend.current().await, MARK_DONE_TOOL, item_id).await?;
    emit_tasks_changed(&app_handle);
    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    fn item(state: &str) -> serde_json::Value {
        json!({
            "item_id": "h1", "intent": "remote intent", "priority": 2, "state": state,
            "task_id": 1, "session_id": null, "agent_id": null, "remote_node": null
        })
    }

    #[tokio::test]
    async fn list_reads_the_hopper_route() {
        let b = FakeBackend::new().on_get(
            "/hopper/list",
            Ok(json!({ "v": 1, "data": [item("inbox")] })),
        );
        let rows = list_impl(&b).await.unwrap();
        assert_eq!(rows[0].intent, "remote intent");
        assert_eq!(b.calls()[0].0, "get:/hopper/list");
    }

    #[tokio::test]
    async fn mutations_go_to_the_hopper_ops_with_the_tauri_arguments() {
        let ok = |state: &str| Ok(json!({ "success": true, "data": item(state) }));
        let b = FakeBackend::new()
            .on_tool(SUBMIT_TOOL, ok("inbox"))
            .on_tool(REPRIORITIZE_TOOL, ok("inbox"))
            .on_tool(CANCEL_TOOL, ok("cancelled"))
            .on_tool(MARK_DONE_TOOL, ok("done"));
        submit_impl(&b, "remote intent".into(), vec!["rust".into()])
            .await
            .unwrap();
        reprioritize_impl(&b, "h1".into(), 2).await.unwrap();
        item_op_impl(&b, CANCEL_TOOL, "h1".into()).await.unwrap();
        let done = item_op_impl(&b, MARK_DONE_TOOL, "h1".into()).await.unwrap();
        assert_eq!(done.state, "done");
        let calls = b.calls();
        assert_eq!(
            calls[0],
            (
                format!("tool:{SUBMIT_TOOL}"),
                json!({ "intent": "remote intent", "affinity": ["rust"] })
            )
        );
        assert_eq!(calls[1].1, json!({ "item_id": "h1", "priority": 2 }));
        assert_eq!(calls[2].0, format!("tool:{CANCEL_TOOL}"));
        assert_eq!(calls[3].1, json!({ "item_id": "h1" }));
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("hopper.rs");
        assert!(!src.contains(concat!("vox", "_db")));
        assert!(!src.contains(concat!("Sqlite", "Hopper")));
    }
}
