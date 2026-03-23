//! Chat, inline edit, and planning tools for the Vox MCP server.
//!
//! These back the VS Code extension thin-client layer. All context gathering,
//! @mention resolution, LLM routing, and history persistence happen here in Rust.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use super::chat_model_resolve::resolve_chat_llm_model;
use super::chat_socrates_meta::{
    socrates_system_rider, socrates_tool_meta, spawn_socrates_telemetry,
};
use crate::llm_bridge::{
    McpChatModelResolution, McpInferRouting, call_llm, clamp_http_max_output_tokens,
    mcp_infer_completion,
};
use crate::params::ToolResult;
use crate::server::ServerState;
use regex::Regex;
use std::sync::LazyLock;
use turso::params;
use vox_orchestrator::types::AgentId;
use vox_socrates_policy::ConfidencePolicy;
use chrono;

static MENTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@([A-Za-z0-9_.:/\\-]+)").unwrap());
// Regexes for @mentions. TASK_RE and SUMMARY_RE were removed in favor of strict JSON schema decoding.
pub const ANTI_LAZINESS_RIDER: &str = "\nCRITICAL DIRECTIVE: You must output the COMPLETE, fully-implemented replacement code. DO NOT under any circumstances use placeholders, stubs, 'TODOs', or elide implementation details. Writing partial code is a catastrophic failure.";

// ─── Types ───────────────────────────────────────────────────────────────────

/// One persisted chat turn in the session transcript (also returned in history APIs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTranscriptEntry {
    /// Opaque message id (UUID/ulid string).
    pub id: String,
    /// `"user"`, `"assistant"`, or `"system"`.
    pub role: String, // "user" | "assistant" | "system"
    /// Message body after expansion (mentions resolved server-side).
    pub content: String,
    /// Epoch seconds when stored.
    pub timestamp: u64,
    /// Extra files pulled in via @mentions or explicit attachments.
    pub context_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Model id recorded for assistant turns.
    pub model_used: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Approximate token usage when available.
    pub tokens: Option<u64>,
}

/// Arguments for `vox_chat_message` (prompt + rich editor context).
#[derive(Debug, Deserialize)]
pub struct ChatMessageParams {
    /// User message text (`message` is accepted for registry / legacy clients).
    #[serde(alias = "message")]
    pub prompt: String,
    #[serde(default)]
    /// Explicit @mention or attachment paths from the client.
    pub context_files: Vec<String>,
    /// Open file paths provided by the editor for implicit context injection
    #[serde(default)]
    pub open_files: Vec<String>,
    /// Active editor file path (workspace-relative)
    #[serde(default)]
    pub active_file: Option<String>,
    /// Active editor cursor line (1-indexed)
    #[serde(default)]
    pub active_line: Option<u32>,
    /// Selected text in the active editor
    #[serde(default)]
    pub selected_text: Option<String>,
    /// Active LSP diagnostics to inject as context
    #[serde(default)]
    pub diagnostics: Vec<Value>,
    /// Optional logical grouping identifier for this chat thread.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Optionally selects a specific LLM routing profile (e.g. "reasoning", "fast", "creative").
    #[serde(default)]
    pub cognitive_profile: Option<String>,
    /// If true, enforces strict JSON output from the LLM.
    #[serde(default)]
    pub json_mode: bool,
}

/// Retrieve history for a specific session ID.
#[derive(Debug, Deserialize)]
pub struct ChatHistoryParams {
    /// Logical grouping identifier to fetch history for.
    pub session_id: String,
}

/// Arguments for `vox_inline_edit` (range replacement inside one file).
#[derive(Debug, Deserialize)]
pub struct InlineEditParams {
    /// The edit instruction / prompt from the user (`instruction` is a legacy alias).
    #[serde(alias = "instruction")]
    pub prompt: String,
    /// Workspace-relative file path (`file_path` is a legacy alias).
    #[serde(alias = "file_path")]
    pub file: String,
    /// Start line of target range (1-indexed)
    pub start_line: u32,
    /// End line of target range (1-indexed, inclusive)
    pub end_line: u32,
    /// The current text in the range (sent by editor; `selection` is a legacy alias).
    #[serde(alias = "selection")]
    pub current_text: String,
    /// Language ID of the file
    #[serde(default)]
    pub language: Option<String>,
    /// Surrounding context lines before and after the range (0-40 lines typically)
    #[serde(default)]
    pub context_before: Option<String>,
    #[serde(default)]
    /// Optional lines after the selection for better LLM grounding.
    pub context_after: Option<String>,
    /// If true, enforces strict JSON output from the LLM (rarely used for raw code edits).
    #[serde(default)]
    pub json_mode: bool,
}

/// Successful inline edit payload returned to the editor host.
#[derive(Debug, Serialize)]
pub struct InlineEditResult {
    /// Replacement text for the range [`start_line`, `end_line`]
    pub replacement: String,
    /// Human-readable explanation of what was changed
    pub explanation: String,
    /// Estimated token usage
    pub tokens: u64,
    /// Model that produced this edit
    pub model_used: String,
}

/// Arguments for `vox_plan` structured planning tool.
#[derive(Debug, Deserialize)]
pub struct PlanParams {
    /// The request / goal to plan for
    pub goal: String,
    /// Optional files to scope the plan to
    #[serde(default)]
    pub scope_files: Vec<String>,
    /// Whether to write the plan to PLAN.md in the workspace root
    #[serde(default)]
    pub write_to_disk: bool,
    /// Maximum number of tasks to generate (default: 30)
    #[serde(default)]
    pub max_tasks: Option<usize>,
}

/// Arguments for `vox_replan` — forwards to DeI `ai.plan.replan` when `vox-dei-d` is available.
#[derive(Debug, Deserialize)]
pub struct PlanReplanParams {
    /// Session id from a prior `vox_plan` or `ai.plan.new`.
    pub session_id: String,
    /// What changed since the last plan version.
    pub delta_hint: String,
    #[serde(default)]
    pub write_to_disk: bool,
    #[serde(default)]
    pub mode: Option<String>,
}

/// Arguments for `vox_plan_status` — forwards to DeI `ai.plan.status`.
#[derive(Debug, Deserialize)]
pub struct PlanStatusParams {
    /// Plan session id to query.
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// One row inside a generated plan (dependencies + complexity estimate).
pub struct PlanTask {
    /// Monotonic task index inside the plan.
    pub id: usize,
    /// Short imperative description.
    pub description: String,
    /// Related file paths for affinity routing.
    pub files: Vec<String>,
    /// Heuristic difficulty on a 1-10 scale.
    pub estimated_complexity: u8, // 1-10
    /// Task ids that should complete first.
    pub depends_on: Vec<usize>,
}

#[derive(Debug, Serialize)]
/// Full structured plan returned to the IDE / LLM.
pub struct PlanResult {
    /// Echo of the user goal string.
    pub goal: String,
    /// Ordered task breakdown.
    pub tasks: Vec<PlanTask>,
    /// One-line executive summary.
    pub summary: String,
    /// Markdown document (may mirror on-disk `PLAN.md`).
    pub plan_md: String,
    /// Whether `PLAN.md` was written under the workspace root.
    pub written_to_disk: bool,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Simple ISO date formatter (YYYY-MM-DD) without external chrono/time deps.
fn ts_to_date_str(secs: u64) -> String {
    let days = secs / 86400;
    // Base 1970-01-01 was a Thursday
    // Simple proleptic Gregorian algorithm (good until 2100)
    let z = (days as i64) + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", y + if m <= 2 { 1 } else { 0 }, m, d)
}

fn ghost_grounding_score(params: &GhostTextParams) -> f64 {
    let mut n = 0u32;
    if params.file_path.is_some() {
        n += 1;
    }
    if !params.prefix.trim().is_empty() {
        n += 1;
    }
    if !params.suffix.trim().is_empty() {
        n += 1;
    }
    (0.50 + 0.12 * f64::from(n.min(3))).min(0.88)
}

fn chat_grounding_score(params: &ChatMessageParams, mention_count: usize) -> f64 {
    let mut n = 0u32;
    if !params.open_files.is_empty() {
        n += 1;
    }
    if params.active_file.is_some() {
        n += 1;
    }
    if !params.diagnostics.is_empty() {
        n += 1;
    }
    n += (mention_count.min(5)) as u32;
    (0.52 + 0.07 * f64::from(n)).min(0.94)
}

fn rebuild_mention_basename_index(
    workspace_root: &std::path::Path,
) -> std::collections::HashMap<String, std::path::PathBuf> {
    let mut map = std::collections::HashMap::new();
    for entry in walkdir::WalkDir::new(workspace_root)
        .follow_links(false)
        .max_depth(10)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let entry_path = entry.path().to_path_buf();
        let Some(name) = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        map.entry(name).or_insert(entry_path);
    }
    map
}

/// Resolve @filename mentions using a cached basename → path index (refreshed when workspace changes).
fn resolve_mentions(
    prompt: &str,
    workspace_root: &std::path::Path,
    cache: &std::sync::Arc<
        std::sync::Mutex<
            Option<(
                std::path::PathBuf,
                std::sync::Arc<std::collections::HashMap<String, std::path::PathBuf>>,
            )>,
        >,
    >,
) -> (String, Vec<String>) {
    let mut expanded = prompt.to_string();
    let mut resolved_files = Vec::new();

    let index: std::sync::Arc<std::collections::HashMap<String, std::path::PathBuf>> = {
        let mut guard = match cache.lock() {
            Ok(g) => g,
            Err(_) => {
                return (expanded, resolved_files);
            }
        };
        let need_rebuild = guard
            .as_ref()
            .map(|(root, _)| root != workspace_root)
            .unwrap_or(true);
        if need_rebuild {
            let m = rebuild_mention_basename_index(workspace_root);
            *guard = Some((workspace_root.to_path_buf(), std::sync::Arc::new(m)));
        }
        guard
            .as_ref()
            .map(|(_, m)| std::sync::Arc::clone(m))
            .unwrap_or_else(|| std::sync::Arc::new(std::collections::HashMap::new()))
    };

    for cap in MENTION_RE.captures_iter(prompt) {
        let filename = &cap[1];
        let found = index.get(filename).cloned().or_else(|| {
            index.iter().find_map(|(_base, path)| {
                let rel = path
                    .strip_prefix(workspace_root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/");
                if rel == filename || rel.ends_with(filename) {
                    Some(path.clone())
                } else {
                    None
                }
            })
        });
        if let Some(path) = found
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            let rel = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let truncated = if content.len() > 8000 {
                format!("{}\n...[truncated]...", &content[..8000])
            } else {
                content.clone()
            };
            let replacement = format!("\n\n--- @{filename} ({rel}) ---\n{truncated}\n---\n");
            expanded = expanded.replace(&cap[0], &replacement);
            resolved_files.push(rel);
        }
    }
    (expanded, resolved_files)
}

/// Build the full system prompt for the Vox chat assistant.
async fn build_system_prompt(state: &ServerState) -> String {
    let ws_root = state
        .workspace_root
        .as_deref()
        .unwrap_or(std::path::Path::new("."));

    let mut prompt = String::from(
        "You are assisting with the **Vox** programming language and its ecosystem. \
         Vox is AI-native, full-stack, and compiles to Rust/TypeScript/WASM. \
         Prefer `Option[T]` and explicit errors over null.\n\n",
    );

    for rel in ["VOX.md", ".vox/MEMORY.md"] {
        let p = ws_root.join(rel);
        if let Ok(content) = std::fs::read_to_string(&p) {
            prompt.push_str("## ");
            prompt.push_str(rel);
            prompt.push_str("\n\n");
            prompt.push_str(&content);
            prompt.push_str("\n\n");
        }
    }

    prompt.push_str(&format!(
        "## Environment\nWorkspace Root: {}\n\nYou are Vox, an elite AI coding assistant. You have access to the Vox MCP toolbelt. You can read and modify files, run tests, inspect VCS history, manage agents, and query the knowledge graph.\n\nRules:\n- Be concise and precise. Prefer code over prose.\n- Always cite which files you modified or plan to modify.\n- When generating code, produce valid, complete implementations — no stubs or placeholders.\n- Use Markdown code blocks with language tags.\n- For multi-file changes, use a structured diff or list each file separately.\n- When asked to plan, produce a numbered task list in Markdown.\n",
        ws_root.display()
    ));

    prompt.push_str(ANTI_LAZINESS_RIDER);

    let ts = now_ts();
    let date_str = ts_to_date_str(ts);
    let last_call = {
        let orch = state.orchestrator.lock().await;
        orch.last_activity_ms() / 1000
    };
    let server_idle_secs = ts.saturating_sub(last_call);

    prompt.push_str(&format!(
        "\n\n## Temporal Context\nCurrent date: {date_str}.\nUnix timestamp: {ts}s.\n\
         Server last active: {server_idle_secs}s ago.\n\
         **Enforcement**: Before triggering any compilation, re-reindexing, or full file walk, \
         check if things are fresh (< 30s since last run).\n"
    ));

    let pol = state.orchestrator_config.effective_socrates_policy();
    prompt.push_str(&socrates_system_rider(&pol));
    prompt
}

// ─── Tool Handlers ────────────────────────────────────────────────────────────

/// Handle a user chat message. Resolves @mentions, injects context from the editor,
/// calls the best available LLM, persists to session history, and returns the updated history.
///
/// **Session Isolation**: History is keyed by `params.session_id` (defaulting to `"default"`).
/// Each unique session_id maintains a completely independent chat transcript in the
/// orchestrator `ContextStore`. Pass a stable UUID/slug per-window to prevent context bleeding.
///
/// **Autonomous Research**: Before invoking the LLM, this function silently queries the
/// `MemoryManager` and knowledge graph for facts related to the prompt. High-relevance hits
/// are injected as `[AUTONOMOUS RESEARCH]` preamble blocks so the model has evidence without
/// the user needing to explicitly invoke search tools.
///
/// **Cognitive Profile Routing**: Pass `"fast"`, `"reasoning"`, or `"creative"` to influence
/// model selection and temperature without changing the MCP tool contract.
pub async fn chat_message(state: &ServerState, params: ChatMessageParams) -> String {
    // 1. Resolve @mentions in the prompt
    let workspace_root = state
        .workspace_root
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let (expanded_prompt, mention_files) =
        resolve_mentions(&params.prompt, &workspace_root, &state.mention_path_cache);
    let mention_count = mention_files.len();

    // 2a. Build context preamble from editor state
    let mut context_parts = Vec::new();

    if let Some(active_file) = &params.active_file {
        let line_info = params
            .active_line
            .map(|l| format!(" (line {l})"))
            .unwrap_or_default();
        context_parts.push(format!("[ACTIVE FILE]: {active_file}{line_info}"));
    }

    if let Some(selected) = &params.selected_text
        && !selected.is_empty()
    {
        context_parts.push(format!("[SELECTED TEXT]:\n{selected}"));
    }

    if !params.diagnostics.is_empty() {
        let diag_str: Vec<String> = params
            .diagnostics
            .iter()
            .filter_map(|d| {
                let msg = d["message"].as_str()?;
                let line = d["line"].as_u64().unwrap_or(0);
                let sev = d["severity"].as_str().unwrap_or("error");
                Some(format!("  Line {line} [{sev}]: {msg}"))
            })
            .collect();
        if !diag_str.is_empty() {
            context_parts.push(format!(
                "[ACTIVE ERRORS/WARNINGS]:\n{}",
                diag_str.join("\n")
            ));
        }
    }

    if !params.open_files.is_empty() {
        context_parts.push(format!("[OPEN FILES]: {}", params.open_files.join(", ")));
    }

    // 2b. Autonomous Research Injection:
    // Silently query MemoryManager (MEMORY.md + daily logs) for facts related to the
    // expanded prompt. Inject top-3 hits as a labelled preamble block so the LLM has
    // project-local evidence without the user needing to call `vox_memory_search`.
    let mem_config = state.orchestrator_config.memory.clone();
    if let Ok(mgr) = vox_orchestrator::MemoryManager::new(mem_config) {
        if let Ok(hits) = mgr.search(&expanded_prompt) {
            let relevant: Vec<_> = hits.into_iter().take(3).collect();
            if !relevant.is_empty() {
                let snippets = relevant
                    .iter()
                    .map(|h| format!("- [{}:{}] {}", h.source, h.line, h.content))
                    .collect::<Vec<_>>()
                    .join("\n");
                context_parts.push(format!(
                    "[AUTONOMOUS RESEARCH — MEMORY.md]:\n{snippets}"
                ));
                tracing::debug!(
                    target: "vox_mcp::autonomous_research",
                    hits = relevant.len(),
                    "memory search injected into chat context"
                );
            }
        }
    }

    // 2c. Autonomous Knowledge Graph Search:
    // When Codex/VoxDb is attached, also probe the knowledge graph for related concepts.
    if let Some(ref db) = state.db {
        match db.query_knowledge_nodes(&expanded_prompt, 3).await {
            Ok(nodes) if !nodes.is_empty() => {
                let formatted = nodes
                    .into_iter()
                    .map(|(id, ntype, label)| format!("- [node:{id}] {label} ({ntype})"))
                    .collect::<Vec<_>>()
                    .join("\n");
                context_parts.push(format!(
                    "[AUTONOMOUS RESEARCH — KNOWLEDGE GRAPH]:\n{formatted}"
                ));
                tracing::debug!(
                    target: "vox_mcp::autonomous_research",
                    "knowledge graph nodes injected into chat context"
                );
            }
            Ok(_) => {}
            Err(e) => {
                tracing::debug!(
                    target: "vox_mcp::autonomous_research",
                    error = %e,
                    "knowledge graph query failed — skipping injection"
                );
            }
        }
    }

    let all_context_files: Vec<String> = {
        let mut v = params.context_files.clone();
        v.extend(mention_files);
        v.dedup();
        v
    };

    let user_prompt = if context_parts.is_empty() {
        expanded_prompt.clone()
    } else {
        format!("{}\n\n{}", context_parts.join("\n"), expanded_prompt)
    };

    // 3. Call LLM with cognitive-profile aware routing.
    // When cognitive_profile is set we use mcp_infer_completion() with an explicit
    // resolution template — the same pattern already used by inline_edit() and ghost_text().
    let session_id = params.session_id.as_deref().unwrap_or("default");
    let session_ts = {
        let orch = state.orchestrator.lock().await;
        orch.context()
            .age_secs(&format!("chat_history:{session_id}"))
            .map(|a| format!(" Session last active: {a}s ago."))
            .unwrap_or_default()
    };
    let system_prompt = format!(
        "{}{}\n\n{}",
        build_system_prompt(state).await,
        session_ts,
        ANTI_LAZINESS_RIDER
    );
    let llm_started = std::time::Instant::now();

    let (response_text, model_used, tokens) = match params.cognitive_profile.as_deref() {
        Some(profile) => {
            let resolution_template = McpChatModelResolution {
                allow_cheapest_fallback: profile == "fast",
                complexity: match profile {
                    "reasoning" => 9,
                    "creative" => 7,
                    _ => 5,
                },
                ..Default::default()
            };
            let temperature = if profile == "creative" { 0.8_f32 } else { 0.3_f32 };
            match resolve_chat_llm_model(state, &user_prompt, resolution_template.clone()).await {
                Ok((model, free_only)) => {
                    let pref = state.mcp_chat_model_override.read().await.clone();
                    let max_tokens =
                        crate::llm_bridge::clamp_http_max_output_tokens(model.max_tokens);
                    let routing = McpInferRouting {
                        user_prompt: &user_prompt,
                        sticky_model_pref: pref.as_deref(),
                        resolution_template,
                        free_only,
                        allow_cloud_ollama_fallback: true,
                    };
                    match crate::llm_bridge::mcp_infer_completion(
                        state,
                        model,
                        "vox_chat_message",
                        &system_prompt,
                        &routing,
                        max_tokens,
                        temperature,
                        params.json_mode,
                    )
                    .await
                    {
                        Ok(r) => r,
                        Err(e) => {
                            return ToolResult::<String>::err(format!("LLM error: {e}")).to_json();
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "vox_mcp::cognitive_routing",
                        profile,
                        error = %e,
                        "cognitive profile model resolution failed — using standard routing"
                    );
                    match call_llm(state, &system_prompt, &user_prompt).await {
                        Ok(r) => r,
                        Err(e2) => {
                            return ToolResult::<String>::err(format!("LLM error: {e2}")).to_json();
                        }
                    }
                }
            }
        }
        None => match call_llm(state, &system_prompt, &user_prompt).await {
            Ok(r) => r,
            Err(e) => {
                return ToolResult::<String>::err(format!("LLM error: {e}")).to_json();
            }
        },
    };

    tracing::info!(
        target: "vox_mcp::populi_kpi",
        tool = "vox_chat_message",
        model_id = %model_used,
        tokens,
        elapsed_ms = llm_started.elapsed().as_millis() as u64,
        cognitive_profile = params.cognitive_profile.as_deref().unwrap_or("standard"),
        "mcp chat LLM round-trip"
    );

    // 4. Persist to session-scoped history.
    //
    // The history key is derived from `params.session_id` (defaulting to `"default"`).
    // Each distinct value yields an independent key, preventing context bleeding
    // across concurrent VS Code windows, agent threads, or other logical sessions.
    let session_id = params.session_id.as_deref().unwrap_or("default");
    let history_key = format!("chat_history:{session_id}");

    let user_msg = ChatTranscriptEntry {
        id: format!("usr-{}", now_ts()),
        role: "user".to_string(),
        content: params.prompt.clone(),
        timestamp: now_ts(),
        context_files: all_context_files,
        model_used: None,
        tokens: None,
    };
    let asst_msg = ChatTranscriptEntry {
        id: format!("asst-{}", now_ts() + 1),
        role: "assistant".to_string(),
        content: response_text.clone(),
        timestamp: now_ts() + 1,
        context_files: vec![],
        model_used: Some(model_used.clone()),
        tokens: Some(tokens),
    };

    let orch = state.orchestrator.lock().await;
    let existing_history: Vec<ChatTranscriptEntry> = orch
        .context()
        .get(&history_key)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    drop(orch);

    let mut history = existing_history;
    history.push(user_msg.clone());
    history.push(asst_msg.clone());
    // Keep last 100 messages per session to bound memory usage.
    if history.len() > 100 {
        let trim_to = history.len() - 100;
        history.drain(0..trim_to);
    }

    match serde_json::to_string(&history) {
        Ok(history_json) => {
            let orch = state.orchestrator.lock().await;
            orch.context()
                .set(AgentId(0), &history_key, &history_json, 0);
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                session_id,
                "chat_message: failed to serialize chat history — \
                 history will not persist for this turn"
            );
        }
    }

    if let Some(db) = &state.db {
        let repo_id = &state.repository.repository_id;
        let q_session = session_id.to_string();
        let q_repo = repo_id.to_string();
        
        // Insert user turn
        let _ = db.connection()
            .execute(
                "INSERT INTO chat_transcripts (id, session_id, role, content, model_used, tokens, context_files, repository_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    user_msg.id.clone(),
                    q_session.clone(),
                    user_msg.role.clone(),
                    user_msg.content.clone(),
                    user_msg.model_used.clone(),
                    user_msg.tokens.map(|t| t as i64),
                    serde_json::to_string(&user_msg.context_files).unwrap_or_default(),
                    q_repo.clone(),
                ],
            ).await;

        // Insert assistant turn into chat_transcripts (V17 legacy / VS Code history API)
        let _ = db.connection()
            .execute(
                "INSERT INTO chat_transcripts (id, session_id, role, content, model_used, tokens, context_files, repository_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    asst_msg.id.clone(),
                    q_session.clone(),
                    asst_msg.role.clone(),
                    asst_msg.content.clone(),
                    asst_msg.model_used.clone(),
                    asst_msg.tokens.map(|t| t as i64),
                    serde_json::to_string(&asst_msg.context_files).unwrap_or_default(),
                    q_repo,
                ],
            ).await;

        let now_s = now_ts();
        let date_str = ts_to_date_str(now_s);
        let server_idle_secs = {
            let orch = state.orchestrator.lock().await;
            now_s.saturating_sub(orch.last_activity_ms() / 1000)
        };
        let session_age_secs = {
            let orch = state.orchestrator.lock().await;
            orch.context().age_secs(&format!("chat_history:{session_id}")).unwrap_or(0)
        };

        // Record high-quality LLM turn in agent_events for Populi replay/SFT
        let mut payload = serde_json::json!({
            "type": "llm_turn",
            "prompt": user_prompt,
            "response": response_text,
            "model": model_used,
            "tokens": tokens,
            "session_id": q_session,
            "repository_id": state.repository.repository_id,
            "temporal_context": {
                "date": date_str,
                "server_idle_secs": server_idle_secs,
                "session_age_secs": session_age_secs,
            }
        });
        let _ = vox_ludus::db::insert_event(
            db,
            "0", // Global AI/Orchestrator surface agent_id
            "llm_turn",
            Some(&payload.to_string()),
        ).await;
    }

    // 5. Return updated history + the new assistant message

    let grounding = chat_grounding_score(&params, mention_count);
    let pol = state.orchestrator_config.effective_socrates_policy();
    let soc = socrates_tool_meta(&pol, grounding, false);
    spawn_socrates_telemetry(
        state,
        "vox_chat_message",
        soc.clone(),
        Some(model_used.clone()),
    );
    let result = serde_json::json!({
        "message": asst_msg,
        "history": history,
        "model_used": model_used,
        "tokens": tokens,
        "session_id": session_id,
        "socrates": soc,
    });

    ToolResult::ok(result).to_json()
}

/// Return the full chat history for a session.
///
/// Pass `params.session_id` to retrieve the isolated transcript for a specific session.
/// When `session_id` is `None`, falls back to `"default"` which matches the baseline
/// session used by `chat_message` when no session id is provided.
pub async fn chat_history(state: &ServerState, params: ChatHistoryParams) -> String {
    let session_id = &params.session_id;
    let history_key = format!("chat_history:{session_id}");
    let orch = state.orchestrator.lock().await;
    let history: Vec<ChatTranscriptEntry> = orch
        .context()
        .get(&history_key)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    ToolResult::ok(history).to_json()
}

/// Perform an inline edit on a range in a file.
/// The editor sends the current text; Rust queries the LLM and returns the replacement.
pub async fn inline_edit(state: &ServerState, params: InlineEditParams) -> String {
    let language = params.language.as_deref().unwrap_or("text");
    let context_before = params.context_before.as_deref().unwrap_or("");
    let context_after = params.context_after.as_deref().unwrap_or("");

    let user_prompt = format!(
        r"You are an expert {language} programmer. Edit the following code snippet as instructed.

INSTRUCTION: {prompt}

CONTEXT BEFORE (do not modify):
```{language}
{context_before}
```

CODE TO EDIT (lines {start_line}-{end_line} of file `{file}`):
```{language}
{current_text}
```

CONTEXT AFTER (do not modify):
```{language}
{context_after}
```

OUTPUT RULES:
- Output ONLY the replacement code for lines {start_line}-{end_line}.
- Do NOT include context_before or context_after.
- Do NOT wrap output in markdown fences — output raw code only.
- Preserve indentation consistent with context_before.
- Do NOT add placeholder comments or TODOs.",
        prompt = params.prompt,
        file = params.file,
        start_line = params.start_line,
        end_line = params.end_line,
        current_text = params.current_text,
    );

    let pol = state.orchestrator_config.effective_socrates_policy();
    let system_prompt = format!(
        "You are an expert inline code editor. You output ONLY replacement code, no markdown fences, no explanation.{}\n{}",
        ANTI_LAZINESS_RIDER,
        socrates_system_rider(&pol)
    );

    let resolution_template = McpChatModelResolution {
        allow_cheapest_fallback: true,
        ..Default::default()
    };
    let (model, free_only) =
        match resolve_chat_llm_model(state, &user_prompt, resolution_template.clone()).await {
            Ok(pair) => pair,
            Err(e) => return ToolResult::<String>::err(e).to_json(),
        };
    let pref = state.mcp_chat_model_override.read().await.clone();
    let max_tokens = clamp_http_max_output_tokens(model.max_tokens);
    let temperature = 0.3_f32;
    let routing = McpInferRouting {
        user_prompt: &user_prompt,
        sticky_model_pref: pref.as_deref(),
        resolution_template,
        free_only,
        allow_cloud_ollama_fallback: true,
    };

    let (replacement, model_used, tokens) = match crate::llm_bridge::mcp_infer_completion(
        state,
        model,
        "mcp_inline_edit",
        &system_prompt,
        &routing,
        max_tokens,
        temperature,
        params.json_mode,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("LLM error: {e}")).to_json(),
    };

    let result = InlineEditResult {
        replacement: replacement.trim().to_string(),
        explanation: params.prompt.clone(),
        tokens,
        model_used,
    };

    let grounding = 0.66_f64;
    let soc = socrates_tool_meta(&pol, grounding, params.current_text.len() < 8);
    spawn_socrates_telemetry(
        state,
        "vox_inline_edit",
        soc.clone(),
        Some(result.model_used.clone()),
    );

    let mut v = serde_json::to_value(&result).unwrap_or(serde_json::Value::Null);
    if let Some(obj) = v.as_object_mut() {
        obj.insert("socrates".to_string(), soc);
    }
    ToolResult::ok(v).to_json()
}

/// Generate a structured plan for a goal. Optionally writes PLAN.md to the workspace root.
/// This backs the Cursor-style "Planning Mode" in the extension and in Vox agents.
pub async fn plan_goal(state: &ServerState, params: PlanParams) -> String {
    let max_tasks = params.max_tasks.unwrap_or(30);
    let scope_note = if params.scope_files.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nScope this plan to these files:\n{}",
            params.scope_files.join("\n")
        )
    };

    let user_prompt = format!(
        r#"You are an expert software architect and planner.

GOAL: {goal}{scope_note}

Generate a comprehensive, ordered task list to achieve this goal. You MUST output a valid JSON object matching this schema, embedded in a ```json codeblock.

{{
  "summary": "2-3 sentence executive summary of the approach",
  "tasks": [
    {{
      "id": 1,
      "description": "Short imperative description of what to implement.",
      "files": ["path/to/file.rs"],
      "estimated_complexity": 5,
      "depends_on": []
    }}
  ]
}}

Rules:
- Every task must be atomic and independently verifiable.
- "estimated_complexity" must be an integer from 1 (trivial edit) to 10 (full subsystem build).
- "depends_on" must be an array of prior task IDs that must complete first.
- If files are unknown, leave the array empty or use `["TBD"]`.
- Include test tasks explicitly.
- Maximum {max_tasks} tasks.
- Do NOT include filler tasks like 'Review and refactor'."#,
        goal = params.goal,
        max_tasks = max_tasks,
        scope_note = scope_note
    );

    let system_prompt = build_system_prompt(state).await;
    let resolution_template = McpChatModelResolution {
        complexity: match params.max_tasks {
            Some(n) if n > 10 => 9,
            _ => 7,
        },
        ..Default::default()
    };

    let (model, free_only) = match resolve_chat_llm_model(state, &user_prompt, resolution_template.clone()).await {
        Ok(pair) => pair,
        Err(e) => return ToolResult::<String>::err(format!("No model found for plan: {e}")).to_json(),
    };

    let pref = state.mcp_chat_model_override.read().await.clone();
    let routing = McpInferRouting {
        user_prompt: &user_prompt,
        sticky_model_pref: pref.as_deref(),
        resolution_template,
        free_only,
        allow_cloud_ollama_fallback: true,
    };

    let (response_json, model_used, tokens) = match crate::llm_bridge::mcp_infer_completion(
        state,
        model,
        "vox_plan",
        &system_prompt,
        &routing,
        4096,
        0.3,
        true, // Enforce strict JSON mode for planning
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("LLM error: {e}")).to_json(),
    };

    // Strip any markdown fences if the model still included them despite JSON mode
    let block = response_json.trim();
    let cleaned = if block.starts_with("```json") {
        block.strip_prefix("```json").unwrap_or(block).strip_suffix("```").unwrap_or(block).trim()
    } else if block.starts_with("```") {
        block.strip_prefix("```").unwrap_or(block).strip_suffix("```").unwrap_or(block).trim()
    } else {
        block
    };

    let parsed: PlanResponseSchema = match serde_json::from_str(cleaned) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, raw = cleaned, "plan_goal: JSON decode failed after cleanup");
            return ToolResult::<String>::err(format!("Failed to parse task list JSON: {e}")).to_json();
        }
    };

    let summary = if parsed.summary.is_empty() { "No summary provided.".to_string() } else { parsed.summary };
    let tasks = parsed.tasks;

    // Manual markdown generation for the on-disk/visual summary
    let mut base_plan_md = format!("## Plan\n\n**Overall Summary**: {summary}\n\n### Tasks\n\n");
    if tasks.is_empty() {
        base_plan_md.push_str("*(No tasks generated)*\n");
    } else {
        for t in &tasks {
            let deps = if t.depends_on.is_empty() {
                String::new()
            } else {
                let dep_strs: Vec<String> = t.depends_on.iter().map(|d| d.to_string()).collect();
                format!(" [depends: {}]", dep_strs.join(", "))
            };
            base_plan_md.push_str(&format!(
                "{}. **{}** — [files: {}] [complexity: {}/10]{}\n\n",
                t.id,
                t.description,
                t.files.join(", "),
                t.estimated_complexity,
                deps
            ));
        }
    }

    // Optionally write PLAN.md
    let written_to_disk = if params.write_to_disk {
        let plan_path = state
            .workspace_root
            .as_deref()
            .unwrap_or(std::path::Path::new("."))
            .join("PLAN.md");
        let header = format!(
            "# Vox Plan\n\n**Goal**: {}\n**Generated**: {}\n**Model**: {}\n\n",
            params.goal,
            chrono::Local::now().format("%Y-%m-%d %H:%M"),
            model_used,
        );
        let full = header + &base_plan_md;
        std::fs::write(&plan_path, &full).is_ok()
    } else {
        false
    };

    let result = PlanResult {
        goal: params.goal,
        tasks,
        summary,
        plan_md: base_plan_md,
        written_to_disk,
    };

    let grounding = if params.scope_files.is_empty() {
        0.56_f64
    } else {
        0.74_f64
    };
    let pol = state.orchestrator_config.effective_socrates_policy();
    let soc = socrates_tool_meta(&pol, grounding, false);
    spawn_socrates_telemetry(state, "vox_plan", soc.clone(), Some(model_used.clone()));
    let mut v = serde_json::to_value(&result).unwrap_or(serde_json::Value::Null);
    if let Some(obj) = v.as_object_mut() {
        obj.insert("socrates".to_string(), soc);
    }
    ToolResult::ok(v).to_json()
}

/// Replan an existing DeI plan session (`vox-dei-d` on PATH or next to the MCP binary).
pub async fn plan_replan(state: &ServerState, params: PlanReplanParams) -> String {
    let body = serde_json::json!({
        "session_id": params.session_id,
        "delta_hint": params.delta_hint,
        "write_to_disk": params.write_to_disk,
        "mode": params.mode,
    });
    match crate::dei_ipc::call_dei_daemon("ai.plan.replan", body).await {
        Ok(mut v) => {
            let pol = state.orchestrator_config.effective_socrates_policy();
            let soc = socrates_tool_meta(&pol, 0.62, false);
            spawn_socrates_telemetry(state, "vox_replan", soc.clone(), None);
            if let Some(obj) = v.as_object_mut() {
                obj.insert("socrates".to_string(), soc);
            }
            ToolResult::ok(v).to_json()
        }
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

/// Read structured plan session status from `vox-dei-d`.
pub async fn plan_status(state: &ServerState, params: PlanStatusParams) -> String {
    let body = serde_json::json!({ "session_id": params.session_id });
    match crate::dei_ipc::call_dei_daemon("ai.plan.status", body).await {
        Ok(mut v) => {
            let pol = state.orchestrator_config.effective_socrates_policy();
            let soc = socrates_tool_meta(&pol, 0.58, false);
            spawn_socrates_telemetry(state, "vox_plan_status", soc.clone(), None);
            if let Some(obj) = v.as_object_mut() {
                obj.insert("socrates".to_string(), soc);
            }
            ToolResult::ok(v).to_json()
        }
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

#[derive(Deserialize)]
struct PlanResponseSchema {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    tasks: Vec<PlanTask>,
}

// Retired parse_plan_json in favor of direct structural decoding.

// ─── Ghost Text (IDE inference bridge) ───────────────────────────────────────

/// Parameters for the `vox_ghost_text` MCP tool.
#[derive(Debug, Deserialize)]
pub struct GhostTextParams {
    /// Source code prefix (up to 20 lines before cursor).
    pub prefix: String,
    /// Source code suffix (up to 5 lines after cursor).
    pub suffix: String,
    /// VS Code language ID (e.g. "vox", "rust", "typescript").
    #[serde(default)]
    pub language: Option<String>,
    /// Workspace-relative file path for context.
    #[serde(default)]
    pub file_path: Option<String>,
    /// Maximum tokens to generate. Defaults to 128 for low latency.
    #[serde(default)]
    pub max_tokens: Option<u64>,
}

/// Response from `vox_ghost_text`.
#[derive(Debug, Serialize)]
pub struct GhostTextResult {
    /// The generated completion text.
    pub completion: String,
    /// Model that produced this completion.
    pub model_used: String,
    /// Approximate token count.
    pub tokens: u64,
    /// Latency to first token (milliseconds, best-effort).
    pub latency_ms: u64,
}

/// Handle the `vox_ghost_text` tool call.
///
/// Builds a fill-in-the-middle (FIM) prompt optimised for single-line editor
/// completions and routes it to the fastest available LLM. Targets p95 < 50 ms
/// time-to-first-token when using a local Ollama / Populi inference server.
pub async fn ghost_text(state: &ServerState, params: GhostTextParams) -> String {
    let language = params.language.as_deref().unwrap_or("vox");
    let file_hint = params
        .file_path
        .as_deref()
        .map(|p| format!("File: {p}\n"))
        .unwrap_or_default();
    let max_tokens = params.max_tokens.unwrap_or(128);

    // FIM-style prompt: give the model clear boundaries.
    let user_prompt = format!(
        r"{file_hint}Complete the following {language} code. Output ONLY the completion — no markdown, no explanation, no fences.

<|fim_prefix|>{prefix}<|fim_suffix|>{suffix}<|fim_middle|>",
        prefix = params.prefix,
        suffix = params.suffix,
    );

    let pol = state.orchestrator_config.effective_socrates_policy();
    let system_prompt = format!(
        "You are an expert {language} code completion engine. Produce only the missing code fragment that naturally continues the prefix. \
         Keep completions concise (typically 1-3 lines). Never repeat the prefix or suffix. Never add markdown.\n{}",
        socrates_system_rider(&pol)
    );

    let t0 = std::time::Instant::now();

    let resolution_template = McpChatModelResolution {
        complexity: 2,
        free_tier_latency_critical: true,
        free_tier_fill_in_middle: true,
        allow_cheapest_fallback: true,
        enforce_free_tier_only: true,
        ..Default::default()
    };
    let (model, free_only) =
        match resolve_chat_llm_model(state, &user_prompt, resolution_template.clone()).await {
            Ok(pair) => pair,
            Err(e) => return ToolResult::<String>::err(format!("No model: {e}")).to_json(),
        };
    let pref = state.mcp_chat_model_override.read().await.clone();
    let temperature = 0.2_f32;
    let routing = McpInferRouting {
        user_prompt: &user_prompt,
        sticky_model_pref: pref.as_deref(),
        resolution_template,
        free_only,
        allow_cloud_ollama_fallback: true,
    };

    let (mut completion, model_used, tokens) = match mcp_infer_completion(
        state,
        model,
        "mcp_ghost_text",
        &system_prompt,
        &routing,
        max_tokens,
        temperature,
        false,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("LLM error: {e}")).to_json(),
    };

    let latency_ms = t0.elapsed().as_millis() as u64;

    // Strip any accidental fence wrappers the model may emit.
    if let Some(inner) = completion
        .strip_prefix(&format!("```{language}"))
        .or_else(|| completion.strip_prefix("```"))
    {
        completion = inner
            .trim_start_matches('\n')
            .trim_end_matches("```")
            .trim_end()
            .to_string();
    }

    // Cap at max_tokens * 4 bytes as a rough UTF-8 token proxy.
    if completion.len() > max_tokens as usize * 4 {
        completion = completion[..max_tokens as usize * 4].to_string();
    }

    let result = GhostTextResult {
        completion: completion.trim().to_string(),
        model_used,
        tokens,
        latency_ms,
    };

    tracing::debug!(
        latency_ms,
        model = %result.model_used,
        "ghost_text: {} chars generated",
        result.completion.len()
    );

    let thin_context = params.prefix.len() + params.suffix.len() < 40;
    let grounding = ghost_grounding_score(&params);
    let soc = socrates_tool_meta(&pol, grounding, thin_context);
    spawn_socrates_telemetry(
        state,
        "vox_ghost_text",
        soc.clone(),
        Some(result.model_used.clone()),
    );
    let mut v = serde_json::to_value(&result).unwrap_or(serde_json::Value::Null);
    if let Some(obj) = v.as_object_mut() {
        obj.insert("socrates".to_string(), soc);
    }
    ToolResult::ok(v).to_json()
}

// ─── Ambient State (orchestrator → editor projection) ────────────────────────

/// Parameters for `vox_ambient_state`.
#[derive(Debug, Deserialize)]
pub struct AmbientStateParams {
    /// Optional workspace-relative path filter. Returns only decorations for this path prefix.
    #[serde(default)]
    pub path_prefix: Option<String>,
    /// Maximum number of decorations to return. Defaults to 100.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Handle the `vox_ambient_state` tool call.
///
/// Snapshots the current DEI orchestrator state (active locks, conflicts, task-to-file
/// assignments) and converts it to a list of `AmbientDecoration` records. The VS Code
/// extension polls this every 2-3 seconds and renders gutter stripes + file-explorer
/// badges without interrupting the user's flow.
pub async fn ambient_state(state: &ServerState, params: AmbientStateParams) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let prefix_filter = params.path_prefix.as_deref().unwrap_or("");
    let limit = params.limit.unwrap_or(100);

    fn is_file_lock_row(d: &Value) -> bool {
        d.get("decoration")
            .and_then(|x| x.get("type"))
            .and_then(|t| t.as_str())
            == Some("file_lock")
    }

    let orch = state.orchestrator.lock().await;
    let mut decorations: Vec<Value> = Vec::new();

    // 1. Active file locks → FileLock decorations
    for (path, holder, exclusive) in orch.lock_manager().list_locks() {
        let path_str = path.to_string_lossy().to_string();
        if !prefix_filter.is_empty() && !path_str.contains(prefix_filter) {
            continue;
        }
        let (severity, tooltip) = if exclusive {
            (
                "error",
                format!("\u{1f512} Agent {holder} holding exclusive write lock"),
            )
        } else {
            (
                "warning",
                format!("\u{1f50d} Agent {holder} reading this file"),
            )
        };
        decorations.push(serde_json::json!({
            "path": path_str,
            "decoration": {
                "type": "file_lock",
                "agent_id": holder.0,
                "exclusive": exclusive,
            },
            "severity": severity,
            "timestamp_ms": now_ms,
            "tooltip": tooltip,
        }));
    }

    // 2. Active conflicts → Conflict decorations
    for conflict in orch.conflict_manager().active_conflicts() {
        let path_str = conflict.path.to_string_lossy().to_string();
        if !prefix_filter.is_empty() && !path_str.contains(prefix_filter) {
            continue;
        }
        let agent_ids: Vec<u64> = conflict.sides.iter().map(|s| s.agent_id.0).collect();
        decorations.push(serde_json::json!({
            "path": path_str,
            "decoration": {
                "type": "conflict",
                "conflict_id": conflict.id.to_string(),
                "agent_ids": agent_ids,
            },
            "severity": "error",
            "timestamp_ms": now_ms,
            "tooltip": format!(
                "\u{26a0} Conflict between {} agents — resolve before proceeding",
                conflict.sides.len()
            ),
        }));
    }

    // 3. Agent-to-file affinity (active tasks) → AgentActive decorations
    for agent_id in orch.agent_ids() {
        let Some(queue) = orch.agent_queue(agent_id) else {
            continue;
        };
        if let Some(task) = queue.current_task() {
            for fa in &task.file_manifest {
                let path_str = fa.path.to_string_lossy().to_string();
                if !prefix_filter.is_empty() && !path_str.contains(prefix_filter) {
                    continue;
                }
                if decorations.iter().any(|d| {
                    d.get("path").and_then(|p| p.as_str()) == Some(path_str.as_str())
                        && is_file_lock_row(d)
                }) {
                    continue;
                }
                decorations.push(serde_json::json!({
                    "path": path_str,
                    "decoration": {
                        "type": "agent_active",
                        "agent_id": agent_id.0,
                        "activity": format!("{:.60}", task.description),
                    },
                    "severity": "info",
                    "timestamp_ms": now_ms,
                    "tooltip": format!(
                        "\u{1f916} Agent {} working on: {:.80}",
                        agent_id, task.description
                    ),
                }));
            }
        }
    }

    drop(orch);

    let total = decorations.len().min(limit);
    decorations.truncate(limit);

    let active_conflicts = decorations
        .iter()
        .filter(|d| d.get("severity").and_then(|s| s.as_str()) == Some("error"))
        .count();

    let result = serde_json::json!({
        "decorations": decorations,
        "total": total,
        "active_conflicts": active_conflicts,
        "timestamp_ms": now_ms,
    });

    ToolResult::ok(result).to_json()
}

#[cfg(test)]
mod routing_tests {
    use super::super::chat_socrates_meta::{SocratesJsonMeta, socrates_tool_meta};
    use super::{ChatMessageParams, GhostTextParams, chat_grounding_score, ghost_grounding_score};
    use crate::llm_bridge::clamp_http_max_output_tokens;
    use vox_socrates_policy::ConfidencePolicy;

    #[test]
    fn clamp_http_max_output_respects_bounds() {
        assert_eq!(clamp_http_max_output_tokens(0), 1);
        assert_eq!(clamp_http_max_output_tokens(100), 100);
        assert_eq!(clamp_http_max_output_tokens(9000), 8192);
    }

    #[test]
    fn socrates_meta_contains_required_fields() {
        let p = ConfidencePolicy::workspace_default();
        let v = socrates_tool_meta(&p, 0.61, false);
        assert!(v.get("risk_decision").is_some());
        assert!(v.get("confidence_estimate").is_some());
        assert!(v.get("contradiction_ratio").is_some());
    }

    #[test]
    fn socrates_tool_meta_matches_telemetry_deserializer() {
        let p = ConfidencePolicy::workspace_default();
        let v = socrates_tool_meta(&p, 0.71, true);
        let m: SocratesJsonMeta = serde_json::from_value(v).expect("telemetry JSON must parse");
        assert!((m.confidence_estimate - 0.71).abs() < 1e-9);
        assert!((m.contradiction_ratio - 0.35).abs() < 1e-9);
    }

    #[test]
    fn ghost_grounding_score_respects_file_and_fim_boundaries() {
        let thin = GhostTextParams {
            prefix: "a".into(),
            suffix: "".into(),
            language: None,
            file_path: None,
            max_tokens: None,
        };
        let rich = GhostTextParams {
            prefix: "fn main() {\n    let x = 1;\n".into(),
            suffix: "\n}\n".into(),
            language: Some("rust".into()),
            file_path: Some("src/main.rs".into()),
            max_tokens: None,
        };
        assert!(ghost_grounding_score(&rich) > ghost_grounding_score(&thin));
    }

    #[test]
    fn grounding_score_increases_with_context() {
        let empty = ChatMessageParams {
            prompt: "Hi".into(),
            context_files: vec![],
            open_files: vec![],
            active_file: None,
            active_line: None,
            selected_text: None,
            diagnostics: vec![],
            session_id: None,
            cognitive_profile: None,
            json_mode: false,
        };
        let rich = ChatMessageParams {
            prompt: "Hi".into(),
            context_files: vec!["foo.rs".into()],
            open_files: vec!["bar.rs".into()],
            active_file: Some("src/main.rs".into()),
            active_line: Some(42),
            selected_text: Some("let x = 1;".into()),
            diagnostics: vec![],
            session_id: None,
            cognitive_profile: None,
            json_mode: false,
        };
        let a = chat_grounding_score(&empty, 0);
        let b = chat_grounding_score(&rich, 3);
        assert!(b > a);
    }

    #[test]
    fn test_plan_response_schema_extraction() {
        use super::PlanTask;
        // Tests the PlanResponseSchema deserialization path used by plan_goal.
        // parse_plan_json was retired in favor of direct serde deserialization.
        let json = r#"{
            "summary": "Fixing the bug",
            "tasks": [
                { "id": 1, "description": "Identify root cause", "files": ["src/main.rs"], "estimated_complexity": 2, "depends_on": [] },
                { "id": 2, "description": "Write fix", "files": ["src/main.rs"], "estimated_complexity": 3, "depends_on": [1] }
            ]
        }"#;
        let parsed: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
        assert_eq!(parsed["summary"], "Fixing the bug");
        let tasks = parsed["tasks"].as_array().expect("tasks array");
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0]["id"], 1);
        let deps: Vec<usize> = serde_json::from_value(tasks[1]["depends_on"].clone()).unwrap();
        assert_eq!(deps, vec![1]);
    }

    #[test]
    fn test_plan_schema_empty_tasks_is_valid() {
        let json = r#"{"summary": "Empty plan", "tasks": []}"#;
        let parsed: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
        assert_eq!(parsed["summary"], "Empty plan");
        assert_eq!(parsed["tasks"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_plan_schema_raw_json_no_fence() {
        use super::PlanTask;
        // Verifies PlanTask structure: id, description, files, estimated_complexity, depends_on
        let json = r#"{
            "summary": "Raw JSON",
            "tasks": [
                { "id": 1, "description": "Do thing", "files": [], "estimated_complexity": 1, "depends_on": [] }
            ]
        }"#;
        let tasks: Vec<PlanTask> =
            serde_json::from_value(serde_json::from_str::<serde_json::Value>(json).unwrap()["tasks"].clone())
                .expect("PlanTask deserialization");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].description, "Do thing");
        assert_eq!(tasks[0].estimated_complexity, 1);
        assert!(tasks[0].depends_on.is_empty());
    }
}
