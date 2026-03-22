//! Codex **relational** helpers (manifest V16/V17) over [`ServerState::db`].
//!
//! Mirrors `vox-codex-api` POST bodies where applicable; no HTTP required from MCP.

use crate::params::ToolResult;
use crate::server::ServerState;

fn require_db(state: &ServerState) -> Result<&std::sync::Arc<vox_db::VoxDb>, String> {
    state
        .db
        .as_ref()
        .ok_or_else(|| "VoxDb is not connected (Codex tools need a Turso-backed DB).".to_string())
}

/// `vox_codex_research_session_upsert`
pub async fn codex_research_session_upsert(state: &ServerState, args: serde_json::Value) -> String {
    let db = match require_db(state) {
        Ok(d) => d,
        Err(e) => return ToolResult::<serde_json::Value>::err(e).to_json(),
    };
    let session_key = args
        .get("session_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if session_key.is_empty() {
        return ToolResult::<serde_json::Value>::err("Missing non-empty 'session_key'.").to_json();
    }
    let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let status = args
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("active");
    let mut repository_id = args
        .get("repository_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if repository_id.is_empty() {
        repository_id = state.repository.repository_id.clone();
    }
    let config_s = args
        .get("config_json")
        .and_then(|v| serde_json::to_string(v).ok());
    let summary_s = args
        .get("summary_json")
        .and_then(|v| serde_json::to_string(v).ok());
    match db
        .research_session_upsert(
            session_key,
            title,
            status,
            &repository_id,
            config_s.as_deref(),
            summary_s.as_deref(),
        )
        .await
    {
        Ok(id) => ToolResult::ok(serde_json::json!({
            "id": id,
            "session_key": session_key,
            "repository_id": repository_id,
        }))
        .to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

/// `vox_codex_conversation_version_append`
pub async fn codex_conversation_version_append(
    state: &ServerState,
    args: serde_json::Value,
) -> String {
    let db = match require_db(state) {
        Ok(d) => d,
        Err(e) => return ToolResult::<serde_json::Value>::err(e).to_json(),
    };
    let conversation_id = args.get("conversation_id").and_then(|v| v.as_i64());
    let version_index = args.get("version_index").and_then(|v| v.as_i64());
    let (Some(conversation_id), Some(version_index)) = (conversation_id, version_index) else {
        return ToolResult::<serde_json::Value>::err(
            "Require integer 'conversation_id' and 'version_index'.",
        )
        .to_json();
    };
    let label = args.get("label").and_then(|v| v.as_str()).unwrap_or("");
    let snap = args
        .get("snapshot_json")
        .and_then(|v| serde_json::to_string(v).ok());
    match db
        .conversation_version_append(conversation_id, version_index, label, snap.as_deref())
        .await
    {
        Ok(id) => ToolResult::ok(serde_json::json!({
            "id": id,
            "conversation_id": conversation_id,
            "version_index": version_index,
        }))
        .to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

/// `vox_codex_conversation_edge_insert`
pub async fn codex_conversation_edge_insert(
    state: &ServerState,
    args: serde_json::Value,
) -> String {
    let db = match require_db(state) {
        Ok(d) => d,
        Err(e) => return ToolResult::<serde_json::Value>::err(e).to_json(),
    };
    let from_id = args.get("from_conversation_id").and_then(|v| v.as_i64());
    let to_id = args.get("to_conversation_id").and_then(|v| v.as_i64());
    let (Some(from_id), Some(to_id)) = (from_id, to_id) else {
        return ToolResult::<serde_json::Value>::err(
            "Require integer 'from_conversation_id' and 'to_conversation_id'.",
        )
        .to_json();
    };
    let kind = args
        .get("edge_kind")
        .and_then(|v| v.as_str())
        .unwrap_or("related");
    let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let meta = args
        .get("metadata_json")
        .and_then(|v| serde_json::to_string(v).ok());
    match db
        .conversation_edge_insert(from_id, to_id, kind, weight, meta.as_deref())
        .await
    {
        Ok(id) => ToolResult::ok(serde_json::json!({ "id": id })).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

/// `vox_codex_topic_evolution_append`
pub async fn codex_topic_evolution_append(state: &ServerState, args: serde_json::Value) -> String {
    let db = match require_db(state) {
        Ok(d) => d,
        Err(e) => return ToolResult::<serde_json::Value>::err(e).to_json(),
    };
    let topic_id = args.get("topic_id").and_then(|v| v.as_i64());
    let event_kind = args
        .get("event_kind")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let Some(topic_id) = topic_id else {
        return ToolResult::<serde_json::Value>::err(
            "Require integer 'topic_id' and 'event_kind'.",
        )
        .to_json();
    };
    if event_kind.is_empty() {
        return ToolResult::<serde_json::Value>::err(
            "Require integer 'topic_id' and 'event_kind'.",
        )
        .to_json();
    }
    let prior = args.get("prior_label").and_then(|v| v.as_str());
    let new_l = args.get("new_label").and_then(|v| v.as_str());
    let detail = args
        .get("detail_json")
        .and_then(|v| serde_json::to_string(v).ok());
    match db
        .topic_evolution_event_append(topic_id, event_kind, prior, new_l, detail.as_deref())
        .await
    {
        Ok(id) => ToolResult::ok(serde_json::json!({
            "id": id,
            "topic_id": topic_id,
        }))
        .to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

/// `vox_codex_research_metric_linked` — ensures `research_sessions.session_key` aligns with `research_metrics.session_id`.
pub async fn codex_research_metric_linked(state: &ServerState, args: serde_json::Value) -> String {
    let db = match require_db(state) {
        Ok(d) => d,
        Err(e) => return ToolResult::<serde_json::Value>::err(e).to_json(),
    };
    let session_key = args
        .get("session_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let metric_type = args
        .get("metric_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if session_key.is_empty() || metric_type.is_empty() {
        return ToolResult::<serde_json::Value>::err(
            "Require non-empty 'session_key' and 'metric_type'.",
        )
        .to_json();
    }
    let metric_value = args.get("metric_value").and_then(|v| v.as_f64());
    let metadata_json: Option<String> = match args.get("metadata_json") {
        None => None,
        Some(v) if v.is_null() => None,
        Some(v) => v
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| serde_json::to_string(v).ok()),
    };
    let mut repository_id = args
        .get("repository_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if repository_id.is_empty() {
        repository_id = state.repository.repository_id.clone();
    }
    match db
        .research_metric_append_linked(
            session_key,
            metric_type,
            metric_value,
            metadata_json.as_deref(),
            &repository_id,
        )
        .await
    {
        Ok(j) => ToolResult::ok(j).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}
