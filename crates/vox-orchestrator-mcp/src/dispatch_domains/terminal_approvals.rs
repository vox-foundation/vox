//! Terminal HITL approval helpers (`TERM-AGENT-03`): floor classification,
//! compound-safe session `Ask`-tier allowlist checks, `Edit & Run` AST/tier
//! re-validation, `Reject + Steer` secret scrubbing, and `Autopilot -> Manual` preemption.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::json;
use vox_orchestrator::ApprovalOutcome;
use vox_orchestrator::oplog::OperationKind;
use vox_orchestrator::oversight::exec_policy::{
    SecurityTier, default_exec_policy_v1, evaluate_command_ast, evaluate_security_tier,
    extract_effective_commands, tokenize_and_split_segments,
};
use vox_orchestrator::oversight::floor::{FloorInput, classify_floor};
use vox_orchestrator::terminal_bridge::{TermSessionSummary, TerminalBridge};

use crate::dispatch_domains::terminal_tools::scrub_terminal_mcp_str;
use crate::pending_approvals::{PendingApprovalMeta, PendingApprovals, PendingResolution};
use crate::server_state::ServerState;

const TERMINAL_APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_STEERING_NOTE_BYTES: usize = 1024;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn truncate_utf8(s: &str, max_bytes: usize) -> &str {
    (0..=max_bytes.min(s.len()))
        .rev()
        .find(|&i| s.is_char_boundary(i))
        .map_or("", |i| &s[..i])
}

async fn rec_op(state: &ServerState, k: OperationKind, msg: String) {
    state
        .orchestrator
        .record_operation(vox_orchestrator::AgentId(0), k, msg, None, None, None, None)
        .await;
}

#[must_use]
pub(crate) fn classify_terminal_floor(tool_name: &str, cmd: &str) -> Option<String> {
    classify_floor(&FloorInput::tool(
        tool_name,
        &json!({ "command": cmd }),
        Path::new("."),
    ))
    .map(|cat| cat.id().to_owned())
}

#[must_use]
pub(crate) fn session_mode_and_tier(
    sessions: &[TermSessionSummary],
    sid: &str,
) -> (&'static str, SecurityTier) {
    sessions.iter().find(|s| s.session_id == sid).map_or(
        ("manual", SecurityTier::L1ReadOnly),
        |f| {
            let mode_raw = f.control_mode.trim();
            let m = if mode_raw.eq_ignore_ascii_case("autopilot") {
                "autopilot"
            } else if mode_raw.eq_ignore_ascii_case("co_pilot") {
                "co_pilot"
            } else {
                "manual"
            };
            let sec_raw = f.security_level.trim();
            let t = if sec_raw.eq_ignore_ascii_case("l2_workspace_build") {
                SecurityTier::L2WorkspaceBuild
            } else if sec_raw.eq_ignore_ascii_case("l3_structured_default") {
                SecurityTier::L3StructuredDefault
            } else if sec_raw.eq_ignore_ascii_case("l4_host_gated") {
                SecurityTier::L4HostGated
            } else if sec_raw.eq_ignore_ascii_case("l5_autopilot_floor") {
                SecurityTier::L5AutopilotFloor
            } else {
                SecurityTier::L1ReadOnly
            };
            (m, t)
        },
    )
}

#[must_use]
pub(crate) fn can_auto_approve_in_session(
    approvals: &PendingApprovals,
    session_id: &str,
    cmd: &str,
    tier: SecurityTier,
    floor_category: Option<&str>,
    credential_prompt: bool,
) -> bool {
    if credential_prompt
        || floor_category.is_some()
        || classify_terminal_floor("vox_terminal_exec", cmd).is_some()
    {
        return false;
    }
    if tier == SecurityTier::L1ReadOnly && evaluate_security_tier(cmd, tier).is_err() {
        return false;
    }
    let ast = evaluate_command_ast(cmd, &default_exec_policy_v1());
    if ast.iter().any(|v| v.rule != "command-not-allowed") {
        return false;
    }
    let Ok(segments) = tokenize_and_split_segments(cmd) else {
        return false;
    };
    if segments.is_empty()
        || segments.iter().any(|seg| {
            seg.argv.is_empty()
                || seg.has_output_redirect
                || seg.has_input_redirect
                || seg.has_command_subst
                || seg.has_process_subst
                || seg.argv[0].contains('=')
                || !approvals.matches_leaf_command(session_id, &seg.argv.join(" "))
        })
    {
        return false;
    }
    let leaves = extract_effective_commands(cmd);
    !leaves.is_empty()
        && leaves
            .iter()
            .all(|leaf| approvals.matches_leaf_command(session_id, &leaf.join(" ")))
}

pub(crate) fn validate_edited_terminal_command(
    edited: &str,
    tier: SecurityTier,
) -> Result<String, String> {
    let trimmed = edited.trim();
    if trimmed.is_empty() {
        return Err("Edited command must not be empty.".into());
    }
    let ast = evaluate_command_ast(trimmed, &default_exec_policy_v1());
    if let Some(v) = ast.iter().find(|v| v.rule != "command-not-allowed") {
        return Err(format!(
            "Exec policy rejected edited command ({}): {}",
            v.rule, v.message
        ));
    }
    if tier == SecurityTier::L1ReadOnly
        && let Err(v) = evaluate_security_tier(trimmed, tier)
    {
        return Err(format!(
            "Security tier L1ReadOnly rejected edited command ({}): {}",
            v.rule, v.message
        ));
    }
    Ok(trimmed.to_owned())
}

fn validate_edited_and_floor(
    tool: &str,
    raw: Option<&str>,
    tier: SecurityTier,
    floor_cat: Option<&str>,
    summary: &str,
) -> Result<String, String> {
    let raw = raw.ok_or("Modified approval missing 'modified_command'.")?;
    let validated = validate_edited_terminal_command(raw, tier)?;
    if let Some(new_floor) = classify_terminal_floor(tool, &validated) {
        let same = floor_cat == Some(new_floor.as_str())
            || (floor_cat.is_none() && summary.starts_with(&format!("[floor:{new_floor}]")));
        if !same {
            return Err(format!(
                "Edited command escalated into safety floor '{new_floor}'; submit directly for explicit floor approval."
            ));
        }
    }
    Ok(validated)
}

pub(crate) async fn validate_terminal_resolution_before_commit(
    state: &ServerState,
    approval_id: &str,
    resolution: &PendingResolution,
) -> Result<(), String> {
    if resolution.outcome != ApprovalOutcome::Modified {
        return Ok(());
    }
    let Some(entry) = state.pending_approvals.peek(approval_id) else {
        return Ok(());
    };
    if !matches!(
        entry.tool.as_str(),
        "vox_terminal_exec" | "vox_terminal_send_input"
    ) {
        return Ok(());
    }
    let sessions = state
        .resolve_terminal_bridge()
        .await
        .list_sessions()
        .await
        .unwrap_or_default();
    let (_, tier) = session_mode_and_tier(&sessions, entry.session_id.as_deref().unwrap_or(""));
    validate_edited_and_floor(
        &entry.tool,
        resolution.modified_command.as_deref(),
        tier,
        entry.floor_category.as_deref(),
        &entry.summary,
    )
    .map(drop)
}

pub(crate) fn check_autopilot_preempted(
    approvals: &PendingApprovals,
    session_id: &str,
    pre_mode: &str,
    current_mode: &str,
) -> Result<(), String> {
    if pre_mode == "autopilot" && current_mode == "manual" {
        approvals.clear_session_patterns(session_id);
        return Err(
            "Terminal command cancelled: Autopilot preempted by operator Manual takeover.".into(),
        );
    }
    Ok(())
}

pub(crate) async fn park_terminal_hitl(
    state: &ServerState,
    tool: &str,
    summary: String,
    meta: PendingApprovalMeta,
    tier: SecurityTier,
    run_id: Option<String>,
) -> Result<Option<String>, String> {
    let t0 = now_ms();
    let (id, rx) =
        state
            .pending_approvals
            .register_with_meta(tool.into(), summary.clone(), t0, meta.clone());
    if let Some(db) = state.db.as_ref() {
        let _ = db
            .hitl_approval_record(&id, tool, &summary, t0 as i64)
            .await;
    }
    rec_op(
        state,
        OperationKind::ApprovalRequested {
            approval_id: id.clone(),
            tool: tool.into(),
            run_id,
        },
        format!("Approval requested for {tool}"),
    )
    .await;
    let resolution = match tokio::time::timeout(TERMINAL_APPROVAL_TIMEOUT, rx).await {
        Ok(Ok(res)) => res,
        Ok(Err(_)) => PendingResolution::simple(ApprovalOutcome::Rejected),
        Err(_) => {
            state.pending_approvals.cancel(&id);
            PendingResolution::simple(ApprovalOutcome::TimedOut)
        }
    };
    let out = resolution.outcome;
    let st = format!("{out:?}").to_lowercase();
    if let Some(db) = state.db.as_ref() {
        let _ = db.hitl_approval_resolve(&id, &st, now_ms() as i64).await;
    }
    rec_op(
        state,
        OperationKind::ApprovalResolved {
            approval_id: id,
            outcome: st,
            resolver: None,
        },
        format!("Approval resolved for {tool}"),
    )
    .await;
    match out {
        ApprovalOutcome::Approved | ApprovalOutcome::AutoApproved => Ok(None),
        ApprovalOutcome::Modified => validate_edited_and_floor(
            tool,
            resolution.modified_command.as_deref(),
            tier,
            meta.floor_category.as_deref(),
            &summary,
        )
        .map(Some),
        ApprovalOutcome::Rejected | ApprovalOutcome::TimedOut => {
            let note_suffix = resolution
                .steering_note
                .as_deref()
                .map(scrub_terminal_mcp_str)
                .and_then(|s| {
                    let t = truncate_utf8(s.trim(), MAX_STEERING_NOTE_BYTES);
                    (!t.is_empty()).then(|| format!(": {t}"))
                })
                .unwrap_or_default();
            Err(format!(
                "Operation '{tool}' was not approved (outcome: {out:?}){note_suffix}"
            ))
        }
    }
}

pub(crate) async fn gate_terminal_exec(
    state: &ServerState,
    bridge: &Arc<dyn TerminalBridge>,
    sid: &str,
    cmd: &str,
    run_id: Option<String>,
    outer_hitl_approved: bool,
) -> Result<(String, bool), String> {
    let sessions = bridge
        .list_sessions()
        .await
        .map_err(|e| scrub_terminal_mcp_str(&e.to_string()))?;
    let (mode, tier) = session_mode_and_tier(&sessions, sid);
    let floor_category = classify_terminal_floor("vox_terminal_exec", cmd);
    let tier_err = evaluate_security_tier(cmd, tier).is_err();
    let mut effective_cmd = cmd.to_owned();
    let mut hitl_approved = outer_hitl_approved;

    if !hitl_approved
        && (mode == "manual" || floor_category.is_some() || (mode == "co_pilot" && tier_err))
    {
        if can_auto_approve_in_session(
            &state.pending_approvals,
            sid,
            cmd,
            tier,
            floor_category.as_deref(),
            false,
        ) {
            hitl_approved = true;
        } else {
            let scrubbed_cmd = scrub_terminal_mcp_str(cmd);
            let summary = match floor_category.as_deref() {
                Some(cat) => format!("[floor:{cat}] [terminal:{mode}] {scrubbed_cmd}"),
                None => format!("[terminal:{mode}] {scrubbed_cmd}"),
            };
            let meta = PendingApprovalMeta {
                floor_category,
                session_id: Some(sid.to_owned()),
                command: Some(scrubbed_cmd),
                control_mode: Some(mode.to_owned()),
                credential_prompt: Some(false),
            };
            if let Some(edited) =
                park_terminal_hitl(state, "vox_terminal_exec", summary, meta, tier, run_id).await?
            {
                effective_cmd = edited;
            }
            hitl_approved = true;
        }
    }

    if mode == "autopilot" {
        let post_sessions = bridge
            .list_sessions()
            .await
            .map_err(|e| scrub_terminal_mcp_str(&e.to_string()))?;
        let (post_mode, _) = session_mode_and_tier(&post_sessions, sid);
        check_autopilot_preempted(&state.pending_approvals, sid, mode, post_mode)?;
    }

    Ok((effective_cmd, hitl_approved))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_allowlist_compound_tier_floor_and_credential_guards() {
        let reg = PendingApprovals::default();
        assert!(reg.allow_session_pattern("s-1", "cargo test"));
        let (l1, l3) = (SecurityTier::L1ReadOnly, SecurityTier::L3StructuredDefault);
        let ok = |s, c, t, f, cr| can_auto_approve_in_session(&reg, s, c, t, f, cr);
        let tgt = "cargo test -p vox-terminal-core";
        assert!(ok("s-1", tgt, l3, None, false) && !ok("s-2", tgt, l3, None, false));
        for c in [
            "cargo test && rm -rf /",
            "cargo test | sh",
            "cargo test; id",
            "cargo test > /tmp/out",
            "sudo cargo test",
            "LD_PRELOAD=/tmp/x.so cargo test",
        ] {
            assert!(!ok("s-1", c, l3, None, false), "smuggled: {c}");
        }
        assert!(!ok("s-1", "cargo test", l3, Some("history_rewrite"), false));
        assert!(!ok("s-1", "cargo test", l3, None, true));
        assert!(!ok("s-1", "cargo test", l1, None, false));
        let fl = classify_terminal_floor("vox_terminal_exec", "git push --force origin main");
        assert_eq!(fl.as_deref(), Some("history_rewrite"));
        let v = validate_edited_terminal_command;
        assert_eq!(
            v("  cargo check -p vox-cli  ", l3).unwrap(),
            "cargo check -p vox-cli"
        );
        for b in ["   ", "rm -rf /", "echo $(whoami)"] {
            assert!(v(b, l3).is_err());
        }
        assert!(v("cargo test", l1).is_err() && v("pwd", l1).unwrap() == "pwd");
    }

    #[tokio::test]
    async fn park_hitl_reject_steer_scrubs_secrets_and_autopilot_preempts() {
        let st = ServerState::new_test().await;
        st.pending_approvals
            .allow_session_pattern("s-1", "cargo test");
        let pre = check_autopilot_preempted(&st.pending_approvals, "s-1", "autopilot", "manual");
        assert!(pre.is_err() && st.pending_approvals.session_patterns("s-1").is_empty());

        let (cl, sec) = (
            st.clone(),
            format!("ghp_{}", "A1b2C3d4E5f6G7h8I9j0K1l2M3n4"),
        );
        let note = format!("Use --bin vox-gui; token={sec}");
        tokio::spawn(async move {
            for _ in 0..80 {
                if let Some(f) = cl.pending_approvals.list().first() {
                    let mut res = PendingResolution::simple(ApprovalOutcome::Rejected);
                    res.steering_note = Some(note);
                    cl.pending_approvals
                        .resolve_with_detail(&f.approval_id, res);
                    return;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
        let m = PendingApprovalMeta {
            session_id: Some("s-1".into()),
            ..Default::default()
        };
        let sum = "[terminal:co_pilot] cargo run".into();
        let t = SecurityTier::L3StructuredDefault;
        let err = park_terminal_hitl(&st, "vox_terminal_exec", sum, m, t, None)
            .await
            .unwrap_err();
        assert!(
            err.contains("Use --bin vox-gui")
                && !err.contains(&sec)
                && err.contains("[REDACTED_KEY]")
        );
    }
}
