//! Unified tool registry and dispatcher for the Vox MCP server.

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
mod dispatch;
/// Thin `git` CLI wrappers scoped to the discovered git root.
pub mod git_tools;
mod input_schemas;
/// Introspection tools for language visualization (AST, surface, pipeline).
pub mod introspection_tools;
/// Unified News Publishing System tools
pub mod news_tools;
/// Oratio speech-to-text (Candle Whisper).
pub mod oratio_tools;
/// Local mens registry status (`vox_populi_local_status`).
pub mod populi_tools;
mod registry;
/// Bounded repo walk + on-disk JSON cache under `.vox/cache/repos/...`.
pub mod repo_index;
/// Scientia publication lifecycle tools (manifest, approval, submission).
pub mod scientia_tools;
/// Orchestrator task submit/status/cancel/drain tools.
pub mod task_tools;
/// TOESTUB (Todo/Stubs/Empty) finding ingestion and queue management.
pub mod toestub_tools;
mod tool_aliases;
// Wired from sibling modules (`dispatch`, `registry`, …); anchor for unwired-module scans.
use self::{input_schemas as _, tool_aliases as _};
/// Training-intent submission via orchestrator (Mens CLI remains canonical executor).
pub mod training_tools;
/// Snapshot / oplog / workspace orchestrator VCS tools.
pub mod vcs_tools;

/// Names and descriptions of all available tools (SSOT: `vox-mcp-registry` / `contracts/mcp/tool-registry.canonical.yaml`).
pub use vox_mcp_registry::TOOL_REGISTRY;

pub use dispatch::handle_tool_call;
pub use registry::tool_registry;
