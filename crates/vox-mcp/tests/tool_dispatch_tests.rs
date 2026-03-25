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
    assert!(val.get("runtime").is_some(), "status should embed runtime config snapshot");
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
