#![allow(missing_docs)]

use vox_orchestrator::OrchestratorConfig;
use vox_orchestrator_mcp::{ServerState, handle_tool_call as tools};

#[tokio::test]
async fn test_skill_install_tool_availability() {
    let config = OrchestratorConfig::default();
    let state = ServerState::new_full(config);

    // 1. Install a test skill via vox_skill_install
    let test_skill = r#"---
id = "test.macro"
name = "Test Macro"
version = "1.0.0"
author = "test"
description = "Test macro tool"
category = "custom"
tools = ["vox_test_macro_tool"]
tags = []
permissions = []
---
# Test Macro
Instructions inside.
"#;

    let bundle = vox_skills::parser::parse_skill_md(test_skill).unwrap();
    let bundle_json = serde_json::to_string(&bundle).unwrap();

    let install_req = serde_json::json!({
        "bundle_json": bundle_json
    });

    let resp: String = tools(&state, "vox_skill_install", install_req)
        .await
        .unwrap();
    assert!(
        resp.contains("\"success\": true") || resp.contains("\"success\":true"),
        "Failed to install skill: {}",
        resp
    );

    // 2. Verify it appears in vox_skill_list
    let list_req = serde_json::json!({});
    let list_resp: String = tools(&state, "vox_skill_list", list_req).await.unwrap();
    assert!(list_resp.contains("test.macro"));
    assert!(list_resp.contains("Test Macro"));
}
