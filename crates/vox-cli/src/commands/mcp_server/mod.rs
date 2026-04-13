//! Vox MCP Server — Model Context Protocol (stdio) for Vox: orchestrator, Codex/Turso tools, LLM
//! bridge, and mesh/Mens registry surfaces.
//!
//! **OpenClaw / ClawHub:** External TypeScript agent platform and skills marketplace; not part of this
//! crate. See `docs/src/explanation/expl-openclaw-analysis.md` and CLI `vox openclaw` (`vox-ars`).
//!
//! **Docs SSOT:** tool names in [`tools::TOOL_REGISTRY`]; per-tool JSON
//! schemas live in `tools/input_schemas.rs` (`tool_input_schema`, wired from `server`). Long-form: mdBook
//! [`vox-mcp.md`](../../../docs/src/api/vox-mcp.md), [`reference/cli.md`](../../../docs/src/reference/cli.md),
//! [`mens-training-ssot.md`](../../../docs/src/architecture/mens-training-ssot.md), repo
//! [`AGENTS.md`](../../../AGENTS.md) §2.2.1 (Codex / Arca / Turso).
//!

#![allow(unused)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::new_without_default)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::manual_unwrap_or_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::explicit_auto_deref)]
#![allow(clippy::type_complexity)]

mod sync_poison;

/// Native MCP short-circuit client, tool schema cache, and streaming transport helpers.
pub mod client;
/// Shared orchestrator key-value context, token budgets, and agent handoff summaries.
pub mod context;
/// Thin `vox-dei-d` JSON-line RPC client for planning (`ai.plan.*`) without linking `vox-cli`.
pub mod dei_ipc;
/// Optional HTTP/WebSocket gateway for remote/mobile clients.
pub mod http_gateway;
/// Journey envelope v1 JSON for Codex transcript joins (see contracts/orchestration).
pub mod journey_envelope;
/// Resolves sticky chat/inline model overrides and performs HTTP LLM calls (OpenRouter, etc.).
pub mod llm_bridge;
/// Best-effort mens registry publish on MCP startup (`VOX_MESH_ENABLED`).
pub mod populi_startup;
/// [`ServerState`], MCP initialize/handler, and stdio server wiring.
pub mod server;
/// Speech-to-code grammar / constrained-decode hooks (scaffold).
pub mod speech_constraints;
/// Sync locking helpers (re-exported from vox-orchestrator).
pub mod sync_lock;

// Re-export selected crate-root types (avoid `pub use ...::*` ambiguity).
pub use vox_orchestrator::mcp_tools::params::{
    AgentInfo, CancelTaskParams, DrainAgentParams, MapAgentSessionParams, ReorderTaskParams,
    StatusResponse, ToolResult,
};
pub use server::{VoxMcpServer, tool_json_envelope_is_error};
pub use vox_orchestrator::mcp_tools::server_state::ServerState;
