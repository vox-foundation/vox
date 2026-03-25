//! Vox MCP Server — Model Context Protocol (stdio) for Vox: orchestrator, Codex/Turso tools, LLM
//! bridge, and Mens-adjacent registry surfaces.
//!
//! **Docs SSOT:** tool names in [`tools::TOOL_REGISTRY`]; per-tool JSON
//! schemas live in `tools/input_schemas.rs` (`tool_input_schema`, wired from `server`). Long-form: mdBook
//! [`vox-mcp.md`](../../../docs/src/api/vox-mcp.md), [`ref-cli.md`](../../../docs/src/ref-cli.md),
//! [`mens-training-ssot.md`](../../../docs/src/architecture/mens-training-ssot.md), repo
//! [`AGENTS.md`](../../../AGENTS.md) §2.2.1 (Codex / Arca / Turso).
//!
//! **Shared event sink:** set **`VOX_ORCHESTRATOR_EVENT_LOG`** to a path; [`ServerState`](crate::server::ServerState) appends one JSON line per [`vox_orchestrator::AgentEvent`] (see [`ServerState::spawn_orchestrator_event_log_sink`](crate::server::ServerState::spawn_orchestrator_event_log_sink)). `vox live` can tail the same file when built with the `live` feature.

#![allow(unused)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::new_without_default)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::manual_unwrap_or_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::explicit_auto_deref)]
#![allow(clippy::type_complexity)]

/// Agent-to-agent messaging over the orchestrator bus (send, inbox, ack, broadcast, history).
pub mod a2a;
/// File-path affinity: which agent owns a path, claim/transfer, and list files per agent.
pub mod affinity;
/// Native MCP short-circuit client, tool schema cache, and streaming transport helpers.
pub mod client;
/// Shared orchestrator key-value context, token budgets, and agent handoff summaries.
pub mod context;
/// Thin `vox-dei-d` JSON-line RPC client for planning (`ai.plan.*`) without linking `vox-cli`.
pub mod dei_ipc;
/// Gamify companions and orchestrator queue status surfaced as MCP tool JSON.
pub mod gamify;
/// Resolves sticky chat/inline model overrides and performs HTTP LLM calls (OpenRouter, etc.).
pub mod llm_bridge;
/// Long-term MEMORY.md, Codex knowledge graph, sessions, and user preference tools.
pub mod memory;
/// Best-effort mens registry publish on MCP startup (`VOX_MESH_ENABLED`).
pub mod populi_startup;
/// Model registry MCP tools: list models, suggest by task category, per-agent overrides.
pub mod models;
/// Live DEI orchestrator inspection: queues, locks, VCS, config, costs, task submit, heartbeats.
pub mod dei_tools;
/// Shared `ToolResult` envelope and Deserialize/Serialize shapes for MCP tool arguments.
pub mod params;
/// Bulletin-board Q&A between agents (ask, answer, pending, broadcast).
pub mod qa;
/// Sync locking helpers (re-exported from vox-orchestrator).
pub mod sync_lock;
/// [`ServerState`], MCP initialize/handler, and stdio server wiring.
pub mod server;
/// vox-skills marketplace: install, search, parse `SKILL.md`, list installed skills.
pub mod skills;
/// Tool name registry, `handle_tool_call` dispatcher, and submodule implementations.
pub mod tools;

#[cfg(feature = "wasm")]
/// Wasmtime loader for MCP servers compiled to WebAssembly (optional feature).
pub mod wasm;

// Re-export common types
/// Re-exports [`crate::params`] (`ToolResult`, task types, status DTOs).
pub use params::*;
/// Re-exports [`crate::server`] (`ServerState`, `VoxMcpServer`).
pub use server::*;
