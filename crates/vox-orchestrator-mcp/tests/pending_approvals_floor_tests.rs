mod common;

use serde_json::{Value, json};
use std::sync::Arc;
use vox_config::timeouts::D_20MS;
use vox_orchestrator::ApprovalOutcome;
use vox_orchestrator::oversight::floor::FloorCategory;
use vox_orchestrator_mcp::pending_approvals::PendingApprovalInfo;
use vox_orchestrator_mcp::server::tool_json_envelope_is_error;
use vox_orchestrator_mcp::{ServerState, handle_tool_call_with_mode, load_config};

const D_15S: std::time::Duration = std::time::Duration::from_secs(15);

struct IsolatedDbEnv {
    _tempdir: tempfile::TempDir,
    prev_db_path: Option<String>,
}

impl IsolatedDbEnv {
    #[allow(unsafe_code)]
    fn new() -> Self {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let db_path = tempdir.path().join("t03-approval-allowlist-test.db");
        let prev_db_path = std::env::var("VOX_DB_PATH").ok();
        // SAFETY: `#[serial_test::serial]` on every caller serializes env
        // mutation against the rest of this crate's tests.
        unsafe {
            std::env::set_var("VOX_DB_PATH", db_path.to_string_lossy().as_ref());
        }
        Self {
            _tempdir: tempdir,
            prev_db_path,
        }
    }
}

impl Drop for IsolatedDbEnv {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: `#[serial_test::serial]` on every caller serializes env
        // mutation against the rest of this crate's tests.
        unsafe {
            match &self.prev_db_path {
                Some(v) => std::env::set_var("VOX_DB_PATH", v),
                None => std::env::remove_var("VOX_DB_PATH"),
            }
        }
    }
}

async fn assert_parks(
    state: &Arc<ServerState>,
    tool: &str,
    args: Value,
    mode: &str,
) -> PendingApprovalInfo {
    let s2 = state.clone();
    let tool_owned = tool.to_string();
    let mode_owned = mode.to_string();
    let call = tokio::spawn(async move {
        handle_tool_call_with_mode(&s2, &tool_owned, args, Some(&mode_owned)).await
    });

    let deadline = tokio::time::Instant::now() + D_15S;
    loop {
        let list = state.pending_approvals.list();
        if !list.is_empty() {
            assert_eq!(list.len(), 1, "expected exactly one pending approval");
            assert_eq!(list[0].tool, tool, "expected tool name to match");
            let entry = list[0].clone();
            assert!(
                state
                    .pending_approvals
                    .resolve(&entry.approval_id, ApprovalOutcome::Rejected),
                "must resolve pending approval"
            );
            let raw = call.await.expect("join").expect("dispatch ok");
            assert!(
                tool_json_envelope_is_error(&raw),
                "rejected tool call must return error envelope"
            );
            return entry;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "tool {tool} under {mode} never parked inside 15s"
        );
        tokio::time::sleep(D_20MS).await;
    }
}

#[tokio::test]
#[serial_test::serial]
async fn floor_policy_grant_tools_park_under_accept_all() {
    let _env = IsolatedDbEnv::new();
    let state = Arc::new(ServerState::new_full(load_config()));

    let e1 = assert_parks(
        &state,
        "vox_config_set",
        json!({"max_agents": 2}),
        "accept_all",
    )
    .await;
    assert!(e1.summary.starts_with("[floor:policy_grant]"));

    let e2 = assert_parks(
        &state,
        "vox_trust_override",
        json!({"agent_id": 1, "trust_score": 1.0}),
        "accept_all",
    )
    .await;
    assert!(e2.summary.starts_with("[floor:policy_grant]"));

    let e3 = assert_parks(
        &state,
        "vox_add_approval_allowlist_entry",
        json!({"repo_id": "floor-test-repo", "tool": "vox_run_shell"}),
        "accept_all",
    )
    .await;
    assert!(e3.summary.starts_with("[floor:policy_grant]"));

    let e4 = assert_parks(
        &state,
        "vox_config_set",
        json!({"max_agents": 4242, "model": "FLOOR_TEST_CONFIG_VALUE"}),
        "accept_all",
    )
    .await;
    assert!(e4.summary.starts_with("[floor:policy_grant]"));
    assert!(e4.summary.contains("max_agents"));
    assert!(!e4.summary.contains("4242"));
    assert!(!e4.summary.contains("FLOOR_TEST_CONFIG_VALUE"));
}

#[tokio::test]
async fn floor_force_push_parks_under_accept_all() {
    let state = Arc::new(ServerState::new_full(load_config()));

    let e1 = assert_parks(
        &state,
        "vox_run_shell",
        json!({"command": "git push --force floor-test-no-such-remote HEAD:refs/heads/floor-test"}),
        "accept_all",
    )
    .await;
    assert!(e1.summary.starts_with("[floor:history_rewrite]"));

    let e2 = assert_parks(
        &state,
        "vox_skill_run",
        json!({
            "id": "floor-test-no-such-skill",
            "command": "git push floor-test-no-such-remote +HEAD:refs/heads/floor-test"
        }),
        "accept_all",
    )
    .await;
    assert!(e2.summary.starts_with("[floor:history_rewrite]"));
}

#[tokio::test]
async fn floor_history_rewrite_tools_park() {
    let state = Arc::new(ServerState::new_full(load_config()));

    for tool in ["vox_undo", "vox_snapshot_restore", "vox_workspace_merge"] {
        let e = assert_parks(&state, tool, json!({}), "accept_all").await;
        assert!(
            e.summary.starts_with("[floor:history_rewrite]"),
            "tool {tool} summary expected [floor:history_rewrite]"
        );
    }
}

#[tokio::test]
async fn floor_external_effects_park_under_accept_all() {
    let state = Arc::new(ServerState::new_full(load_config()));

    let tools = vox_orchestrator::oversight::floor::tools_for(FloorCategory::ExternalEffect);
    assert!(
        tools.len() >= 9,
        "expected at least 9 external effect tools, found {}",
        tools.len()
    );

    for &tool in tools {
        let e = assert_parks(&state, tool, json!({}), "accept_all").await;
        assert!(
            e.summary.starts_with("[floor:external_effect]"),
            "tool {tool} summary expected [floor:external_effect]"
        );
    }
}

#[tokio::test]
async fn floor_secrets_path_write_parks() {
    let state = Arc::new(ServerState::new_full(load_config()));

    let e1 = assert_parks(
        &state,
        "vox_apply_structured_edit",
        json!({
            "file_path": "target/floor-test-no-such-dir/.env",
            "replacement_code": "FLOOR_TEST_SECRET_VALUE"
        }),
        "accept_all",
    )
    .await;
    assert!(e1.summary.starts_with("[floor:secrets_write]"));
    assert!(!e1.summary.contains("FLOOR_TEST_SECRET_VALUE"));

    let e2 = assert_parks(
        &state,
        "vox_config_set",
        json!({"openai_api_key": "FLOOR_TEST_SECRET_VALUE"}),
        "accept_all",
    )
    .await;
    assert!(e2.summary.starts_with("[floor:secrets_write]"));
    assert!(!e2.summary.contains("FLOOR_TEST_SECRET_VALUE"));
}

#[tokio::test]
async fn floor_policy_file_write_parks() {
    let state = Arc::new(ServerState::new_full(load_config()));

    let e1 = assert_parks(
        &state,
        "vox_apply_structured_edit",
        json!({
            "file_path": "target/floor-test-no-such-dir/Vox.toml",
            "replacement_code": "FLOOR_TEST_CONFIG_VALUE"
        }),
        "accept_all",
    )
    .await;
    assert!(e1.summary.starts_with("[floor:policy_grant]"));
    assert!(
        e1.summary
            .contains("target/floor-test-no-such-dir/Vox.toml")
    );
    assert!(!e1.summary.contains("FLOOR_TEST_CONFIG_VALUE"));

    let e2 = assert_parks(
        &state,
        "vox_apply_structured_edit",
        json!({
            "file_path": "target/floor-test-no-such-dir/.vox/agents/floor-test.md",
            "replacement_code": "FLOOR_TEST_CONFIG_VALUE"
        }),
        "accept_all",
    )
    .await;
    assert!(e2.summary.starts_with("[floor:policy_grant]"));
    assert!(
        e2.summary
            .contains("target/floor-test-no-such-dir/.vox/agents/floor-test.md")
    );
    assert!(!e2.summary.contains("FLOOR_TEST_CONFIG_VALUE"));
}

#[tokio::test]
#[serial_test::serial]
async fn floor_ignores_allowlist() {
    let _env = IsolatedDbEnv::new();
    let state = Arc::new(ServerState::new_full(load_config()));

    vox_orchestrator_mcp::approval_allowlist::add_entry(
        state.repository.repository_id.as_str(),
        "vox_run_shell",
    )
    .await
    .expect("add_entry ok");

    {
        let bm_handle = state.orchestrator.budget_manager_handle();
        let bm = bm_handle.read().unwrap();
        bm.force_trust_score(vox_orchestrator::AgentId(0), 1.0);
    }

    // Precondition: unfloored call under ask does NOT park (allowlisted; returns phantom-tool error promptly)
    let pre_call = handle_tool_call_with_mode(
        &state,
        "vox_run_shell",
        json!({"command": "echo hi"}),
        Some("ask"),
    )
    .await;
    assert!(pre_call.is_err());
    assert!(state.pending_approvals.list().is_empty());

    // Assertion: floored force push DOES park despite allowlist + trust 1.0
    let e = assert_parks(
        &state,
        "vox_run_shell",
        json!({"command": "git push -f floor-test-no-such-remote"}),
        "ask",
    )
    .await;
    assert!(e.summary.starts_with("[floor:history_rewrite]"));
}
