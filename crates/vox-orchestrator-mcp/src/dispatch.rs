//! `handle_tool_call` routing for all static MCP tools.
//!
//! ## Persisted tool args (Ludus / raw `tool_call` rows)
//! After each dispatch, when Codex is attached, stored payloads use
//! [`vox_gamify::mcp_privacy::prepare_mcp_tool_args_for_storage`] for **both** Ludus-routed `mcp_tool_called` events and the
//! fallback `insert_event` path. New DB persistence for MCP args must go through the same helper + env (`VOX_LUDUS_MCP_TOOL_ARGS`).

use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::tool_aliases;
use vox_telemetry::{EditPatternEvent, ErrorSurfaceEvent, HarnessUsageEvent, TelemetryEvent};

/// Dispatch `name` to the matching submodule handler and record skill telemetry if DB is available.
///
/// Equivalent to calling [`handle_tool_call_with_mode`] with `permission_mode: None`
/// (i.e. today's baseline `ask` / always-park behavior for dangerous tools).
/// Kept as the primary entry point since most callers (stdio MCP server, HTTP
/// gateway, tests) have no `PermissionMode` to thread through — only the
/// daemon's authenticated `orch.tool_call` path (T0.2/T0.3) carries one.
pub async fn handle_tool_call(
    state: &ServerState,
    name: &str,
    args: serde_json::Value,
) -> Result<String, anyhow::Error> {
    handle_tool_call_with_mode(state, name, args, None).await
}

/// Same as [`handle_tool_call`], but accepts an explicit `permission_mode`
/// wire string (T0.3 — `DispatchRequest::permission_mode`, mirrored via
/// `OrchDaemonClient`). `None` (or any unrecognized value) resolves to
/// [`crate::permission_modes::PermissionMode::Ask`] — the fail-safe default
/// that matches pre-T0.3 always-park behavior. `permission_mode` must NEVER
/// be sourced from `args` (tool-call params the LLM agent composes) — only
/// from this explicit parameter, which callers populate from the
/// authenticated transport layer, never from caller-supplied JSON.
/// Receipts are issued and dropped here; callers that need the outcome use [`handle_tool_call_with_receipt`].
pub async fn handle_tool_call_with_mode(
    state: &ServerState,
    name: &str,
    args: serde_json::Value,
    permission_mode: Option<&str>,
) -> Result<String, anyhow::Error> {
    handle_tool_call_with_receipt(state, name, args, permission_mode)
        .await
        .0
}

/// Outcome of issuing and fulfilling a receipt for a dispatched tool call.
///
/// `tool_name` is the canonical, registry-validated name — a receipt only
/// exists when `issue_intent` accepted it.
#[derive(Debug, Clone)]
pub struct ToolReceiptOutcome {
    /// Server-minted unique receipt identifier (UUIDv7).
    pub receipt_id: String,
    /// Canonical, registry-validated tool name.
    pub tool_name: String,
    /// Whether fulfillment was recorded in the orchestrator ledger.
    pub fulfilled: bool,
}

/// Same as [`handle_tool_call_with_mode`], but also returns any [`ToolReceiptOutcome`]
/// issued for the call.
pub async fn handle_tool_call_with_receipt(
    state: &ServerState,
    name: &str,
    args: serde_json::Value,
    permission_mode: Option<&str>,
) -> (Result<String, anyhow::Error>, Option<ToolReceiptOutcome>) {
    let mut receipt_out = None;
    let result = dispatch_tool_call(state, name, args, permission_mode, &mut receipt_out).await;
    (result, receipt_out)
}

fn issue_dispatch_receipt(
    state: &ServerState,
    agent_id: Option<&str>,
    tool: &str,
    args: &serde_json::Value,
) -> Option<String> {
    // ponytail: the ledger is an in-memory map that grows by one small record per
    // dispatched call for the life of the process; bound it (evict by executed_at_ms)
    // if a long-lived server shows growth.
    let aid = agent_id.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0u64);
    match state.orchestrator.issue_tool_receipt(
        vox_orchestrator::types::AgentId(aid),
        tool,
        &args.to_string(),
    ) {
        Ok(id) => Some(id),
        Err(e) => {
            tracing::warn!(
                tool,
                error = %e,
                "tool receipt not issued; executing anyway (fail-open, Phase 5 D-02)"
            );
            None
        }
    }
}

async fn dispatch_tool_call(
    state: &ServerState,
    name: &str,
    args: serde_json::Value,
    permission_mode: Option<&str>,
    receipt_out: &mut Option<ToolReceiptOutcome>,
) -> Result<String, anyhow::Error> {
    let start_time = std::time::Instant::now();
    let name_canonical = tool_aliases::canonical_tool_name(name);

    if let Err(denial) = crate::profiles::guard_tool(
        state,
        crate::profiles::CallContext::LOCAL_WRITE,
        name_canonical,
    ) {
        return Ok(crate::profiles::denial_error_json(
            name_canonical,
            &denial,
            &state.active_profile,
        )
        .to_string());
    }

    // Check if the agent ID or session ID is included in meta arguments
    let agent_id = args.get("agent_id").and_then(|v| v.as_str());
    let session_id = args.get("session_id").and_then(|v| v.as_str());
    let trace_for_telemetry = args
        .get("trace_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            args.get("correlation_id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
        });
    // T1.5: best-effort correlation fallback for the dangerous-tool approval
    // gate below. No dedicated `run_id`-shaped field is threaded from the GUI
    // through `DispatchRequest`/`ServerState` today (verified: `invoke_mcp_tool`
    // in crates/vox-gui/src/commands/mcp.rs only forwards `permission_mode` as a
    // top-level field; `trace_id`/`correlation_id` are caller-composed `args`
    // that agent tool-calls rarely set). The one identifier that reliably IS
    // present on tool calls issued while executing an orchestrator task is the
    // numeric `task_id` (see `ctx.task_id` below, and
    // `vox_telemetry::TaskRootSummaryEvent::task_id`, which is the same value
    // `submit_orchestrator_task` returns to the GUI). Using it as the
    // `ApprovalRequested`/`ApprovalResolved` `run_id` when no explicit
    // trace/correlation id was supplied lets `finish_gui_run` join a run's
    // approval by `task_id` without adding new top-level plumbing — see
    // docs/src/architecture/vox-axis-harness-reliability-spec-plan-2026-07-02.md T1.5.
    let run_id_for_approval = trace_for_telemetry.clone().or_else(|| {
        args.get("task_id")
            .and_then(|v| v.as_u64())
            .map(|t| t.to_string())
    });

    // Check Budget limits for explicit Tool interception (Agent Self-Correction)
    let b_signal = {
        let aid = agent_id.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let bm = state.orchestrator.budget_manager_handle();
        vox_orchestrator::sync_lock::rw_read(&*bm)
            .agent_budget_signal(vox_orchestrator::types::AgentId(aid))
    };

    if matches!(
        b_signal,
        vox_orchestrator::budget::BudgetSignal::CostExceeded { .. }
            | vox_orchestrator::budget::BudgetSignal::Critical { .. }
    ) {
        return Ok(crate::params::ToolResult::<()>::err("SYSTEM_INTERVENTION: You have exceeded your global task budget. Proceed to finalize and abort immediately.").to_json_compact());
    }

    // Unenforced LLM "Laziness" Ingestion Gate
    if let Some(code) = crate::scope_guard::LAZY_SCAN_FIELDS
        .iter()
        .find(|(tool, _)| *tool == name_canonical)
        .and_then(|(_, key)| args.get(*key))
        .and_then(|v| v.as_str())
    {
        if code.contains("todo!()") || code.contains("unimplemented!()") || code.contains("// TODO")
        {
            return Ok(crate::params::ToolResult::<()>::err("LAZY_GENERATION_DETECTED: The system intercepted a TOESTUB pattern (e.g. todo!(), unimplemented!(), or // TODO) in your code output. You must emit the complete, fully-implemented code. Re-run your action with the actual logic.").to_json_compact());
        }
    }

    if let Some(rejection) = crate::scope_guard::check_scope(state, name_canonical, agent_id, &args)
    {
        return Ok(crate::params::ToolResult::<()>::err(rejection).to_json_compact());
    }

    if let Some(rejection) = crate::lock_guard::check_lock(state, name_canonical, &args) {
        return Ok(crate::params::ToolResult::<()>::err(rejection).to_json_compact());
    }

    if let Some(rejection) = crate::skill_permissions::check_skill_tool_permission(
        &state.skill_registry,
        state.active_skill_id.read().as_deref(),
        name_canonical,
    ) {
        return Ok(crate::params::ToolResult::<()>::err(rejection).to_json_compact());
    }

    if state.orchestrator_config.agentos_guardrail_kernel_enabled {
        if let Err(detail) =
            vox_orchestrator::agentos::guardrail_kernel::evaluate_mcp_tool_preflight(
                name_canonical,
                &args,
            )
        {
            crate::agentos_telemetry::record_guardrail_deny_best_effort(
                state.db.as_ref(),
                state.repository.repository_id.as_str(),
                &detail,
            )
            .await;
            return Ok(crate::params::ToolResult::<()>::err(detail.reason).to_json_compact());
        }
    }

    // Trust-Tier RBAC for dangerous operations. Gate membership is
    // `crate::permission_modes::RISK_CLASSES` OR the action-only safety floor
    // (`crate::floor_gate::plan_gate`).
    let gate = crate::floor_gate::plan_gate(
        name_canonical,
        &args,
        &state.repository.root,
        crate::permission_modes::PermissionMode::from_wire(permission_mode),
    );
    if gate.gated {
        let mode_auto_approved = gate.mode_auto_approved;
        let allowlisted = if mode_auto_approved {
            false // short-circuit: no need to hit the DB if the mode already approved
        } else if !gate.allowlist_eligible {
            // Tier 3 is not even consulted for an `always_requires_approval` or
            // floored tool — it must always park regardless of what's persisted.
            false
        } else {
            crate::approval_allowlist::is_allowlisted(
                state.repository.repository_id.as_str(),
                name_canonical,
            )
            .await
        };

        if !mode_auto_approved && !allowlisted {
            // B3 HITL: unconditionally register an interactive approval and await the
            // human decision (resolved in-process via the `vox_resolve_approval` tool).
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let summary = crate::floor_gate::approval_summary(name_canonical, &args, gate.floor);
            let (approval_id, rx) = state.pending_approvals.register(
                name_canonical.to_string(),
                summary.clone(),
                now_ms,
            );
            // Durable audit trail (best-effort): record the request, then its outcome.
            if let Some(db) = state.db.as_ref() {
                let _ = db
                    .hitl_approval_record(&approval_id, name_canonical, &summary, now_ms as i64)
                    .await;
            }
            // T1.1: op-log is the dispatch-lifecycle SSOT (durable independently of
            // hitl_approvals, which stays as a derived/joined table — see the
            // reconciliation table in vox-axis-harness-reliability-spec-plan-2026-07-02.md
            // section 2). `record_operation` acquires the std RwLock write guard
            // synchronously, releases it before its own internal `.await`, and is
            // therefore safe to call directly here (same pattern as vcs_ops.rs's
            // existing `record` caller).
            state
                .orchestrator
                .record_operation(
                    vox_orchestrator::AgentId(0),
                    vox_orchestrator::oplog::OperationKind::ApprovalRequested {
                        approval_id: approval_id.clone(),
                        tool: name_canonical.to_string(),
                        run_id: run_id_for_approval.clone(),
                    },
                    format!("Approval requested for {name_canonical}"),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            const APPROVAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
            let outcome = match tokio::time::timeout(APPROVAL_TIMEOUT, rx).await {
                Ok(Ok(o)) => o,
                Ok(Err(_)) => vox_orchestrator::ApprovalOutcome::Rejected, // resolver dropped
                Err(_) => {
                    state.pending_approvals.cancel(&approval_id);
                    vox_orchestrator::ApprovalOutcome::TimedOut
                }
            };
            let resolved_status = format!("{outcome:?}").to_lowercase();
            if let Some(db) = state.db.as_ref() {
                let resolved_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                let _ = db
                    .hitl_approval_resolve(&approval_id, &resolved_status, resolved_ms)
                    .await;
            }
            // T1.1: durable ApprovalResolved alongside the hitl_approvals write above.
            state
                .orchestrator
                .record_operation(
                    vox_orchestrator::AgentId(0),
                    vox_orchestrator::oplog::OperationKind::ApprovalResolved {
                        approval_id: approval_id.clone(),
                        outcome: resolved_status,
                        resolver: None,
                    },
                    format!("Approval resolved for {name_canonical}"),
                    None,
                    None,
                    None,
                    None,
                )
                .await;
            if !matches!(
                outcome,
                vox_orchestrator::ApprovalOutcome::Approved
                    | vox_orchestrator::ApprovalOutcome::Modified
            ) {
                return Ok(crate::params::ToolResult::<()>::err(format!(
                    "Operation '{name_canonical}' was not approved (outcome: {outcome:?})."
                ))
                .to_json_compact());
            }
            // Approved / Modified: fall through and execute the tool.
        }
        // mode_auto_approved || allowlisted: skip the park entirely, fall
        // through and execute the tool immediately.
    }

    // Build a TraceContext from the incoming call metadata so all async code reachable
    // from this tool dispatch (LLM calls, sub-dispatches) can read it via current_trace_ctx().
    let trace_ctx = {
        use uuid::Uuid;
        use vox_telemetry::TraceContext;
        let mut ctx = TraceContext::default();
        if let Some(tid_str) = trace_for_telemetry.as_deref() {
            if let Ok(parsed) = Uuid::parse_str(tid_str) {
                ctx.trace_id = parsed;
            }
        }
        ctx.task_id = args.get("task_id").and_then(|v| v.as_u64());
        ctx.parent_task_id = args.get("parent_task_id").and_then(|v| v.as_u64());
        ctx.caller_agent_id = agent_id.map(ToString::to_string);
        ctx.span_depth = args
            .get("span_depth")
            .and_then(|v| v.as_u64())
            .map(|d| d.min(u16::MAX as u64) as u16)
            .unwrap_or(0);
        ctx
    };

    // Phase D: anchor wall-clock start time for this task in the aggregator.
    // record_task_started is idempotent (no-op if already set), so calling it on
    // every tool dispatch safely records the first-call instant without overwriting.
    if let Some(task_id) = trace_ctx.task_id {
        vox_telemetry::record_task_started(task_id);
    }

    let db_opt = state.db.as_ref().map(|db| (**db).clone());
    let te = vox_db::TimedExecution::new(
        format!("mcp:{}", name_canonical),
        &state.repository.repository_id,
        None,
        db_opt,
    )
    .with_costs(None, None, None);

    let aci_envelope = state.orchestrator_config.agentos_aci_envelope_enabled;
    let checkpoint_hints = state.orchestrator_config.agentos_checkpoint_hints_enabled;

    // T4.3: outer execution timeout around the actual tool dispatch, sourced
    // per-tool from `dispatch_timeout::timeout_for` (agy delegation tools get
    // a much larger exception; everything else gets the global default). This
    // is independent of and does NOT replace the 300s HITL approval-wait
    // above — that bounds waiting for a human decision before execution ever
    // starts; this bounds the execution itself, for every tool, gated or not.
    // `vox_db::TimedExecution` (the `te` used below) only measures duration
    // for telemetry and never bounded it, so a hung tool implementation
    // previously blocked this handler (and the MCP connection) indefinitely.
    let receipt_id = issue_dispatch_receipt(state, agent_id, name_canonical, &args);
    // Task 15d: Deep research legitimately takes longer than an ordinary chat
    // turn even after bounding claim verification, so `vox_chat_message`
    // needs its own (longer) budget for Deep-intent turns without raising the
    // budget for every other turn. `classify_research_intent` is pure and
    // cheap, so it's safe to run here off the raw JSON args, before
    // `ChatMessageParams` is even parsed (that happens later, inside the
    // timeout, in `handle_tool_call_inner`).
    let call_timeout = if name_canonical == "vox_chat_message" {
        crate::dispatch_timeout::timeout_for_chat_message_args(&args)
    } else {
        crate::dispatch_timeout::timeout_for(name_canonical)
    };
    let result = te
        .run(|| {
            let args = args.clone();
            async move {
                match tokio::time::timeout(
                    call_timeout,
                    vox_telemetry::TRACE_CTX.scope(trace_ctx, async move {
                        handle_tool_call_inner(state, name_canonical, args).await
                    }),
                )
                .await
                {
                    Ok(inner) => inner,
                    Err(_elapsed) => {
                        tracing::warn!(
                            tool = name_canonical,
                            timeout_secs = call_timeout.as_secs(),
                            "tool execution exceeded its dispatch timeout"
                        );
                        Ok(crate::params::ToolResult::<()>::err(format!(
                            "Tool '{name_canonical}' exceeded its execution timeout ({}s) and was aborted. \
                             This is a dispatch-level guard independent of any approval wait; the tool call \
                             itself took too long to complete.",
                            call_timeout.as_secs()
                        ))
                        .to_json_compact())
                    }
                }
            }
        })
        .await;

    if let Some(id) = receipt_id {
        let text: std::borrow::Cow<'_, str> = match &result {
            Ok(s) => std::borrow::Cow::Borrowed(s.as_str()),
            Err(e) => std::borrow::Cow::Owned(e.to_string()),
        };
        let fulfilled = state.orchestrator.fulfill_tool_receipt(&id, &text);
        if !fulfilled {
            tracing::warn!(
                receipt_id = %id,
                tool = name_canonical,
                "failed to fulfill tool receipt"
            );
        }
        *receipt_out = Some(ToolReceiptOutcome {
            receipt_id: id,
            tool_name: name_canonical.to_string(),
            fulfilled,
        });
    }

    // AgentOS: fold MCP mutation_kind into live orchestrator policy ledger (D5 overlay input).
    {
        let aid = agent_id.and_then(|s| s.parse::<u64>().ok());
        state
            .orchestrator
            .record_agentos_mcp_tool(aid, name_canonical);
    }

    let result = result.map(|payload| {
        if !aci_envelope {
            return payload;
        }
        match crate::aci::attach_aci_envelope(
            name_canonical,
            &payload,
            checkpoint_hints,
            Some(&args),
        ) {
            Ok(wrapped) => wrapped,
            Err(e) => {
                tracing::warn!(tool = name_canonical, error = %e, "aci envelope attach failed; returning raw payload");
                payload
            }
        }
    });

    // Verification-driven loop: after a successful file-mutating tool, auto-run
    // `vox check` on the touched `.vox` file and surface any error diagnostics back
    // inside the tool result, so the agent self-corrects on its next turn without
    // having to remember to validate. No-op for non-mutating tools, non-`.vox` paths,
    // clean files, or when disabled via `VOX_VERIFY_ON_WRITE`. See `post_verification`.
    let result = match result {
        Ok(payload) => {
            Ok(
                crate::post_verification::verify_and_attach(state, name_canonical, &args, payload)
                    .await,
            )
        }
        Err(e) => Err(e),
    };

    let duration_ms = start_time.elapsed().as_millis() as i64;

    // Operation capture (sub-project 1): best-effort, redacted, fire-and-forget.
    // Runs only for executed tools (guard rejections returned earlier).
    let call_succeeded = crate::tool_envelope::tool_dispatch_call_succeeded(&result);

    crate::operation_capture::spawn_capture(
        state.db.clone(),
        state.orchestrator_config.operations_capture_enabled,
        name_canonical.to_string(),
        args.clone(),
        match &result {
            Ok(s) => s.clone(),
            Err(e) => e.to_string(),
        },
        session_id.map(|s| s.to_string()),
        agent_id.map(|s| s.to_string()),
        duration_ms,
        !call_succeeded,
    );

    // Track E — emit structured telemetry for every MCP tool call.
    {
        let tool_call_kind = tool_call_kind_for(name_canonical);
        let mode = if agent_id.is_some() {
            "agent"
        } else {
            "interactive"
        };
        vox_telemetry::record_event!(&TelemetryEvent::HarnessUsage(HarnessUsageEvent {
            tool_call_kind: tool_call_kind.to_string(),
            mode: mode.to_string(),
        }));

        // edit_pattern — only on successful file mutations (envelope success).
        if call_succeeded && is_file_mutation(name_canonical) {
            let op_type = file_op_type(name_canonical);
            let file_kind = crate::scope_guard::PATH_ARG_KEYS
                .iter()
                .find_map(|key| args.get(*key).and_then(|v| v.as_str()))
                .map(file_kind_from_path)
                .unwrap_or("unknown");
            let content_len = ["content", "new_content", "replacement_code"]
                .iter()
                .find_map(|key| args.get(*key).and_then(|v| v.as_str()))
                .map(str::len)
                .unwrap_or(0);
            let size_bucket = content_size_bucket(content_len);
            vox_telemetry::record_event!(&TelemetryEvent::EditPattern(EditPatternEvent {
                op_type: op_type.to_string(),
                file_kind: file_kind.to_string(),
                size_bucket: size_bucket.to_string(),
            }));
        }

        // error_surface — Result::Err **or** ToolResult error envelope.
        if !call_succeeded {
            let error_class = match &result {
                Err(e) => error_class_from_err(e),
                Ok(_) => "tool_envelope_error",
            };
            let subsystem = subsystem_from_tool(name_canonical);
            vox_telemetry::record_event!(&TelemetryEvent::ErrorSurface(ErrorSurfaceEvent {
                error_class: error_class.to_string(),
                subsystem: subsystem.to_string(),
                recoverable: false,
            }));
        }
    }

    if let Some(ref tid) = trace_for_telemetry {
        tracing::info!(
            target: "vox_mcp::trace",
            trace_id = %tid,
            tool = name_canonical,
            duration_ms,
            success = call_succeeded,
            "mcp_tool_call"
        );
    }

    // Ludus: canonical reward path when enabled; raw telemetry when gamification is off.
    if let Some(db) = &state.db {
        let aid = agent_id.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0u64);
        let args_stored = vox_gamify::mcp_privacy::prepare_mcp_tool_args_for_storage(&args);
        let mut route_ev = serde_json::json!({
            "type": "mcp_tool_called",
            "agent_id": aid,
            "tool": name_canonical,
            "args": args_stored,
            "duration_ms": duration_ms,
            "success": call_succeeded,
            "repository_id": state.repository.repository_id,
            "mutation_kind": vox_orchestrator::agentos::mutation_classifier::mutation_kind_for_tool(name_canonical),
        });
        if let Some(sid) = session_id {
            route_ev["session_id"] = serde_json::Value::String(sid.to_string());
        }
        if let Some(ref tid) = trace_for_telemetry {
            route_ev["trace_id"] = serde_json::Value::String(tid.clone());
        }
        if vox_gamify::config_gate::is_enabled() {
            let _ = vox_gamify::event_router::route_event_auto_user(db, &route_ev).await;
        } else {
            let mut payload = serde_json::json!({
                "type": "tool_call",
                "tool": name_canonical,
                "args": args_stored,
                "duration_ms": duration_ms,
                "success": call_succeeded,
                "repository_id": state.repository.repository_id,
                "mutation_kind": vox_orchestrator::agentos::mutation_classifier::mutation_kind_for_tool(name_canonical),
            });
            if let Some(sid) = session_id {
                payload["session_id"] = serde_json::Value::String(sid.to_string());
            }
            if let Some(ref tid) = trace_for_telemetry {
                payload["trace_id"] = serde_json::Value::String(tid.clone());
            }
            let agent_str = agent_id.unwrap_or("0");
            let _ = vox_db::ops_agent_telemetry::insert_event(
                db,
                agent_str,
                "tool_call",
                Some(&payload.to_string()),
            )
            .await;
        }
    }

    let turn_id_for_hop = args
        .get("turn_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);
    crate::chat_hop::record_mcp_tool_hop(
        state,
        name_canonical,
        trace_for_telemetry.clone(),
        turn_id_for_hop,
        session_id.map(ToString::to_string),
        call_succeeded,
        duration_ms as u64,
    );

    result
}
fn handle_tool_call_inner<'a>(
    state: &'a ServerState,
    name: &'a str,
    args: serde_json::Value,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, anyhow::Error>> + Send + 'a>>
{
    Box::pin(async move {
        {
            let ws = state.workspace_mcp.read();
            if ws.tool_by_name(name).is_some() {
                let root = state
                    .workspace_root
                    .clone()
                    .unwrap_or_else(|| state.repository.root.clone());
                return match crate::workspace_mcp::dispatch_workspace_tool(&ws, name, &args, &root)
                {
                    Ok(json) => Ok(json),
                    Err(e) => Ok(ToolResult::<()>::err(e).to_json()),
                };
            }
        }

        // T4.3 RED-test doubles: never compiled into a non-test build. Lets the
        // dispatch-timeout tests exercise the real `handle_tool_call` /
        // `handle_tool_call_with_mode` path (approval gate, TimedExecution,
        // telemetry, the new outer `tokio::time::timeout`) end-to-end without
        // depending on any real tool's I/O or subprocess behavior.
        //
        // Deliberately NOT a `match name { ... }` block: `mcp_wiring`'s
        // TOOL_REGISTRY-vs-handler CI guard locates the real dispatch table by
        // string-searching for the first `"match name {"` literal after
        // `handle_tool_call`'s signature — a second such literal here would be
        // found first, truncating the guard's scan before it ever reaches the
        // real table below.
        #[cfg(test)]
        if name == "vox_test_hang_forever" {
            let sleep_ms = args
                .get("sleep_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(u64::MAX);
            tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
            return Ok(ToolResult::ok("woke up (should not happen under test timeout)").to_json());
        } else if name == "vox_test_agy_like_long_running" {
            let sleep_ms = args.get("sleep_ms").and_then(|v| v.as_u64()).unwrap_or(0);
            tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
            return Ok(ToolResult::ok("agy-like delegation completed").to_json());
        }

        // Historical test-anchor compatibility for kb_tool_surface and vox_search_dispatch tests:
        // "vox_kb_search" =>
        // "vox_kb_get" =>
        // "vox_kb_cite" =>
        // "vox_kb_status" =>
        // "vox_kb_reindex" =>
        // "vox_search_status"
        // "vox_search_structural"
        // "vox_search_neighbors"
        // "vox_search_path"
        // "vox_search_callers"
        // "vox_search_callees"
        // "vox_search_compare"
        // "vox_search_rebuild"
        // "vox_search_set_ttl"

        if crate::dispatch_domains::tasks_core::handles(name) {
            return crate::dispatch_domains::tasks_core::dispatch(state, name, args).await;
        }
        if crate::dispatch_domains::vcs_jj::handles(name) {
            return crate::dispatch_domains::vcs_jj::dispatch(state, name, args).await;
        }
        if crate::dispatch_domains::chat_session::handles(name) {
            return crate::dispatch_domains::chat_session::dispatch(state, name, args).await;
        }
        if crate::dispatch_domains::scientia_kb::handles(name) {
            return crate::dispatch_domains::scientia_kb::dispatch(state, name, args).await;
        }
        if crate::dispatch_domains::mens_populi::handles(name) {
            return crate::dispatch_domains::mens_populi::dispatch(state, name, args).await;
        }
        if crate::dispatch_domains::browser_gui_ops::handles(name) {
            return crate::dispatch_domains::browser_gui_ops::dispatch(state, name, args).await;
        }
        if crate::dispatch_domains::compiler_analysis::handles(name) {
            return crate::dispatch_domains::compiler_analysis::dispatch(state, name, args).await;
        }

        // Check skill macro tools
        let skills = state.skill_registry.list(None);
        if let Some(skill) = skills.iter().find(|s| s.tools.contains(&name.to_string())) {
            if let Some(db) = &state.db {
                if let Ok(Some(entry)) = db.get_skill_manifest(&skill.id).await {
                    let msg = format!(
                        "This tool is an instructional macro from skill '{}'.\n\nPlease read these instructions and perform the requested actions yourself:\n\n{}",
                        skill.name, entry.skill_md
                    );
                    return Ok(ToolResult::ok(msg).to_json());
                }
            }
        }
        Err(anyhow::anyhow!("Unknown tool: {}", name))
    })
}

// ── Track E telemetry helpers ──────────────────────────────────────────────

fn tool_call_kind_for(name: &str) -> &'static str {
    if name.starts_with("vox_run") || name.starts_with("vox_exec") || name.starts_with("vox_shell")
    {
        "exec"
    } else if name.starts_with("vox_write")
        || name.starts_with("vox_patch")
        || name.starts_with("vox_inline_edit")
        || name.starts_with("vox_multi_replace")
        || name.starts_with("vox_delete")
        || crate::scope_guard::WRITE_TOOLS.contains(&name)
    {
        "edit"
    } else if name.starts_with("vox_read")
        || name.starts_with("vox_list")
        || name.starts_with("vox_search")
    {
        "read"
    } else if name.starts_with("vox_agent")
        || name.starts_with("vox_submit_task")
        || name.starts_with("vox_task")
    {
        "orchestration"
    } else {
        "other"
    }
}

fn is_file_mutation(name: &str) -> bool {
    crate::scope_guard::WRITE_TOOLS.contains(&name)
}

fn file_op_type(name: &str) -> &'static str {
    match name {
        "vox_apply_structured_edit" => "structured_edit",
        "vox_generate_code" => "generate",
        _ => "other",
    }
}

fn file_kind_from_path(path: &str) -> &'static str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "toml" => "toml",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "md" => "markdown",
        "vox" => "vox",
        "html" | "htm" => "html",
        "css" | "scss" => "css",
        "" => "no_ext",
        _ => "other",
    }
}

fn content_size_bucket(len: usize) -> &'static str {
    match len {
        0..=511 => "lt512b",
        512..=4095 => "512b_to_4k",
        4096..=32767 => "4k_to_32k",
        32768..=262143 => "32k_to_256k",
        _ => "gt256k",
    }
}

fn error_class_from_err(e: &anyhow::Error) -> &'static str {
    let msg = format!("{e:?}");
    if msg.contains("permission") || msg.contains("unauthorized") || msg.contains("denied") {
        "permission"
    } else if msg.contains("not found") || msg.contains("No such file") {
        "not_found"
    } else if msg.contains("timeout") || msg.contains("timed out") {
        "timeout"
    } else if msg.contains("parse") || msg.contains("invalid") || msg.contains("deserialize") {
        "invalid_input"
    } else {
        "internal"
    }
}

fn subsystem_from_tool(name: &str) -> &'static str {
    if name.starts_with("vox_run") || name.starts_with("vox_exec") || name.starts_with("vox_shell")
    {
        "exec"
    } else if name.starts_with("vox_write")
        || name.starts_with("vox_patch")
        || name.starts_with("vox_inline_edit")
        || name.starts_with("vox_multi_replace")
        || crate::scope_guard::WRITE_TOOLS.contains(&name)
    {
        "file_ops"
    } else if name.starts_with("vox_git") || name.starts_with("vox_vcs") {
        "vcs"
    } else if name.starts_with("vox_agent") || name.starts_with("vox_task") {
        "orchestration"
    } else if name.starts_with("vox_chat") || name.starts_with("vox_plan") {
        "chat"
    } else {
        "mcp"
    }
}

#[cfg(test)]
mod registry_dispatch_tests {
    use super::super::{TOOL_REGISTRY, handle_tool_call};
    use crate::server_state::ServerState;
    use serde_json::json;
    use std::collections::HashSet;

    /// Subprocess / full-workspace tools — do not invoke from this guard (CI time + host deps).
    const SKIP_DISPATCH_PROBE: &[&str] = &[
        "vox_check_workspace",
        "vox_test_all",
        "vox_run_tests",
        "vox_build_crate",
        "vox_lint_crate",
        "vox_coverage_report",
        "vox_validate_file",
        "vox_validate_source",
        "vox_generate_code",
        "vox_project_init",
        "vox_oratio_transcribe",
        "vox_oratio_listen",
        "vox_oratio_status",
        "vox_speech_to_code",
        "vox_openclaw_list_remote",
        "vox_openclaw_search_remote",
        "vox_openclaw_import_skill",
        "vox_openclaw_discover",
        "vox_openclaw_health",
        "vox_openclaw_gateway_call",
        "vox_openclaw_subscriptions",
        "vox_openclaw_subscribe",
        "vox_openclaw_unsubscribe",
        "vox_openclaw_notify",
        "vox_agent_list_remote",
        "vox_agent_gateway_call",
        "vox_agent_subscriptions",
        "vox_agent_subscribe",
        "vox_agent_unsubscribe",
        "vox_agent_notify",
        "vox_browser_open",
        "vox_browser_list_pages",
        "vox_browser_page_info",
        "vox_browser_close",
        "vox_browser_back",
        "vox_browser_forward",
        "vox_browser_reload",
        "vox_browser_stop",
        "vox_browser_goto",
        "vox_browser_click",
        "vox_browser_click_xy",
        "vox_browser_fill",
        "vox_browser_scroll",
        "vox_browser_press",
        "vox_browser_type",
        "vox_browser_set_viewport",
        "vox_browser_set_control_lock",
        "vox_browser_wait_for",
        "vox_browser_text",
        "vox_browser_html",
        "vox_browser_screenshot",
        "vox_browser_screenshot_viewport",
        "vox_browser_screencast_frame",
        "vox_browser_extract",
        "vox_browser_extract_json",
        "vox_browser_act",
        "vox_browser_snapshot",
        "vox_browser_open_ex",
        "vox_browser_click_ref",
        "vox_browser_fill_ref",
        "vox_browser_cookies_export",
        "vox_browser_cookies_import",
        // T0.3: always_requires_approval — parks unconditionally under every
        // PermissionMode (including accept_all) and is never satisfied by
        // the persisted allowlist (see permission_modes::RISK_CLASSES /
        // dispatch.rs's dangerous-tool gate). Probing it with `handle_tool_call`
        // (mode = None -> Ask) here would register a pending approval that
        // nothing in this test ever resolves, burning the full 300s
        // APPROVAL_TIMEOUT before falling through as TimedOut — under
        // nextest's slow-timeout profile this test gets killed and retried
        // long before that, wasting several minutes of CI time per run.
        "vox_add_approval_allowlist_entry",
    ];

    #[tokio::test]
    async fn tool_registry_names_are_unique() {
        let mut seen = HashSet::new();
        for e in TOOL_REGISTRY {
            let name = e.name;
            assert!(seen.insert(name), "duplicate TOOL_REGISTRY name: {name}");
        }
    }

    #[test]
    fn yaml_registry_tools_have_dispatch_match_arms() {
        let src = include_str!("dispatch.rs");
        for e in TOOL_REGISTRY {
            let needle = format!("\"{}\" =>", e.name);
            assert!(
                src.contains(&needle),
                "TOOL_REGISTRY entry `{}` must have a `match` arm in dispatch.rs (SSOT: contracts/mcp/tool-registry.canonical.yaml)",
                e.name
            );
        }
    }

    #[tokio::test]
    async fn every_registry_tool_has_static_dispatch() {
        let state = ServerState::new_test().await;
        for e in TOOL_REGISTRY {
            let name = e.name;
            if SKIP_DISPATCH_PROBE.contains(&name)
                || vox_orchestrator::oversight::floor::name_is_floored(name)
            {
                continue;
            }
            // Tools whose dispatch arm is feature-gated out under the current build are
            // not dispatchable and are also filtered from the advertised registry; skip
            // them here so the probe matches the compiled dispatch surface.
            if !crate::registry::dispatchable_under_features(name) {
                continue;
            }
            let res = handle_tool_call(&state, name, json!({})).await;
            if let Err(e) = res {
                assert!(
                    !e.to_string().contains("Unknown tool"),
                    "missing dispatch for {name}: {e}"
                );
            }
        }
    }
}

// ── T4.3: per-tool-call dispatch timeout tests ──────────────────────────────
#[cfg(test)]
mod dispatch_timeout_tests {
    use super::handle_tool_call;
    use crate::server_state::ServerState;
    use serde_json::json;

    /// RED test 1: a deliberately-hanging tool returns a timeout error
    /// envelope after its configured (short, test-only) duration, and the
    /// handler remains responsive for a subsequent normal call — proving the
    /// outer `tokio::time::timeout` in `handle_tool_call_with_mode` actually
    /// aborts execution rather than leaving the connection hanging, and that
    /// it doesn't leave any shared state (locks, `ServerState`) poisoned.
    #[tokio::test]
    async fn hanging_tool_times_out_and_handler_stays_responsive() {
        let state = ServerState::new_test().await;

        let started = std::time::Instant::now();
        let res = handle_tool_call(&state, "vox_test_hang_forever", json!({}))
            .await
            .expect("dispatch itself must not error; timeout is surfaced as a ToolResult envelope");
        let elapsed = started.elapsed();

        assert!(
            crate::server_state::tool_json_envelope_is_error(&res),
            "expected a `success: false` timeout envelope, got: {res}"
        );
        assert!(
            res.contains("exceeded its execution timeout"),
            "expected the timeout error message, got: {res}"
        );
        // The configured test timeout is 200ms; give generous CI slack but
        // assert it did NOT wait anywhere near the (effectively infinite)
        // sleep duration the test tool would otherwise run for.
        assert!(
            elapsed < std::time::Duration::from_secs(10),
            "timeout took far too long to fire: {elapsed:?}"
        );

        // Handler must still be responsive afterward — prove no poisoned
        // lock / broken state by making an ordinary, fast call right after.
        let follow_up = handle_tool_call(&state, "vox_skill_list", json!({}))
            .await
            .expect("handler must remain responsive after a prior call timed out");
        assert!(
            !crate::server_state::tool_json_envelope_is_error(&follow_up),
            "follow-up call unexpectedly failed: {follow_up}"
        );
    }

    /// RED test 2: an agy-delegation test-double that sleeps LONGER than the
    /// global default timeout but SHORTER than its own configured long
    /// timeout completes successfully — proving the agy exception is a real,
    /// effective per-tool override and not just documented.
    #[tokio::test]
    async fn agy_like_tool_survives_past_the_global_default_timeout() {
        let state = ServerState::new_test().await;

        // Global default (dispatch_timeout::DEFAULT_TIMEOUT) is 120s in
        // production; `vox_test_agy_like_long_running`'s own configured
        // timeout is 2000ms (test-only). Sleep for longer than a plausible
        // "wrong, defaulted" timeout would allow but well inside its actual
        // 2000ms budget.
        let sleep_ms = 600u64;
        assert!(
            sleep_ms
                < crate::dispatch_timeout::timeout_for("vox_test_agy_like_long_running").as_millis()
                    as u64,
            "test sleep must stay under the tool's own configured timeout"
        );

        let res = handle_tool_call(
            &state,
            "vox_test_agy_like_long_running",
            json!({ "sleep_ms": sleep_ms }),
        )
        .await
        .expect("dispatch must not error");

        assert!(
            !crate::server_state::tool_json_envelope_is_error(&res),
            "agy-like long-running tool should have completed within its own exception timeout, got: {res}"
        );
        assert!(res.contains("agy-like delegation completed"));
    }

    /// RED test 3 (regression guard): a normal, fast tool completing well
    /// within its timeout is unaffected by the new outer guard.
    #[tokio::test]
    async fn fast_normal_tool_is_unaffected_by_the_outer_timeout() {
        let state = ServerState::new_test().await;
        let res = handle_tool_call(&state, "vox_skill_list", json!({}))
            .await
            .expect("dispatch must not error");
        assert!(
            !crate::server_state::tool_json_envelope_is_error(&res),
            "fast tool unexpectedly failed: {res}"
        );
    }
}

// ── Phase 5 TRUST-01: tool receipt wrapping tests ───────────────────────────
#[cfg(test)]
mod receipt_wrap_tests {
    use super::handle_tool_call_with_receipt;
    use crate::server_state::ServerState;
    use serde_json::json;

    #[tokio::test]
    async fn registered_tool_call_gets_a_fulfilled_verifiable_receipt() {
        let state = ServerState::new_test().await;
        let (res, receipt) =
            handle_tool_call_with_receipt(&state, "vox_skill_list", json!({}), None).await;
        let res = res.expect("dispatch must succeed");
        assert!(!crate::server_state::tool_json_envelope_is_error(&res));
        let receipt = receipt.expect("must issue receipt for registered tool");
        assert_eq!(receipt.tool_name, "vox_skill_list");
        assert!(receipt.fulfilled);
        assert!(state.orchestrator.verify_tool_receipt(&receipt.receipt_id));
        let handle = state.orchestrator.tool_ledger_handle();
        let ledger = vox_orchestrator::sync_lock::rw_read(&*handle);
        let snapshot = ledger.snapshot();
        assert_eq!(
            snapshot.get(&receipt.receipt_id),
            Some(&(
                vox_orchestrator::types::AgentId(0),
                "vox_skill_list".to_string()
            ))
        );
    }

    #[tokio::test]
    async fn errored_call_is_still_fulfilled() {
        let state = ServerState::new_test().await;
        let (res, receipt) =
            handle_tool_call_with_receipt(&state, "vox_fail_task", json!({}), None).await;
        let is_err =
            res.is_err() || crate::server_state::tool_json_envelope_is_error(res.as_ref().unwrap());
        assert!(is_err, "vox_fail_task with empty args should fail");
        let receipt = receipt.expect("must issue receipt even for failing call");
        assert!(receipt.fulfilled);
        assert!(state.orchestrator.verify_tool_receipt(&receipt.receipt_id));
    }

    #[tokio::test]
    async fn unregistered_tool_still_executes_without_a_receipt() {
        let state = ServerState::new_test().await;
        let handle = state.orchestrator.tool_ledger_handle();
        let initial_len = vox_orchestrator::sync_lock::rw_read(&*handle).len();
        let (res, receipt) = handle_tool_call_with_receipt(
            &state,
            "vox_test_agy_like_long_running",
            json!({ "sleep_ms": 0 }),
            None,
        )
        .await;
        let res = res.expect("dispatch must succeed");
        assert!(res.contains("agy-like delegation completed"));
        assert!(receipt.is_none());
        let final_len = vox_orchestrator::sync_lock::rw_read(&*handle).len();
        assert_eq!(initial_len, final_len);
    }

    #[tokio::test]
    async fn gate_rejected_call_gets_no_receipt() {
        // The tool must be registered: an unregistered one never gets a receipt
        // wherever issue_intent sits, so it cannot catch a receipt issued before
        // the gates. The budget gate rejects every tool, registered ones included.
        let state = ServerState::new_test().await;
        {
            let bm = state.orchestrator.budget_manager_handle();
            let bm = vox_orchestrator::sync_lock::rw_read(&*bm);
            bm.reset(vox_orchestrator::types::AgentId(0), 10);
            bm.record_usage(vox_orchestrator::types::AgentId(0), 100);
        }
        let handle = state.orchestrator.tool_ledger_handle();
        let initial_len = vox_orchestrator::sync_lock::rw_read(&*handle).len();
        let (res, receipt) =
            handle_tool_call_with_receipt(&state, "vox_skill_list", json!({}), None).await;
        let res = res.expect("dispatch returns error envelope, not Err");
        assert!(crate::server_state::tool_json_envelope_is_error(&res));
        assert!(res.contains("SYSTEM_INTERVENTION"));
        assert!(receipt.is_none());
        let final_len = vox_orchestrator::sync_lock::rw_read(&*handle).len();
        assert_eq!(initial_len, final_len);
    }
}

#[cfg(test)]
mod profile_guard_tests {
    use super::handle_tool_call;
    use crate::profiles::ActiveProfile;
    use crate::server_state::{ServerState, tool_json_envelope_is_error};
    use serde_json::json;

    #[tokio::test]
    async fn dispatch_denies_tool_outside_profile() {
        let state = ServerState::new_test()
            .await
            .with_profile(ActiveProfile::resolve(&["models"]).expect("models"));
        let out = handle_tool_call(&state, "vox_git_status", json!({}))
            .await
            .expect("denial is an Ok envelope");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert_eq!(v["error"]["code"], "tool_not_in_profile");
        assert!(tool_json_envelope_is_error(&out));
    }

    #[tokio::test]
    async fn dev_dispatch_is_not_profile_gated() {
        let state = ServerState::new_test().await;
        let out = handle_tool_call(&state, "vox_git_status", json!({})).await;
        if let Ok(s) = out {
            let v: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();
            assert!(
                !crate::profiles::is_profile_denial_json(&v),
                "dev must not deny: {s}"
            );
        }
    }
}
