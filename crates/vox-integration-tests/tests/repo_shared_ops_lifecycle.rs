//! Shared repo ops: discovery status parity and MCP `vox_repo_status`.

use serde_json::json;
use tempfile::TempDir;
use vox_mcp::ServerState;
use vox_mcp::tools;

#[tokio::test]
async fn mcp_vox_repo_status_returns_repository_id() {
    let state = ServerState::new_test().await;
    let raw = tools::handle_tool_call(&state, "vox_repo_status", json!({}))
        .await
        .expect("tool returns");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], true, "{raw}");
    let data = &v["data"];
    assert!(
        data["repository_id"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "{raw}"
    );
}

#[test]
fn repo_workspace_status_matches_temp_root() {
    let d = TempDir::new().expect("tempdir");
    let s = vox_repository::repo_workspace_status_for_cwd(d.path());
    assert_eq!(s.root, d.path().canonicalize().unwrap());
    assert!(!s.repository_id.is_empty());
}
