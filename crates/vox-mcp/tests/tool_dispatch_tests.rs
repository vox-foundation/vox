use vox_mcp::{ServerState, tools};
use serde_json::json;

#[tokio::test]
async fn test_mcp_tool_dispatch_list_queues() {
    let state = ServerState::new_test().await;
    
    // Test a basic orchestrator tool
    let result = tools::handle_tool_call(
        &state,
        "vox_orchestrator_status",
        json!({})
    ).await;
    
    assert!(result.is_ok(), "Tool call should succeed, got: {:?}", result.err());
    let json_str = result.unwrap();
    let val: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON");
    
    assert_eq!(val["success"], true);
}

#[tokio::test]
async fn test_mcp_tool_dispatch_invalid_tool() {
    let state = ServerState::new_test().await;
    
    let result = tools::handle_tool_call(
        &state,
        "non_existent_tool",
        json!({})
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mcp_tool_dispatch_skill_list() {
    let state = ServerState::new_test().await;
    let result = tools::handle_tool_call(&state, "vox_skill_list", json!({})).await;
    
    assert!(result.is_ok(), "Skill list should succeed, got: {:?}", result.err());
    let json_str = result.unwrap();
    let val: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON");
    
    assert_eq!(val["success"], true);
    assert!(val["data"].is_array());
}
