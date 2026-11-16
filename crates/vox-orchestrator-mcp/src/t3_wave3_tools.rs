//! Live MCP tool handlers for T3 Wave 2/3 subsystems (`T3-MCP-RPC-WIRE-01..03`):
//! - `vox_consensus_run` (`ConsensusFanOutEngine` + `analyze_response_quality`)
//! - `vox_context_handoff` (`ContextHandoffService::build_package`)
//! - `vox_checkpoint_capture`, `vox_checkpoint_list`, `vox_checkpoint_restore` (`GitPlumbingCheckpointStore`)
//! - `vox_muse_pool_spawn`, `vox_muse_pool_adopt` (`MultiVariantWorktreePool`)
//! - `vox_guest_share_mint`, `vox_guest_share_moderate` (`GuestShareService`)
//! - `vox_session_import_external`, `vox_session_export_bundle`, `vox_session_import_bundle` (`external_importer`)

use crate::mcp_tool_access::{InteractionAccessMode, McpToolAccessRequirement, RuntimeAccessMode};
use crate::params::ToolResult;
use crate::server_state::ServerState;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
use vox_orchestrator::agentos::checkpoint_engine::GitPlumbingCheckpointStore;
use vox_orchestrator::agentos::muse_pool::MultiVariantWorktreePool;
use vox_orchestrator::context_handoff::{
    ContextHandoffService, HandoffStrategy, HandoffTranscriptTurn,
};
use vox_orchestrator::external_importer::{
    ImportedSessionMessage, VoxSessionExportBundleV1, decode_cursor_root_blob_hex,
    parse_jsonl_external_session,
};
use vox_orchestrator::models::consensus::{ConsensusFanOutEngine, ConsensusStreamEvent};
use vox_orchestrator::models::quality_analyzer::{
    ResponseQualityMetrics, analyze_response_quality,
};

/// Return the canonical [`McpToolAccessRequirement`] for a Wave 3 tool (`T3-MCP-RPC-WIRE-01..03`).
#[must_use]
pub fn access_requirement_for_wave3_tool(tool_name: &str) -> McpToolAccessRequirement {
    match tool_name {
        "vox_checkpoint_list"
        | "vox_guest_share_list"
        | "vox_session_export_bundle"
        | "vox_session_list_external"
        | "vox_quarantine_list"
        | "vox_vcs_checkpoint_diff_summary"
        | "vox_file_diff_and_diagnostics"
        | "vox_list_mcp_servers"
        | "vox_list_mcp_tools"
        | "vox_battery_status"
        | "vox_pr_review_guide" => McpToolAccessRequirement::reads(),
        "vox_consensus_run" | "vox_muse_pool_spawn" => McpToolAccessRequirement::starts_threads(),
        "vox_checkpoint_capture" | "vox_checkpoint_restore" | "vox_muse_pool_adopt" => {
            McpToolAccessRequirement::writes_with_capability("vcs_worktree")
        }
        "vox_guest_share_mint"
        | "vox_guest_share_moderate"
        | "vox_guest_share_redeem"
        | "vox_guest_share_submit" => {
            McpToolAccessRequirement::writes_with_capability("guest_moderation")
        }
        _ => McpToolAccessRequirement::writes(),
    }
}

fn default_server_capabilities() -> Vec<String> {
    vec![
        "spawn_threads".to_string(),
        "vcs_worktree".to_string(),
        "guest_moderation".to_string(),
    ]
}

fn resolve_server_policy_from_env() -> (RuntimeAccessMode, InteractionAccessMode, Vec<String>) {
    let server_runtime_mode = match std::env::var("VOX_MCP_RUNTIME_MODE") {
        Ok(raw) if !raw.trim().is_empty() => match raw.trim() {
            "read_only" => RuntimeAccessMode::ReadOnly,
            "workspace_write" => RuntimeAccessMode::WorkspaceWrite,
            "full_access" => RuntimeAccessMode::FullAccess,
            _ => RuntimeAccessMode::ReadOnly,
        },
        _ => RuntimeAccessMode::FullAccess,
    };
    let server_interaction_mode = match std::env::var("VOX_MCP_INTERACTION_MODE") {
        Ok(raw) if !raw.trim().is_empty() => match raw.trim() {
            "chat" => InteractionAccessMode::Chat,
            "plan" => InteractionAccessMode::Plan,
            "agent" => InteractionAccessMode::Agent,
            _ => InteractionAccessMode::Chat,
        },
        _ => InteractionAccessMode::Agent,
    };
    let server_caps = match std::env::var("VOX_MCP_GRANTED_CAPABILITIES") {
        Ok(raw) => raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        Err(_) => default_server_capabilities(),
    };
    (server_runtime_mode, server_interaction_mode, server_caps)
}

fn authorize_wave3_call_with_server_policy(
    tool_name: &str,
    args: &Value,
    server_runtime_mode: RuntimeAccessMode,
    server_interaction_mode: InteractionAccessMode,
    server_caps: &[String],
) -> Result<(), String> {
    let req = access_requirement_for_wave3_tool(tool_name);
    let effective_runtime_mode = match args.get("runtime_mode").filter(|v| !v.is_null()) {
        Some(v) => {
            let caller_mode = serde_json::from_value::<RuntimeAccessMode>(v.clone())
                .map_err(|e| format!("invalid runtime_mode: {e}"))?;
            server_runtime_mode.min(caller_mode)
        }
        None => server_runtime_mode,
    };
    let effective_interaction_mode = match args.get("interaction_mode").filter(|v| !v.is_null()) {
        Some(v) => {
            let caller_mode = serde_json::from_value::<InteractionAccessMode>(v.clone())
                .map_err(|e| format!("invalid interaction_mode: {e}"))?;
            server_interaction_mode.min(caller_mode)
        }
        None => server_interaction_mode,
    };
    let effective_caps: Vec<String> =
        match args.get("granted_capabilities").filter(|v| !v.is_null()) {
            Some(v) => {
                let caller_caps = serde_json::from_value::<Vec<String>>(v.clone())
                    .map_err(|e| format!("invalid granted_capabilities: {e}"))?;
                server_caps
                    .iter()
                    .filter(|sc| caller_caps.iter().any(|cc| cc == *sc))
                    .cloned()
                    .collect()
            }
            None => server_caps.to_vec(),
        };
    req.authorize(
        effective_runtime_mode,
        effective_interaction_mode,
        &effective_caps,
    )
}

fn authorize_wave3_call(tool_name: &str, args: &Value) -> Result<(), String> {
    let (server_runtime, server_interaction, server_caps) = resolve_server_policy_from_env();
    authorize_wave3_call_with_server_policy(
        tool_name,
        args,
        server_runtime,
        server_interaction,
        &server_caps,
    )
}

fn resolve_repo_root(state: &ServerState, args: &Value) -> Result<PathBuf, String> {
    let server_root = state
        .repository
        .git_root
        .clone()
        .unwrap_or_else(|| state.repository.root.clone());
    let canon_server_root = server_root
        .canonicalize()
        .unwrap_or_else(|_| server_root.clone());
    if let Some(raw) = args
        .get("repo_root")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let req_path = std::path::Path::new(raw);
        if req_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("invalid `repo_root`: parent directory traversal is forbidden".to_string());
        }
        let canon_req = req_path
            .canonicalize()
            .map_err(|e| format!("invalid `repo_root`: {e}"))?;
        if canon_req != canon_server_root && !canon_req.starts_with(&canon_server_root) {
            return Err("`repo_root` escapes server repository root".to_string());
        }
        return Ok(canon_req);
    }
    Ok(canon_server_root)
}

fn validate_slug(raw: &str, field_name: &str) -> Result<(), String> {
    if raw.is_empty() || raw.len() > 64 {
        return Err(format!("invalid `{field_name}`: length must be 1..=64"));
    }
    if !raw
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(format!(
            "invalid `{field_name}`: only ASCII [a-zA-Z0-9_-] characters are allowed"
        ));
    }
    Ok(())
}

fn is_strictly_inside_muse_worktrees(
    repo_root: &std::path::Path,
    candidate: &std::path::Path,
) -> bool {
    let muse_root = repo_root.join(".vox").join("muse-worktrees");
    let Ok(rel) = candidate.strip_prefix(&muse_root) else {
        return false;
    };
    let mut count = 0usize;
    for comp in rel.components() {
        match comp {
            std::path::Component::Normal(seg) => {
                let s = seg.to_string_lossy();
                if s.is_empty() || s == "." || s == ".." {
                    return false;
                }
                count += 1;
            }
            _ => return false,
        }
    }
    if count == 0 {
        return false;
    }
    if let Ok(muse_meta) = std::fs::symlink_metadata(&muse_root) {
        if muse_meta.file_type().is_symlink() {
            return false;
        }
        let Ok(canon_repo) = repo_root.canonicalize() else {
            return false;
        };
        let Ok(canon_muse) = muse_root.canonicalize() else {
            return false;
        };
        if !canon_muse.starts_with(&canon_repo) || canon_muse == canon_repo {
            return false;
        }
        if let Ok(cand_meta) = std::fs::symlink_metadata(candidate) {
            if cand_meta.file_type().is_symlink() {
                return false;
            }
            let Ok(canon_cand) = candidate.canonicalize() else {
                return false;
            };
            if !canon_cand.starts_with(&canon_muse) || canon_cand == canon_muse {
                return false;
            }
        }
    }
    true
}

fn validate_repo_relative_file_path(
    repo_root: &std::path::Path,
    raw: &str,
) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains(':')
        || trimmed.contains('\0')
    {
        return Err(
            "invalid `file_path`: must be a normalized relative path inside `repo_root`"
                .to_string(),
        );
    }
    let normalized = trimmed.replace('\\', "/");
    if normalized.contains("//") {
        return Err("invalid `file_path`: empty path segment".to_string());
    }
    let mut clean_parts = Vec::new();
    for comp in std::path::Path::new(&normalized).components() {
        match comp {
            std::path::Component::Normal(seg) => {
                let Some(seg_str) = seg.to_str() else {
                    return Err("invalid `file_path`: non-UTF-8 path segment".to_string());
                };
                if seg_str.is_empty() || seg_str == "." || seg_str == ".." {
                    return Err("invalid `file_path`: traversal segment forbidden".to_string());
                }
                clean_parts.push(seg_str);
            }
            _ => {
                return Err(
                    "invalid `file_path`: only normal relative path components are allowed"
                        .to_string(),
                );
            }
        }
    }
    if clean_parts.is_empty() {
        return Err("invalid `file_path`: empty path".to_string());
    }
    let canon_root = repo_root
        .canonicalize()
        .map_err(|e| format!("invalid `repo_root`: {e}"))?;
    let mut cursor = canon_root.clone();
    let mut deepest_existing = canon_root.clone();
    for seg in &clean_parts {
        cursor.push(seg);
        if let Ok(meta) = std::fs::symlink_metadata(&cursor) {
            let canon_cur = cursor
                .canonicalize()
                .map_err(|e| format!("invalid `file_path`: {e}"))?;
            if !canon_cur.starts_with(&canon_root) {
                return Err("invalid `file_path`: path escapes `repo_root`".to_string());
            }
            if meta.file_type().is_symlink() && canon_cur == canon_root {
                return Err("invalid `file_path`: symlink loop to `repo_root`".to_string());
            }
            deepest_existing = canon_cur;
        }
    }
    if !deepest_existing.starts_with(&canon_root) {
        return Err("invalid `file_path`: target escapes `repo_root`".to_string());
    }
    Ok(clean_parts.join("/"))
}

fn allowed_external_session_roots(repo_root: &std::path::Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(canon_repo) = repo_root.canonicalize() {
        roots.push(canon_repo);
    }
    if let Some(home) = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
    {
        for dir_name in [".claude", ".codex", ".cursor"] {
            if let Ok(canon_dir) = home.join(dir_name).canonicalize() {
                roots.push(canon_dir);
            }
        }
    }
    roots
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusLegInput {
    pub model_id: String,
    pub content: Option<String>,
    pub reasoning: Option<String>,
    pub response_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusLegRichResult {
    pub model_id: String,
    pub status: String,
    pub content: String,
    pub reasoning: String,
    pub response_time_ms: u64,
    pub quality: ResponseQualityMetrics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

enum ConsensusLegExecution {
    Completed {
        model_id: String,
        content: String,
        reasoning: String,
        response_time_ms: u64,
    },
    Failed {
        model_id: String,
        error: String,
        response_time_ms: u64,
    },
}

/// Dispatch `vox_consensus_run` (`T3-MCP-RPC-WIRE-01` & `T3-STATEFUL-TOOLS-01`).
pub async fn handle_consensus_run(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_consensus_run", &args) {
        return ToolResult::<Value>::err_with_remediation(
            reason,
            "Elevate runtime_mode/interaction_mode or grant `spawn_threads` capability.",
        )
        .to_json();
    }
    let prompt = args
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let models: Vec<String> = args
        .get("models")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| default_variant_models(2));

    let mut engine = match ConsensusFanOutEngine::new(models.clone()) {
        Ok(e) => e,
        Err(err) => {
            return ToolResult::<Value>::err_with_remediation(
                err.to_string(),
                "Supply between 2 and 4 distinct model IDs in `models`.",
            )
            .to_json();
        }
    };

    let provided_legs: Vec<ConsensusLegInput> = args
        .get("legs")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let any_provider_secret = [
        vox_secrets::SecretId::OpenRouterApiKey,
        vox_secrets::SecretId::GeminiApiKey,
        vox_secrets::SecretId::AnthropicApiKey,
        vox_secrets::SecretId::OpenaiApiKey,
        vox_secrets::SecretId::GroqApiKey,
    ]
    .iter()
    .any(|&id| {
        vox_secrets::resolve_secret(id)
            .expose()
            .is_some_and(|s| !s.trim().is_empty())
    });
    let run_live_llm = args
        .get("live_infer")
        .and_then(Value::as_bool)
        .unwrap_or(any_provider_secret);

    let mut events = vec![engine.start_event()];
    let mut rich_results = Vec::with_capacity(models.len());
    let mut max_ms = 0u64;

    // Fan out concurrent tasks across all legs (`T3-STATEFUL-TOOLS-01`).
    let leg_futs = models.iter().enumerate().map(|(idx, model_id)| {
        let model_id = model_id.clone();
        let prompt = prompt.clone();
        let leg_override = provided_legs
            .iter()
            .find(|l| l.model_id == model_id)
            .cloned();
        async move {
            if let Some(ovr) = leg_override {
                let ms = ovr.response_time_ms.unwrap_or(180 + (idx as u64) * 65);
                let Some(content) = ovr.content.filter(|c| !c.trim().is_empty()) else {
                    return ConsensusLegExecution::Failed {
                        model_id,
                        error: "caller-supplied leg override omitted non-empty `content`"
                            .to_string(),
                        response_time_ms: ms,
                    };
                };
                let reasoning = ovr
                    .reasoning
                    .unwrap_or_else(|| format!("Caller-provided leg output for {model_id}."));
                return ConsensusLegExecution::Completed {
                    model_id,
                    content,
                    reasoning,
                    response_time_ms: ms,
                };
            }
            if run_live_llm {
                if prompt.trim().is_empty() {
                    return ConsensusLegExecution::Failed {
                        model_id,
                        error: "`prompt` must be non-empty for live consensus inference"
                            .to_string(),
                        response_time_ms: 0,
                    };
                }
                let started = std::time::Instant::now();
                let infer_res = tokio::time::timeout(
                    std::time::Duration::from_secs(12),
                    crate::llm_bridge::call_llm_with_pref(
                        state,
                        "You are an expert consensus leg participant. Provide a concise, structured technical response.",
                        &prompt,
                        None,
                        Some(0.3),
                        None,
                        None,
                        Some(model_id.as_str()),
                        None,
                        Some(&prompt),
                    ),
                )
                .await;
                let elapsed = started.elapsed().as_millis().max(1) as u64;
                return match infer_res {
                    Ok(Ok((text, resolved_model, _tokens))) if !text.trim().is_empty() => {
                        ConsensusLegExecution::Completed {
                            model_id,
                            content: text,
                            reasoning: format!("Live LLM inference via {resolved_model}"),
                            response_time_ms: elapsed,
                        }
                    }
                    Ok(Ok(_)) => ConsensusLegExecution::Failed {
                        model_id,
                        error: "LLM provider returned an empty response".to_string(),
                        response_time_ms: elapsed,
                    },
                    Ok(Err(err)) => ConsensusLegExecution::Failed {
                        model_id,
                        error: err.to_string(),
                        response_time_ms: elapsed,
                    },
                    Err(_) => ConsensusLegExecution::Failed {
                        model_id,
                        error: "consensus leg inference timed out after 12s".to_string(),
                        response_time_ms: elapsed,
                    },
                };
            }
            ConsensusLegExecution::Failed {
                model_id,
                error: "no live LLM credentials configured and no precomputed `legs` supplied"
                    .to_string(),
                response_time_ms: 0,
            }
        }
    });
    let completed_legs = futures_util::future::join_all(leg_futs).await;

    for outcome in completed_legs {
        match outcome {
            ConsensusLegExecution::Completed {
                model_id,
                content,
                reasoning,
                response_time_ms,
            } => {
                max_ms = max_ms.max(response_time_ms);
                if let Some(ev) = engine.apply_delta(&model_id, &content, Some(&reasoning)) {
                    events.push(ev);
                }
                if let Some(ev) = engine.complete_leg(&model_id, &prompt, response_time_ms) {
                    events.push(ev);
                }
                let quality = analyze_response_quality(&prompt, &content);
                rich_results.push(ConsensusLegRichResult {
                    model_id,
                    status: "completed".to_string(),
                    content,
                    reasoning,
                    response_time_ms,
                    quality,
                    error: None,
                });
            }
            ConsensusLegExecution::Failed {
                model_id,
                error,
                response_time_ms,
            } => {
                max_ms = max_ms.max(response_time_ms);
                if let Some(ev) = engine.fail_leg(&model_id, error.clone()) {
                    events.push(ev);
                }
                let quality = analyze_response_quality(&prompt, "");
                rich_results.push(ConsensusLegRichResult {
                    model_id,
                    status: "failed".to_string(),
                    content: String::new(),
                    reasoning: String::new(),
                    response_time_ms,
                    quality,
                    error: Some(error),
                });
            }
        }
    }

    if rich_results.iter().all(|r| r.status == "failed") {
        let joined_errors = rich_results
            .iter()
            .map(|r| {
                format!(
                    "{}: {}",
                    r.model_id,
                    r.error.as_deref().unwrap_or("leg failed")
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        return ToolResult::<Value>::err(format!("all consensus legs failed ({joined_errors})"))
            .to_json();
    }

    let synthesized_consensus = engine.synthesize_consensus_response(&prompt);
    let final_event = engine.finalize(max_ms);
    let final_legs = match &final_event {
        ConsensusStreamEvent::Final { responses, .. } => responses.clone(),
        _ => Vec::new(),
    };
    events.push(final_event);

    ToolResult::ok(json!({
        "prompt": prompt,
        "models": models,
        "legs": final_legs,
        "rich_results": rich_results,
        "synthesized_consensus": synthesized_consensus,
        "events": events,
        "total_elapsed_ms": max_ms,
    }))
    .to_json()
}

/// Dispatch `vox_context_handoff` (`T3-MCP-RPC-WIRE-01` & `T3-STATEFUL-TOOLS-02`).
pub async fn handle_context_handoff(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_context_handoff", &args) {
        return ToolResult::<Value>::err_with_remediation(
            reason,
            "Grant write access to synthesize a cross-provider context handoff.",
        )
        .to_json();
    }
    let source_thread_id = args
        .get("source_thread_id")
        .and_then(Value::as_str)
        .unwrap_or("thread-default");
    let target_thread_id = args.get("target_thread_id").and_then(Value::as_str);
    let source_provider = args
        .get("source_provider")
        .and_then(Value::as_str)
        .unwrap_or("anthropic");
    let target_provider = args
        .get("target_provider")
        .and_then(Value::as_str)
        .unwrap_or("openai");
    let strategy: HandoffStrategy = args
        .get("strategy")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or(HandoffStrategy::Summarized);
    let mut turns: Vec<HandoffTranscriptTurn> = args
        .get("turns")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // Hydrate from `chat_history:{source_thread_id}` when caller omits explicit `turns` (`T3-STATEFUL-TOOLS-02`).
    if turns.is_empty() {
        let source_key = format!("chat_history:{source_thread_id}");
        let hydrated = crate::chat_tools::chat::hydrate::context_history_or_hydrate(
            state,
            source_key.as_str(),
            source_thread_id,
        )
        .await;
        turns = hydrated
            .into_iter()
            .map(|e| {
                let is_plan_or_code = e.content.contains("```")
                    || e.content.contains("## Plan")
                    || e.content.contains("1. ");
                HandoffTranscriptTurn {
                    role: e.role,
                    content: e.content,
                    is_plan_or_code,
                }
            })
            .collect();
    }

    let pkg = ContextHandoffService::build_package(
        source_thread_id,
        source_provider,
        target_provider,
        strategy,
        &turns,
    );

    // If `target_thread_id` is specified, persist the synthesized handoff brief into `chat_history:{target_thread_id}`.
    if let Some(target_id) = target_thread_id.filter(|s| !s.trim().is_empty()) {
        let target_key = format!("chat_history:{target_id}");
        let mut target_history = crate::chat_tools::chat::hydrate::context_history_or_hydrate(
            state,
            target_key.as_str(),
            target_id,
        )
        .await;
        let now = crate::chat_tools::now_ts();
        let handoff_msg_id = {
            use vox_orchestrator::id_allocator::VoxIdAllocatorV2;
            state
                .id_allocator
                .derive("sys", &[source_thread_id, target_id, &now.to_string()])
        };
        target_history.push(crate::chat_tools::params::ChatTranscriptEntry {
            id: handoff_msg_id,
            role: "system".to_string(),
            content: pkg.formatted_prompt.clone(),
            timestamp: now,
            context_files: vec![],
            model_used: Some(format!("{source_provider}->{target_provider}")),
            tokens: None,
            thought_signature: None,
            edited_at: None,
        });
        if let Ok(json_str) = serde_json::to_string(&target_history)
            && let Ok(ctx) = crate::sync_poison::poison_rw_write(
                state.orchestrator.context_handle().write(),
                "orchestrator context",
            )
        {
            ctx.set(vox_orchestrator::AgentId(0), &target_key, &json_str, 0);
        }
    }

    ToolResult::ok(pkg).to_json()
}

/// Dispatch `vox_checkpoint_capture` (`T3-MCP-RPC-WIRE-02`).
pub async fn handle_checkpoint_capture(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_checkpoint_capture", &args) {
        return ToolResult::<Value>::err_with_remediation(
            reason,
            "Grant `vcs_worktree` capability to capture Git checkpoints.",
        )
        .to_json();
    }
    let repo_root = match resolve_repo_root(state, &args) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let thread_id = args
        .get("thread_id")
        .and_then(Value::as_str)
        .unwrap_or("default-session");
    if let Err(err) = validate_slug(thread_id, "thread_id") {
        return ToolResult::<Value>::err(err).to_json();
    }
    let turn_count = args.get("turn_count").and_then(Value::as_u64).unwrap_or(1) as u32;
    let label = args.get("restore_label").and_then(Value::as_str);
    if let Some(lbl) = label
        && let Err(err) = validate_slug(lbl, "restore_label")
    {
        return ToolResult::<Value>::err(err).to_json();
    }

    let res = if let Some(lbl) = label {
        GitPlumbingCheckpointStore::create_restore_point(&repo_root, thread_id, turn_count, lbl)
    } else {
        GitPlumbingCheckpointStore::capture_checkpoint(&repo_root, thread_id, turn_count)
    };
    match res {
        Ok(rec) => ToolResult::ok(rec).to_json(),
        Err(err) => ToolResult::<Value>::err_with_remediation(
            err,
            "Ensure `repo_root` points to an initialized Git repository.",
        )
        .to_json(),
    }
}

/// Dispatch `vox_checkpoint_list` (`T3-MCP-RPC-WIRE-02`).
pub async fn handle_checkpoint_list(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_checkpoint_list", &args) {
        return ToolResult::<Value>::err_with_remediation(reason, "Read access required.")
            .to_json();
    }
    let repo_root = match resolve_repo_root(state, &args) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let thread_id = args
        .get("thread_id")
        .and_then(Value::as_str)
        .unwrap_or("default-session");
    if let Err(err) = validate_slug(thread_id, "thread_id") {
        return ToolResult::<Value>::err(err).to_json();
    }
    match GitPlumbingCheckpointStore::list_checkpoints(&repo_root, thread_id) {
        Ok(list) => ToolResult::ok(json!({
            "thread_id": thread_id,
            "checkpoints": list,
        }))
        .to_json(),
        Err(err) => ToolResult::<Value>::err_with_remediation(
            err,
            "Ensure `repo_root` points to an initialized Git repository.",
        )
        .to_json(),
    }
}

/// Dispatch `vox_checkpoint_restore` (`T3-MCP-RPC-WIRE-02`).
pub async fn handle_checkpoint_restore(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_checkpoint_restore", &args) {
        return ToolResult::<Value>::err_with_remediation(
            reason,
            "Grant `vcs_worktree` capability to restore a Git checkpoint.",
        )
        .to_json();
    }
    let repo_root = match resolve_repo_root(state, &args) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let thread_id = args
        .get("thread_id")
        .and_then(Value::as_str)
        .unwrap_or("default-session");
    if let Err(err) = validate_slug(thread_id, "thread_id") {
        return ToolResult::<Value>::err(err).to_json();
    }
    let target_turn = args.get("target_turn").and_then(Value::as_u64).unwrap_or(1) as u32;
    let max_known_turn = args
        .get("max_known_turn")
        .and_then(Value::as_u64)
        .unwrap_or(u64::from(target_turn)) as u32;

    match GitPlumbingCheckpointStore::restore_checkpoint(
        &repo_root,
        thread_id,
        target_turn,
        max_known_turn,
    ) {
        Ok(()) => ToolResult::ok(json!({
            "thread_id": thread_id,
            "restored_turn": target_turn,
            "ref_name": GitPlumbingCheckpointStore::checkpoint_ref_name(thread_id, target_turn),
        }))
        .to_json(),
        Err(err) => ToolResult::<Value>::err_with_remediation(
            err,
            "Verify that the target checkpoint ref exists in `refs/vox/checkpoints/`.",
        )
        .to_json(),
    }
}

/// Dispatch `vox_muse_pool_spawn` (`T3-MCP-RPC-WIRE-02` & `T3-WORKTREE-CKPT-03`).
pub async fn handle_muse_pool_spawn(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_muse_pool_spawn", &args) {
        return ToolResult::<Value>::err_with_remediation(
            reason,
            "Grant `spawn_threads` capability to plan Muse worktree variants.",
        )
        .to_json();
    }
    let repo_root = match resolve_repo_root(state, &args) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let run_id = args
        .get("run_id")
        .or_else(|| args.get("session_id"))
        .and_then(Value::as_str)
        .unwrap_or("muse-run-1");
    if let Err(err) = validate_slug(run_id, "run_id") {
        return ToolResult::<Value>::err(err).to_json();
    }
    let model_ids: Vec<String> = args
        .get("model_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| {
            let count = args
                .get("variant_count")
                .and_then(Value::as_u64)
                .unwrap_or(2)
                .clamp(1, 4) as usize;
            default_variant_models(count)
        });

    match MultiVariantWorktreePool::plan_variants(&repo_root, run_id, &model_ids) {
        Ok(pool) => {
            for slot in pool.variants() {
                if !is_strictly_inside_muse_worktrees(&repo_root, &slot.worktree_path) {
                    return ToolResult::<Value>::err(
                        "refusing to create Muse worktree outside `.vox/muse-worktrees`"
                            .to_string(),
                    )
                    .to_json();
                }
                if let Err(err) = std::fs::create_dir_all(&slot.worktree_path) {
                    return ToolResult::<Value>::err(format!(
                        "failed to create Muse worktree directory: {err}"
                    ))
                    .to_json();
                }
                if !is_strictly_inside_muse_worktrees(&repo_root, &slot.worktree_path) {
                    return ToolResult::<Value>::err(
                        "refusing to provision `.env` files into worktree escaping `.vox/muse-worktrees`"
                            .to_string(),
                    )
                    .to_json();
                }
                let _ = crate::agy_worktree::provision_untracked_env_files(
                    &repo_root,
                    &slot.worktree_path,
                    &[".env".to_string(), ".env.local".to_string()],
                );
            }
            let variants = pool.variants().to_vec();
            state.muse_pools.lock().insert(run_id.to_string(), pool);
            ToolResult::ok(json!({
                "run_id": run_id,
                "variants": variants,
            }))
            .to_json()
        }
        Err(err) => ToolResult::<Value>::err_with_remediation(
            err,
            "Provide between 1 and 4 model IDs in `model_ids`.",
        )
        .to_json(),
    }
}

/// First `count` (max 4) distinct-vendor default model ids for consensus / Muse variants,
/// sourced from `contracts/orchestration/model-defaults.v1.yaml` (never pinned here).
fn default_variant_models(count: usize) -> Vec<String> {
    use vox_config::model_defaults as d;
    [
        d::PREMIUM_CODEGEN,
        d::PREMIUM_PLANNING,
        d::PREMIUM_RESEARCH,
        "mens-qwen3-8b",
    ][..count.min(4)]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// Dispatch `vox_muse_pool_adopt` (`T3-MCP-RPC-WIRE-02` & `T3-WORKTREE-CKPT-03`).
pub async fn handle_muse_pool_adopt(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_muse_pool_adopt", &args) {
        return ToolResult::<Value>::err_with_remediation(
            reason,
            "Grant `vcs_worktree` capability to adopt a Muse variant.",
        )
        .to_json();
    }
    let repo_root = match resolve_repo_root(state, &args) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let run_id = args
        .get("run_id")
        .or_else(|| args.get("session_id"))
        .and_then(Value::as_str)
        .unwrap_or("muse-run-1");
    if let Err(err) = validate_slug(run_id, "run_id") {
        return ToolResult::<Value>::err(err).to_json();
    }
    let winning_variant_id_arg = args
        .get("winning_variant_id")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty());
    let mut model_ids: Vec<String> = args
        .get("model_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| default_variant_models(2));
    if args.get("model_ids").is_none()
        && let Some(wid) = winning_variant_id_arg
        && !wid.starts_with(&format!("{run_id}-v"))
        && !matches!(wid, "1" | "2" | "3" | "4" | "v1" | "v2" | "v3" | "v4")
        && !model_ids.iter().any(|m| m == wid)
        && !model_ids.is_empty()
    {
        model_ids[0] = wid.to_string();
    }

    let mut pool = {
        let existing = state.muse_pools.lock().get(run_id).cloned();
        if let Some(p) = existing {
            p
        } else {
            match MultiVariantWorktreePool::plan_variants(&repo_root, run_id, &model_ids) {
                Ok(p) => p,
                Err(err) => return ToolResult::<Value>::err(err).to_json(),
            }
        }
    };
    let variant_index = args
        .get("variant_index")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .or_else(|| {
            args.get("winning_slot_index")
                .and_then(Value::as_u64)
                .map(|v| (v as usize) + 1)
        })
        .or_else(|| {
            winning_variant_id_arg.and_then(|wid| {
                pool.variants()
                    .iter()
                    .find(|s| {
                        format!("{run_id}-v{}", s.variant_index) == wid
                            || format!("v{}", s.variant_index) == wid
                            || s.variant_index.to_string() == wid
                            || s.model_id == wid
                    })
                    .map(|s| s.variant_index)
            })
        })
        .unwrap_or(1);
    let pruned_variant_ids: Vec<String> = pool
        .variants()
        .iter()
        .filter(|s| s.variant_index != variant_index)
        .map(|s| format!("{run_id}-v{}", s.variant_index))
        .collect();

    match pool.adopt_variant(variant_index) {
        Ok((winner, pruned_paths)) => {
            for loser in &pruned_paths {
                if !is_strictly_inside_muse_worktrees(&repo_root, loser) {
                    return ToolResult::<Value>::err(
                        "refusing to prune worktree path outside `.vox/muse-worktrees`".to_string(),
                    )
                    .to_json();
                }
                if loser.exists() {
                    let _ = std::fs::remove_dir_all(loser);
                }
            }
            let adopted_variant_id = winning_variant_id_arg
                .map(str::to_string)
                .unwrap_or_else(|| format!("{run_id}-v{}", winner.variant_index));
            let adopted_branch_name = winner.branch_name.clone();
            state.muse_pools.lock().insert(run_id.to_string(), pool);
            ToolResult::ok(json!({
                "run_id": run_id,
                "winner": winner,
                "adopted_variant_id": adopted_variant_id,
                "adopted_branch_name": adopted_branch_name,
                "pruned_variant_ids": pruned_variant_ids,
                "pruned_worktree_paths": pruned_paths,
            }))
            .to_json()
        }
        Err(err) => ToolResult::<Value>::err(err).to_json(),
    }
}

fn normalize_guest_permission(raw: Option<&str>) -> &'static str {
    match raw {
        Some("read_only" | "view_only") => "read_only",
        _ => "suggest_prompt",
    }
}

fn guest_invite_to_json(record: &vox_orchestrator::guest_share::GuestInviteRecord) -> Value {
    let permission = normalize_guest_permission(Some(record.scope.as_str()));
    json!({
        "invite_id": record.invite_id,
        "thread_id": record.thread_id,
        "scope": record.scope,
        "expires_at_ms": record.expires_at_ms,
        "signature_hex": record.signature_hex,
        "redeemed_by_session_id": record.redeemed_by_session_id,
        "revoked": record.revoked,
        "token": format!("{}:{}", record.invite_id, record.signature_hex),
        "session_id": record.thread_id,
        "permission": permission,
    })
}

fn guest_submission_to_json(msg: &vox_orchestrator::guest_share::GuestPendingMessage) -> Value {
    let status = serde_json::to_value(msg.status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "pending".to_string());
    json!({
        "message_id": msg.message_id,
        "thread_id": msg.thread_id,
        "guest_session_id": msg.guest_session_id,
        "guest_display_name": msg.guest_display_name,
        "prompt": msg.prompt,
        "status": status,
        "submission_id": msg.message_id,
        "session_id": msg.thread_id,
        "guest_name": msg.guest_display_name,
    })
}

/// Dispatch `vox_guest_share_list` (`AXIS-IPC-HONESTY-01`): list guest share invites and pending submissions.
pub async fn handle_guest_share_list(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_guest_share_list", &args) {
        return ToolResult::<Value>::err_with_remediation(reason, "Read access required.")
            .to_json();
    }
    let thread_filter = args
        .get("thread_id")
        .or_else(|| args.get("session_id"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty());
    let (invites, submissions) = {
        let svc = state.guest_share_service.lock();
        let invites: Vec<Value> = svc
            .list_invites(thread_filter)
            .iter()
            .map(guest_invite_to_json)
            .collect();
        let submissions: Vec<Value> = svc
            .list_pending_messages(thread_filter)
            .iter()
            .map(guest_submission_to_json)
            .collect();
        (invites, submissions)
    };
    ToolResult::ok(json!({
        "invites": invites,
        "submissions": submissions,
    }))
    .to_json()
}

/// Dispatch `vox_guest_share_mint` (`T3-MCP-RPC-WIRE-03` & `T3-STATEFUL-TOOLS-03`).
pub async fn handle_guest_share_mint(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_guest_share_mint", &args) {
        return ToolResult::<Value>::err_with_remediation(
            reason,
            "Grant `guest_moderation` capability to mint guest share tokens.",
        )
        .to_json();
    }
    let thread_id = args
        .get("thread_id")
        .or_else(|| args.get("session_id"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("thread-1");
    let permission = normalize_guest_permission(
        args.get("permission")
            .or_else(|| args.get("scope"))
            .or_else(|| args.get("policy"))
            .and_then(Value::as_str),
    );
    let now_ms = args
        .get("now_ms")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| crate::chat_tools::now_ts().saturating_mul(1000));
    let ttl_ms = args
        .get("ttl_ms")
        .and_then(Value::as_u64)
        .unwrap_or(vox_orchestrator::guest_share::GUEST_SHARE_TTL_MS);
    let record = {
        let mut svc = state.guest_share_service.lock();
        let invite_id = args
            .get("invite_id")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| {
                let next_idx = svc.list_invites(None).len() + 1;
                format!("invite-{next_idx}")
            });
        svc.create_invite_with_options(invite_id, thread_id, permission, now_ms, ttl_ms)
    };
    ToolResult::ok(guest_invite_to_json(&record)).to_json()
}

/// Dispatch `vox_guest_share_moderate` (`T3-MCP-RPC-WIRE-03` & `T3-STATEFUL-TOOLS-03`).
pub async fn handle_guest_share_moderate(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_guest_share_moderate", &args) {
        return ToolResult::<Value>::err_with_remediation(
            reason,
            "Grant `guest_moderation` capability to moderate guest messages.",
        )
        .to_json();
    }
    let action = if let Some(act) = args.get("action").and_then(Value::as_str) {
        act
    } else if let Some(approve) = args.get("approve").and_then(Value::as_bool) {
        if approve { "approve" } else { "dismiss" }
    } else {
        "approve"
    };
    let message_id = args
        .get("message_id")
        .or_else(|| args.get("submission_id"))
        .and_then(Value::as_str)
        .unwrap_or("guest-msg-1");
    let outcome = {
        let mut svc = state.guest_share_service.lock();
        if action == "dismiss" {
            svc.dismiss_guest_message(message_id)
        } else {
            svc.approve_guest_message(message_id)
        }
    };
    match outcome {
        Ok(msg) => {
            if action != "dismiss" {
                let target_thread = msg.thread_id.as_str();
                let history_key = format!("chat_history:{target_thread}");
                let mut history = crate::chat_tools::chat::hydrate::context_history_or_hydrate(
                    state,
                    history_key.as_str(),
                    target_thread,
                )
                .await;
                let now = crate::chat_tools::now_ts();
                history.push(crate::chat_tools::params::ChatTranscriptEntry {
                    id: msg.message_id.clone(),
                    role: "user".to_string(),
                    content: format!("[Guest ({})]: {}", msg.guest_display_name, msg.prompt),
                    timestamp: now,
                    context_files: vec![],
                    model_used: None,
                    tokens: None,
                    thought_signature: None,
                    edited_at: None,
                });
                if let Ok(json_str) = serde_json::to_string(&history)
                    && let Ok(ctx) = crate::sync_poison::poison_rw_write(
                        state.orchestrator.context_handle().write(),
                        "orchestrator context",
                    )
                {
                    ctx.set(vox_orchestrator::AgentId(0), &history_key, &json_str, 0);
                }
            }
            ToolResult::ok(guest_submission_to_json(&msg)).to_json()
        }
        Err(err) => ToolResult::<Value>::err(err.to_string()).to_json(),
    }
}

/// Dispatch `vox_session_import_external` (`T3-MCP-RPC-WIRE-03` & `T3-STATEFUL-TOOLS-04`).
pub async fn handle_session_import_external(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_session_import_external", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let repo_root = match resolve_repo_root(state, &args) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let source_harness = args
        .get("source_harness")
        .and_then(Value::as_str)
        .unwrap_or("claude");
    let external_session_id = args
        .get("external_session_id")
        .and_then(Value::as_str)
        .unwrap_or("ext-session-1");
    let target_session_id = args
        .get("target_session_id")
        .and_then(Value::as_str)
        .unwrap_or(external_session_id);

    if source_harness == "cursor" {
        if let Some(hex_blob) = args.get("cursor_root_blob_hex").and_then(Value::as_str) {
            match decode_cursor_root_blob_hex(hex_blob) {
                Ok(decoded) => {
                    let messages: Vec<ImportedSessionMessage> = decoded
                        .get("messages")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();
                    let title = decoded
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("Imported Cursor Session")
                        .to_string();
                    persist_imported_messages(state, target_session_id, &messages).await;
                    return ToolResult::ok(json!({
                        "source_harness": "cursor",
                        "external_session_id": external_session_id,
                        "target_session_id": target_session_id,
                        "title": title,
                        "messages": messages,
                    }))
                    .to_json();
                }
                Err(err) => return ToolResult::<Value>::err(err).to_json(),
            }
        }
    }

    let resolved_jsonl = if let Some(raw) = args
        .get("raw_jsonl")
        .or_else(|| args.get("raw_payload"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        raw.to_string()
    } else if let Some(fp) = args
        .get("file_path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let raw_path = std::path::Path::new(fp);
        if raw_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return ToolResult::<Value>::err(
                "invalid `file_path`: parent directory traversal is forbidden".to_string(),
            )
            .to_json();
        }
        if raw_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            return ToolResult::<Value>::err(
                "invalid `file_path`: only `.jsonl` session files are permitted".to_string(),
            )
            .to_json();
        }
        let canon_fp = match raw_path.canonicalize() {
            Ok(p) => p,
            Err(err) => {
                return ToolResult::<Value>::err(format!(
                    "failed to resolve external session file: {err}"
                ))
                .to_json();
            }
        };
        if canon_fp.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            return ToolResult::<Value>::err(
                "invalid `file_path`: canonical target must have `.jsonl` extension".to_string(),
            )
            .to_json();
        }
        let allowed_roots = allowed_external_session_roots(&repo_root);
        if !allowed_roots
            .iter()
            .any(|root| canon_fp.starts_with(root) && canon_fp != *root)
        {
            return ToolResult::<Value>::err(
                "`file_path` is outside allowed external session roots (`~/.claude`, `~/.codex`, `~/.cursor`, or repository root)".to_string(),
            )
            .to_json();
        }
        match std::fs::read_to_string(&canon_fp) {
            Ok(text) => text,
            Err(err) => {
                return ToolResult::<Value>::err(format!(
                    "failed to read external session file: {err}"
                ))
                .to_json();
            }
        }
    } else {
        let Some(discovered) = discover_latest_external_jsonl(source_harness) else {
            return ToolResult::<Value>::err(format!(
                "no external `{source_harness}` `.jsonl` session files found"
            ))
            .to_json();
        };
        discovered
    };
    let session =
        parse_jsonl_external_session(source_harness, external_session_id, &resolved_jsonl);
    persist_imported_messages(state, target_session_id, &session.messages).await;
    ToolResult::ok(session).to_json()
}

fn walk_session_files_recursive(
    dir: &std::path::Path,
    source_harness: &str,
    depth: usize,
    out: &mut Vec<Value>,
) {
    if depth > 4 || out.len() >= 100 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            walk_session_files_recursive(&path, source_harness, depth + 1, out);
        } else {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "jsonl" || ext == "json" || ext == "vscdb" {
                let modified_ms = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let session_id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("ext-session")
                    .to_string();
                out.push(json!({
                    "source_harness": source_harness,
                    "session_id": session_id,
                    "file_path": path.to_string_lossy().replace('\\', "/"),
                    "modified_ms": modified_ms,
                }));
            }
        }
    }
}

fn list_external_sessions_catalog(
    source_filter: Option<&str>,
    root_override: Option<&std::path::Path>,
) -> Vec<Value> {
    let mut out = Vec::new();
    if let Some(root) = root_override {
        let harness = source_filter.unwrap_or("claude");
        walk_session_files_recursive(root, harness, 0, &mut out);
    } else if let Some(home) = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
    {
        let targets: &[(&str, PathBuf)] = &[
            ("claude", home.join(".claude").join("projects")),
            ("codex", home.join(".codex").join("sessions")),
            ("cursor", home.join(".cursor").join("chats")),
        ];
        for (harness, root) in targets {
            if source_filter.is_none_or(|f| f == *harness) {
                walk_session_files_recursive(root, harness, 0, &mut out);
            }
        }
    }
    out.sort_by(|a, b| {
        let ma = a.get("modified_ms").and_then(Value::as_u64).unwrap_or(0);
        let mb = b.get("modified_ms").and_then(Value::as_u64).unwrap_or(0);
        mb.cmp(&ma)
    });
    out
}

fn discover_latest_external_jsonl(source_harness: &str) -> Option<String> {
    let catalog = list_external_sessions_catalog(Some(source_harness), None);
    for item in catalog {
        if let Some(fp) = item.get("file_path").and_then(Value::as_str)
            && fp.ends_with(".jsonl")
            && let Ok(content) = std::fs::read_to_string(fp)
            && !content.trim().is_empty()
        {
            return Some(content);
        }
    }
    None
}

async fn persist_imported_messages(
    state: &ServerState,
    session_id: &str,
    messages: &[ImportedSessionMessage],
) {
    if messages.is_empty() {
        return;
    }
    let history_key = format!("chat_history:{session_id}");
    let mut history = crate::chat_tools::chat::hydrate::context_history_or_hydrate(
        state,
        history_key.as_str(),
        session_id,
    )
    .await;
    let base_ts = crate::chat_tools::now_ts();
    for (idx, m) in messages.iter().enumerate() {
        history.push(crate::chat_tools::params::ChatTranscriptEntry {
            id: format!("imp-{session_id}-{idx}"),
            role: m.role.clone(),
            content: m.content.clone(),
            timestamp: base_ts + (idx as u64),
            context_files: vec![],
            model_used: None,
            tokens: None,
            thought_signature: None,
            edited_at: None,
        });
    }
    if let Ok(json_str) = serde_json::to_string(&history)
        && let Ok(ctx) = crate::sync_poison::poison_rw_write(
            state.orchestrator.context_handle().write(),
            "orchestrator context",
        )
    {
        ctx.set(vox_orchestrator::AgentId(0), &history_key, &json_str, 0);
    }
}

/// Dispatch `vox_session_export_bundle` (`T3-MCP-RPC-WIRE-03` & `T3-STATEFUL-TOOLS-04`).
pub async fn handle_session_export_bundle(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_session_export_bundle", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let session_id = args
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("vox-session-1");
    let title = args
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Vox Portable Session");
    let mut messages: Vec<ImportedSessionMessage> = args
        .get("messages")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    if messages.is_empty() {
        let history_key = format!("chat_history:{session_id}");
        let hydrated = crate::chat_tools::chat::hydrate::context_history_or_hydrate(
            state,
            history_key.as_str(),
            session_id,
        )
        .await;
        messages = hydrated
            .into_iter()
            .map(|e| ImportedSessionMessage {
                role: e.role,
                content: e.content,
            })
            .collect();
    }
    let bundle = VoxSessionExportBundleV1::new(session_id, title, messages);
    ToolResult::ok(bundle).to_json()
}

/// Dispatch `vox_session_import_bundle` (`T3-MCP-RPC-WIRE-03` & `T3-STATEFUL-TOOLS-04`).
pub async fn handle_session_import_bundle(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_session_import_bundle", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let parsed_bundle = if let Some(raw_json_str) = args
        .get("bundle_json")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        serde_json::from_str::<VoxSessionExportBundleV1>(raw_json_str)
    } else {
        let bundle_val = args.get("bundle").cloned().unwrap_or(args);
        serde_json::from_value::<VoxSessionExportBundleV1>(bundle_val)
    };
    match parsed_bundle {
        Ok(bundle) => {
            if bundle.schema_version != 1 {
                return ToolResult::<Value>::err(format!(
                    "unsupported VoxSessionExportBundle schema_version {}",
                    bundle.schema_version
                ))
                .to_json();
            }
            persist_imported_messages(state, &bundle.session_id, &bundle.messages).await;
            ToolResult::ok(bundle).to_json()
        }
        Err(err) => ToolResult::<Value>::err(format!("invalid session bundle: {err}")).to_json(),
    }
}

/// Dispatch `vox_quarantine_set` (`URBS-QUARANTINE-01`): toggle read-only Clausura Ferrea on a district prefix.
pub async fn handle_quarantine_set(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_quarantine_set", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let prefix = args
        .get("path_prefix")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string();
    if prefix.is_empty() {
        return ToolResult::<Value>::err("`path_prefix` must be non-empty".to_string()).to_json();
    }
    let quarantined = args
        .get("quarantined")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mut guard = state.quarantined_paths.write();
    if quarantined {
        guard.insert(prefix.clone());
    } else {
        guard.remove(&prefix);
    }
    let list: Vec<String> = guard.iter().cloned().collect();
    ToolResult::ok(json!({
        "path_prefix": prefix,
        "quarantined": quarantined,
        "quarantined_paths": list,
    }))
    .to_json()
}

/// Dispatch `vox_quarantine_list` (`URBS-QUARANTINE-01`): list all quarantined district path prefixes.
pub async fn handle_quarantine_list(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_quarantine_list", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let list: Vec<String> = state.quarantined_paths.read().iter().cloned().collect();
    ToolResult::ok(json!({
        "quarantined_paths": list,
    }))
    .to_json()
}

/// Dispatch `vox_vcs_checkpoint_diff_summary` (`URBS-SCRUBBER-01` & `T3-WORKTREE-CKPT-02`): return per-checkpoint modified file deltas.
pub async fn handle_vcs_checkpoint_diff_summary(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_vcs_checkpoint_diff_summary", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let repo_root = match resolve_repo_root(state, &args) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let thread_id = args
        .get("thread_id")
        .and_then(Value::as_str)
        .unwrap_or("default-session");
    if let Err(err) = validate_slug(thread_id, "thread_id") {
        return ToolResult::<Value>::err(err).to_json();
    }
    let target_turn = args.get("target_turn").and_then(Value::as_u64).unwrap_or(1) as u32;
    let ref_name = GitPlumbingCheckpointStore::checkpoint_ref_name(thread_id, target_turn);
    let unified_patch =
        match GitPlumbingCheckpointStore::diff_checkpoints(&repo_root, &ref_name, "HEAD") {
            Ok(patch) => patch,
            Err(err) => return ToolResult::<Value>::err(err).to_json(),
        };

    let out = match crate::git_exec::GitExec::new(&repo_root)
        .run(&["diff", "--numstat", &ref_name, "HEAD"])
        .await
    {
        Ok(o) => o,
        Err(err) => return ToolResult::<Value>::err(err.to_string()).to_json(),
    };

    let mut files = Vec::new();
    for line in out.stdout.lines() {
        let mut parts = line.splitn(3, '\t');
        let Some(add_s) = parts.next() else { continue };
        let Some(del_s) = parts.next() else { continue };
        let Some(path_s) = parts.next() else { continue };
        let clean_path = path_s.trim_end_matches('\r').replace('\\', "/");
        if clean_path.is_empty() {
            continue;
        }
        files.push(json!({
            "file_path": clean_path,
            "added_lines": add_s.trim().parse::<u32>().unwrap_or(0),
            "deleted_lines": del_s.trim().parse::<u32>().unwrap_or(0),
        }));
    }

    ToolResult::ok(json!({
        "thread_id": thread_id,
        "target_turn": target_turn,
        "ref_name": ref_name,
        "files": files,
        "unified_patch": unified_patch,
    }))
    .to_json()
}

/// Dispatch `vox_file_diff_and_diagnostics` (`URBS-LOUPE-01` & `T3-TOOLS-HARDENING-04`): return unified Git diff + diagnostics for a file.
pub async fn handle_file_diff_and_diagnostics(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_file_diff_and_diagnostics", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let repo_root = match resolve_repo_root(state, &args) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let raw_file_path = args.get("file_path").and_then(Value::as_str).unwrap_or("");
    let file_path = match validate_repo_relative_file_path(&repo_root, raw_file_path) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let unified_diff = crate::git_exec::GitExec::new(&repo_root)
        .run(&["diff", "--unified=3", "HEAD", "--", &file_path])
        .await
        .ok()
        .map(|o| o.stdout)
        .unwrap_or_default();
    let mut added_lines = 0u32;
    let mut deleted_lines = 0u32;
    for line in unified_diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            added_lines += 1;
        } else if line.starts_with('-') {
            deleted_lines += 1;
        }
    }
    let mut diagnostics = Vec::new();
    let full_path = repo_root.join(&file_path);
    if let Ok(src_text) = std::fs::read_to_string(&full_path) {
        for (idx, line) in src_text.lines().enumerate() {
            let line_no = (idx + 1) as u32;
            if line.contains("todo!(") || line.contains("unimplemented!(") {
                diagnostics.push(json!({
                    "file": file_path,
                    "line": line_no,
                    "column": 1u32,
                    "severity": "warning",
                    "code": "vox/stub/unimplemented",
                    "message": "Unimplemented stub macro found in source line.",
                }));
            }
        }
    }
    ToolResult::ok(json!({
        "file_path": file_path,
        "added_lines": added_lines,
        "deleted_lines": deleted_lines,
        "unified_diff": unified_diff,
        "diagnostics": diagnostics,
    }))
    .to_json()
}

/// Dispatch `vox_stage_file` (`URBS-LOUPE-03`): run `git add -- <file_path>`.
pub async fn handle_stage_file(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_stage_file", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let repo_root = match resolve_repo_root(state, &args) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let raw_file_path = args.get("file_path").and_then(Value::as_str).unwrap_or("");
    let file_path = match validate_repo_relative_file_path(&repo_root, raw_file_path) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    if let Some(prefix) = state.is_path_quarantined(&file_path) {
        return ToolResult::<Value>::err(format!(
            "path `{file_path}` is quarantined under `{prefix}`"
        ))
        .to_json();
    }
    match crate::git_exec::GitExec::new(&repo_root)
        .run(&["add", "--", &file_path])
        .await
    {
        Ok(_) => ToolResult::ok(json!({ "file_path": file_path, "staged": true })).to_json(),
        Err(err) => ToolResult::<Value>::err(err.to_string()).to_json(),
    }
}

/// Dispatch `vox_revert_file` (`URBS-LOUPE-03`): run `git checkout -- <file_path>`.
pub async fn handle_revert_file(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_revert_file", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let repo_root = match resolve_repo_root(state, &args) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let raw_file_path = args.get("file_path").and_then(Value::as_str).unwrap_or("");
    let file_path = match validate_repo_relative_file_path(&repo_root, raw_file_path) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    if let Some(prefix) = state.is_path_quarantined(&file_path) {
        return ToolResult::<Value>::err(format!(
            "path `{file_path}` is quarantined under `{prefix}`"
        ))
        .to_json();
    }
    match crate::git_exec::GitExec::new(&repo_root)
        .run(&["checkout", "--", &file_path])
        .await
    {
        Ok(_) => ToolResult::ok(json!({ "file_path": file_path, "reverted": true })).to_json(),
        Err(err) => ToolResult::<Value>::err(err.to_string()).to_json(),
    }
}

/// Dispatch `vox_list_mcp_servers` (`T3-PROMPT-LOOP-04`): tier-1 lazy MCP server group discovery.
pub async fn handle_list_mcp_servers(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_list_mcp_servers", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    if let Some(tools_arr) = args.get("prefixed_tools").and_then(Value::as_array) {
        let defs: Vec<crate::mcp_client::ToolDefinition> = tools_arr
            .iter()
            .filter_map(|v| {
                let name = v.get("name")?.as_str()?.to_string();
                let description = v
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let input_schema = v
                    .get("input_schema")
                    .cloned()
                    .unwrap_or_else(|| json!({"type": "object"}));
                Some(crate::mcp_client::ToolDefinition {
                    name,
                    description,
                    input_schema,
                })
            })
            .collect();
        *state.lazy_mcp_registry.lock() =
            crate::external_mcp::LazyMcpGroupRegistry::from_prefixed_tools(&defs);
    }
    let payload = state.lazy_mcp_registry.lock().list_mcp_servers();
    ToolResult::ok(payload).to_json()
}

/// Dispatch `vox_list_mcp_tools` (`T3-PROMPT-LOOP-04`): tier-2 lazy MCP tool schema inspection for one server group.
pub async fn handle_list_mcp_tools(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_list_mcp_tools", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let mcp_name = args
        .get("mcp_name")
        .or_else(|| args.get("server"))
        .and_then(Value::as_str)
        .unwrap_or("source_01");
    let payload = state.lazy_mcp_registry.lock().list_mcp_tools(mcp_name);
    ToolResult::ok(payload).to_json()
}

/// Dispatch `vox_call_mcp_lazy` (`T3-PROMPT-LOOP-04`): tier-3 lazy MCP tool resolution and execution.
pub async fn handle_call_mcp_lazy(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_call_mcp_lazy", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let mcp_name = args
        .get("mcp_name")
        .or_else(|| args.get("server"))
        .and_then(Value::as_str)
        .unwrap_or("source_01");
    let tool_name = args
        .get("tool_name")
        .or_else(|| args.get("tool"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let tool_args = args.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let resolved = state
        .lazy_mcp_registry
        .lock()
        .resolve_call_mcp(mcp_name, tool_name, tool_args);
    match resolved {
        Ok((client_tool_name, normalized_args)) => ToolResult::ok(json!({
            "client_tool_name": client_tool_name,
            "arguments": normalized_args,
        }))
        .to_json(),
        Err(err) => ToolResult::<Value>::err(err.to_string()).to_json(),
    }
}

/// Dispatch `vox_battery_status` (`T3-STREAM-QUEUE-03`): return dual-unit BU + USD battery ledger and 4-hour/monthly quota windows.
pub async fn handle_battery_status(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_battery_status", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let ledger = state.battery_ledger.lock().clone();
    ToolResult::ok(json!({
        "remaining_bu": ledger.remaining_balance_bu(),
        "spent_bu": ledger.total_spent_bu(),
        "spent_usd_micros": ledger.total_spent_usd_micros(),
        "turns_recorded": ledger.transactions().len() as u64,
        "local_turns_recorded": ledger.local_turns_recorded(),
        "total_cache_read_tokens": ledger.total_cache_read_tokens,
        "four_hour_spent_bu": ledger.four_hour_spent_bu,
        "four_hour_limit_bu": ledger.four_hour_limit_bu,
        "four_hour_pct": ledger.four_hour_pct(),
        "four_hour_reset_ms": ledger.four_hour_reset_ms,
        "monthly_spent_bu": ledger.monthly_spent_bu,
        "monthly_limit_bu": ledger.monthly_limit_bu,
        "monthly_pct": ledger.monthly_pct(),
    }))
    .to_json()
}

/// Dispatch `vox_session_list_external` (`T3-TOOLS-HARDENING-03`): discover available Claude Code, Codex, and Cursor session files.
pub async fn handle_session_list_external(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_session_list_external", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let repo_root = match resolve_repo_root(state, &args) {
        Ok(p) => p,
        Err(err) => return ToolResult::<Value>::err(err).to_json(),
    };
    let source_filter = args.get("source_harness").and_then(Value::as_str);
    let root_override = if let Some(raw_root) = args
        .get("root_dir")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let raw_path = std::path::Path::new(raw_root);
        if raw_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return ToolResult::<Value>::err(
                "invalid `root_dir`: parent directory traversal is forbidden".to_string(),
            )
            .to_json();
        }
        let canon_dir = match raw_path.canonicalize() {
            Ok(p) => p,
            Err(err) => {
                return ToolResult::<Value>::err(format!("invalid `root_dir`: {err}")).to_json();
            }
        };
        let allowed_roots = allowed_external_session_roots(&repo_root);
        if !allowed_roots
            .iter()
            .any(|root| canon_dir == *root || canon_dir.starts_with(root))
        {
            return ToolResult::<Value>::err(
                "`root_dir` is outside allowed external session roots (`~/.claude`, `~/.codex`, `~/.cursor`, or repository root)".to_string(),
            )
            .to_json();
        }
        Some(canon_dir)
    } else {
        None
    };
    let sessions = list_external_sessions_catalog(source_filter, root_override.as_deref());
    ToolResult::ok(json!({
        "sessions": sessions,
    }))
    .to_json()
}

/// Dispatch `vox_guest_share_redeem` (`T3-TOOLS-HARDENING-03`): redeem a signed guest invite token.
pub async fn handle_guest_share_redeem(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_guest_share_redeem", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let token_pair = args
        .get("token")
        .and_then(Value::as_str)
        .and_then(|t| t.split_once(':'));
    let invite_id = args
        .get("invite_id")
        .or_else(|| args.get("token_id"))
        .and_then(Value::as_str)
        .or_else(|| token_pair.map(|(id, _)| id))
        .unwrap_or("invite-1");
    let signature_hex = args
        .get("signature_hex")
        .and_then(Value::as_str)
        .or_else(|| token_pair.map(|(_, sig)| sig))
        .unwrap_or("");
    let guest_session_id = args
        .get("guest_session_id")
        .and_then(Value::as_str)
        .unwrap_or("guest-browser");
    let now_ms = args
        .get("now_ms")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| crate::chat_tools::now_ts().saturating_mul(1000));
    let res = state.guest_share_service.lock().redeem_invite(
        invite_id,
        signature_hex,
        guest_session_id,
        now_ms,
    );
    match res {
        Ok(record) => {
            if record.revoked {
                return ToolResult::<Value>::err("guest share invite has been revoked".to_string())
                    .to_json();
            }
            let permission = normalize_guest_permission(Some(record.scope.as_str()));
            ToolResult::ok(json!({
                "invite_id": invite_id,
                "thread_id": record.thread_id,
                "session_id": record.thread_id,
                "permission": permission,
                "expires_at_ms": record.expires_at_ms,
                "guest_session_id": guest_session_id,
                "redeemed": true,
                "valid": true,
            }))
            .to_json()
        }
        Err(err) => ToolResult::<Value>::err(err.to_string()).to_json(),
    }
}

/// Dispatch `vox_guest_share_submit` (`T3-TOOLS-HARDENING-03`): enqueue a guest prompt for owner moderation.
pub async fn handle_guest_share_submit(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_guest_share_submit", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let Some((tok_inv, tok_sig)) = args
        .get("token")
        .and_then(Value::as_str)
        .map(str::trim)
        .and_then(|t| t.split_once(':'))
        .filter(|(inv, sig)| !inv.is_empty() && !sig.is_empty())
    else {
        return ToolResult::<Value>::err(
            "missing or malformed `token` (`invite_id:signature_hex` required)".to_string(),
        )
        .to_json();
    };
    let guest_session_id = args
        .get("guest_session_id")
        .and_then(Value::as_str)
        .unwrap_or("guest-browser");
    let guest_display_name = args
        .get("guest_author")
        .or_else(|| args.get("guest_display_name"))
        .or_else(|| args.get("guest_name"))
        .and_then(Value::as_str)
        .unwrap_or("Guest");
    let prompt = args.get("prompt").and_then(Value::as_str).unwrap_or("");
    let now_ms = args
        .get("now_ms")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| crate::chat_tools::now_ts().saturating_mul(1000));
    let res = {
        let mut svc = state.guest_share_service.lock();
        let rec = match svc.redeem_invite(tok_inv, tok_sig, guest_session_id, now_ms) {
            Ok(rec) => rec,
            Err(err) => return ToolResult::<Value>::err(err.to_string()).to_json(),
        };
        if rec.revoked {
            return ToolResult::<Value>::err("guest share invite has been revoked".to_string())
                .to_json();
        }
        if normalize_guest_permission(Some(rec.scope.as_str())) == "read_only" {
            return ToolResult::<Value>::err(
                "guest share invite is `read_only` and cannot submit prompts".to_string(),
            )
            .to_json();
        }
        let resolved_thread_id = rec.thread_id;
        let message_id = args
            .get("message_id")
            .or_else(|| args.get("submission_id"))
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| {
                let next_idx = svc.list_pending_messages(None).len() + 1;
                format!("guest-msg-{next_idx}")
            });
        svc.submit_guest_message(
            message_id,
            &resolved_thread_id,
            guest_session_id,
            guest_display_name,
            prompt,
        )
    };
    match res {
        Ok(msg) => ToolResult::ok(guest_submission_to_json(&msg)).to_json(),
        Err(err) => ToolResult::<Value>::err(err.to_string()).to_json(),
    }
}

/// Dispatch `vox_pr_review_guide` (`T3-TOOLS-HARDENING-04`): generate or retrieve cached structured PR review guide.
pub async fn handle_pr_review_guide(state: &ServerState, args: Value) -> String {
    if let Err(reason) = authorize_wave3_call("vox_pr_review_guide", &args) {
        return ToolResult::<Value>::err(reason).to_json();
    }
    let pr_number = args.get("pr_number").and_then(Value::as_u64).unwrap_or(1);
    let head_sha = args
        .get("head_sha")
        .and_then(Value::as_str)
        .unwrap_or("HEAD");
    let changed_files: Vec<String> = args
        .get("changed_files")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let guide =
        state
            .pr_review_guide_service
            .lock()
            .get_or_generate(pr_number, head_sha, &changed_files);
    ToolResult::ok(guide).to_json()
}

#[cfg(test)]
mod tests {
    use super::{
        InteractionAccessMode, RuntimeAccessMode, authorize_wave3_call_with_server_policy,
        default_server_capabilities,
    };
    use crate::dispatch::handle_tool_call;
    use crate::server_state::ServerState;
    use serde_json::{Value, json};

    #[test]
    fn authorize_wave3_call_only_narrows_server_posture_never_widens() {
        let all_caps = default_server_capabilities();
        let escalating_args = json!({
            "runtime_mode": "full_access",
            "interaction_mode": "agent",
            "granted_capabilities": ["spawn_threads", "vcs_worktree", "guest_moderation"]
        });

        // Server ReadOnly rejects mutating/spawning calls even when caller requests full_access + agent + all caps.
        assert!(
            authorize_wave3_call_with_server_policy(
                "vox_consensus_run",
                &escalating_args,
                RuntimeAccessMode::ReadOnly,
                InteractionAccessMode::Agent,
                &all_caps,
            )
            .is_err()
        );
        assert!(
            authorize_wave3_call_with_server_policy(
                "vox_stage_file",
                &escalating_args,
                RuntimeAccessMode::ReadOnly,
                InteractionAccessMode::Agent,
                &all_caps,
            )
            .is_err()
        );

        // Server Plan mode rejects Agent-level tools even when caller passes interaction_mode="agent".
        assert!(
            authorize_wave3_call_with_server_policy(
                "vox_checkpoint_capture",
                &escalating_args,
                RuntimeAccessMode::FullAccess,
                InteractionAccessMode::Plan,
                &all_caps,
            )
            .is_err()
        );

        // Server capability set without `guest_moderation` rejects caller attempting to inject it via args.
        let limited_server_caps = vec!["vcs_worktree".to_string()];
        assert!(
            authorize_wave3_call_with_server_policy(
                "vox_guest_share_mint",
                &escalating_args,
                RuntimeAccessMode::FullAccess,
                InteractionAccessMode::Agent,
                &limited_server_caps,
            )
            .is_err()
        );

        // Conversely, when server is FullAccess + Agent + all_caps, caller passing read_only narrows downward.
        let narrowing_args = json!({ "runtime_mode": "read_only" });
        assert!(
            authorize_wave3_call_with_server_policy(
                "vox_stage_file",
                &narrowing_args,
                RuntimeAccessMode::FullAccess,
                InteractionAccessMode::Agent,
                &all_caps,
            )
            .is_err()
        );
        assert!(
            authorize_wave3_call_with_server_policy(
                "vox_checkpoint_list",
                &narrowing_args,
                RuntimeAccessMode::FullAccess,
                InteractionAccessMode::Agent,
                &all_caps,
            )
            .is_ok()
        );
    }

    #[tokio::test]
    async fn wave3_mcp_tools_dispatch_end_to_end() {
        let state = ServerState::new_test().await;

        // 1. Consensus run (with precomputed legs so test is deterministic offline/with any keychain state)
        let consensus_raw = handle_tool_call(
            &state,
            "vox_consensus_run",
            json!({
                "prompt": "How does VoxIdAllocatorV2 derive IDs?",
                "models": ["anthropic/claude-sonnet-4-5", "openai/gpt-4o", "mens-qwen3-8b"],
                "legs": [
                    { "model_id": "anthropic/claude-sonnet-4-5", "content": "BLAKE3 keyed derivation over namespace + parts." },
                    { "model_id": "openai/gpt-4o", "content": "Deterministic BLAKE3 derivation with monotonic prefix." },
                    { "model_id": "mens-qwen3-8b", "content": "Uses VoxIdAllocatorV2::derive over structured components." }
                ]
            }),
        )
        .await
        .expect("consensus_run dispatch");
        let consensus_val: Value = serde_json::from_str(&consensus_raw).expect("json");
        assert_eq!(consensus_val["success"], true);
        assert_eq!(
            consensus_val["data"]["rich_results"]
                .as_array()
                .unwrap()
                .len(),
            3
        );

        // 2. Context handoff
        let handoff_raw = handle_tool_call(
            &state,
            "vox_context_handoff",
            json!({
                "source_thread_id": "t-1",
                "source_provider": "anthropic",
                "target_provider": "openai",
                "turns": [{"role": "user", "content": "Wire checkpoint store", "is_plan_or_code": true}]
            }),
        )
        .await
        .expect("context_handoff dispatch");
        let handoff_val: Value = serde_json::from_str(&handoff_raw).expect("json");
        assert_eq!(handoff_val["success"], true);
        assert!(
            handoff_val["data"]["formatted_prompt"]
                .as_str()
                .unwrap()
                .contains("<context_handoff>")
        );

        // 3. Muse pool spawn & adopt
        let muse_raw = handle_tool_call(
            &state,
            "vox_muse_pool_adopt",
            json!({
                "run_id": "muse-test",
                "model_ids": ["m1", "m2"],
                "variant_index": 2
            }),
        )
        .await
        .expect("muse_pool_adopt dispatch");
        let muse_val: Value = serde_json::from_str(&muse_raw).expect("json");
        assert_eq!(muse_val["success"], true);
        assert_eq!(muse_val["data"]["winner"]["variant_index"], 2);

        // 4. Guest share mint & moderate
        let guest_raw = handle_tool_call(
            &state,
            "vox_guest_share_mint",
            json!({ "invite_id": "inv-99", "thread_id": "t-99" }),
        )
        .await
        .expect("guest_share_mint dispatch");
        let guest_val: Value = serde_json::from_str(&guest_raw).expect("json");
        assert_eq!(guest_val["success"], true);
        assert_eq!(guest_val["data"]["invite_id"], "inv-99");

        // Moderating a non-existent pending message must fail honestly without fabricating an invite/submission.
        let mod_missing_raw = handle_tool_call(
            &state,
            "vox_guest_share_moderate",
            json!({ "message_id": "does-not-exist", "action": "approve" }),
        )
        .await
        .expect("guest_share_moderate dispatch");
        let mod_missing_val: Value = serde_json::from_str(&mod_missing_raw).expect("json");
        assert_eq!(mod_missing_val["success"], false);

        // 5. External session import & bundle round-trip
        let import_raw = handle_tool_call(
            &state,
            "vox_session_import_external",
            json!({
                "source_harness": "claude",
                "external_session_id": "claude-42",
                "raw_jsonl": "{\"role\":\"user\",\"content\":\"Hello from Claude Code\"}\n{\"role\":\"assistant\",\"content\":\"Hi from Vox\"}"
            }),
        )
        .await
        .expect("session_import_external dispatch");
        let import_val: Value = serde_json::from_str(&import_raw).expect("json");
        assert_eq!(import_val["success"], true);
        assert_eq!(import_val["data"]["messages"].as_array().unwrap().len(), 2);

        // 6. Quarantine set, list, and ServerState::is_path_quarantined enforcement (`URBS-QUARANTINE-01..04`)
        let q_set_raw = handle_tool_call(
            &state,
            "vox_quarantine_set",
            json!({ "path_prefix": "crates/vox-crypto", "quarantined": true }),
        )
        .await
        .expect("quarantine_set dispatch");
        let q_set_val: Value = serde_json::from_str(&q_set_raw).expect("json");
        assert_eq!(q_set_val["success"], true);
        assert_eq!(
            state.is_path_quarantined("crates/vox-crypto/src/lib.rs"),
            Some("crates/vox-crypto".to_string())
        );
        assert_eq!(
            state.is_path_quarantined("crates/vox-gui/src/main.rs"),
            None
        );

        let q_list_raw = handle_tool_call(&state, "vox_quarantine_list", json!({}))
            .await
            .expect("quarantine_list dispatch");
        let q_list_val: Value = serde_json::from_str(&q_list_raw).expect("json");
        assert_eq!(
            q_list_val["data"]["quarantined_paths"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn guest_share_list_and_aligned_dto_fields_round_trip() {
        let state = ServerState::new_test().await;

        // 1. Mint a read_only invite and verify redeem returns string thread_id while submit is rejected.
        let mint_ro_raw = handle_tool_call(
            &state,
            "vox_guest_share_mint",
            json!({
                "session_id": "sess-433",
                "permission": "read_only",
                "ttl_ms": 60_000
            }),
        )
        .await
        .expect("guest_share_mint dispatch");
        let mint_ro_val: Value = serde_json::from_str(&mint_ro_raw).expect("json");
        assert_eq!(mint_ro_val["success"], true);
        let ro_token = mint_ro_val["data"]["token"]
            .as_str()
            .expect("token")
            .to_string();
        assert!(ro_token.contains(':'));
        assert_eq!(mint_ro_val["data"]["session_id"], "sess-433");
        assert_eq!(mint_ro_val["data"]["thread_id"], "sess-433");
        assert_eq!(mint_ro_val["data"]["permission"], "read_only");
        assert_eq!(mint_ro_val["data"]["scope"], "read_only");
        assert_eq!(mint_ro_val["data"]["revoked"], false);

        let redeem_ro_raw = handle_tool_call(
            &state,
            "vox_guest_share_redeem",
            json!({ "token": ro_token, "guest_session_id": "guest-ro" }),
        )
        .await
        .expect("guest_share_redeem dispatch");
        let redeem_ro_val: Value = serde_json::from_str(&redeem_ro_raw).expect("json");
        assert_eq!(redeem_ro_val["success"], true);
        assert_eq!(redeem_ro_val["data"]["thread_id"], "sess-433");
        assert_eq!(redeem_ro_val["data"]["permission"], "read_only");

        let submit_ro_raw = handle_tool_call(
            &state,
            "vox_guest_share_submit",
            json!({
                "token": ro_token,
                "guest_session_id": "guest-ro",
                "guest_name": "Reader",
                "prompt": "Attempt write on read_only token"
            }),
        )
        .await
        .expect("guest_share_submit read_only dispatch");
        let submit_ro_val: Value = serde_json::from_str(&submit_ro_raw).expect("json");
        assert_eq!(submit_ro_val["success"], false);

        // Missing token must also be rejected.
        let submit_unauth_raw = handle_tool_call(
            &state,
            "vox_guest_share_submit",
            json!({
                "thread_id": "sess-433",
                "guest_name": "Anon",
                "prompt": "No token supplied"
            }),
        )
        .await
        .expect("guest_share_submit unauth dispatch");
        let submit_unauth_val: Value = serde_json::from_str(&submit_unauth_raw).expect("json");
        assert_eq!(submit_unauth_val["success"], false);

        // 2. Mint a comment / suggest_prompt invite and succeed through submit -> list -> moderate.
        let mint_comment_raw = handle_tool_call(
            &state,
            "vox_guest_share_mint",
            json!({
                "session_id": "sess-433",
                "permission": "comment",
                "ttl_ms": 60_000
            }),
        )
        .await
        .expect("guest_share_mint comment dispatch");
        let mint_comment_val: Value = serde_json::from_str(&mint_comment_raw).expect("json");
        assert_eq!(mint_comment_val["success"], true);
        assert_eq!(mint_comment_val["data"]["permission"], "suggest_prompt");
        let comment_token = mint_comment_val["data"]["token"]
            .as_str()
            .expect("comment token")
            .to_string();

        let submit_raw = handle_tool_call(
            &state,
            "vox_guest_share_submit",
            json!({
                "token": comment_token,
                "guest_session_id": "guest-commenter",
                "guest_name": "Reviewer",
                "prompt": "Check IPC boundary"
            }),
        )
        .await
        .expect("guest_share_submit dispatch");
        let submit_val: Value = serde_json::from_str(&submit_raw).expect("json");
        assert_eq!(submit_val["success"], true);
        assert_eq!(submit_val["data"]["session_id"], "sess-433");
        assert_eq!(submit_val["data"]["guest_name"], "Reviewer");
        assert_eq!(submit_val["data"]["status"], "pending");
        let submission_id = submit_val["data"]["submission_id"]
            .as_str()
            .expect("submission_id")
            .to_string();

        let list_raw = handle_tool_call(
            &state,
            "vox_guest_share_list",
            json!({ "session_id": "sess-433" }),
        )
        .await
        .expect("guest_share_list dispatch");
        let list_val: Value = serde_json::from_str(&list_raw).expect("json");
        assert_eq!(list_val["success"], true);
        let invites = list_val["data"]["invites"].as_array().expect("invites");
        let submissions = list_val["data"]["submissions"]
            .as_array()
            .expect("submissions");
        assert_eq!(invites.len(), 2);
        assert_eq!(submissions.len(), 1);
        assert_eq!(submissions[0]["submission_id"], submission_id);
        assert_eq!(submissions[0]["guest_name"], "Reviewer");
        assert_eq!(submissions[0]["prompt"], "Check IPC boundary");

        let moderate_raw = handle_tool_call(
            &state,
            "vox_guest_share_moderate",
            json!({
                "submission_id": submission_id,
                "action": "approve"
            }),
        )
        .await
        .expect("guest_share_moderate dispatch");
        let moderate_val: Value = serde_json::from_str(&moderate_raw).expect("json");
        assert_eq!(moderate_val["success"], true);
        assert_eq!(moderate_val["data"]["status"], "approved");

        let adopt_raw = handle_tool_call(
            &state,
            "vox_muse_pool_adopt",
            json!({
                "run_id": "muse-433",
                "model_ids": ["alpha", "beta", "gamma"],
                "winning_variant_id": "muse-433-v2"
            }),
        )
        .await
        .expect("muse_pool_adopt dispatch");
        let adopt_val: Value = serde_json::from_str(&adopt_raw).expect("json");
        assert_eq!(adopt_val["success"], true);
        assert_eq!(adopt_val["data"]["run_id"], "muse-433");
        assert_eq!(adopt_val["data"]["winner"]["variant_index"], 2);
        assert_eq!(adopt_val["data"]["adopted_variant_id"], "muse-433-v2");
        assert_eq!(
            adopt_val["data"]["adopted_branch_name"],
            "vox/muse/muse-433-v2"
        );
        assert_eq!(
            adopt_val["data"]["pruned_variant_ids"],
            json!(["muse-433-v1", "muse-433-v3"])
        );
    }

    #[tokio::test]
    async fn path_containment_quarantine_and_honest_errors_hold() {
        let mut state = ServerState::new_test().await;
        let tmp_dir = std::env::temp_dir().join(format!(
            "vox-wave3-sec-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp_dir).expect("mkdir tmp");
        let canon_tmp = tmp_dir.canonicalize().expect("canon tmp");
        state.repository.root = canon_tmp.clone();
        state.repository.git_root = Some(canon_tmp.clone());
        state.workspace_root = Some(canon_tmp.clone());

        // 1. Escape repo_root rejected.
        let escape_repo_raw = handle_tool_call(
            &state,
            "vox_checkpoint_list",
            json!({ "repo_root": "/etc" }),
        )
        .await
        .expect("checkpoint_list escape");
        let escape_repo_val: Value = serde_json::from_str(&escape_repo_raw).expect("json");
        assert_eq!(escape_repo_val["success"], false);

        // 2. Invalid run_id slug rejected in muse_pool_spawn and muse_pool_adopt.
        for tool in ["vox_muse_pool_spawn", "vox_muse_pool_adopt"] {
            let bad_slug_raw = handle_tool_call(&state, tool, json!({ "run_id": "../escape" }))
                .await
                .expect("muse bad slug");
            let bad_slug_val: Value = serde_json::from_str(&bad_slug_raw).expect("json");
            assert_eq!(bad_slug_val["success"], false);
        }

        // 3. External session import/list reject outside paths and non-.jsonl files, and allow repo_root .jsonl.
        let bad_ext_raw = handle_tool_call(
            &state,
            "vox_session_import_external",
            json!({ "file_path": "/etc/passwd" }),
        )
        .await
        .expect("import /etc/passwd");
        let bad_ext_val: Value = serde_json::from_str(&bad_ext_raw).expect("json");
        assert_eq!(bad_ext_val["success"], false);

        let env_file = canon_tmp.join(".env");
        std::fs::write(&env_file, "SECRET=1\n").expect("write .env");
        let bad_env_raw = handle_tool_call(
            &state,
            "vox_session_import_external",
            json!({ "file_path": env_file.to_string_lossy() }),
        )
        .await
        .expect("import .env");
        let bad_env_val: Value = serde_json::from_str(&bad_env_raw).expect("json");
        assert_eq!(bad_env_val["success"], false);

        let missing_jsonl = canon_tmp.join("missing.jsonl");
        let missing_ext_raw = handle_tool_call(
            &state,
            "vox_session_import_external",
            json!({ "file_path": missing_jsonl.to_string_lossy() }),
        )
        .await
        .expect("import missing.jsonl");
        let missing_ext_val: Value = serde_json::from_str(&missing_ext_raw).expect("json");
        assert_eq!(missing_ext_val["success"], false);

        let valid_jsonl = canon_tmp.join("valid-session.jsonl");
        std::fs::write(
            &valid_jsonl,
            "{\"role\":\"user\",\"content\":\"From repo jsonl\"}\n",
        )
        .expect("write valid jsonl");
        let ok_ext_raw = handle_tool_call(
            &state,
            "vox_session_import_external",
            json!({ "file_path": valid_jsonl.to_string_lossy() }),
        )
        .await
        .expect("import valid.jsonl");
        let ok_ext_val: Value = serde_json::from_str(&ok_ext_raw).expect("json");
        assert_eq!(ok_ext_val["success"], true);

        let bad_list_raw = handle_tool_call(
            &state,
            "vox_session_list_external",
            json!({ "root_dir": "/etc" }),
        )
        .await
        .expect("list /etc");
        let bad_list_val: Value = serde_json::from_str(&bad_list_raw).expect("json");
        assert_eq!(bad_list_val["success"], false);

        // 4. Quarantine enforcement + file_path containment + diagnostics file/column fields.
        let src_dir = canon_tmp.join("src");
        std::fs::create_dir_all(&src_dir).expect("mkdir src");
        std::fs::write(src_dir.join("stub.rs"), "fn f() { todo!() }\n").expect("write stub.rs");

        let diag_raw = handle_tool_call(
            &state,
            "vox_file_diff_and_diagnostics",
            json!({ "file_path": "src/stub.rs" }),
        )
        .await
        .expect("file_diff_and_diagnostics");
        let diag_val: Value = serde_json::from_str(&diag_raw).expect("json");
        assert_eq!(diag_val["success"], true);
        let diags = diag_val["data"]["diagnostics"].as_array().expect("diags");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0]["file"], "src/stub.rs");
        assert_eq!(diags[0]["line"], 1);
        assert_eq!(diags[0]["column"], 1);

        let _ = handle_tool_call(
            &state,
            "vox_quarantine_set",
            json!({ "path_prefix": "src", "quarantined": true }),
        )
        .await;
        for mut_tool in ["vox_stage_file", "vox_revert_file"] {
            let q_block_raw =
                handle_tool_call(&state, mut_tool, json!({ "file_path": "src/stub.rs" }))
                    .await
                    .expect("quarantined file op");
            let q_block_val: Value = serde_json::from_str(&q_block_raw).expect("json");
            assert_eq!(q_block_val["success"], false);
        }

        // Unquarantined git add/checkout in non-git dir must propagate git failure as success=false!
        let _ = handle_tool_call(
            &state,
            "vox_quarantine_set",
            json!({ "path_prefix": "src", "quarantined": false }),
        )
        .await;
        let stage_fail_raw = handle_tool_call(
            &state,
            "vox_stage_file",
            json!({ "file_path": "src/stub.rs" }),
        )
        .await
        .expect("stage_file in non-git dir");
        let stage_fail_val: Value = serde_json::from_str(&stage_fail_raw).expect("json");
        assert_eq!(stage_fail_val["success"], false);

        // Missing checkpoint ref in vcs_checkpoint_diff_summary must return success=false!
        let ckpt_fail_raw = handle_tool_call(
            &state,
            "vox_vcs_checkpoint_diff_summary",
            json!({ "thread_id": "t-none", "target_turn": 99 }),
        )
        .await
        .expect("vcs_checkpoint_diff_summary");
        let ckpt_fail_val: Value = serde_json::from_str(&ckpt_fail_raw).expect("json");
        assert_eq!(ckpt_fail_val["success"], false);

        // Consensus run with all failing legs must return success=false, while partial leg failure records status="failed".
        let consensus_all_fail_raw = handle_tool_call(
            &state,
            "vox_consensus_run",
            json!({
                "prompt": "",
                "live_infer": true,
                "models": ["m-a", "m-b"]
            }),
        )
        .await
        .expect("consensus all fail");
        let consensus_all_fail_val: Value =
            serde_json::from_str(&consensus_all_fail_raw).expect("json");
        assert_eq!(consensus_all_fail_val["success"], false);

        let consensus_partial_raw = handle_tool_call(
            &state,
            "vox_consensus_run",
            json!({
                "prompt": "",
                "live_infer": true,
                "models": ["m-ok", "m-fail"],
                "legs": [
                    { "model_id": "m-ok", "content": "Real leg output." }
                ]
            }),
        )
        .await
        .expect("consensus partial");
        let consensus_partial_val: Value =
            serde_json::from_str(&consensus_partial_raw).expect("json");
        assert_eq!(consensus_partial_val["success"], true);
        let rich = consensus_partial_val["data"]["rich_results"]
            .as_array()
            .expect("rich_results");
        assert_eq!(rich[0]["status"], "completed");
        assert_eq!(rich[1]["status"], "failed");
        assert_eq!(rich[1]["content"], "");
        assert!(rich[1]["error"].as_str().is_some());

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
