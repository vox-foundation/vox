//! Unified tool registry and dispatcher for the Vox MCP server.

use crate::params::{SubmitTaskParams, TaskStatusParams, ToolResult};
use crate::server::ServerState;

/// Benchmark telemetry query tools (`research_metrics`).
pub mod benchmark_tools;
/// Shared LLM model resolution for chat tools.
pub mod chat_model_resolve;
/// Socrates grounding + telemetry helpers for chat tools.
pub mod chat_socrates_meta;
/// Chat, inline edit, ghost text, planning, and ambient editor decorations.
pub mod chat_tools;
/// Codex relational V17/V16 helpers over connected `VoxDb`.
pub mod codex_tools;
/// `cargo`/LSP validation helpers (`vox_validate_file`, `vox_run_tests`, ...).
pub mod compiler_tools;
/// Codex schema digest + sample row tools for `.vox` modules.
pub mod db_tools;
/// Thin `git` CLI wrappers scoped to the discovered git root.
pub mod git_tools;
/// Introspection tools for language visualization (AST, surface, pipeline).
pub mod introspection_tools;
/// Unified News Publishing System tools
pub mod news_tools;
/// Scientia publication lifecycle tools (manifest, approval, submission).
pub mod scientia_tools;
/// Oratio speech-to-text (Candle Whisper).
pub mod oratio_tools;
/// Local mens registry status (`vox_populi_local_status`).
pub mod populi_tools;
/// Bounded repo walk + on-disk JSON cache under `.vox/cache/repos/...`.
pub mod repo_index;
/// Orchestrator task submit/status/cancel/drain tools.
pub mod task_tools;
/// TOESTUB (Todo/Stubs/Empty) finding ingestion and queue management.
pub mod toestub_tools;
/// Training-intent submission via orchestrator (Mens CLI remains canonical executor).
pub mod training_tools;
/// Snapshot / oplog / workspace orchestrator VCS tools.
pub mod vcs_tools;

mod input_schemas;
mod tool_aliases;

/// Names and descriptions of all available tools.
pub const TOOL_REGISTRY: &[(&str, &str)] = &[
    (
        "vox_submit_task",
        "Submit a new task to the orchestrator. Routes to the best agent by file affinity.",
    ),
    (
        "vox_task_status",
        "Get the current status of a specific task by ID.",
    ),
    (
        "vox_orchestrator_status",
        "Get a full snapshot of the orchestrator state: agents, queues, and completed tasks.",
    ),
    (
        "vox_orchestrator_start",
        "Start the AgentFleet runtime programmatically from a Vox agent session.",
    ),
    (
        "vox_complete_task",
        "Mark a task as completed, releasing its file locks.",
    ),
    (
        "vox_fail_task",
        "Mark a task as failed with a reason string.",
    ),
    (
        "vox_check_file_owner",
        "Check which agent currently owns a given file path.",
    ),
    (
        "vox_validate_file",
        "Validate a .vox file using the full compiler pipeline (lexer → parser → typeck → HIR).",
    ),
    (
        "vox_run_tests",
        "Run cargo test for a specific crate, optionally filtered by test name.",
    ),
    (
        "vox_check_workspace",
        "Run cargo check for the entire workspace and return diagnostics.",
    ),
    ("vox_test_all", "Run cargo test for the entire workspace."),
    (
        "vox_publish_message",
        "Publish a message to the bulletin board for all agents to receive.",
    ),
    (
        "vox_set_context",
        "Set a key-value pair in the shared orchestrator context store. Supports TTL.",
    ),
    (
        "vox_get_context",
        "Retrieve a value from the shared context.",
    ),
    ("vox_list_context", "List available context keys by prefix."),
    (
        "vox_context_budget",
        "Get the token budget status and summarize recommendation for an agent.",
    ),
    (
        "vox_handoff_context",
        "Handoff summarized context from one agent to another.",
    ),
    (
        "vox_check_mood",
        "Returns the current gamification mood and status of the agent companion.",
    ),
    (
        "vox_agent_status",
        "Returns current agent state, activity, mood, queue depth.",
    ),
    (
        "vox_agent_continue",
        "Triggers auto-continuation for idle agents.",
    ),
    (
        "vox_agent_assess",
        "Evaluates remaining work, returns completion estimate.",
    ),
    (
        "vox_agent_handoff",
        "Passes plan/context from one agent to another.",
    ),
    (
        "vox_queue_status",
        "Returns the specific queue and tasks for an agent.",
    ),
    (
        "vox_lock_status",
        "Returns a list of all current file locks.",
    ),
    (
        "vox_budget_status",
        "Returns token usage and approximate costs across all agents.",
    ),
    ("vox_cancel_task", "Cancels an active or queued task."),
    (
        "vox_rebalance",
        "Rebalances tasks dynamically across agents.",
    ),
    ("vox_agent_events", "Streams event history for agents."),
    (
        "vox_my_files",
        "Returns all files currently owned by the specified agent.",
    ),
    ("vox_claim_file", "Request ownership of a specific file."),
    (
        "vox_transfer_file",
        "Transfer ownership of a file to another agent.",
    ),
    ("vox_ask_agent", "Ask another agent a question."),
    (
        "vox_answer_question",
        "Answer a pending question from another agent.",
    ),
    (
        "vox_pending_questions",
        "List all questions waiting for my answer.",
    ),
    (
        "vox_broadcast",
        "Broadcast a message to all agents on the board.",
    ),
    (
        "vox_memory_store",
        "Persist a key-value fact to long-term memory (MEMORY.md).",
    ),
    (
        "vox_memory_recall",
        "Retrieve a fact from long-term memory by key.",
    ),
    (
        "vox_memory_search",
        "Search daily logs and MEMORY.md for a keyword query.",
    ),
    (
        "vox_memory_log",
        "Append an entry to today's daily memory log.",
    ),
    (
        "vox_memory_list_keys",
        "List all section keys from MEMORY.md.",
    ),
    (
        "vox_knowledge_query",
        "Query the knowledge graph (VoxDB) for related concepts by keyword.",
    ),
    (
        "vox_skill_install",
        "Install a skill from a VoxSkillBundle JSON payload.",
    ),
    ("vox_skill_uninstall", "Uninstall an installed skill by ID."),
    ("vox_skill_list", "List all installed skills."),
    ("vox_skill_search", "Search installed skills by keyword."),
    (
        "vox_skill_info",
        "Get detailed info on a specific skill by ID.",
    ),
    (
        "vox_skill_parse",
        "Parse a SKILL.md and preview its manifest before installing.",
    ),
    (
        "vox_compaction_status",
        "Get current context token usage and whether compaction is recommended.",
    ),
    (
        "vox_session_create",
        "Create a new persistent session for an agent.",
    ),
    (
        "vox_session_list",
        "List all active sessions with state and token usage.",
    ),
    (
        "vox_session_reset",
        "Reset a session's conversation history (keeps metadata).",
    ),
    (
        "vox_session_compact",
        "Replace a session's history with a summary string.",
    ),
    (
        "vox_session_info",
        "Get detailed info about a specific session.",
    ),
    (
        "vox_session_cleanup",
        "Tick lifecycle and remove archived sessions.",
    ),
    (
        "vox_preference_get",
        "Get a user preference value by key from VoxDb.",
    ),
    (
        "vox_preference_set",
        "Set a user preference key to a value in VoxDb.",
    ),
    (
        "vox_preference_list",
        "List all user preferences, optionally filtered by a key prefix.",
    ),
    (
        "vox_learn_pattern",
        "Record a learned behavioral pattern with confidence score.",
    ),
    (
        "vox_behavior_record",
        "Record a user behavior event and receive pattern suggestions.",
    ),
    (
        "vox_behavior_summary",
        "Analyze recent behavior and summarize detected patterns.",
    ),
    (
        "vox_memory_save_db",
        "Persist a typed memory fact to VoxDb agent_memory table.",
    ),
    (
        "vox_memory_recall_db",
        "Recall typed memory facts for an agent from VoxDb.",
    ),
    (
        "vox_build_crate",
        "Run cargo build for a crate or the whole workspace.",
    ),
    (
        "vox_lint_crate",
        "Run cargo clippy for a crate or whole workspace.",
    ),
    (
        "vox_coverage_report",
        "Get code coverage report for a crate using cargo-llvm-cov.",
    ),
    ("vox_reorder_task", "Change the priority of a queued task."),
    (
        "vox_drain_agent",
        "Remove all queued tasks from an agent without retiring it.",
    ),
    (
        "vox_cost_history",
        "Get a time-series cost breakdown of operations.",
    ),
    (
        "vox_file_graph",
        "Get a JSON graph of all files and their owning agents (affinity map).",
    ),
    (
        "vox_config_get",
        "Get the current runtime orchestrator and toolchain configuration (wire aliases: vox_get_config).",
    ),
    (
        "vox_config_set",
        "Update the orchestrator configuration dynamically (wire alias: vox_set_config).",
    ),
    (
        "vox_map_agent_session",
        "Map a Vox agent session ID to an existing orchestrator agent (wire aliases: vox_map_opencode_session, vox_map_vscode_session).",
    ),
    (
        "vox_poll_events",
        "Poll recent orchestrator events for all agents.",
    ),
    (
        "vox_heartbeat",
        "Send an active heartbeat from a Vox agent session.",
    ),
    (
        "vox_record_cost",
        "Record a cost event from a Vox agent session token usage.",
    ),
    ("vox_git_log", "Show recent git commits (default: last 10)."),
    (
        "vox_git_diff",
        "Show uncommitted git diff for a file or the whole tree.",
    ),
    ("vox_git_status", "Get current git working tree status."),
    ("vox_git_blame", "Show line-by-line git blame for a file."),
    (
        "vox_repo_index_status",
        "Return repo index cache status (bounded walk under repo root, `.vox/cache/repos/...`).",
    ),
    (
        "vox_repo_index_refresh",
        "Refresh the on-disk repo index cache for the current workspace.",
    ),
    (
        "vox_language_surface",
        "Return all primary keywords, decorators, types, and builtins in the Vox language.",
    ),
    (
        "vox_compiler::ast_inspect",
        "Parse a .vox file and return its AST as a JSON tree. Argument: path (relative to repo root).",
    ),
    (
        "vox_pipeline_status",
        "Get current compiler pipeline health and status (lexer, parser, typeck, codegen).",
    ),
    (
        "vox_decorator_registry",
        "Return detailed metadata for all @decorators in the Vox language.",
    ),
    (
        "vox_builtin_registry",
        "Return detailed signatures and docs for all builtin functions.",
    ),
    (
        "vox_workspace_modules",
        "Returns a list of all .vox files in the workspace repository.",
    ),
    (
        "vox_a2a_tasks",
        "Returns the full DAG of current orchestrator tasks.",
    ),
    (
        "vox_snapshot_list",
        "List recent file snapshots for an agent.",
    ),
    (
        "vox_snapshot_diff",
        "Show the file-level diff between two snapshots.",
    ),
    (
        "vox_snapshot_restore",
        "Restore files to a previous snapshot state.",
    ),
    ("vox_oplog", "Show recent operations with undo support."),
    (
        "vox_undo",
        "Undo the last operation or a specific operation by ID.",
    ),
    ("vox_redo", "Redo a previously undone operation."),
    (
        "vox_conflicts",
        "List active file conflicts between agents.",
    ),
    ("vox_resolve_conflict", "Resolve a file conflict."),
    ("vox_conflict_diff", "Show the N-way diff of a conflict."),
    (
        "vox_workspace_create",
        "Create an isolated workspace for an agent.",
    ),
    (
        "vox_workspace_merge",
        "Merge an agent's workspace changes back to main.",
    ),
    (
        "vox_workspace_status",
        "Show files modified in an agent's workspace.",
    ),
    ("vox_change_create", "Start tracking a new logical change."),
    ("vox_change_log", "Show the history of a change."),
    ("vox_vcs_status", "Get unified VCS status."),
    (
        "vox_a2a_send",
        "Send a targeted A2A message from one agent to another.",
    ),
    (
        "vox_a2a_inbox",
        "Read unacknowledged messages in an agent's inbox.",
    ),
    ("vox_a2a_ack", "Acknowledge a message in an agent's inbox."),
    (
        "vox_a2a_broadcast",
        "Broadcast an A2A message to all agents.",
    ),
    ("vox_a2a_history", "Query the A2A message audit trail."),
    (
        "vox_db_schema",
        "Return the complete database schema digest as JSON.",
    ),
    (
        "vox_db_relationships",
        "Return the entity-relationship graph for the database.",
    ),
    ("vox_db_data_flow", "Return the data flow map."),
    (
        "vox_db_sample_data",
        "Fetch sample data from a given database table.",
    ),
    (
        "vox_db_explain_query",
        "Explain a query or mutation in plain English.",
    ),
    (
        "vox_db_suggest_query",
        "Suggest the correct Vox query expression for an intent.",
    ),
    (
        "vox_db_research_session_upsert",
        "Upsert research_sessions by session_key (V17). Empty repository_id defaults to workspace repository_id.",
    ),
    (
        "vox_db_conversation_version_append",
        "Append conversation_versions for a conversation_id (V17).",
    ),
    (
        "vox_db_conversation_edge_insert",
        "Insert conversation_edges between two conversations (V17).",
    ),
    (
        "vox_db_topic_evolution_append",
        "Append topic_evolution_events for a topic_id (V17).",
    ),
    (
        "vox_db_research_metric_linked",
        "Upsert research_sessions then append research_metrics with matching session_id text (links structured + legacy telemetry).",
    ),
    (
        "vox_benchmark_record",
        "Record a new benchmark observation (e.g. wall-clock time for a build phase).",
    ),
    (
        "vox_toestub_findings_upsert",
        "Record or update TOESTUB anti-pattern findings from external reviews (GitHub/CodeRabbit).",
    ),
    (
        "vox_generate_code",
        "Generate validated Vox code from a prompt.",
    ),
    (
        "vox_list_models",
        "List all models in the orchestrator registry (ids, providers, free/paid).",
    ),
    (
        "vox_suggest_model",
        "Suggest the best model for a task category string (codegen, review, etc.).",
    ),
    (
        "vox_set_model",
        "Set per-agent model override in the orchestrator registry.",
    ),
    (
        "vox_set_active_model",
        "Set sticky MCP chat model id for chat / inline / ghost (empty string clears).",
    ),
    (
        "vox_get_active_model",
        "Show sticky MCP chat override and resolved ModelSpec (no LLM call).",
    ),
    (
        "vox_oratio_transcribe",
        "Transcribe audio to text via Vox Oratio (Candle Whisper). Arg: path (workspace-relative or absolute).",
    ),
    (
        "vox_oratio_status",
        "Oratio / Candle Whisper backend status and default model env (JSON).",
    ),
    // ── Chat & Inline AI ──────────────────────────────────────────────────────
    (
        "vox_chat_message",
        "Send a chat message to the Vox AI. Resolves @mentions, injects editor context, queries LLM, persists history.",
    ),
    (
        "vox_chat_history",
        "Retrieve the full chat history for the current session.",
    ),
    (
        "vox_inline_edit",
        "AI inline edit on a file range. Editor sends current text; Rust queries LLM and returns replacement.",
    ),
    (
        "vox_plan",
        "Generate a Cursor-style structured task plan for a goal. Optionally writes PLAN.md to workspace root.",
    ),
    (
        "vox_replan",
        "Replan via vox-dei-d (ai.plan.replan): session_id + delta_hint; optional write_to_disk and mode.",
    ),
    (
        "vox_plan_status",
        "Plan session status via vox-dei-d (ai.plan.status) for a session_id.",
    ),
    (
        "vox_benchmark_list",
        "List recent benchmark_event rows from Codex for this repository (requires VoxDb). Also covers build times and eval scores.",
    ),
    (
        "vox_schola_submit",
        "Enqueue a background orchestrator task for Mens training intent; canonical execution remains `vox schola train`.",
    ),
    (
        "vox_populi_local_status",
        "Return mens environment variables and the local mens registry file contents (CPU-first node records).",
    ),
    (
        "vox_news_test_syndicate",
        "Validates a markdown string against the UnifiedNewsItem parser and executes a pure dry_run without posting anything live.",
    ),
    (
        "vox_news_draft_research",
        "Writes docs/news/drafts/{id}.md from the embedded research template (dry_run in frontmatter).",
    ),
    (
        "vox_news_approve",
        "Record a maker-checker approval for a news_id in VoxDb (two distinct approvers required before live syndication).",
    ),
    (
        "vox_news_approval_status",
        "Return distinct approver count and whether dual approval is satisfied for a news_id.",
    ),
    (
        "vox_news_simulate_publish_gate",
        "Parse news markdown and report what would block live publish (dry_run, approvals, armed) without posting.",
    ),
    (
        "vox_scientia_publication_prepare",
        "Create/update a canonical scientia publication manifest and return its content digest.",
    ),
    (
        "vox_scientia_publication_approve",
        "Record one digest-bound approval for a scientia publication manifest.",
    ),
    (
        "vox_scientia_publication_submit_local",
        "Submit an approved scientia publication through the local scholarly ledger adapter.",
    ),
    (
        "vox_scientia_publication_status",
        "Return manifest state, digest-bound approval count, and scholarly submission rows.",
    ),
];

/// Convert the static [`TOOL_REGISTRY`] table into RMCP [`rmcp::model::Tool`] descriptors.
pub fn tool_registry() -> Vec<rmcp::model::Tool> {
    TOOL_REGISTRY
        .iter()
        .map(|(n, d)| rmcp::model::Tool {
            name: std::borrow::Cow::Owned(n.to_string()),
            description: Some(std::borrow::Cow::Owned(d.to_string())),
            input_schema: std::sync::Arc::new(input_schemas::tool_input_schema(n)),
            output_schema: None,
            meta: None,
            annotations: None,
            execution: None,
            icons: None,
            title: None,
        })
        .collect()
}

/// Dispatch `name` to the matching submodule handler and record skill telemetry if DB is available.
pub async fn handle_tool_call(
    state: &ServerState,
    name: &str,
    args: serde_json::Value,
) -> Result<String, anyhow::Error> {
    let start_time = std::time::Instant::now();
    let name_canonical = tool_aliases::canonical_tool_name(name);

    // Check if the agent ID or session ID is included in meta arguments
    let agent_id = args.get("agent_id").and_then(|v| v.as_str());
    let session_id = args.get("session_id").and_then(|v| v.as_str());

    let result = handle_tool_call_inner(state, name_canonical, args.clone()).await;
    let duration_ms = start_time.elapsed().as_millis() as i64;

    // Record tool telemetry in agent_events if DB is enabled
    if let Some(db) = &state.db {
        let mut payload = serde_json::json!({
            "type": "tool_call",
            "tool": name_canonical,
            "args": args,
            "duration_ms": duration_ms,
            "success": result.is_ok(),
            "repository_id": state.repository.repository_id,
        });
        if let Some(sid) = session_id {
            payload["session_id"] = serde_json::Value::String(sid.to_string());
        }

        let agent_str = agent_id.unwrap_or("0");
        let _ = vox_ludus::db::insert_event(db, agent_str, "tool_call", Some(&payload.to_string()))
            .await;
    }

    result
}

/// Internal core dispatch
async fn handle_tool_call_inner(
    state: &ServerState,
    name: &str,
    args: serde_json::Value,
) -> Result<String, anyhow::Error> {
    match name {
        "vox_submit_task" => {
            Ok(task_tools::submit_task(state, serde_json::from_value(args)?).await)
        }
        "vox_task_status" => {
            Ok(task_tools::task_status(state, serde_json::from_value(args)?).await)
        }
        "vox_orchestrator_status" => Ok(crate::dei_tools::orchestrator_status(state).await),
        "vox_orchestrator_start" => Ok(crate::dei_tools::orchestrator_start(state).await),
        "vox_complete_task" => {
            Ok(task_tools::complete_task(state, serde_json::from_value(args)?).await)
        }
        "vox_fail_task" => Ok(task_tools::fail_task(state, serde_json::from_value(args)?).await),
        "vox_check_file_owner" => Ok(crate::dei_tools::check_file_owner(
            state,
            args.get("path").and_then(|v| v.as_str()).unwrap_or("."),
        )
        .await),

        "vox_validate_file" => {
            Ok(compiler_tools::validate_file(serde_json::from_value(args)?).await)
        }
        "vox_run_tests" => {
            Ok(compiler_tools::run_tests(state, serde_json::from_value(args)?).await)
        }
        "vox_check_workspace" => Ok(compiler_tools::check_workspace(state).await),
        "vox_test_all" => Ok(compiler_tools::test_all(state).await),
        "vox_publish_message" => {
            Ok(task_tools::publish_message(state, serde_json::from_value(args)?).await)
        }

        "vox_git_log" => Ok(git_tools::git_log(
            state,
            args.get("max_commits")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
        )
        .await),
        "vox_git_diff" => {
            Ok(git_tools::git_diff(state, args.get("path").and_then(|v| v.as_str())).await)
        }
        "vox_git_status" => Ok(git_tools::git_status(state).await),
        "vox_git_blame" => Ok(git_tools::git_blame(
            state,
            args.get("path").and_then(|v| v.as_str()).unwrap_or("."),
        )
        .await),
        "vox_repo_index_status" => Ok(repo_index::repo_index_status(state).await),
        "vox_repo_index_refresh" => Ok(repo_index::repo_index_refresh(state).await),

        "vox_language_surface" => Ok(introspection_tools::language_surface().to_string()),
        "vox_compiler::ast_inspect" => Ok(introspection_tools::ast_inspect(
            state,
            args.get("path").and_then(|v| v.as_str()).unwrap_or("."),
        )
        .await?
        .to_string()),
        "vox_pipeline_status" => Ok(introspection_tools::pipeline_status().await.to_string()),
        "vox_decorator_registry" => Ok(introspection_tools::decorator_registry().to_string()),
        "vox_builtin_registry" => Ok(introspection_tools::builtin_registry().to_string()),
        "vox_workspace_modules" => Ok(introspection_tools::workspace_modules(state)
            .await?
            .to_string()),
        "vox_a2a_tasks" => Ok(introspection_tools::a2a_tasks(state).await?.to_string()),

        "vox_snapshot_list" => Ok(vcs_tools::snapshot_list(state, args).await),
        "vox_snapshot_diff" => Ok(vcs_tools::snapshot_diff(state, args).await),
        "vox_snapshot_restore" => Ok(vcs_tools::snapshot_restore(state, args).await),
        "vox_oplog" => Ok(vcs_tools::oplog_list(state, args).await),
        "vox_undo" => Ok(vcs_tools::oplog_undo(state, args).await),
        "vox_redo" => Ok(vcs_tools::oplog_redo(state, args).await),
        "vox_conflicts" => Ok(vcs_tools::conflicts_list(state).await),
        "vox_resolve_conflict" => Ok(vcs_tools::resolve_conflict(state, args).await),
        "vox_conflict_diff" => Ok(vcs_tools::conflict_diff(state, args).await),
        "vox_workspace_create" => Ok(vcs_tools::workspace_create(state, args).await),
        "vox_workspace_merge" => Ok(vcs_tools::workspace_merge(state, args).await),
        "vox_workspace_status" => Ok(vcs_tools::workspace_status(state, args).await),
        "vox_change_create" => Ok(vcs_tools::change_create(state, args).await),
        "vox_change_log" => Ok(vcs_tools::change_log(state, args).await),
        "vox_vcs_status" => Ok(crate::dei_tools::vcs_status(state).await),

        "vox_db_schema" => Ok(db_tools::vox_db_schema(args)),
        "vox_db_relationships" => Ok(db_tools::vox_db_relationships(args)),
        "vox_db_data_flow" => Ok(db_tools::vox_db_data_flow(args)),
        "vox_db_sample_data" => Ok(db_tools::vox_db_sample_data(state, args).await),
        "vox_db_explain_query" => Ok(db_tools::vox_db_explain_query(state, args).await),
        "vox_db_suggest_query" => Ok(db_tools::vox_db_suggest_query(state, args).await),

        "vox_db_research_session_upsert" => {
            Ok(codex_tools::codex_research_session_upsert(state, args).await)
        }
        "vox_db_conversation_version_append" => {
            Ok(codex_tools::codex_conversation_version_append(state, args).await)
        }
        "vox_db_conversation_edge_insert" => {
            Ok(codex_tools::codex_conversation_edge_insert(state, args).await)
        }
        "vox_db_topic_evolution_append" => {
            Ok(codex_tools::codex_topic_evolution_append(state, args).await)
        }
        "vox_db_research_metric_linked" => {
            Ok(codex_tools::codex_research_metric_linked(state, args).await)
        }

        "vox_generate_code" => Ok(compiler_tools::generate_vox_code(state, args).await),
        "vox_list_models" => {
            Ok(crate::models::list_models(state, serde_json::from_value(args)?).await)
        }
        "vox_suggest_model" => {
            Ok(crate::models::suggest_model(state, serde_json::from_value(args)?).await)
        }
        "vox_set_model" => Ok(crate::models::set_model(state, serde_json::from_value(args)?).await),
        "vox_set_active_model" => Ok(crate::models::set_active_mcp_chat_model(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_get_active_model" => Ok(crate::models::get_active_mcp_chat_model(state).await),
        "vox_build_crate" => Ok(compiler_tools::build_crate(
            state,
            args.get("crate_name").and_then(|v| v.as_str()),
        )
        .await),
        "vox_lint_crate" => Ok(compiler_tools::lint_crate(
            state,
            args.get("crate_name").and_then(|v| v.as_str()),
        )
        .await),
        "vox_coverage_report" => Ok(compiler_tools::coverage_report(
            state,
            args.get("crate_name").and_then(|v| v.as_str()),
        )
        .await),

        // ── Chat & Inline AI ──────────────────────────────────────────────
        "vox_chat_message" => {
            Ok(chat_tools::chat_message(state, serde_json::from_value(args)?).await)
        }
        "vox_chat_history" => {
            Ok(chat_tools::chat_history(state, serde_json::from_value(args)?).await)
        }
        "vox_inline_edit" => {
            Ok(chat_tools::inline_edit(state, serde_json::from_value(args)?).await)
        }
        "vox_plan" => Ok(chat_tools::plan_goal(state, serde_json::from_value(args)?).await),
        "vox_replan" => Ok(chat_tools::plan_replan(state, serde_json::from_value(args)?).await),
        "vox_plan_status" => {
            Ok(chat_tools::plan_status(state, serde_json::from_value(args)?).await)
        }

        "vox_schola_submit" => {
            Ok(training_tools::train_submit(state, serde_json::from_value(args)?).await)
        }

        "vox_news_test_syndicate" => {
            Ok(news_tools::vox_news_test_syndicate(state, serde_json::from_value(args)?).await)
        }

        "vox_news_draft_research" => {
            Ok(news_tools::vox_news_draft_research(state, serde_json::from_value(args)?).await)
        }
        "vox_news_approve" => {
            Ok(news_tools::vox_news_approve(state, serde_json::from_value(args)?).await)
        }
        "vox_news_approval_status" => {
            Ok(news_tools::vox_news_approval_status(state, serde_json::from_value(args)?).await)
        }
        "vox_news_simulate_publish_gate" => Ok(news_tools::vox_news_simulate_publish_gate(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_scientia_publication_prepare" => Ok(scientia_tools::vox_scientia_publication_prepare(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_scientia_publication_approve" => Ok(scientia_tools::vox_scientia_publication_approve(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_scientia_publication_submit_local" => Ok(
            scientia_tools::vox_scientia_publication_submit_local(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        "vox_scientia_publication_status" => Ok(scientia_tools::vox_scientia_publication_status(
            state,
            serde_json::from_value(args)?,
        )
        .await),

        // Delegate others to existing modules
        "vox_my_files" => Ok(crate::affinity::my_files(state, serde_json::from_value(args)?).await),
        "vox_claim_file" => {
            Ok(crate::affinity::claim_file(state, serde_json::from_value(args)?).await)
        }
        "vox_transfer_file" => {
            Ok(crate::affinity::transfer_file(state, serde_json::from_value(args)?).await)
        }

        "vox_ask_agent" => Ok(crate::qa::ask_agent(state, serde_json::from_value(args)?).await),
        "vox_answer_question" => {
            Ok(crate::qa::answer_question(state, serde_json::from_value(args)?).await)
        }
        "vox_pending_questions" => {
            Ok(crate::qa::pending_questions(state, serde_json::from_value(args)?).await)
        }
        "vox_broadcast" => Ok(crate::qa::broadcast(state, serde_json::from_value(args)?).await),

        "vox_memory_store" => {
            Ok(crate::memory::memory_store(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_recall" => {
            Ok(crate::memory::memory_recall(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_search" => {
            Ok(crate::memory::memory_search(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_log" => {
            Ok(crate::memory::memory_daily_log(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_list_keys" => Ok(crate::memory::memory_list_keys(state).await),
        "vox_knowledge_query" => {
            Ok(crate::memory::knowledge_query(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_save_db" => {
            Ok(crate::memory::memory_save_db(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_recall_db" => {
            Ok(crate::memory::memory_recall_db(state, serde_json::from_value(args)?).await)
        }

        "vox_compaction_status" => {
            Ok(crate::memory::compaction_status(state, serde_json::from_value(args)?).await)
        }
        "vox_session_create" => {
            Ok(crate::memory::session_create(state, serde_json::from_value(args)?).await)
        }
        "vox_session_list" => Ok(crate::memory::session_list(state).await),
        "vox_session_reset" => {
            Ok(crate::memory::session_reset(state, serde_json::from_value(args)?).await)
        }
        "vox_session_compact" => {
            Ok(crate::memory::session_compact(state, serde_json::from_value(args)?).await)
        }
        "vox_session_info" => {
            Ok(crate::memory::session_info(state, serde_json::from_value(args)?).await)
        }
        "vox_session_cleanup" => Ok(crate::memory::session_cleanup(state).await),

        "vox_preference_get" => {
            Ok(crate::memory::preference_get(state, serde_json::from_value(args)?).await)
        }
        "vox_preference_set" => {
            Ok(crate::memory::preference_set(state, serde_json::from_value(args)?).await)
        }
        "vox_preference_list" => {
            Ok(crate::memory::preference_list(state, serde_json::from_value(args)?).await)
        }
        "vox_learn_pattern" => {
            Ok(crate::memory::learn_pattern(state, serde_json::from_value(args)?).await)
        }
        "vox_behavior_record" => {
            Ok(crate::memory::behavior_record(state, serde_json::from_value(args)?).await)
        }
        "vox_behavior_summary" => {
            Ok(crate::memory::behavior_summary(state, serde_json::from_value(args)?).await)
        }

        "vox_check_mood" => {
            Ok(crate::gamify::check_mood(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_status" => {
            Ok(crate::gamify::agent_status(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_continue" => {
            Ok(crate::gamify::agent_continue(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_assess" => {
            Ok(crate::gamify::agent_assess(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_handoff" => {
            Ok(crate::gamify::agent_handoff(state, serde_json::from_value(args)?).await)
        }

        "vox_queue_status" => {
            Ok(crate::dei_tools::queue_status(state, serde_json::from_value(args)?).await)
        }
        "vox_lock_status" => Ok(crate::dei_tools::lock_status(state).await),
        "vox_budget_status" => Ok(crate::dei_tools::budget_status(state).await),
        "vox_cancel_task" => {
            Ok(crate::dei_tools::cancel_task(state, serde_json::from_value(args)?).await)
        }
        "vox_reorder_task" => {
            Ok(crate::dei_tools::reorder_task(state, serde_json::from_value(args)?).await)
        }
        "vox_drain_agent" => {
            Ok(crate::dei_tools::drain_agent(state, serde_json::from_value(args)?).await)
        }
        "vox_cost_history" => {
            Ok(crate::dei_tools::cost_history(state, serde_json::from_value(args)?).await)
        }
        "vox_file_graph" => Ok(crate::dei_tools::file_graph(state).await),
        "vox_config_get" => Ok(crate::dei_tools::config_get(state).await),
        "vox_config_set" => Ok(crate::dei_tools::config_set(state, args).await),
        "vox_map_agent_session" => {
            Ok(crate::dei_tools::map_agent_session(state, serde_json::from_value(args)?).await)
        }
        "vox_poll_events" => {
            Ok(crate::dei_tools::poll_events(state, serde_json::from_value(args)?).await)
        }
        "vox_heartbeat" => {
            Ok(crate::dei_tools::heartbeat(state, serde_json::from_value(args)?).await)
        }
        "vox_record_cost" => {
            Ok(crate::dei_tools::record_cost(state, serde_json::from_value(args)?).await)
        }
        "vox_rebalance" => Ok(crate::dei_tools::rebalance(state).await),
        "vox_agent_events" => {
            Ok(crate::dei_tools::agent_events(state, serde_json::from_value(args)?).await)
        }

        "vox_a2a_send" => Ok(crate::a2a::a2a_send(state, serde_json::from_value(args)?).await),
        "vox_a2a_inbox" => Ok(crate::a2a::a2a_inbox(state, serde_json::from_value(args)?).await),
        "vox_a2a_ack" => Ok(crate::a2a::a2a_ack(state, serde_json::from_value(args)?).await),
        "vox_a2a_broadcast" => {
            Ok(crate::a2a::a2a_broadcast(state, serde_json::from_value(args)?).await)
        }
        "vox_a2a_history" => {
            Ok(crate::a2a::a2a_history(state, serde_json::from_value(args)?).await)
        }

        "vox_skill_install" => {
            Ok(crate::skills::skill_install(state, serde_json::from_value(args)?).await)
        }
        "vox_skill_uninstall" => {
            Ok(crate::skills::skill_uninstall(state, serde_json::from_value(args)?).await)
        }
        "vox_skill_list" => Ok(crate::skills::skill_list(state)),
        "vox_skill_search" => Ok(crate::skills::skill_search(
            state,
            serde_json::from_value(args)?,
        )),
        "vox_skill_info" => Ok(crate::skills::skill_info(
            state,
            serde_json::from_value(args)?,
        )),
        "vox_skill_parse" => Ok(crate::skills::skill_parse(serde_json::from_value(args)?)),

        "vox_set_context" => {
            Ok(crate::context::set_context(state, serde_json::from_value(args)?).await)
        }
        "vox_get_context" => {
            Ok(crate::context::get_context(state, serde_json::from_value(args)?).await)
        }
        "vox_list_context" => {
            Ok(crate::context::list_context(state, serde_json::from_value(args)?).await)
        }
        "vox_context_budget" => {
            Ok(crate::context::context_budget(state, serde_json::from_value(args)?).await)
        }
        "vox_handoff_context" => {
            Ok(crate::context::handoff_context(state, serde_json::from_value(args)?).await)
        }

        "vox_oratio_transcribe" => Ok(oratio_tools::transcribe(state, args)?),
        "vox_oratio_status" => Ok(oratio_tools::status()),

        "vox_populi_local_status" => Ok(populi_tools::mesh_local_status(args)?),

        "vox_benchmark_list" => {
            Ok(benchmark_tools::benchmark_list(state, serde_json::from_value(args)?).await)
        }
        "vox_benchmark_record" => {
            Ok(benchmark_tools::benchmark_record(state, serde_json::from_value(args)?).await)
        }
        "vox_toestub_findings_upsert" => {
            Ok(toestub_tools::toestub_findings_upsert(state, serde_json::from_value(args)?).await)
        }

        _ => {
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
        }
    }
}

#[cfg(test)]
mod registry_dispatch_tests {
    use super::{TOOL_REGISTRY, handle_tool_call};
    use crate::server::ServerState;
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
        "vox_generate_code",
        "vox_oratio_transcribe",
    ];

    #[tokio::test]
    async fn tool_registry_names_are_unique() {
        let mut seen = HashSet::new();
        for (name, _) in TOOL_REGISTRY {
            assert!(seen.insert(*name), "duplicate TOOL_REGISTRY name: {name}");
        }
    }

    #[tokio::test]
    async fn every_registry_tool_has_static_dispatch() {
        let state = ServerState::new_test().await;
        for (name, _) in TOOL_REGISTRY {
            if SKIP_DISPATCH_PROBE.contains(name) {
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
