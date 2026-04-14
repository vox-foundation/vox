//! Unified tool registry and dispatcher for the Vox MCP server.

pub mod params;
pub mod server_state;

pub(crate) mod attention_policy;
/// Benchmark telemetry query tools (`research_metrics`).
pub mod benchmark_tools;
/// Chromium CDP browser automation (`vox_browser_*`).
pub mod browser_tools;
/// Shared LLM model resolution for chat tools.
pub mod chat_model_resolve;
/// Socrates grounding + telemetry helpers for chat tools.
pub mod chat_socrates_meta;
/// Chat, inline edit, ghost text, planning, and ambient editor decorations.
pub mod chat_tools;
/// Clavis credential tools.
pub mod clavis_tools;
/// Structured .vox diagnostics and repair tools.
pub mod code_validator;
/// Codex relational V17/V16 helpers over connected `VoxDb`.
pub mod codex_tools;
/// `cargo`/LSP validation helpers (`vox_validate_file`, `vox_run_tests`, ...).
pub mod compiler_tools;
/// Codex schema digest + sample row tools for `.vox` modules.
pub mod db_tools;
pub mod dispatch;
/// Execution time tracking tools.
pub mod exec_time_tools;
/// Thin `git` CLI wrappers scoped to the discovered git root.
pub mod git_tools;
/// Grammar export tools
pub mod grammar_tools;
pub mod input_schemas;
/// Introspection tools for language visualization (AST, surface, pipeline).
pub mod introspection_tools;
/// Unified News Publishing System tools
pub mod news_tools;
/// OpenClaw native gateway and skill tools.
pub mod openclaw_tools;
/// Oratio speech-to-text (Candle Whisper).
pub mod oratio_tools;
/// Orchestrator persistence outbox inspection helpers.
pub mod persistence_tools;
/// Local mens registry status (`vox_populi_local_status`).
pub mod populi_tools;
/// `vox init` parity scaffold (`vox_project_init`).
pub mod project_init_tools;
/// Socrates questioning / clarification answer persistence (`VoxDb`).
pub mod questioning_tools;
pub mod registry;
/// Explicit repo catalog + read-only polyrepo query tools.
pub mod repo_catalog_tools;
/// Bounded repo walk + on-disk JSON cache under `.vox/cache/repos/...`.
pub mod repo_index;
/// Scientia publication lifecycle tools (manifest, approval, submission).
pub mod scientia_tools;
pub mod scope_guard;
pub mod session_identity;
/// Speech → codegen orchestration (`vox_speech_to_code`).
pub mod speech_pipeline_tools;
/// Orchestrator task submit/status/cancel/drain tools.
pub mod task_tools;
pub(crate) mod text_normalization;
/// TOESTUB (Todo/Stubs/Empty) finding ingestion and queue management.
pub mod toestub_tools;
pub mod tool_aliases;
/// Trust rollup inspection tools (`trust_rollups` over VoxDb).
pub mod trust_tools;
pub mod sync_poison;
pub mod llm_bridge;
/// Workspace-relative path resolution (repo root joining, in-repo canonical checks).
pub(crate) mod workspace_path;
/// Training-intent submission via orchestrator (Mens CLI remains canonical executor).
pub mod training_tools;
/// Snapshot / oplog / workspace orchestrator VCS tools.
pub mod vcs_tools;

pub mod a2a_tools;
pub use a2a_tools as a2a;
pub mod affinity_tools;
pub use affinity_tools as affinity;
pub mod gamify_tools;
pub use gamify_tools as gamify;
pub mod memory_tools;
pub use memory_tools as memory;
pub mod qa_tools;
pub use qa_tools as qa;
pub mod models_tools;
pub use models_tools as models;
pub mod skills_tools;
pub use skills_tools as skills;
pub mod trace_tools;
pub use trace_tools as trace;
pub mod dei_tools;

pub mod mcp_context;
pub use mcp_context as context;
pub mod mcp_client;
pub use mcp_client as client;
pub mod dei_ipc;
pub mod http_gateway;
pub mod journey_envelope;
pub mod populi_startup;
pub mod speech_constraints;

// Wired from sibling modules (`dispatch`, `registry`, …); anchor for unwired-module scans.
pub use vox_mcp_registry::TOOL_REGISTRY;

pub use dispatch::handle_tool_call;
pub use registry::tool_registry;
pub use tool_aliases::canonical_tool_name;
pub mod server;
pub mod lifecycle;

pub use server::VoxMcpServer;
pub use server_state::{ServerState, CachedCatalog};
pub use lifecycle::{run_stdio_server_blocking, load_config, mcp_agent_fleet_env_enabled};
pub use params::ToolResult;
