//! End-to-end tests for `vox_gsd_*` MCP tools over `.planning/` and in-memory `VoxDb`.

use serde_json::json;
use tempfile::TempDir;
use vox_db::{DbConfig, VoxDb};
use vox_orchestrator_mcp::server_state::ServerState;

#[tokio::test]
async fn gsd_mcp_end_to_end_lifecycle() {
    let dir = TempDir::new().expect("tempdir");
    let db = VoxDb::connect(DbConfig::Memory)
        .await
        .expect("in-memory db");
    let mut state = ServerState::new_test()
        .await
        .with_db_initialized(std::sync::Arc::new(db))
        .await;
    state.workspace_root = Some(dir.path().to_path_buf());

    // 1. /gsd-new-milestone
    let raw = vox_orchestrator_mcp::handle_tool_call(
        &state,
        "vox_gsd_new_milestone",
        json!({
            "version": "v7.0",
            "title": "Deep GSD Integration",
            "start_phase": 407,
            "phases": 2
        }),
    )
    .await
    .expect("new_milestone");
    let res: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(res["success"], true);
    assert_eq!(res["data"]["active_milestone"]["milestone_id"], "v7.0");
    assert!(dir.path().join(".planning/ROADMAP.md").is_file());

    // 2. Discuss phase + Plan phase -> PlanNode DAG
    let discuss_raw = vox_orchestrator_mcp::handle_tool_call(
        &state,
        "vox_gsd_discuss_phase",
        json!({
            "phase": 407,
            "summary": "Keep brainstorming active while shipping 30 MIT GSD skills."
        }),
    )
    .await
    .expect("discuss_phase");
    let discuss: serde_json::Value = serde_json::from_str(&discuss_raw).expect("json");
    assert_eq!(discuss["success"], true);

    let plan_raw = vox_orchestrator_mcp::handle_tool_call(
        &state,
        "vox_gsd_plan_phase",
        json!({ "phase": 407, "mvp": false }),
    )
    .await
    .expect("plan_phase");
    let plan: serde_json::Value = serde_json::from_str(&plan_raw).expect("json");
    assert_eq!(plan["success"], true);
    assert!(!plan["data"].as_array().expect("nodes").is_empty());

    // 3. Store & retrieve CAS-backed document
    let store_raw = vox_orchestrator_mcp::handle_tool_call(
        &state,
        "vox_gsd_store_document",
        json!({
            "rel_path": ".planning/phases/407/SPEC.md",
            "content": "# Phase 407 Spec\n\nVerified end-to-end.\n",
            "phase": 407
        }),
    )
    .await
    .expect("store_document");
    let store: serde_json::Value = serde_json::from_str(&store_raw).expect("json");
    assert_eq!(store["success"], true);

    let get_raw = vox_orchestrator_mcp::handle_tool_call(
        &state,
        "vox_gsd_get_document",
        json!({ "doc_id_or_path": ".planning/phases/407/SPEC.md" }),
    )
    .await
    .expect("get_document");
    let got: serde_json::Value = serde_json::from_str(&get_raw).expect("json");
    assert_eq!(got["success"], true);
    assert!(
        got["data"]["content"]
            .as_str()
            .unwrap()
            .contains("Verified end-to-end.")
    );

    // 4. Execute & verify work + check progress + complete milestone
    let exec_raw = vox_orchestrator_mcp::handle_tool_call(
        &state,
        "vox_gsd_execute_phase",
        json!({ "phase": 407 }),
    )
    .await
    .expect("execute_phase");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&exec_raw).unwrap()["success"],
        true
    );

    let ver_raw = vox_orchestrator_mcp::handle_tool_call(
        &state,
        "vox_gsd_verify_work",
        json!({ "phase": 407, "passed": true, "evidence": "cargo test passed" }),
    )
    .await
    .expect("verify_work");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&ver_raw).unwrap()["success"],
        true
    );

    let comp_raw = vox_orchestrator_mcp::handle_tool_call(
        &state,
        "vox_gsd_complete_milestone",
        json!({ "version": "v7.0" }),
    )
    .await
    .expect("complete_milestone");
    let comp: serde_json::Value = serde_json::from_str(&comp_raw).expect("json");
    assert_eq!(comp["success"], true);
    assert_eq!(comp["data"]["active_milestone"]["status"], "completed");
}
