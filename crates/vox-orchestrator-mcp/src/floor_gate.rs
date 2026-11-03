//! Dispatch-level safety floor gate and approval summary generation.

use crate::permission_modes::PermissionMode;
use std::path::Path;
use vox_orchestrator::ApprovalOutcome;
use vox_orchestrator::oversight::floor::{FloorCategory, FloorInput, classify_floor};

tokio::task_local! {
    /// Origin is set only by the two in-process dispatchers, by construction;
    /// nothing on the wire is consulted; a `tokio::spawn` from inside the scope does not inherit it;
    /// external MCP clients are the Phase 14 residual.
    static AGENT_ORIGIN: bool;
}

pub(crate) async fn with_agent_origin<F: std::future::Future>(fut: F) -> F::Output {
    AGENT_ORIGIN.scope(true, fut).await
}

pub(crate) fn in_agent_origin() -> bool {
    AGENT_ORIGIN.try_with(|v| *v).unwrap_or(false)
}

pub(crate) const FLOOR_SELF_APPROVAL_REFUSED: &str = "FLOOR_SELF_APPROVAL_REFUSED";

pub(crate) fn refuse_agent_floor_resolution(
    state: &crate::server_state::ServerState,
    approval_id: &str,
    outcome: ApprovalOutcome,
) -> Option<String> {
    if !in_agent_origin() {
        return None;
    }
    if outcome == ApprovalOutcome::Rejected {
        return None;
    }
    let list = state.pending_approvals.list();
    let info = list.iter().find(|e| e.approval_id == approval_id)?;
    #[rustfmt::skip]
    let floored = info.summary.starts_with("[floor:") || vox_orchestrator::oversight::floor::name_is_floored(&info.tool);
    if floored {
        Some(
            crate::params::ToolResult::<()>::err(format!(
                "{FLOOR_SELF_APPROVAL_REFUSED}: floored approval must be resolved by a human in the GUI Approvals view"
            ))
            .to_json(),
        )
    } else {
        None
    }
}

/// These names are gated or floored before they are registered in the MCP tool registry,
/// so a future tool with one of these names is born gated; any other gated name must be registered.
#[allow(dead_code)]
pub(crate) const RESERVED_GATED_NAMES: &[&str] = &[
    "vox_write_file",
    "vox_multi_replace",
    "vox_multi_replace_file",
    "vox_delete_file",
    "vox_run_shell",
    "vox_deploy",
];

pub(crate) struct GatePlan {
    pub floor: Option<FloorCategory>,
    pub gated: bool,
    pub mode_auto_approved: bool,
    pub allowlist_eligible: bool,
}

pub(crate) fn plan_gate(
    name: &str,
    args: &serde_json::Value,
    repo_root: &Path,
    mode: PermissionMode,
) -> GatePlan {
    let input = FloorInput::tool(name, args, repo_root);
    let floor = classify_floor(&input);
    let gated = floor.is_some() || crate::permission_modes::is_gated_tool(name);
    let mode_auto_approved =
        floor.is_none() && crate::permission_modes::mode_auto_approves(mode, name);
    let allowlist_eligible = floor.is_none() && crate::permission_modes::allowlist_eligible(name);
    GatePlan {
        floor,
        gated,
        mode_auto_approved,
        allowlist_eligible,
    }
}

fn summary_args(name: &str, args: &serde_json::Value, floor: Option<FloorCategory>) -> String {
    if name == "vox_config_set" {
        let mut keys = Vec::new();
        collect_json_keys("", args, &mut keys);
        keys.sort();
        keys.dedup();
        return format!("keys=[{}]", keys.join(", "));
    }

    if floor.is_some()
        && (vox_orchestrator::oversight::floor::FILE_WRITE_TOOLS.contains(&name)
            || name == "vox_apply_structured_edit"
            || name == "vox_generate_code")
    {
        for key in ["path", "file_path", "target_file", "output_path"] {
            if let Some(val) = args.get(key).and_then(|v| v.as_str()) {
                return format!("path={val}");
            }
        }
    }

    let a = args.to_string();
    let a: String = a.chars().take(200).collect();
    a
}

fn collect_json_keys(prefix: &str, val: &serde_json::Value, out: &mut Vec<String>) {
    if let serde_json::Value::Object(map) = val {
        for (k, v) in map {
            let key = if prefix.is_empty() {
                k.clone()
            } else {
                format!("{prefix}.{k}")
            };
            out.push(key.clone());
            collect_json_keys(&key, v, out);
        }
    }
}

pub(crate) fn approval_summary(
    name: &str,
    args: &serde_json::Value,
    floor: Option<FloorCategory>,
) -> String {
    let args_str = summary_args(name, args, floor);
    match floor {
        Some(cat) => format!("[floor:{}] {name} {args_str}", cat.id()),
        None => format!("{name} {args_str}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server_state::ServerState;
    use serde_json::json;

    #[test]
    fn plan_gate_floor_overrides_accept_all() {
        let root = Path::new("/tmp/test-repo");
        let plan = plan_gate(
            "vox_config_set",
            &json!({}),
            root,
            PermissionMode::AcceptAll,
        );
        assert!(plan.gated);
        assert!(!plan.mode_auto_approved);
        assert!(!plan.allowlist_eligible);
        assert_eq!(plan.floor, Some(FloorCategory::PolicyGrant));
    }

    #[test]
    fn plan_gate_unfloored_gated_tool_keeps_t03_behavior() {
        let root = Path::new("/tmp/test-repo");
        let plan = plan_gate(
            "vox_write_file",
            &json!({"path": "src/a.rs"}),
            root,
            PermissionMode::AcceptAll,
        );
        assert!(plan.gated);
        assert!(plan.mode_auto_approved);
        assert_eq!(plan.floor, None);
    }

    #[test]
    fn approval_summary_config_writes_show_keys_only() {
        let args = json!({"max_agents": 4242, "model": "FLOOR_TEST_CONFIG_VALUE"});
        let summary = approval_summary("vox_config_set", &args, Some(FloorCategory::PolicyGrant));
        assert!(summary.starts_with("[floor:policy_grant] vox_config_set"));
        assert!(summary.contains("max_agents"));
        assert!(summary.contains("model"));
        assert!(!summary.contains("4242"));
        assert!(!summary.contains("FLOOR_TEST_CONFIG_VALUE"));

        let policy_file_args = json!({
            "file_path": "target/floor-test/Vox.toml",
            "replacement_code": "FLOOR_TEST_CONFIG_VALUE"
        });
        let summary_file = approval_summary(
            "vox_apply_structured_edit",
            &policy_file_args,
            Some(FloorCategory::PolicyGrant),
        );
        assert!(summary_file.contains("path=target/floor-test/Vox.toml"));
        assert!(!summary_file.contains("FLOOR_TEST_CONFIG_VALUE"));
    }

    #[test]
    fn approval_summary_redacts_secret_values() {
        let args = json!({"openai_api_key": "FLOOR_TEST_SECRET_VALUE"});
        let summary = approval_summary("vox_config_set", &args, Some(FloorCategory::SecretsWrite));
        assert!(summary.contains("[floor:secrets_write]"));
        assert!(summary.contains("openai_api_key"));
        assert!(!summary.contains("FLOOR_TEST_SECRET_VALUE"));

        let path_args = json!({
            "file_path": ".env",
            "replacement_code": "FLOOR_TEST_SECRET_VALUE"
        });
        let summary_path = approval_summary(
            "vox_apply_structured_edit",
            &path_args,
            Some(FloorCategory::SecretsWrite),
        );
        assert!(summary_path.contains("path=.env"));
        assert!(!summary_path.contains("FLOOR_TEST_SECRET_VALUE"));
    }

    #[tokio::test]
    async fn tracer_config_set_parks_under_accept_all() {
        let state = ServerState::new_test().await;
        let initial_max_agents = state.orchestrator_config.max_agents;
        let state_clone = state.clone();

        let call = tokio::spawn(async move {
            crate::dispatch::handle_tool_call_with_mode(
                &state_clone,
                "vox_config_set",
                json!({"max_agents": 2}),
                Some("accept_all"),
            )
            .await
        });

        // drift-allow(timeout-literal,duplicate-numeric-literal): local test approval poll deadline
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
        let mut approval_id = None;
        while tokio::time::Instant::now() < deadline {
            let list = state.pending_approvals.list();
            if !list.is_empty() {
                assert_eq!(list.len(), 1);
                assert_eq!(list[0].tool, "vox_config_set");
                approval_id = Some(list[0].approval_id.clone());
                break;
            }
            // drift-allow(timeout-literal,duplicate-numeric-literal): local test approval poll interval
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        let aid = approval_id.expect("did not park approval within 15s");
        assert!(
            state
                .pending_approvals
                .resolve(&aid, vox_orchestrator::ApprovalOutcome::Rejected)
        );

        let raw = call
            .await
            .expect("join")
            .expect("dispatch returns Ok envelope");
        assert!(crate::server_state::tool_json_envelope_is_error(&raw));
        assert_eq!(state.orchestrator_config.max_agents, initial_max_agents);
    }

    #[test]
    fn every_gated_tool_is_registered_or_reserved() {
        use std::collections::HashSet;
        use vox_orchestrator::oversight::floor::{
            ALL, FILE_WRITE_TOOLS, SHELL_COMMAND_TOOLS, tools_for,
        };

        let mut gated_names = HashSet::new();

        // 1. Every RISK_CLASSES tool
        for rc in crate::permission_modes::RISK_CLASSES {
            gated_names.insert(rc.tool);
        }

        // 2. Every name in tools_for(c) for every FloorCategory
        for &cat in &ALL {
            for &t in tools_for(cat) {
                gated_names.insert(t);
            }
        }

        // 3. SHELL_COMMAND_TOOLS and FILE_WRITE_TOOLS
        for &t in SHELL_COMMAND_TOOLS {
            gated_names.insert(t);
        }
        for &t in FILE_WRITE_TOOLS {
            gated_names.insert(t);
        }

        for &name in &gated_names {
            let is_registered = vox_mcp_registry::TOOL_REGISTRY
                .iter()
                .any(|e| e.name == name);
            let is_reserved = RESERVED_GATED_NAMES.contains(&name);
            assert!(
                is_registered || is_reserved,
                "gated tool '{name}' is neither registered in TOOL_REGISTRY nor listed in RESERVED_GATED_NAMES"
            );
        }

        // No RESERVED_GATED_NAMES entry is registered, so the list cannot rot.
        for &reserved in RESERVED_GATED_NAMES {
            assert!(
                !vox_mcp_registry::TOOL_REGISTRY
                    .iter()
                    .any(|e| e.name == reserved),
                "reserved tool '{reserved}' is now registered in TOOL_REGISTRY — remove it from RESERVED_GATED_NAMES"
            );
        }
    }

    #[test]
    fn floor_write_tools_match_scope_guard() {
        for &tool in crate::scope_guard::WRITE_TOOLS {
            assert!(
                vox_orchestrator::oversight::floor::FILE_WRITE_TOOLS.contains(&tool),
                "scope_guard WRITE_TOOLS contains '{tool}' which is missing from oversight floor FILE_WRITE_TOOLS"
            );
        }
    }

    #[tokio::test]
    async fn agent_origin_scope_is_by_construction_only() {
        assert!(!in_agent_origin());

        let inside = with_agent_origin(async {
            assert!(in_agent_origin());

            let spawned = tokio::spawn(async { in_agent_origin() })
                .await
                .expect("join");
            assert!(!spawned, "task-local must not cross tokio::spawn boundary");

            in_agent_origin()
        })
        .await;

        assert!(inside);
        assert!(!in_agent_origin());
    }

    #[tokio::test]
    async fn refuse_agent_floor_resolution_matrix() {
        let state = ServerState::new_test().await;

        // 1. approve or modify of a floored park -> Some, containing marker
        let (id1, _) = state.pending_approvals.register(
            "vox_skill_install".to_string(),
            "[floor:external_effect] vox_skill_install {}".to_string(),
            0,
        );
        with_agent_origin(async {
            let res = refuse_agent_floor_resolution(&state, &id1, ApprovalOutcome::Approved);
            assert!(res.is_some());
            assert!(res.unwrap().contains(FLOOR_SELF_APPROVAL_REFUSED));

            let res_mod = refuse_agent_floor_resolution(&state, &id1, ApprovalOutcome::Modified);
            assert!(res_mod.is_some());
            assert!(res_mod.unwrap().contains(FLOOR_SELF_APPROVAL_REFUSED));

            // 2. reject of a floored park -> None
            let res_rej = refuse_agent_floor_resolution(&state, &id1, ApprovalOutcome::Rejected);
            assert!(res_rej.is_none());
        })
        .await;

        // 3. approve of [recovered-on-restart] vox_undo -> Some (name fallback)
        let (id2, _) = state.pending_approvals.register(
            "vox_undo".to_string(),
            "[recovered-on-restart] vox_undo".to_string(),
            0,
        );
        with_agent_origin(async {
            let res = refuse_agent_floor_resolution(&state, &id2, ApprovalOutcome::Approved);
            assert!(res.is_some());
            assert!(res.unwrap().contains(FLOOR_SELF_APPROVAL_REFUSED));
        })
        .await;

        // 4. approve of an arg-floored park, tool vox_skill_run with the summary from
        // approval_summary("vox_skill_run", {"command":"git push --force floor-test-no-such-remote"}, Some(HistoryRewrite)) -> Some
        let summary_arg_floored = approval_summary(
            "vox_skill_run",
            &json!({"command": "git push --force floor-test-no-such-remote"}),
            Some(FloorCategory::HistoryRewrite),
        );
        let (id3, _) =
            state
                .pending_approvals
                .register("vox_skill_run".to_string(), summary_arg_floored, 0);
        with_agent_origin(async {
            let res = refuse_agent_floor_resolution(&state, &id3, ApprovalOutcome::Approved);
            assert!(res.is_some());
            assert!(res.unwrap().contains(FLOOR_SELF_APPROVAL_REFUSED));
        })
        .await;

        // 5. approve of an unfloored park (vox_write_file-style summary with no prefix, tool not floored) -> None
        let (id4, _) = state.pending_approvals.register(
            "vox_write_file".to_string(),
            "vox_write_file path=src/lib.rs".to_string(),
            0,
        );
        with_agent_origin(async {
            let res = refuse_agent_floor_resolution(&state, &id4, ApprovalOutcome::Approved);
            assert!(res.is_none());

            // 6. unknown id -> None
            let res_unk =
                refuse_agent_floor_resolution(&state, "no_such_id", ApprovalOutcome::Approved);
            assert!(res_unk.is_none());
        })
        .await;

        // Outside the scope, every case -> None
        assert!(refuse_agent_floor_resolution(&state, &id1, ApprovalOutcome::Approved).is_none());
        assert!(refuse_agent_floor_resolution(&state, &id2, ApprovalOutcome::Approved).is_none());
        assert!(refuse_agent_floor_resolution(&state, &id3, ApprovalOutcome::Approved).is_none());
        assert!(refuse_agent_floor_resolution(&state, &id4, ApprovalOutcome::Approved).is_none());
    }
}
