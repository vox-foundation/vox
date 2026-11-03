//! A2: SCIENTIA research runs and research-doc publishing, served by the daemon so they work
//! against a local or a remote Vox server.
//!
//! `research.run` (see [`vox_foundation::protocol::dei_method::RESEARCH_RUN`]) creates the
//! session row, spawns the pipeline in the daemon process, and returns
//! `{session_id, task_id, status: "running"}` immediately; routing through the persistent daemon
//! keeps the long-running pipeline alive after this command returns. Doc drafts and publishing go
//! through `GET /api/v2/research/sessions/{id}/doc-draft` and `vox_research_publish_doc`, which
//! write under the **server** workspace.

use std::sync::Arc;

use serde_json::{Value, json};
use tauri::State;
use vox_foundation::protocol::dei_method;
pub use vox_orchestrator_mcp::gui_ops::dto::research::{DocDraftPreview, PublishDocResult};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, tool_data};

const FLAG_TOOL: &str = "vox_research_flag_misleading";
const PUBLISH_DOC_TOOL: &str = "vox_research_publish_doc";
const PERSIST_CLAIMS_METHOD: &str = "research.persist_claims";

fn research_run_params(
    query: String,
    scope: Option<String>,
    max_sources: Option<u32>,
    verify_claims: Option<bool>,
    waves: Option<u32>,
    domain_mode: Option<String>,
    lane: Option<String>,
    site_scope: Option<String>,
) -> Value {
    json!({
        "query": query,
        "scope": scope,
        "max_sources": max_sources,
        "verify_claims": verify_claims,
        "waves": waves,
        "domain_mode": domain_mode,
        "lane": lane,
        "site_scope": site_scope,
    })
}

pub(crate) async fn flag_misleading_impl<B: BackendCalls>(
    b: &B,
    params: Value,
) -> Result<i64, String> {
    tool_data(
        FLAG_TOOL,
        b.call_tool(FLAG_TOOL, json!({ "params": params })).await?,
    )
}

pub(crate) async fn doc_draft_impl<B: BackendCalls>(
    b: &B,
    session_id: i64,
) -> Result<DocDraftPreview, String> {
    api_data(
        b.api_v2_get(&format!("/research/sessions/{session_id}/doc-draft"))
            .await?,
    )
}

pub(crate) async fn publish_doc_impl<B: BackendCalls>(
    b: &B,
    session_id: i64,
    slug: String,
    content: String,
) -> Result<PublishDocResult, String> {
    let args = json!({ "session_id": session_id, "slug": slug, "content": content });
    tool_data(PUBLISH_DOC_TOOL, b.call_tool(PUBLISH_DOC_TOOL, args).await?)
}

/// Start a research run asynchronously and return the daemon's
/// `{session_id, task_id, status: "running"}` envelope without waiting for the
/// pipeline to finish. Status transitions are observed by the GUI via the
/// Scientia-queue watcher + session-detail polling.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_research_async(
    backend: State<'_, Arc<BackendState>>,
    query: String,
    scope: Option<String>,
    max_sources: Option<u32>,
    verify_claims: Option<bool>,
    waves: Option<u32>,
    domain_mode: Option<String>,
    lane: Option<String>,
    site_scope: Option<String>,
) -> Result<Value, String> {
    let params = research_run_params(
        query,
        scope,
        max_sources,
        verify_claims,
        waves,
        domain_mode,
        lane,
        site_scope,
    );
    backend
        .call(dei_method::RESEARCH_RUN, params)
        .await
        .map_err(String::from)
}

/// Flag a citation or claim as misleading research (updates the domain reputation penalty).
/// `params` is a `RecordMisguidanceParams` object, validated server-side.
#[tauri::command]
pub async fn flag_research_misleading(
    backend: State<'_, Arc<BackendState>>,
    params: Value,
) -> Result<i64, String> {
    flag_misleading_impl(&backend.current().await, params).await
}

/// Persists verified claims from a research session into the server DB.
#[tauri::command]
pub async fn persist_research_claims(
    backend: State<'_, Arc<BackendState>>,
    session_id: i64,
) -> Result<Value, String> {
    backend
        .call(PERSIST_CLAIMS_METHOD, json!({ "session_id": session_id }))
        .await
        .map_err(String::from)
}

/// Generates a preview draft for an Architecture SSOT document using the `vox-scientia` scaffold.
#[tauri::command]
pub async fn generate_research_doc_draft(
    backend: State<'_, Arc<BackendState>>,
    session_id: i64,
) -> Result<DocDraftPreview, String> {
    doc_draft_impl(&backend.current().await, session_id).await
}

/// Atomically publishes a research document to the server's `docs/src/architecture/` and
/// stores it as a research artifact.
#[tauri::command]
pub async fn publish_research_doc(
    backend: State<'_, Arc<BackendState>>,
    session_id: i64,
    slug: String,
    content: String,
) -> Result<PublishDocResult, String> {
    publish_doc_impl(&backend.current().await, session_id, slug, content).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    #[test]
    fn research_run_params_carry_every_field() {
        let p = research_run_params(
            "q".into(),
            Some("web".into()),
            Some(5),
            Some(true),
            Some(2),
            None,
            Some("deep".into()),
            Some("docs.rs".into()),
        );
        assert_eq!(p["query"], "q");
        assert_eq!(p["max_sources"], 5);
        assert_eq!(p["lane"], "deep");
        assert_eq!(p["site_scope"], "docs.rs");
        assert!(p["domain_mode"].is_null());
    }

    #[tokio::test]
    async fn doc_commands_go_to_the_research_route_and_ops() {
        let b = FakeBackend::new()
            .on_get(
                "/research/sessions/7/doc-draft",
                Ok(json!({ "v": 1, "data": {
                    "slug": "session-7-research", "title": "t", "filename": "session-7-research-2026.md",
                    "markdown_content": "---", "is_valid": true, "validation_errors": []
                }})),
            )
            .on_tool(
                PUBLISH_DOC_TOOL,
                Ok(json!({ "success": true, "data": {
                    "file_path": "/srv/docs/src/architecture/x-2026.md",
                    "relative_path": "docs/src/architecture/x-2026.md", "indexed": true
                }})),
            )
            .on_tool(FLAG_TOOL, Ok(json!({ "success": true, "data": 11 })));
        let draft = doc_draft_impl(&b, 7).await.unwrap();
        assert_eq!(draft.slug, "session-7-research");
        let published = publish_doc_impl(&b, 7, "x".into(), "body".into())
            .await
            .unwrap();
        assert!(published.indexed);
        let flagged = flag_misleading_impl(&b, json!({ "session_id": 7 }))
            .await
            .unwrap();
        assert_eq!(flagged, 11);
        let calls = b.calls();
        assert_eq!(calls[0].0, "get:/research/sessions/7/doc-draft");
        assert_eq!(calls[1].0, format!("tool:{PUBLISH_DOC_TOOL}"));
        assert_eq!(calls[1].1["slug"], "x");
        assert_eq!(calls[2].1["params"]["session_id"], 7);
    }
}
