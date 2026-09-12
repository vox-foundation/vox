//! HTTP request/response types for the inference API.

#[cfg(feature = "execution-api")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "execution-api")]
#[derive(Debug, Deserialize, Clone)]
pub struct GenerateRequest {
    pub prompt: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub model: Option<String>,
    /// P015: Optional output mode for constrained decoding (strict_json, jsonl_records, tool_args_json).
    #[serde(default)]
    pub output_mode: Option<String>,
    /// P017: Max retries when output_mode is set and validation fails (default 3).
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// P016: Optional JSON schema for post-generation validation (when output_mode is set).
    #[serde(default)]
    pub schema: Option<serde_json::Value>,
    /// P018: Optional flag to stream the response via SSE.
    #[serde(default)]
    pub stream: bool,
}

#[cfg(feature = "execution-api")]
fn default_max_tokens() -> usize {
    256
}
#[cfg(feature = "execution-api")]
fn default_temperature() -> f32 {
    0.7
}
#[cfg(feature = "execution-api")]
fn default_max_retries() -> u32 {
    3
}

#[cfg(feature = "execution-api")]
#[derive(Debug, Serialize)]
pub struct GenerateResponse {
    pub text: String,
    /// Alias for `text` — the orchestrator's VoxLocalAdapter (provider_endpoints.rs +
    /// provider_adapter.rs) deserializes `{ code, valid, errors }` from this endpoint.
    /// Emitting both lets the same server satisfy both callers without a protocol break.
    pub code: String,
    pub tokens_generated: usize,
    pub model: String,
    pub object: &'static str,
    pub choices: Vec<Choice>,
    /// P017: Number of repair retries when output_mode validation failed (0 if none).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair_attempts: Option<u32>,
    /// Validation status expected by VoxLocalAdapter: true when output parsed cleanly.
    pub valid: bool,
    /// Validation errors expected by VoxLocalAdapter.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

#[cfg(feature = "execution-api")]
#[derive(Debug, Serialize)]
pub struct Choice {
    pub text: String,
    pub index: usize,
    pub finish_reason: &'static str,
}

// --- Task B2 (MENS end-to-end completion): `/v1/chat/completions` ---
//
// A thin OpenAI-chat-shaped adapter over the same `do_generate` worker
// pipeline the three text-prompt routes already share (see `mod.rs`'s
// doc comment) — flattens `messages[]`/`tools[]` into the existing
// `GenerateRequest::prompt` (reusing the `tool_args_json` output_mode +
// `validate_structured_output_with_reason` machinery `prompt.rs` already
// has for post-hoc JSON-shape checking, rather than adding a second
// sampling/validation/retry path), and re-wraps `GenerateResponse` into
// `choices[].message`, populating `tool_calls` only when the generated
// text is schema-valid AND names a tool the request actually offered.
// When it isn't (the expected small-model failure mode), the raw text is
// still returned as `content` — the caller's own salvage policy
// (`vox-orchestrator-mcp`'s `agent_loop.rs`) regex-scans it as a fallback.

/// An incoming OpenAI-chat-shaped message. Only `role`/`content` are used by
/// the prompt-flattening adapter; other OpenAI fields (`tool_calls`,
/// `tool_call_id`, `name`) are accepted but not required, since this server
/// only ever answers as the assistant and never round-trips a prior
/// assistant tool call itself.
#[cfg(feature = "execution-api")]
#[derive(Debug, Deserialize, Clone)]
pub struct ChatCompletionMessage {
    pub role: String,
    #[serde(default)]
    pub content: Option<String>,
}

/// OpenAI-chat-shaped request body for `/v1/chat/completions`.
#[cfg(feature = "execution-api")]
#[derive(Debug, Deserialize, Clone)]
pub struct ChatCompletionRequest {
    pub messages: Vec<ChatCompletionMessage>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<usize>,
    #[serde(default)]
    pub temperature: Option<f32>,
    /// OpenAI tool-definition objects (`{"type":"function","function":{"name",
    /// "description","parameters"}}`), or the flatter `{"name","description"}`
    /// shape — both are accepted by the name/description lookups this route
    /// does when flattening tools into the prompt and validating a candidate
    /// tool call's name against what was actually offered.
    #[serde(default)]
    pub tools: Option<Vec<serde_json::Value>>,
}

#[cfg(feature = "execution-api")]
#[derive(Debug, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: &'static str,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
}

#[cfg(feature = "execution-api")]
#[derive(Debug, Serialize)]
pub struct ChatCompletionChoice {
    pub index: usize,
    pub message: ChatCompletionResponseMessage,
    pub finish_reason: &'static str,
}

#[cfg(feature = "execution-api")]
#[derive(Debug, Serialize)]
pub struct ChatCompletionResponseMessage {
    pub role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatCompletionToolCall>>,
}

#[cfg(feature = "execution-api")]
#[derive(Debug, Serialize, Clone)]
pub struct ChatCompletionToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub function: ChatCompletionToolCallFunction,
}

/// `arguments` is a JSON-encoded **string**, matching the OpenAI wire format
/// (and what `vox_llm_egress::EgressToolCall`'s client-side parser expects) —
/// not a nested JSON object.
#[cfg(feature = "execution-api")]
#[derive(Debug, Serialize, Clone)]
pub struct ChatCompletionToolCallFunction {
    pub name: String,
    pub arguments: String,
}
