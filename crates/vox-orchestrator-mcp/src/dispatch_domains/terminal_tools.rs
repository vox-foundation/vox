//! MCP dispatch handlers for stateful PTY terminal tools (`vox_terminal_*`),
//! enforcing `TermControlMode`, `ExecPolicyV1`, `SecurityTier`, `PendingApprovals`
//! lockstep (without double-parking under `Ask`), fail-closed credential prompts,
//! and 3-stage output scrubbing.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;
use vox_orchestrator::oversight::exec_policy::{default_exec_policy_v1, evaluate_command_ast};
use vox_orchestrator::terminal_bridge::{
    TermExecRequest, TermReadRequest, TermReadResponse, TermSendInputRequest,
};

use crate::dispatch_domains::terminal_approvals::{
    gate_terminal_exec, park_terminal_hitl, session_mode_and_tier,
};
use crate::params::ToolResult;
use crate::pending_approvals::PendingApprovalMeta;
use crate::server_state::ServerState;

/// 3-stage terminal output scrubber: full ECMA-48 + CR normalize -> Clavis secret needles -> `vox_redact`.
#[must_use]
pub(crate) fn scrub_terminal_mcp_str(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let plain = crate::chat_tools::chat::term_mentions::strip_ecma48_and_normalize_cr(raw);
    vox_redact::redact_owned(&vox_secrets::scrub_resolved_secrets(&plain))
}

/// Dispatch `vox_terminal_{exec,read,send_input,sessions}` tool invocations.
pub(crate) async fn dispatch(
    state: &ServerState,
    name_canonical: &str,
    args: Value,
    outer_hitl_approved: bool,
) -> Result<Option<String>, anyhow::Error> {
    let s_arg = |k: &str| {
        args.get(k)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let u_arg = |k: &str| args.get(k).and_then(Value::as_u64);
    let b_arg = |k: &str, d: bool| args.get(k).and_then(Value::as_bool).unwrap_or(d);
    let err_j = |m: String| -> Result<Option<String>, anyhow::Error> {
        Ok(Some(ToolResult::<()>::err(m).to_json()))
    };
    let scrub_r = |mut r: TermReadResponse| {
        r.cwd = r.cwd.map(|c| scrub_terminal_mcp_str(&c));
        for blk in &mut r.blocks {
            blk.input = scrub_terminal_mcp_str(&blk.input);
            blk.output = scrub_terminal_mcp_str(&blk.output);
            blk.cwd = blk.cwd.as_deref().map(scrub_terminal_mcp_str);
        }
        r
    };

    match name_canonical {
        "vox_terminal_exec" => {
            let Some(cmd) = s_arg("command") else {
                return err_j("Missing or empty required 'command' argument".into());
            };
            let ast = evaluate_command_ast(cmd, &default_exec_policy_v1());
            if ast.iter().any(|v| v.rule == "empty-command") {
                return err_j("Empty command is not allowed".into());
            }
            if let Some(v) = ast.iter().find(|v| v.rule != "command-not-allowed") {
                return err_j(format!(
                    "Exec policy rejected command ({}): {}",
                    v.rule, v.message
                ));
            }
            let ex_sid = s_arg("session_id");
            let tid = s_arg("chat_thread_id").or(ex_sid.is_none().then_some("default"));
            let resolved = if ex_sid.is_none()
                && let Some(dir) = s_arg("cwd")
            {
                let b = state.resolve_terminal_bridge().await;
                let live = match b.list_sessions().await {
                    Ok(xs) => xs,
                    Err(e) => return err_j(scrub_terminal_mcp_str(&e.to_string())),
                };
                let bound = tid
                    .and_then(|t| state.chat_terminal_sessions.read().get(t).cloned())
                    .filter(|sid| live.iter().any(|s| s.session_id == *sid));
                match bound {
                    Some(sid) => Ok((b, sid)),
                    None => match b.open_session(Some(PathBuf::from(dir))).await {
                        Ok(sid) => {
                            state
                                .bind_or_resolve_terminal_session(tid, Some(&sid))
                                .await
                        }
                        Err(e) => Err(e),
                    },
                }
            } else {
                state.bind_or_resolve_terminal_session(tid, ex_sid).await
            };
            let (bridge, sid) = match resolved {
                Ok(p) => p,
                Err(e) => return err_j(scrub_terminal_mcp_str(&e.to_string())),
            };
            let (effective_cmd, hitl_approved) = match gate_terminal_exec(
                state,
                &bridge,
                &sid,
                cmd,
                s_arg("run_id").map(Into::into),
                outer_hitl_approved,
            )
            .await
            {
                Ok(pair) => pair,
                Err(e) => return err_j(e),
            };
            let wait_ms = if b_arg("background", false) {
                0
            } else {
                u_arg("wait_ms").unwrap_or(10_000).min(120_000)
            };
            let req = TermExecRequest {
                session_id: sid,
                command: effective_cmd,
                agent_id: s_arg("agent_id").unwrap_or("mcp-agent").into(),
                wait_ms,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                stall_quiescence_ms: 1_500,
                rationale: s_arg("rationale").map(Into::into),
                hitl_approved,
            };
            match bridge.exec(req).await {
                Ok(mut r) => {
                    r.output = scrub_terminal_mcp_str(&r.output);
                    r.cwd = r.cwd.map(|c| scrub_terminal_mcp_str(&c));
                    Ok(Some(ToolResult::ok(r).to_json()))
                }
                Err(e) => err_j(scrub_terminal_mcp_str(&e.to_string())),
            }
        }
        "vox_terminal_read" => {
            let ex_sid = s_arg("session_id");
            let tid = s_arg("chat_thread_id").or(ex_sid.is_none().then_some("default"));
            let (bridge, sid) = match state.bind_or_resolve_terminal_session(tid, ex_sid).await {
                Ok(p) => p,
                Err(e) => return err_j(scrub_terminal_mcp_str(&e.to_string())),
            };
            let blk_id = u_arg("block_id");
            let pinned_only = b_arg("pinned_only", false);
            let failed_only = b_arg("failed_only", false);
            let tail_cap = u_arg("tail_blocks")
                .map(|n| (n as usize).clamp(1, 100))
                .unwrap_or(10);
            let max_blocks = if blk_id.is_some() {
                None
            } else if pinned_only || failed_only {
                Some(500)
            } else {
                Some(tail_cap)
            };
            let req = TermReadRequest {
                session_id: sid,
                block_id: blk_id,
                line_start: u_arg("line_start").map(|n| n as usize),
                line_end: u_arg("line_end").map(|n| n as usize),
                max_blocks,
            };
            match bridge.read(req).await {
                Ok(mut r) => {
                    if u_arg("after_seq").is_some_and(|fl| r.seq <= fl) {
                        r.blocks.clear();
                    }
                    if pinned_only {
                        r.blocks.retain(|b| b.pinned);
                    }
                    if failed_only {
                        r.blocks.retain(|b| {
                            b.status == "failed" || b.exit_code.is_some_and(|c| c != 0)
                        });
                    }
                    if blk_id.is_none() && r.blocks.len() > tail_cap {
                        r.blocks.drain(0..(r.blocks.len() - tail_cap));
                    }
                    Ok(Some(ToolResult::ok(scrub_r(r)).to_json()))
                }
                Err(e) => err_j(scrub_terminal_mcp_str(&e.to_string())),
            }
        }
        "vox_terminal_send_input" => {
            let Some(sid) = s_arg("session_id") else {
                return err_j("Missing required 'session_id' argument".into());
            };
            let Some(data) = args
                .get("data")
                .or_else(|| args.get("text"))
                .and_then(Value::as_str)
            else {
                return err_j("Missing required 'data' argument".into());
            };
            let ex_bid = u_arg("block_id");
            let bridge = state.resolve_terminal_bridge().await;
            let snap_req = TermReadRequest {
                session_id: sid.into(),
                block_id: ex_bid,
                max_blocks: Some(20),
                ..Default::default()
            };
            let snap = match bridge.read(snap_req).await {
                Ok(s) => s,
                Err(e) => return err_j(scrub_terminal_mcp_str(&e.to_string())),
            };
            let target = match ex_bid {
                Some(bid) => snap.blocks.iter().find(|b| b.id == bid),
                None => snap
                    .blocks
                    .iter()
                    .rev()
                    .find(|b| matches!(b.status.as_str(), "awaiting_input" | "running"))
                    .or_else(|| snap.blocks.last()),
            };
            let Some(t) = target else {
                return err_j("No active terminal block found in session".into());
            };
            if t.credential_prompt {
                return err_j(
                    "CREDENTIAL_PROMPT_USER_ONLY: credential prompts cannot be answered by agents"
                        .into(),
                );
            }
            let list = match bridge.list_sessions().await {
                Ok(xs) => xs,
                Err(e) => return err_j(scrub_terminal_mcp_str(&e.to_string())),
            };
            let (mode, tier) = session_mode_and_tier(&list, sid);
            let mut hitl_approved = outer_hitl_approved;
            if !hitl_approved && mode == "manual" {
                let meta = PendingApprovalMeta {
                    floor_category: None,
                    session_id: Some(sid.into()),
                    command: None,
                    control_mode: Some(mode.into()),
                    credential_prompt: Some(t.credential_prompt),
                };
                if let Err(e) = park_terminal_hitl(
                    state,
                    "vox_terminal_send_input",
                    format!("[terminal:manual] send_input to {sid}"),
                    meta,
                    tier,
                    s_arg("run_id").map(Into::into),
                )
                .await
                {
                    return err_j(e);
                }
                hitl_approved = true;
            }
            let bid = t.id;
            let send_req = TermSendInputRequest {
                session_id: sid.into(),
                block_id: bid,
                text: data.into(),
                append_newline: b_arg("append_newline", true),
                hitl_approved,
            };
            if let Err(e) = bridge.send_input(send_req).await {
                return err_j(scrub_terminal_mcp_str(&e.to_string()));
            }
            let wait_ms = u_arg("wait_ms").unwrap_or(5_000).min(120_000);
            let started = std::time::Instant::now();
            let rd_req = TermReadRequest {
                session_id: sid.into(),
                block_id: Some(bid),
                max_blocks: Some(1),
                ..Default::default()
            };
            loop {
                match bridge.read(rd_req.clone()).await {
                    Ok(u) => {
                        let running = u.blocks.first().is_some_and(|b| b.status == "running");
                        if !running
                            || wait_ms == 0
                            || started.elapsed() >= Duration::from_millis(wait_ms)
                        {
                            return Ok(Some(ToolResult::ok(scrub_r(u)).to_json()));
                        }
                    }
                    Err(_) => {
                        return Ok(Some(
                            ToolResult::ok(serde_json::json!({
                                "session_id": sid,
                                "block_id": bid,
                                "delivered": true
                            }))
                            .to_json(),
                        ));
                    }
                }
                // drift-allow(timeout-literal): post-input block settle poll cadence
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
        "vox_terminal_sessions" => {
            match state.resolve_terminal_bridge().await.list_sessions().await {
                Ok(mut xs) => {
                    for s in &mut xs {
                        s.cwd = s.cwd.as_deref().map(scrub_terminal_mcp_str);
                    }
                    Ok(Some(ToolResult::ok(xs).to_json()))
                }
                Err(e) => err_j(scrub_terminal_mcp_str(&e.to_string())),
            }
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
#[path = "terminal_tools_tests.rs"]
mod tests;
