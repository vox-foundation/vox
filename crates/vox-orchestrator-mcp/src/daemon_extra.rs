//! Daemon [`ExtraDispatch`] impl (B5 path-c + B3 cross-process).
//!
//! Serves `orch.tool_call` / `orch.list_pending_approvals` / `orch.resolve_approval`
//! against the daemon's own MCP [`ServerState`], so callers (the GUI, peers) run
//! tools and resolve HITL approvals through the one shared orchestrator instead
//! of a second in-process one. Wired by `vox-orchestrator-d`. Lives here (not in
//! `vox-orchestrator`) to avoid a dependency cycle — the `ExtraDispatch` trait is
//! defined in `vox-orchestrator`, the heavy `ServerState` stays in this crate.

use async_trait::async_trait;

use vox_foundation::protocol::{
    DispatchPayload, DispatchRequest, DispatchResponse, dei_method, orch_daemon_method,
};
use vox_orchestrator::orch_daemon::ExtraDispatch;

use crate::profiles::Subsystem;
use crate::server_state::ServerState;
use crate::server_state::subsystem_not_booted_json;

/// [`ExtraDispatch`] backed by an MCP [`ServerState`].
pub struct McpExtraDispatch {
    state: ServerState,
}

impl McpExtraDispatch {
    // toestub-ignore(skeleton/untested-pub-api) — constructor wires ServerState; exercised via tests/daemon_extra_tests.rs
    #[must_use]
    pub fn new(state: ServerState) -> Self {
        Self { state }
    }
}

fn result(id: &str, value: serde_json::Value) -> DispatchResponse {
    DispatchResponse {
        id: id.to_string(),
        payload: DispatchPayload::Result { value },
    }
}

fn error(id: &str, message: impl Into<String>) -> DispatchResponse {
    DispatchResponse {
        id: id.to_string(),
        payload: DispatchPayload::Error {
            message: message.into(),
            code: 1,
        },
    }
}

#[async_trait]
impl ExtraDispatch for McpExtraDispatch {
    async fn try_handle(&self, req: &DispatchRequest) -> Option<DispatchResponse> {
        match req.method.as_str() {
            // Enqueue a SCIENTIA research run inside this persistent daemon. The
            // underlying MCP handler creates the session row, spawns the pipeline
            // (which advances the session through real stages to a terminal
            // `completed`/`failed`), and returns immediately — fire-and-forget.
            // This is the cross-process executor for `vox research run --async`,
            // which only inserts a `queued` row and never runs the pipeline.
            dei_method::RESEARCH_RUN => {
                if !self.state.subsystem_up(Subsystem::Research) {
                    return Some(error(
                        &req.id,
                        subsystem_not_booted_json(
                            "method",
                            dei_method::RESEARCH_RUN,
                            Subsystem::Research,
                        ),
                    ));
                }
                let params: crate::memory::ResearchStartParams =
                    match serde_json::from_value(req.params.clone()) {
                        Ok(p) => p,
                        Err(e) => {
                            return Some(error(
                                &req.id,
                                format!("invalid research.run params: {e}"),
                            ));
                        }
                    };
                // `research_start` is the async fire-and-forget executor: it
                // creates the session, spawns the pipeline (which advances to a
                // terminal `completed`/`failed`), and returns a `running`
                // envelope immediately. Returns a JSON-string `ToolResult`.
                let json = crate::memory::research_start(&self.state, params).await;
                let envelope = serde_json::from_str::<serde_json::Value>(&json)
                    .unwrap_or_else(|_| serde_json::json!({ "raw": json }));
                // On failure, surface the ToolResult error as a daemon error.
                if envelope.get("success") == Some(&serde_json::Value::Bool(false)) {
                    let msg = envelope
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("research.run failed");
                    return Some(error(&req.id, msg.to_string()));
                }
                // `research_start` nests its `{session_id, task_id, status}`
                // object as a JSON *string* under `data`; unwrap it so callers
                // get a clean object rather than a stringified envelope.
                let value = envelope
                    .get("data")
                    .and_then(|d| d.as_str())
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                    .unwrap_or(envelope);
                Some(result(&req.id, value))
            }
            dei_method::RESEARCH_PERSIST_CLAIMS => {
                if !self.state.subsystem_up(Subsystem::Db) {
                    return Some(error(
                        &req.id,
                        subsystem_not_booted_json(
                            "method",
                            dei_method::RESEARCH_PERSIST_CLAIMS,
                            Subsystem::Db,
                        ),
                    ));
                }
                let Some(db) = self.state.db.clone() else {
                    return Some(error(&req.id, "VoxDb not attached to daemon"));
                };
                let session_id = req
                    .params
                    .get("session_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let mut stored_count = 0;

                if let Some(claims_array) = req.params.get("claims").and_then(|v| v.as_array()) {
                    for item in claims_array {
                        let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        if text.is_empty() {
                            continue;
                        }
                        let mut hash: u64 = 0xcbf29ce484222325;
                        for byte in text.bytes() {
                            hash ^= byte as u64;
                            hash = hash.wrapping_mul(0x100000001b3);
                        }
                        let claim_id = item
                            .get("claim_id")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(hash);
                        let verdict = item
                            .get("verdict")
                            .and_then(|v| v.as_str())
                            .unwrap_or("supported");
                        let confidence = item
                            .get("confidence")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.9);
                        let is_numeric = item
                            .get("is_numeric")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let is_recent = item
                            .get("is_recent")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let is_named_event = item
                            .get("is_named_event")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        let _ = db
                            .store_claim(
                                session_id,
                                claim_id,
                                text,
                                is_numeric,
                                is_recent,
                                is_named_event,
                            )
                            .await;
                        let _ = db
                            .store_claim_verdict(
                                claim_id,
                                verdict,
                                confidence,
                                "orchestrator-daemon",
                            )
                            .await;
                        stored_count += 1;
                    }
                } else if session_id > 0 {
                    if let Ok(existing) = db.list_publication_claims(session_id).await {
                        stored_count = existing.len();
                    }
                }

                Some(result(
                    &req.id,
                    serde_json::json!({
                        "session_id": session_id,
                        "persisted_claims": stored_count
                    }),
                ))
            }
            orch_daemon_method::BOOT_REPORT => Some(
                match serde_json::to_value(self.state.boot_report_snapshot()) {
                    Ok(value) => result(&req.id, value),
                    Err(e) => error(&req.id, format!("serialize boot report: {e}")),
                },
            ),
            orch_daemon_method::API_V2_GET => {
                let path = req
                    .params
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Some(
                    match crate::gui_ops::inproc::api_v2_get(&self.state, path).await {
                        Ok(v) => result(&req.id, v),
                        Err(e) => error(&req.id, e),
                    },
                )
            }
            orch_daemon_method::TOOL_CALL => {
                let Some(name) = req.params.get("name").and_then(|v| v.as_str()) else {
                    return Some(error(&req.id, "params.name (string) required"));
                };
                let args = req
                    .params
                    .get("args")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                // T0.3: `permission_mode` comes from the DispatchRequest's own
                // top-level field (set only by the authenticated transport
                // layer — see `DispatchRequest::permission_mode`'s doc
                // comment), NEVER from `req.params` (tool-call args the
                // caller/LLM composes).
                let permission_mode = req.permission_mode.as_deref();
                // The `agent_origin` flag lives in `params` rather than in a top-level
                // `DispatchRequest` field: unlike permission_mode or auth_token, the flag
                // can only restrict permissions (triggering safety floors). The only potential
                // bypass is omission, which a top-level field would not prevent either.
                let agent_origin = req
                    .params
                    .get("agent_origin")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let fut =
                    crate::handle_tool_call_with_mode(&self.state, name, args, permission_mode);
                let res = if agent_origin {
                    crate::floor_gate::with_agent_origin(fut).await
                } else {
                    fut.await
                };
                match res {
                    Ok(json) => {
                        let value = serde_json::from_str::<serde_json::Value>(&json)
                            .unwrap_or_else(|_| serde_json::json!({ "raw": json }));
                        Some(result(&req.id, value))
                    }
                    Err(e) => Some(error(&req.id, format!("tool '{name}' failed: {e}"))),
                }
            }
            orch_daemon_method::LIST_PENDING_APPROVALS => Some(result(
                &req.id,
                serde_json::json!({ "approvals": self.state.pending_approvals.list() }),
            )),
            orch_daemon_method::RESOLVE_APPROVAL => {
                let approval_id = req
                    .params
                    .get("approval_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let decision = req
                    .params
                    .get("outcome")
                    .or_else(|| req.params.get("decision"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let outcome = match decision {
                    "approve" | "approved" => vox_orchestrator::ApprovalOutcome::Approved,
                    "modify" | "modified" => vox_orchestrator::ApprovalOutcome::Modified,
                    "reject" | "rejected" => vox_orchestrator::ApprovalOutcome::Rejected,
                    other => {
                        return Some(error(
                            &req.id,
                            format!(
                                "unrecognized approval decision {other:?}; expected one of approve|approved|modify|modified|reject|rejected"
                            ),
                        ));
                    }
                };
                let resolution = match crate::pending_approvals::parse_resolution_from_params(
                    outcome,
                    &req.params,
                ) {
                    Ok(r) => r,
                    Err(msg) => return Some(error(&req.id, msg)),
                };
                if let Err(msg) =
                    crate::dispatch_domains::terminal_approvals::validate_terminal_resolution_before_commit(
                        &self.state,
                        approval_id,
                        &resolution,
                    )
                    .await
                {
                    return Some(error(&req.id, msg));
                }
                let resolved = self
                    .state
                    .pending_approvals
                    .resolve_with_detail(approval_id, resolution);
                Some(result(
                    &req.id,
                    serde_json::json!({ "resolved": resolved, "approval_id": approval_id }),
                ))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::McpExtraDispatch;
    use crate::pending_approvals::{PendingApprovalMeta, PendingResolution};
    use crate::profiles::Subsystem;
    use serde_json::json;
    use vox_foundation::protocol::{DispatchPayload, DispatchRequest, orch_daemon_method};
    use vox_orchestrator::ApprovalOutcome;
    use vox_orchestrator::orch_daemon::ExtraDispatch;

    #[tokio::test]
    async fn new_wraps_a_server_state_whose_default_report_is_fully_booted() {
        let state = crate::ServerState::new_full(crate::load_config());
        let extra = McpExtraDispatch::new(state);
        assert!(extra.state.subsystem_up(Subsystem::Db));
    }

    #[tokio::test]
    async fn dual_path_resolve_approval_forwards_detail_and_validates_modified() {
        use vox_orchestrator::terminal_bridge::{EphemeralTerminalBridge, TerminalBridge};
        let schema = crate::input_schemas::tool_input_schema("vox_resolve_approval");
        assert_eq!(schema["additionalProperties"], false);
        for key in ["modified_command", "steering_note", "allow_pattern"] {
            assert_eq!(schema["properties"][key]["type"], "string");
        }

        let state = crate::ServerState::new_test().await;
        let bridge = std::sync::Arc::new(EphemeralTerminalBridge::new(None));
        let sid = bridge.open_session(None).await.expect("open ephem session");
        state.set_terminal_bridge(bridge);
        let extra = McpExtraDispatch::new(state.clone());

        let meta = PendingApprovalMeta {
            session_id: Some(sid.clone()),
            command: Some("cargo test".into()),
            control_mode: Some("co_pilot".into()),
            credential_prompt: Some(false),
            floor_category: None,
        };
        let (id_mcp, rx_mcp) = state.pending_approvals.register_with_meta(
            "vox_terminal_exec".into(),
            "[terminal:co_pilot] cargo test".into(),
            10,
            meta,
        );
        let mcp =
            |a| crate::dispatch_domains::tasks_core::dispatch(&state, "vox_resolve_approval", a);
        let bad_mcp =
            mcp(json!({ "approval_id": id_mcp, "outcome": "modified", "modified_command": "   " }))
                .await
                .unwrap();
        assert!(
            crate::server_state::tool_json_envelope_is_error(&bad_mcp)
                && state.pending_approvals.list().len() == 1
        );

        let ok_mcp = mcp(json!({ "approval_id": id_mcp, "outcome": "modified", "modified_command": "cargo test -p vox-cli", "allow_pattern": "cargo test" })).await.unwrap();
        assert!(!crate::server_state::tool_json_envelope_is_error(&ok_mcp));
        assert_eq!(
            rx_mcp.await.unwrap(),
            PendingResolution {
                outcome: ApprovalOutcome::Modified,
                modified_command: Some("cargo test -p vox-cli".into()),
                steering_note: None,
                allow_pattern: Some("cargo test".into())
            }
        );
        assert_eq!(
            state.pending_approvals.session_patterns(&sid),
            vec!["cargo test"]
        );

        let (id_rpc, rx_rpc) = state.pending_approvals.register_with_meta(
            "vox_terminal_exec".into(),
            "[terminal:co_pilot] cargo build".into(),
            20,
            PendingApprovalMeta {
                session_id: Some(sid.clone()),
                ..Default::default()
            },
        );
        let mk_req = |params: serde_json::Value| DispatchRequest {
            id: "r1".into(),
            method: orch_daemon_method::RESOLVE_APPROVAL.into(),
            params,
            auth_token: None,
            permission_mode: None,
        };
        let bad_rpc = extra
            .try_handle(&mk_req(
                json!({ "approval_id": id_rpc, "outcome": "modified" }),
            ))
            .await
            .unwrap();
        let floor_rpc = extra.try_handle(&mk_req(json!({ "approval_id": id_rpc, "outcome": "modified", "modified_command": "git push --force origin main", "allow_pattern": "cargo build" }))).await.unwrap();
        assert!(
            matches!(bad_rpc.payload, DispatchPayload::Error { .. })
                && matches!(floor_rpc.payload, DispatchPayload::Error { .. })
        );
        assert!(
            state.pending_approvals.peek(&id_rpc).is_some()
                && state.pending_approvals.session_patterns(&sid) == vec!["cargo test"]
        );

        let ok_rpc = extra.try_handle(&mk_req(json!({ "approval_id": id_rpc, "outcome": "rejected", "steering_note": "Use --no-run instead" }))).await.unwrap();
        assert!(matches!(ok_rpc.payload, DispatchPayload::Result { .. }));
        assert_eq!(
            rx_rpc.await.unwrap(),
            PendingResolution {
                outcome: ApprovalOutcome::Rejected,
                modified_command: None,
                steering_note: Some("Use --no-run instead".into()),
                allow_pattern: None
            }
        );
    }
}
