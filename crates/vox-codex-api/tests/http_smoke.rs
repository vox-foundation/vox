//! Integration tests for `codex_router` (Tower `ServiceExt` + in-memory Codex).

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::util::ServiceExt;
use vox_codex_api::{CodexApiState, codex_router};
use vox_db::{Codex, DbConfig};

async fn test_app() -> axum::Router {
    let codex = Arc::new(
        Codex::connect(DbConfig::Memory)
            .await
            .expect("memory codex"),
    );
    let state = CodexApiState {
        codex,
        audio_workspace_root: PathBuf::from("."),
        default_repository_id: "http-smoke-default-repo".to_string(),
    };
    codex_router(state)
}

async fn body_json(res: axum::response::Response) -> serde_json::Value {
    let bytes = res
        .into_body()
        .collect()
        .await
        .expect("body collect")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("json body")
}

#[tokio::test]
async fn get_health_returns_ok() {
    let app = test_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["status"], "ok");
}

#[tokio::test]
async fn get_ready_succeeds_on_baseline_memory_db() {
    let app = test_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["status"], "ready");
    assert_eq!(v["schema_version"], 1);
    assert!(
        v["baseline_digest"]
            .as_str()
            .unwrap_or("")
            .starts_with("0x")
    );
}

#[tokio::test]
async fn post_research_session_upsert_returns_ids() {
    let app = test_app().await;
    let body = serde_json::json!({
        "session_key": "http-smoke-rs",
        "title": "smoke",
        "status": "active",
        "repository_id": "repo-smoke",
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/codex/research-session")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("req"),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["session_key"], "http-smoke-rs");
    assert_eq!(v["repository_id"], "repo-smoke");
    assert!(v["id"].as_i64().unwrap_or(0) > 0);
}

#[tokio::test]
async fn post_research_session_defaults_repository_id_when_omitted() {
    let codex = Arc::new(Codex::connect(DbConfig::Memory).await.expect("db"));
    let default_id = "canonical-default-repo-id";
    let app = codex_router(CodexApiState {
        codex: codex.clone(),
        audio_workspace_root: PathBuf::from("."),
        default_repository_id: default_id.to_string(),
    });
    let body = serde_json::json!({
        "session_key": "session-without-repo-body",
        "title": "t",
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/codex/research-session")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("req"),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["repository_id"], default_id);

    let mut rows = codex
        .store()
        .connection()
        .query(
            "SELECT repository_id FROM research_sessions WHERE session_key = 'session-without-repo-body'",
            (),
        )
        .await
        .expect("q");
    let rid: String = rows
        .next()
        .await
        .expect("n")
        .expect("row")
        .get(0)
        .expect("col");
    assert_eq!(rid, default_id);
}

#[tokio::test]
async fn post_conversation_version_and_edge_and_topic_evolution() {
    let codex = Arc::new(Codex::connect(DbConfig::Memory).await.expect("db"));
    codex
        .store()
        .connection()
        .execute(
            "INSERT OR IGNORE INTO users (id, display_name, role) VALUES ('u1', 'u1', 'user')",
            (),
        )
        .await
        .expect("user");
    let c1 = codex
        .chat_create_conversation(Some("u1"), "a")
        .await
        .expect("c1");
    let c2 = codex
        .chat_create_conversation(Some("u1"), "b")
        .await
        .expect("c2");
    codex
        .store()
        .connection()
        .execute(
            "INSERT OR IGNORE INTO topics (slug, label) VALUES ('http-smoke-topic', 'L')",
            (),
        )
        .await
        .expect("topic");
    let mut rows = codex
        .store()
        .connection()
        .query("SELECT id FROM topics WHERE slug = 'http-smoke-topic'", ())
        .await
        .expect("q");
    let topic_id: i64 = rows
        .next()
        .await
        .expect("n")
        .expect("row")
        .get(0)
        .expect("id");

    let state = CodexApiState {
        codex: codex.clone(),
        audio_workspace_root: PathBuf::from("."),
        default_repository_id: "http-smoke-default-repo".to_string(),
    };

    let res_v = codex_router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/codex/conversations/{c1}/versions"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"version_index": 1, "label": "v1"}).to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("oneshot");
    assert_eq!(
        res_v.status(),
        StatusCode::OK,
        "{:?}",
        body_json(res_v).await
    );

    let res_e = codex_router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/codex/conversation-edges")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "from_conversation_id": c1,
                        "to_conversation_id": c2,
                        "edge_kind": "related",
                        "weight": 1.0
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("oneshot");
    assert_eq!(res_e.status(), StatusCode::OK);

    let res_t = codex_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/codex/topics/{topic_id}/evolution-events"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"event_kind": "rename", "prior_label": "L", "new_label": "L2"})
                        .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("oneshot");
    assert_eq!(res_t.status(), StatusCode::OK);
    let tv = body_json(res_t).await;
    assert_eq!(tv["topic_id"], topic_id);
}

#[tokio::test]
async fn get_search_status_returns_counts() {
    let app = test_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/search/status")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert!(v["search_documents"].is_number());
    assert!(v["search_indexing_jobs"].is_number());
}
