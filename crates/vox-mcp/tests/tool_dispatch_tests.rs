use serde_json::json;
use vox_mcp::{ServerState, tools};

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
