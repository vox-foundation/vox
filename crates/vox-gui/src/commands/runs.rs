//! Tauri commands for the GUI-run journal (`agent_runs`). Served by the daemon
//! (`GET /api/v2/runs/gui*` and the `vox_gui_run_start` / `vox_gui_run_finish` ops).

use std::sync::Arc;

use serde_json::json;
use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::orchestrator::{
    FinishGuiRunInput, GuiRunRecord, StartGuiRunInput,
};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, encode_component, path_with_query, tool_data};

const START_TOOL: &str = "vox_gui_run_start";
const FINISH_TOOL: &str = "vox_gui_run_finish";

pub(crate) async fn start_impl<B: BackendCalls>(
    b: &B,
    input: StartGuiRunInput,
) -> Result<(), String> {
    tool_data(START_TOOL, b.call_tool(START_TOOL, json!(input)).await?)
}

pub(crate) async fn finish_impl<B: BackendCalls>(
    b: &B,
    input: FinishGuiRunInput,
) -> Result<(), String> {
    tool_data(FINISH_TOOL, b.call_tool(FINISH_TOOL, json!(input)).await?)
}

pub(crate) async fn list_impl<B: BackendCalls>(
    b: &B,
    limit: Option<u32>,
) -> Result<Vec<GuiRunRecord>, String> {
    let path = path_with_query("/runs/gui", &[("limit", limit.map(|l| l.to_string()))]);
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn get_impl<B: BackendCalls>(
    b: &B,
    run_id: &str,
) -> Result<Option<GuiRunRecord>, String> {
    api_data(
        b.api_v2_get(&format!("/runs/gui/{}", encode_component(run_id)))
            .await?,
    )
}

#[tauri::command]
pub async fn start_gui_run(
    backend: State<'_, Arc<BackendState>>,
    input: StartGuiRunInput,
) -> Result<(), String> {
    start_impl(&backend.current().await, input).await
}

/// `task_id` is the orchestrator's numeric task id (stringified) when this GUI run wraps a
/// `submit_orchestrator_task` call — the join key for its approval and telemetry totals; `None`
/// for runs with no backing task, which keep whatever explicit cost/token args were passed.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn finish_gui_run(
    backend: State<'_, Arc<BackendState>>,
    run_id: String,
    success: bool,
    completed_steps: Option<i64>,
    error: Option<String>,
    cost_usd: Option<f64>,
    tokens_in: Option<i64>,
    tokens_out: Option<i64>,
    task_id: Option<String>,
) -> Result<(), String> {
    let input = FinishGuiRunInput {
        run_id,
        success,
        completed_steps,
        error,
        cost_usd,
        tokens_in,
        tokens_out,
        task_id,
    };
    finish_impl(&backend.current().await, input).await
}

#[tauri::command]
pub async fn list_gui_runs(
    backend: State<'_, Arc<BackendState>>,
    limit: Option<u32>,
) -> Result<Vec<GuiRunRecord>, String> {
    list_impl(&backend.current().await, limit).await
}

#[tauri::command]
pub async fn get_gui_run(
    backend: State<'_, Arc<BackendState>>,
    run_id: String,
) -> Result<Option<GuiRunRecord>, String> {
    get_impl(&backend.current().await, &run_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    #[tokio::test]
    async fn reads_go_to_the_gui_run_routes() {
        let b = FakeBackend::new()
            .on_get("/runs/gui?limit=5", Ok(json!({ "v": 1, "data": [] })))
            .on_get("/runs/gui", Ok(json!({ "v": 1, "data": [] })))
            .on_get("/runs/gui/gui-1", Ok(json!({ "v": 1, "data": null })));
        assert!(list_impl(&b, Some(5)).await.unwrap().is_empty());
        assert!(list_impl(&b, None).await.unwrap().is_empty());
        assert_eq!(get_impl(&b, "gui-1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn start_and_finish_go_to_the_ops() {
        let ok_null = || Ok(json!({ "success": true, "data": null }));
        let b = FakeBackend::new()
            .on_tool(START_TOOL, ok_null())
            .on_tool(FINISH_TOOL, ok_null());
        start_impl(
            &b,
            StartGuiRunInput {
                run_id: "gui-1".into(),
                workflow_name: "submit".into(),
                planned_steps: None,
                command: None,
                repo: None,
                worktree: None,
                model: Some("m".into()),
            },
        )
        .await
        .unwrap();
        finish_impl(
            &b,
            FinishGuiRunInput {
                run_id: "gui-1".into(),
                success: true,
                completed_steps: Some(1),
                error: None,
                cost_usd: None,
                tokens_in: None,
                tokens_out: None,
                task_id: Some("42".into()),
            },
        )
        .await
        .unwrap();
        let calls = b.calls();
        assert_eq!(calls[0].1["model"], "m");
        assert_eq!(calls[1].1["task_id"], "42");
        assert_eq!(calls[1].1["success"], true);
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("runs.rs");
        assert!(!src.contains(concat!("vox", "_db")));
    }
}
