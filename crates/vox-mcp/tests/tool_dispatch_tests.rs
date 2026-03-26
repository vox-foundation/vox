use serde_json::json;
use vox_db::{DbConfig, PublicationManifestParams, VoxDb};
use vox_mcp::{ServerState, tools};
use vox_orchestrator::OrchestratorConfig;

#[tokio::test]
async fn test_mcp_tool_dispatch_list_queues() {
    let state = ServerState::new_test().await;

    // Test a basic orchestrator tool
    let result = tools::handle_tool_call(&state, "vox_orchestrator_status", json!({})).await;

    assert!(
        result.is_ok(),
        "Tool call should succeed, got: {:?}",
        result.err()
    );
    let json_str = result.unwrap();
    let val: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON");

    assert_eq!(val["success"], true);
}

#[tokio::test]
async fn test_mcp_tool_dispatch_invalid_tool() {
    let state = ServerState::new_test().await;

    let result = tools::handle_tool_call(&state, "non_existent_tool", json!({})).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_mcp_tool_dispatch_skill_list() {
    let state = ServerState::new_test().await;
    let result = tools::handle_tool_call(&state, "vox_skill_list", json!({})).await;

    assert!(
        result.is_ok(),
        "Skill list should succeed, got: {:?}",
        result.err()
    );
    let json_str = result.unwrap();
    let val: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON");

    assert_eq!(val["success"], true);
    assert!(val["data"].is_array());
}

#[tokio::test]
async fn oratio_status_includes_runtime_diagnostic_object() {
    let state = ServerState::new_test().await;
    let result = tools::handle_tool_call(&state, "vox_oratio_status", json!({}))
        .await
        .expect("oratio status");
    let val: serde_json::Value = serde_json::from_str(&result).expect("Valid JSON");
    assert!(
        val.get("runtime").is_some(),
        "status should embed runtime config snapshot"
    );
    assert!(val.get("candle").is_some());
}

#[tokio::test]
async fn test_news_gate_simulation_returns_structured_reason_codes() {
    let state = ServerState::new_test().await;
    let result = tools::handle_tool_call(
        &state,
        "vox_news_simulate_publish_gate",
        json!({
            "news_id": "example-news",
            "content": "not-frontmatter"
        }),
    )
    .await
    .expect("tool call succeeds");

    let val: serde_json::Value = serde_json::from_str(&result).expect("Valid JSON");
    assert_eq!(val["success"], true);
    let reasons = val["data"]["blocking_reasons"]
        .as_array()
        .expect("blocking_reasons array");
    assert!(
        reasons
            .iter()
            .any(|r| r["code"].as_str() == Some("parse_error")),
        "expected parse_error reason code"
    );
}

#[tokio::test]
async fn test_scientia_route_simulate_tool_is_registered_and_returns_json() {
    let state = ServerState::new_test().await;
    let result = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_route_simulate",
        json!({ "publication_id": "missing-id" }),
    )
    .await
    .expect("tool call should return structured json");
    let val: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert!(val.get("success").is_some());
}

#[tokio::test]
async fn test_scientia_publish_and_retry_tools_are_registered() {
    let state = ServerState::new_test().await;
    let publish = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_publish",
        json!({ "publication_id": "missing-id", "dry_run": true }),
    )
    .await
    .expect("publish tool json");
    let retry = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_retry_failed",
        json!({ "publication_id": "missing-id", "dry_run": true }),
    )
    .await
    .expect("retry tool json");
    let p: serde_json::Value = serde_json::from_str(&publish).expect("valid json");
    let r: serde_json::Value = serde_json::from_str(&retry).expect("valid json");
    assert!(p.get("success").is_some());
    assert!(r.get("success").is_some());
}

#[tokio::test]
async fn test_scientia_publish_compact_json_is_single_line() {
    let state = ServerState::new_test().await;
    let publish = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_publish",
        json!({ "publication_id": "missing-id", "dry_run": true, "json": true }),
    )
    .await
    .expect("publish tool json");
    assert!(
        !publish.contains('\n'),
        "compact tool envelope should be one line, got: {publish:?}"
    );
}

#[tokio::test]
async fn test_scientia_retry_failed_uses_current_manifest_digest() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id: "retry-digest-case",
        content_type: "scientia",
        source_ref: None,
        title: "Retry digest test",
        author: "Vox",
        abstract_text: None,
        body_markdown: "Body",
        citations_json: None,
        metadata_json: Some("{}"),
        content_sha3_256: "digest-current",
        state: "draft",
    })
    .await
    .expect("upsert");
    let stale_outcome = serde_json::json!({
        "rss": {"status":"failed","code":"x","message":"x","retryable":true},
        "twitter": {"status":"disabled"},
        "github": {"status":"disabled"},
        "open_collective": {"status":"disabled"},
        "reddit": {"status":"disabled"},
        "hacker_news": {"status":"disabled"},
        "youtube": {"status":"disabled"},
        "crates_io": {"status":"disabled"},
        "decision_reasons": {}
    });
    db.record_publication_attempt(
        "retry-digest-case",
        "digest-old",
        "manual_test",
        &serde_json::to_string(&stale_outcome).expect("json"),
    )
    .await
    .expect("record attempt");

    let state = ServerState::new_test().await.with_db(db);
    let retry = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_retry_failed",
        json!({ "publication_id": "retry-digest-case", "dry_run": true }),
    )
    .await
    .expect("retry tool json");
    let val: serde_json::Value = serde_json::from_str(&retry).expect("valid json");
    assert_eq!(val["success"], false);
    let err = val["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("current manifest digest"),
        "expected digest-scoped retry error, got: {err}"
    );
}

#[tokio::test]
async fn test_scientia_live_publish_honors_digest_gate() {
    let mut orch = OrchestratorConfig::for_testing();
    orch.news.dry_run = false;
    let state = ServerState::new(orch);
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id: "gate-live-mcp",
        content_type: "scientia",
        source_ref: None,
        title: "Title",
        author: "Author",
        abstract_text: None,
        body_markdown: "Body",
        citations_json: None,
        metadata_json: Some(r#"{"syndication":{"dry_run":false,"rss":false}}"#),
        content_sha3_256: "digest-gate-mcp",
        state: "approved",
    })
    .await
    .expect("upsert");
    let state = state.with_db(db);
    let publish = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_publish",
        json!({ "publication_id": "gate-live-mcp", "dry_run": false }),
    )
    .await
    .expect("publish tool");
    let val: serde_json::Value = serde_json::from_str(&publish).expect("valid json");
    assert_eq!(val["success"], false);
    let err = val["error"].as_str().expect("error string");
    assert!(
        err.contains("live publish blocked"),
        "expected gate failure, got: {err}"
    );
}

#[tokio::test]
async fn test_scientia_live_publish_honors_worthiness_floor_when_gate_passes() {
    let mut orch = OrchestratorConfig::for_testing();
    orch.news.dry_run = false;
    orch.news.publish_armed = true;
    orch.news.worthiness_enforce = true;
    orch.news.worthiness_score_min = Some(0.99);
    let state = ServerState::new(orch);
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let publication_id = "worthiness-floor-mcp";
    let digest = "digest-worthiness-mcp";
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id,
        content_type: "scientia",
        source_ref: None,
        title: "Title",
        author: "Author",
        abstract_text: None,
        body_markdown: "Body",
        citations_json: None,
        metadata_json: Some(r#"{"syndication":{"dry_run":false,"rss":false}}"#),
        content_sha3_256: digest,
        state: "approved",
    })
    .await
    .expect("upsert");
    db.record_publication_approval_for_digest(publication_id, digest, "alice")
        .await
        .expect("approve alice");
    db.record_publication_approval_for_digest(publication_id, digest, "bob")
        .await
        .expect("approve bob");
    let state = state.with_db(db);
    let publish = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_publish",
        json!({ "publication_id": publication_id, "dry_run": false }),
    )
    .await
    .expect("publish tool");
    let val: serde_json::Value = serde_json::from_str(&publish).expect("valid json");
    assert_eq!(val["success"], false);
    let err = val["error"].as_str().expect("error string");
    assert!(
        err.contains("worthiness"),
        "expected worthiness floor failure, got: {err}"
    );
}
