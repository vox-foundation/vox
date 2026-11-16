//! Tauri commands for the GSD (Get Shit Done) milestone, phase, wave & CAS document suite.
//! Routed as daemon-class commands via `BackendCalls::call_tool` -> `vox_gsd_*` MCP tools.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::State;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, tool_data};

const TOOL_STATUS: &str = "vox_gsd_get_status";
const TOOL_PROGRESS: &str = "vox_gsd_get_progress";
const TOOL_NEW_MILESTONE: &str = "vox_gsd_new_milestone";
const TOOL_DISCUSS_PHASE: &str = "vox_gsd_discuss_phase";
const TOOL_PLAN_PHASE: &str = "vox_gsd_plan_phase";
const TOOL_EXECUTE_PHASE: &str = "vox_gsd_execute_phase";
const TOOL_VERIFY_WORK: &str = "vox_gsd_verify_work";
const TOOL_COMPLETE_MILESTONE: &str = "vox_gsd_complete_milestone";
const TOOL_LIST_DOCUMENTS: &str = "vox_gsd_list_documents";
const TOOL_STORE_DOCUMENT: &str = "vox_gsd_store_document";
const TOOL_GET_DOCUMENT: &str = "vox_gsd_get_document";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GsdNewMilestoneArgs {
    pub version: String,
    pub title: String,
    #[serde(default = "default_start_phase")]
    pub start_phase: u32,
    #[serde(default = "default_phase_count")]
    pub phases: u32,
}

fn default_start_phase() -> u32 {
    1
}

fn default_phase_count() -> u32 {
    3
}

pub(crate) async fn get_status_impl<B: BackendCalls>(
    b: &B,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    tool_data(
        TOOL_STATUS,
        b.call_tool(TOOL_STATUS, json!({ "milestone_id": milestone_id }))
            .await?,
    )
}

pub(crate) async fn get_progress_impl<B: BackendCalls>(
    b: &B,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    tool_data(
        TOOL_PROGRESS,
        b.call_tool(TOOL_PROGRESS, json!({ "milestone_id": milestone_id }))
            .await?,
    )
}

pub(crate) async fn new_milestone_impl<B: BackendCalls>(
    b: &B,
    input: GsdNewMilestoneArgs,
) -> Result<Value, String> {
    tool_data(
        TOOL_NEW_MILESTONE,
        b.call_tool(TOOL_NEW_MILESTONE, json!(input)).await?,
    )
}

pub(crate) async fn discuss_phase_impl<B: BackendCalls>(
    b: &B,
    phase: u32,
    summary: String,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    tool_data(
        TOOL_DISCUSS_PHASE,
        b.call_tool(
            TOOL_DISCUSS_PHASE,
            json!({ "phase": phase, "summary": summary, "milestone_id": milestone_id }),
        )
        .await?,
    )
}

pub(crate) async fn plan_phase_impl<B: BackendCalls>(
    b: &B,
    phase: u32,
    mvp: bool,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    tool_data(
        TOOL_PLAN_PHASE,
        b.call_tool(
            TOOL_PLAN_PHASE,
            json!({ "phase": phase, "mvp": mvp, "milestone_id": milestone_id }),
        )
        .await?,
    )
}

pub(crate) async fn execute_phase_impl<B: BackendCalls>(
    b: &B,
    phase: u32,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    tool_data(
        TOOL_EXECUTE_PHASE,
        b.call_tool(
            TOOL_EXECUTE_PHASE,
            json!({ "phase": phase, "milestone_id": milestone_id }),
        )
        .await?,
    )
}

pub(crate) async fn verify_work_impl<B: BackendCalls>(
    b: &B,
    phase: u32,
    passed: bool,
    evidence: String,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    tool_data(
        TOOL_VERIFY_WORK,
        b.call_tool(
            TOOL_VERIFY_WORK,
            json!({ "phase": phase, "passed": passed, "evidence": evidence, "milestone_id": milestone_id }),
        )
        .await?,
    )
}

pub(crate) async fn complete_milestone_impl<B: BackendCalls>(
    b: &B,
    version: String,
) -> Result<Value, String> {
    tool_data(
        TOOL_COMPLETE_MILESTONE,
        b.call_tool(TOOL_COMPLETE_MILESTONE, json!({ "version": version }))
            .await?,
    )
}

pub(crate) async fn list_documents_impl<B: BackendCalls>(
    b: &B,
    milestone_id: Option<String>,
    phase: Option<u32>,
) -> Result<Value, String> {
    tool_data(
        TOOL_LIST_DOCUMENTS,
        b.call_tool(
            TOOL_LIST_DOCUMENTS,
            json!({ "milestone_id": milestone_id, "phase": phase }),
        )
        .await?,
    )
}

pub(crate) async fn store_document_impl<B: BackendCalls>(
    b: &B,
    rel_path: String,
    content: String,
    kind: Option<String>,
    phase: Option<u32>,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    tool_data(
        TOOL_STORE_DOCUMENT,
        b.call_tool(
            TOOL_STORE_DOCUMENT,
            json!({
                "rel_path": rel_path,
                "content": content,
                "kind": kind,
                "phase": phase,
                "milestone_id": milestone_id,
            }),
        )
        .await?,
    )
}

pub(crate) async fn get_document_impl<B: BackendCalls>(
    b: &B,
    doc_id_or_path: String,
) -> Result<Value, String> {
    tool_data(
        TOOL_GET_DOCUMENT,
        b.call_tool(
            TOOL_GET_DOCUMENT,
            json!({ "doc_id_or_path": doc_id_or_path }),
        )
        .await?,
    )
}

#[tauri::command]
pub async fn gsd_get_status(
    state: State<'_, Arc<BackendState>>,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    get_status_impl(&state.current().await, milestone_id).await
}

#[tauri::command]
pub async fn gsd_get_progress(
    state: State<'_, Arc<BackendState>>,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    get_progress_impl(&state.current().await, milestone_id).await
}

#[tauri::command]
pub async fn gsd_new_milestone(
    state: State<'_, Arc<BackendState>>,
    input: GsdNewMilestoneArgs,
) -> Result<Value, String> {
    new_milestone_impl(&state.current().await, input).await
}

#[tauri::command]
pub async fn gsd_discuss_phase(
    state: State<'_, Arc<BackendState>>,
    phase: u32,
    summary: String,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    discuss_phase_impl(&state.current().await, phase, summary, milestone_id).await
}

#[tauri::command]
pub async fn gsd_plan_phase(
    state: State<'_, Arc<BackendState>>,
    phase: u32,
    mvp: Option<bool>,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    plan_phase_impl(
        &state.current().await,
        phase,
        mvp.unwrap_or(false),
        milestone_id,
    )
    .await
}

#[tauri::command]
pub async fn gsd_execute_phase(
    state: State<'_, Arc<BackendState>>,
    phase: u32,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    execute_phase_impl(&state.current().await, phase, milestone_id).await
}

#[tauri::command]
pub async fn gsd_verify_work(
    state: State<'_, Arc<BackendState>>,
    phase: u32,
    passed: Option<bool>,
    evidence: Option<String>,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    verify_work_impl(
        &state.current().await,
        phase,
        passed.unwrap_or(true),
        evidence.unwrap_or_default(),
        milestone_id,
    )
    .await
}

#[tauri::command]
pub async fn gsd_complete_milestone(
    state: State<'_, Arc<BackendState>>,
    version: String,
) -> Result<Value, String> {
    complete_milestone_impl(&state.current().await, version).await
}

#[tauri::command]
pub async fn gsd_list_documents(
    state: State<'_, Arc<BackendState>>,
    milestone_id: Option<String>,
    phase: Option<u32>,
) -> Result<Value, String> {
    list_documents_impl(&state.current().await, milestone_id, phase).await
}

#[tauri::command]
pub async fn gsd_store_document(
    state: State<'_, Arc<BackendState>>,
    rel_path: String,
    content: String,
    kind: Option<String>,
    phase: Option<u32>,
    milestone_id: Option<String>,
) -> Result<Value, String> {
    store_document_impl(
        &state.current().await,
        rel_path,
        content,
        kind,
        phase,
        milestone_id,
    )
    .await
}

#[tauri::command]
pub async fn gsd_get_document(
    state: State<'_, Arc<BackendState>>,
    doc_id_or_path: String,
) -> Result<Value, String> {
    get_document_impl(&state.current().await, doc_id_or_path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;
    use serde_json::json;

    #[tokio::test]
    async fn gsd_ipc_routes_to_daemon_tools() {
        let b = FakeBackend::new()
            .on_tool(
                TOOL_STATUS,
                Ok(json!({ "success": true, "data": { "health": { "healthy": true } } })),
            )
            .on_tool(
                TOOL_NEW_MILESTONE,
                Ok(json!({ "success": true, "data": { "active_milestone": { "milestone_id": "v7.0" } } })),
            )
            .on_tool(
                TOOL_STORE_DOCUMENT,
                Ok(json!({ "success": true, "data": { "cas_hash": "abc123" } })),
            );
        let status = get_status_impl(&b, None).await.unwrap();
        assert_eq!(status["health"]["healthy"], true);
        let ms = new_milestone_impl(
            &b,
            GsdNewMilestoneArgs {
                version: "v7.0".into(),
                title: "GSD Integration".into(),
                start_phase: 407,
                phases: 6,
            },
        )
        .await
        .unwrap();
        assert_eq!(ms["active_milestone"]["milestone_id"], "v7.0");
        let doc = store_document_impl(
            &b,
            ".planning/STATE.md".into(),
            "# State".into(),
            Some("state".into()),
            None,
            Some("v7.0".into()),
        )
        .await
        .unwrap();
        assert_eq!(doc["cas_hash"], "abc123");
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("gsd.rs");
        assert!(!src.contains(concat!("vox", "_db")));
        assert!(!src.contains(concat!("Gui", "DbPool")));
    }
}
