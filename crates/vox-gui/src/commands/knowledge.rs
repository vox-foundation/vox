//! Knowledge base management commands for the Vox GUI, served by the daemon
//! (`GET /api/v2/knowledge/*` and the `vox_kb_*` ops) so they work against a local or a
//! remote Vox server: browsing, node deletion, health metrics, manual text ingestion,
//! URL ingestion (server-side scrape) and persisting research session findings.

use std::sync::Arc;

use serde_json::json;
use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::knowledge::{KbHealthDto, KnowledgeNodeRow};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query, tool_data};

const DELETE_TOOL: &str = "vox_kb_node_delete";
const INGEST_TEXT_TOOL: &str = "vox_kb_ingest_text";
const INGEST_URL_TOOL: &str = "vox_kb_ingest_url";
const IMPORT_SESSION_TOOL: &str = "vox_kb_import_research_session";

pub(crate) async fn list_nodes_impl<B: BackendCalls>(
    b: &B,
    node_type: Option<String>,
    page: Option<usize>,
    limit: Option<usize>,
) -> Result<Vec<KnowledgeNodeRow>, String> {
    let path = path_with_query(
        "/knowledge/nodes",
        &[
            ("node_type", node_type),
            ("page", page.map(|p| p.to_string())),
            ("limit", limit.map(|l| l.to_string())),
        ],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn health_impl<B: BackendCalls>(b: &B) -> Result<KbHealthDto, String> {
    api_data(b.api_v2_get("/knowledge/health").await?)
}

pub(crate) async fn delete_node_impl<B: BackendCalls>(b: &B, id: String) -> Result<(), String> {
    let _: String = tool_data(
        DELETE_TOOL,
        b.call_tool(DELETE_TOOL, json!({ "id": id })).await?,
    )?;
    Ok(())
}

pub(crate) async fn ingest_text_impl<B: BackendCalls>(
    b: &B,
    title: String,
    content: String,
    source_url: Option<String>,
) -> Result<String, String> {
    let args = json!({ "title": title, "content": content, "source_url": source_url });
    tool_data(INGEST_TEXT_TOOL, b.call_tool(INGEST_TEXT_TOOL, args).await?)
}

pub(crate) async fn ingest_url_impl<B: BackendCalls>(b: &B, url: String) -> Result<String, String> {
    tool_data(
        INGEST_URL_TOOL,
        b.call_tool(INGEST_URL_TOOL, json!({ "url": url })).await?,
    )
}

pub(crate) async fn import_session_impl<B: BackendCalls>(
    b: &B,
    session_id: i64,
) -> Result<usize, String> {
    tool_data(
        IMPORT_SESSION_TOOL,
        b.call_tool(IMPORT_SESSION_TOOL, json!({ "session_id": session_id }))
            .await?,
    )
}

#[tauri::command]
pub async fn list_knowledge_nodes(
    backend: State<'_, Arc<BackendState>>,
    node_type: Option<String>,
    page: Option<usize>,
    limit: Option<usize>,
) -> Result<Vec<KnowledgeNodeRow>, String> {
    list_nodes_impl(&backend.current().await, node_type, page, limit).await
}

#[tauri::command]
pub async fn delete_knowledge_node(
    backend: State<'_, Arc<BackendState>>,
    id: String,
) -> Result<(), String> {
    delete_node_impl(&backend.current().await, id).await
}

#[tauri::command]
pub async fn get_kb_health(backend: State<'_, Arc<BackendState>>) -> Result<KbHealthDto, String> {
    health_impl(&backend.current().await).await
}

#[tauri::command]
pub async fn ingest_text(
    backend: State<'_, Arc<BackendState>>,
    title: String,
    content: String,
    source_url: Option<String>,
) -> Result<String, String> {
    ingest_text_impl(&backend.current().await, title, content, source_url).await
}

#[tauri::command]
pub async fn ingest_url(
    backend: State<'_, Arc<BackendState>>,
    url: String,
) -> Result<String, String> {
    ingest_url_impl(&backend.current().await, url).await
}

#[tauri::command]
pub async fn save_research_session_to_kb(
    backend: State<'_, Arc<BackendState>>,
    session_id: i64,
) -> Result<usize, String> {
    import_session_impl(&backend.current().await, session_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    fn ok_tool(data: serde_json::Value) -> Result<serde_json::Value, String> {
        Ok(json!({ "success": true, "data": data }))
    }

    #[tokio::test]
    async fn reads_go_to_the_knowledge_routes() {
        let b = FakeBackend::new()
            .on_get(
                "/knowledge/nodes?node_type=document&limit=10",
                Ok(json!({ "v": 1, "data": [{
                    "id": "doc:1", "label": "Remote note", "snippet": "s",
                    "node_type": "document", "created_at": "now"
                }]})),
            )
            .on_get(
                "/knowledge/health",
                Ok(json!({ "v": 1, "data": {
                    "node_count": 1, "edge_count": 0, "fts_available": true, "corpus_counts": {}
                }})),
            );
        let rows = list_nodes_impl(&b, Some("document".into()), None, Some(10))
            .await
            .unwrap();
        assert_eq!(rows[0].label, "Remote note");
        assert_eq!(health_impl(&b).await.unwrap().node_count, 1);
    }

    #[tokio::test]
    async fn mutations_go_through_the_kb_ops_with_unchanged_arguments() {
        let b = FakeBackend::new()
            .on_tool(DELETE_TOOL, ok_tool(json!("doc:1")))
            .on_tool(INGEST_TEXT_TOOL, ok_tool(json!("doc:abc")))
            .on_tool(INGEST_URL_TOOL, ok_tool(json!("web:xyz")))
            .on_tool(IMPORT_SESSION_TOOL, ok_tool(json!(3)));
        delete_node_impl(&b, "doc:1".into()).await.unwrap();
        let id = ingest_text_impl(&b, "T".into(), "body".into(), None)
            .await
            .unwrap();
        assert_eq!(id, "doc:abc");
        assert_eq!(
            ingest_url_impl(&b, "https://e.x".into()).await.unwrap(),
            "web:xyz"
        );
        assert_eq!(import_session_impl(&b, 42).await.unwrap(), 3);
        let calls = b.calls();
        let keys: Vec<&str> = calls.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "tool:vox_kb_node_delete",
                "tool:vox_kb_ingest_text",
                "tool:vox_kb_ingest_url",
                "tool:vox_kb_import_research_session"
            ]
        );
        assert_eq!(calls[1].1["content"], "body");
        assert_eq!(calls[3].1["session_id"], 42);
    }

    #[tokio::test]
    async fn profile_denials_surface_as_errors() {
        let b = FakeBackend::new().on_tool(
            INGEST_URL_TOOL,
            Ok(json!({ "error": { "code": "tool_not_in_profile", "message": "no" } })),
        );
        let e = ingest_url_impl(&b, "https://e.x".into()).await.unwrap_err();
        assert!(e.starts_with("tool_not_in_profile"), "{e}");
    }

    #[test]
    fn knowledge_module_no_longer_touches_the_db_directly() {
        let src = include_str!("knowledge.rs");
        assert!(!src.contains(&["vox_", "db::"].concat()));
        assert!(!src.contains(&["Gui", "DbPool"].concat()));
    }
}
