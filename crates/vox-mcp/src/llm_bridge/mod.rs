//! MCP chat / inline-edit LLM routing: model resolution from the orchestrator registry and
//! HTTP calls (OpenRouter-compatible, Google Gemini `generateContent`, local Ollama).

use vox_orchestrator::types::AgentId;

mod error;
mod infer;
mod limits;
mod model_route_policy;
mod providers;

/// Single agent id for MCP-hosted LLM usage accounting (not per-tool agents).
pub(crate) const MCP_GLOBAL_LLM_AGENT: AgentId = AgentId(0);

pub use infer::{McpInferRouting, call_llm, mcp_infer_completion};
pub use model_route_policy::{
    McpChatModelResolution, mcp_global_llm_context_fill_ratio, resolve_mcp_chat_model_sync,
};

/// Clamp requested max output tokens for HTTP chat APIs (OpenRouter / Gemini caps).
#[must_use]
pub fn clamp_http_max_output_tokens(n: u64) -> u64 {
    n.max(1).min(limits::HTTP_MAX_OUTPUT_TOKENS_CAP)
}
