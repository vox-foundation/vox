#![allow(missing_docs)]

use std::sync::Arc;
use vox_orchestrator::{ApprovalOutcome, OrchestratorConfig};
use vox_orchestrator_mcp::{ServerState, handle_tool_call as tools};

// Adds a skill from a LOCAL PATH source (no network), confirms it lists, then
// removes it (ownership-scoped) and confirms the directory is gone.
#[tokio::test]
async fn add_then_remove_local_skill() {
    // Fixture source: <tmp_src>/skills/fixture-skill/SKILL.md
    let src = tempfile::tempdir().unwrap();
    let skill_dir = src.path().join("skills/fixture-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: fixture-skill\ndescription: A fixture skill for the add/remove test\n---\n\n# body\n",
    )
    .unwrap();

    // Isolated workspace so `.vox/skills` resolves under a temp dir.
    let ws = tempfile::tempdir().unwrap();
    let config = OrchestratorConfig::default();
    let mut state = ServerState::new_full(config);
    state.workspace_root = Some(ws.path().to_path_buf());
    let state = Arc::new(state);

    // 1. Add from the local path.
    let add_req = serde_json::json!({ "source": src.path().to_string_lossy() });
    let state_clone = Arc::clone(&state);
    let call = tokio::spawn(async move { tools(&state_clone, "vox_skill_add", add_req).await });

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut approval_id = None;
    while tokio::time::Instant::now() < deadline {
        let list = state.pending_approvals.list();
        if let Some(entry) = list.iter().find(|e| e.tool == "vox_skill_add") {
            approval_id = Some(entry.approval_id.clone());
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    let aid = approval_id.expect("expected vox_skill_add to park for approval");
    assert!(
        state
            .pending_approvals
            .resolve(&aid, ApprovalOutcome::Approved)
    );

    let add_resp = call.await.unwrap().unwrap();
    assert!(
        add_resp.contains("fixture-skill") && !add_resp.contains("\"is_error\":true"),
        "add failed: {add_resp}"
    );

    // 2. It now lists.
    let list_resp = tools(&state, "vox_skill_list", serde_json::json!({}))
        .await
        .unwrap();
    assert!(
        list_resp.contains("fixture-skill"),
        "not listed: {list_resp}"
    );

    // 3. The directory landed under the isolated workspace's .vox/skills.
    let installed_dir = ws
        .path()
        .join(vox_config::paths::REPO_SKILLS_DIR)
        .join("fixture-skill");
    assert!(installed_dir.join("SKILL.md").is_file());

    // 4. Remove it (ownership-scoped — it is under .vox/skills, so removable).
    let rm_resp = tools(
        &state,
        "vox_skill_remove",
        serde_json::json!({ "id": "fixture-skill" }),
    )
    .await
    .unwrap();
    assert!(rm_resp.contains("Removed"), "remove failed: {rm_resp}");

    // 5. Directory is gone.
    assert!(!installed_dir.exists(), "dir still present after remove");
}
