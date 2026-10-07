//! Scope enforcement for write-capable MCP tools.
//!
//! When an agent has declared `.vox/agents/{agent_id}.md` with a `scope:` front-matter
//! block, write tool calls that reference paths outside that scope are rejected at the
//! admission layer — before any filesystem operation occurs.

use crate::server_state::ServerState;

/// Registered tools that write a file at a path taken from their arguments; they must
/// respect agent scope and file locks. `every_write_tool_is_registered` fails if a name
/// here is not in `TOOL_REGISTRY` (the previous list never was, so no gate ever fired).
/// Not listed: `vox_inline_edit` only returns replacement text (the editor writes it),
/// `vox_kb_*` write DB rows, and the search/scaffold/workspace tools write caches or
/// whole trees rather than one path argument.
pub(crate) const WRITE_TOOLS: &[&str] = &["vox_apply_structured_edit", "vox_generate_code"];

/// Path argument key names for write tools (checked in order).
pub(crate) const PATH_ARG_KEYS: &[&str] = &["path", "file_path", "target_file", "output_path"];

/// `(tool, arg key holding the code it will write)` for the lazy-generation scan. Only the
/// new code is scanned, so an edit that replaces a `todo!()` is not rejected for quoting
/// it. `vox_generate_code` is absent: its code is generated server-side, not in `args`.
pub(crate) const LAZY_SCAN_FIELDS: &[(&str, &str)] =
    &[("vox_apply_structured_edit", "replacement_code")];

/// Returns `Some(rejection message)` when the tool call is outside declared scope;
/// `None` when the call is allowed.
pub fn check_scope(
    state: &ServerState,
    tool_name: &str,
    agent_id: Option<&str>,
    args: &serde_json::Value,
) -> Option<String> {
    if !WRITE_TOOLS.contains(&tool_name) {
        return None;
    }
    let agent_name = agent_id?;
    let scopes = vox_repository::load_agent_scopes(&state.repository.root, agent_name)?;
    let path = PATH_ARG_KEYS
        .iter()
        .find_map(|key| args.get(*key).and_then(|v| v.as_str()))?;
    let norm = vox_repository::normalize_task_path(&state.repository.root, path);
    let allowed = scopes.iter().any(|pat| {
        glob::Pattern::new(pat)
            .ok()
            .map(|p: glob::Pattern| p.matches(&norm))
            .unwrap_or(true)
    });
    if allowed {
        return None;
    }
    Some(format!(
        "SCOPE_VIOLATION: Path '{path}' is outside the declared scope for agent '{agent_name}'. \
         Allowed patterns: {scopes:?}. \
         Expand scope in `.vox/agents/{agent_name}.md` or use a path within the declared scope.",
    ))
}

#[cfg(test)]
mod write_gate_tests {
    use super::WRITE_TOOLS;
    use crate::dispatch::handle_tool_call;
    use crate::server_state::ServerState;
    use serde_json::json;
    use std::path::Path;
    use vox_orchestrator::locks::LockKind;
    use vox_orchestrator::types::AgentId;

    #[test]
    fn every_write_tool_is_registered() {
        for name in WRITE_TOOLS {
            assert!(
                vox_mcp_registry::TOOL_REGISTRY
                    .iter()
                    .any(|e| e.name == *name),
                "{name} is not in TOOL_REGISTRY, so no real call can reach the write gates"
            );
        }
    }

    #[tokio::test]
    async fn lazy_gate_rejects_a_stub_replacement() {
        let state = ServerState::new_test().await;
        let out = handle_tool_call(
            &state,
            "vox_apply_structured_edit",
            json!({ "file_path": "src/nope.vox", "target_content": "x", "replacement_code": "fn f() { todo!() }" }),
        )
        .await
        .unwrap();
        assert!(out.contains("LAZY_GENERATION_DETECTED"), "{out}");
    }

    #[tokio::test]
    async fn lazy_gate_allows_replacing_a_stub() {
        let state = ServerState::new_test().await;
        let out = handle_tool_call(
            &state,
            "vox_apply_structured_edit",
            json!({ "file_path": "src/nope.vox", "target_content": "todo!()", "replacement_code": "1" }),
        )
        .await
        .unwrap();
        assert!(!out.contains("LAZY_GENERATION_DETECTED"), "{out}");
    }

    #[tokio::test]
    async fn scope_gate_rejects_a_path_outside_the_agent_scope() {
        let repo = tempfile::tempdir().unwrap();
        // drift-allow(vox-path-literal): test fixture mirrors the on-disk layout
        std::fs::create_dir_all(repo.path().join(".vox/agents")).unwrap();
        std::fs::write(
            // drift-allow(vox-path-literal): test fixture mirrors the on-disk layout
            repo.path().join(".vox/agents/builder.md"),
            "---\nscope: [\"src/**\"]\n---\n",
        )
        .unwrap();
        let mut state = ServerState::new_test().await;
        state.repository.root = repo.path().to_path_buf();
        let out = handle_tool_call(
            &state,
            "vox_apply_structured_edit",
            json!({ "agent_id": "builder", "file_path": "secrets/x.vox", "target_content": "a", "replacement_code": "b" }),
        )
        .await
        .unwrap();
        assert!(out.contains("SCOPE_VIOLATION"), "{out}");
    }

    #[tokio::test]
    async fn scope_gate_reads_generate_code_output_path() {
        let repo = tempfile::tempdir().unwrap();
        // drift-allow(vox-path-literal): test fixture mirrors the on-disk layout
        std::fs::create_dir_all(repo.path().join(".vox/agents")).unwrap();
        std::fs::write(
            // drift-allow(vox-path-literal): test fixture mirrors the on-disk layout
            repo.path().join(".vox/agents/builder.md"),
            "---\nscope: [\"src/**\"]\n---\n",
        )
        .unwrap();
        let mut state = ServerState::new_test().await;
        state.repository.root = repo.path().to_path_buf();
        let out = handle_tool_call(
            &state,
            "vox_generate_code",
            json!({ "agent_id": "builder", "prompt": "hello", "output_path": "secrets/x.vox" }),
        )
        .await
        .unwrap();
        assert!(out.contains("SCOPE_VIOLATION"), "{out}");
    }

    #[tokio::test]
    async fn lock_gate_rejects_a_path_another_agent_holds() {
        let state = ServerState::new_test().await;
        state
            .orchestrator
            .lock_manager
            .try_acquire(Path::new("src/held.vox"), AgentId(2), LockKind::Exclusive)
            .unwrap();
        let out = handle_tool_call(
            &state,
            "vox_apply_structured_edit",
            json!({ "agent_id": "1", "file_path": "src/held.vox", "target_content": "a", "replacement_code": "b" }),
        )
        .await
        .unwrap();
        assert!(out.contains("LOCK_CONFLICT"), "{out}");
    }
}
