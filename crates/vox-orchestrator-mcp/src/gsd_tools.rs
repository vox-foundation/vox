//! MCP tool handlers for the GSD (Get Shit Done) workflow suite (`vox_gsd_*`).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use vox_orchestrator::planning::gsd;
use vox_orchestrator_types::gsd::GsdDocumentKind;

use crate::params::ToolResult;
use crate::server_state::ServerState;

fn workspace_root(state: &ServerState) -> &Path {
    state
        .workspace_root
        .as_deref()
        .unwrap_or(state.repository.root.as_path())
}

fn now_unix_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(0)
}

async fn resolve_milestone_id(state: &ServerState, requested: Option<&str>) -> String {
    if let Some(id) = requested.filter(|s| !s.trim().is_empty()) {
        return id.trim().to_string();
    }
    let root = workspace_root(state);
    if let Ok(snap) = gsd::load_workspace_status(root, state.db.as_deref()).await {
        if let Some(ms) = snap.active_milestone {
            return ms.milestone_id;
        }
    }
    "v1.0".to_string()
}

#[derive(Debug, Default, Deserialize)]
pub struct GsdStatusParams {
    #[serde(default)]
    pub milestone_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GsdNewMilestoneParams {
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

#[derive(Debug, Deserialize)]
pub struct GsdDiscussPhaseParams {
    pub phase: u32,
    pub summary: String,
    #[serde(default)]
    pub milestone_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GsdPlanPhaseParams {
    pub phase: u32,
    #[serde(default)]
    pub mvp: bool,
    #[serde(default)]
    pub milestone_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GsdExecutePhaseParams {
    pub phase: u32,
    #[serde(default)]
    pub milestone_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GsdVerifyWorkParams {
    pub phase: u32,
    #[serde(default = "vox_config::serde_defaults::default_true")]
    pub passed: bool,
    #[serde(default)]
    pub evidence: String,
    #[serde(default)]
    pub milestone_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GsdCompleteMilestoneParams {
    pub version: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct GsdListDocumentsParams {
    #[serde(default)]
    pub milestone_id: Option<String>,
    #[serde(default)]
    pub phase: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct GsdStoreDocumentParams {
    pub rel_path: String,
    pub content: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub phase: Option<u32>,
    #[serde(default)]
    pub milestone_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GsdGetDocumentParams {
    pub doc_id_or_path: String,
}

pub async fn gsd_status(state: &ServerState, _args: serde_json::Value) -> String {
    let root = workspace_root(state);
    let health = gsd::health_check(root);
    match gsd::load_workspace_status(root, state.db.as_deref()).await {
        Ok(snapshot) => ToolResult::ok(serde_json::json!({
            "health": health,
            "snapshot": snapshot,
        }))
        .to_json_compact(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact(),
    }
}

pub async fn gsd_progress(state: &ServerState, args: serde_json::Value) -> String {
    let params: GsdStatusParams = serde_json::from_value(args).unwrap_or_default();
    let root = workspace_root(state);
    let res = if let (Some(ms), Some(db)) = (params.milestone_id.as_deref(), state.db.as_deref()) {
        db.gsd_progress_snapshot(Some(ms))
            .await
            .map_err(|e| e.to_string())
    } else {
        gsd::load_workspace_status(root, state.db.as_deref())
            .await
            .map_err(|e| e.to_string())
    };
    match res {
        Ok(snapshot) => ToolResult::ok(snapshot).to_json_compact(),
        Err(e) => ToolResult::<serde_json::Value>::err(e).to_json_compact(),
    }
}

pub async fn gsd_new_milestone(state: &ServerState, params: GsdNewMilestoneParams) -> String {
    let root = workspace_root(state);
    match gsd::init_new_milestone(
        root,
        state.db.as_deref(),
        &params.version,
        &params.title,
        params.start_phase,
        params.phases,
    )
    .await
    {
        Ok(snapshot) => ToolResult::ok(snapshot).to_json_compact(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact(),
    }
}

pub async fn gsd_discuss_phase(state: &ServerState, params: GsdDiscussPhaseParams) -> String {
    let root = workspace_root(state);
    let ms_id = resolve_milestone_id(state, params.milestone_id.as_deref()).await;
    match gsd::record_discuss_phase(
        root,
        state.db.as_deref(),
        &ms_id,
        params.phase,
        &params.summary,
    )
    .await
    {
        Ok(doc) => ToolResult::ok(doc).to_json_compact(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact(),
    }
}

pub async fn gsd_plan_phase(state: &ServerState, params: GsdPlanPhaseParams) -> String {
    let root = workspace_root(state);
    let ms_id = resolve_milestone_id(state, params.milestone_id.as_deref()).await;
    match gsd::generate_or_load_phase_plan(
        root,
        state.db.as_deref(),
        &ms_id,
        params.phase,
        params.mvp,
    )
    .await
    {
        Ok(nodes) => ToolResult::ok(nodes).to_json_compact(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact(),
    }
}

pub async fn gsd_execute_phase(state: &ServerState, params: GsdExecutePhaseParams) -> String {
    let root = workspace_root(state);
    let ms_id = resolve_milestone_id(state, params.milestone_id.as_deref()).await;
    match gsd::prepare_execute_phase(root, state.db.as_deref(), &ms_id, params.phase).await {
        Ok(nodes) => ToolResult::ok(serde_json::json!({
            "milestone_id": ms_id,
            "phase": params.phase,
            "nodes": nodes,
        }))
        .to_json_compact(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact(),
    }
}

pub async fn gsd_verify_work(state: &ServerState, params: GsdVerifyWorkParams) -> String {
    let root = workspace_root(state);
    let ms_id = resolve_milestone_id(state, params.milestone_id.as_deref()).await;
    let evidence = if params.evidence.trim().is_empty() {
        "Verified via vox_gsd_verify_work".to_string()
    } else {
        params.evidence
    };
    match gsd::record_verify_work(
        root,
        state.db.as_deref(),
        &ms_id,
        params.phase,
        params.passed,
        &evidence,
    )
    .await
    {
        Ok(doc) => ToolResult::ok(doc).to_json_compact(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact(),
    }
}

pub async fn gsd_complete_milestone(
    state: &ServerState,
    params: GsdCompleteMilestoneParams,
) -> String {
    let root = workspace_root(state);
    match gsd::complete_milestone(root, state.db.as_deref(), &params.version).await {
        Ok(snapshot) => ToolResult::ok(snapshot).to_json_compact(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact(),
    }
}

pub async fn gsd_list_documents(state: &ServerState, params: GsdListDocumentsParams) -> String {
    if let Some(db) = state.db.as_deref() {
        match db
            .list_gsd_documents(params.milestone_id.as_deref(), params.phase)
            .await
        {
            Ok(docs) => return ToolResult::ok(docs).to_json_compact(),
            Err(e) => return ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact(),
        }
    }
    match gsd::load_workspace_status(workspace_root(state), None).await {
        Ok(snap) => ToolResult::ok(snap.documents).to_json_compact(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact(),
    }
}

pub async fn gsd_store_document(state: &ServerState, params: GsdStoreDocumentParams) -> String {
    let root = workspace_root(state);
    let clean_rel = params.rel_path.trim().trim_start_matches('/').to_string();
    if clean_rel.contains("..") {
        return ToolResult::<serde_json::Value>::err("rel_path must stay inside workspace")
            .to_json_compact();
    }
    let abs = root.join(&clean_rel);
    if let Some(parent) = abs.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact();
        }
    }
    if let Err(e) = std::fs::write(&abs, &params.content) {
        return ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact();
    }
    let ms_id = resolve_milestone_id(state, params.milestone_id.as_deref()).await;
    let kind = params
        .kind
        .as_deref()
        .map(GsdDocumentKind::from_db_str)
        .unwrap_or_else(|| gsd::parser::infer_document_kind(&clean_rel));
    if let Some(db) = state.db.as_deref() {
        match db
            .store_gsd_document(
                &format!("{ms_id}:{clean_rel}"),
                &ms_id,
                params.phase,
                kind,
                &clean_rel,
                &params.content,
                now_unix_ms(),
            )
            .await
        {
            Ok(doc) => return ToolResult::ok(doc).to_json_compact(),
            Err(e) => return ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact(),
        }
    }
    ToolResult::ok(serde_json::json!({
        "milestone_id": ms_id,
        "rel_path": clean_rel,
        "byte_len": params.content.len(),
    }))
    .to_json_compact()
}

pub async fn gsd_get_document(state: &ServerState, params: GsdGetDocumentParams) -> String {
    if let Some(db) = state.db.as_deref() {
        match db.get_gsd_document_content(&params.doc_id_or_path).await {
            Ok(Some((doc, content))) => {
                return ToolResult::ok(serde_json::json!({
                    "document": doc,
                    "content": content,
                }))
                .to_json_compact();
            }
            Ok(None) => {}
            Err(e) => return ToolResult::<serde_json::Value>::err(e.to_string()).to_json_compact(),
        }
    }
    let clean_rel = params.doc_id_or_path.trim().trim_start_matches('/');
    if !clean_rel.contains("..") {
        let abs = workspace_root(state).join(clean_rel);
        if let Ok(content) = std::fs::read_to_string(&abs) {
            return ToolResult::ok(serde_json::json!({
                "document": { "rel_path": clean_rel },
                "content": content,
            }))
            .to_json_compact();
        }
    }
    ToolResult::<serde_json::Value>::err(format!(
        "GSD document '{}' not found",
        params.doc_id_or_path
    ))
    .to_json_compact()
}
