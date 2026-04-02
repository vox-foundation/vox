//! OpenAI-compatible **non-streaming** chat completion JSON shapes (`/v1/chat/completions`).
//!
//! Streaming (`text/event-stream`) line assembly lives in `vox-openai-sse`.

use serde::{Deserialize, Serialize};

/// Parsed JSON body from a successful chat completion response.
#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: Option<ChatCompletionUsage>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionChoice {
    pub message: Option<ChatCompletionAssistantMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionAssistantMessage {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ChatCompletionToolCall>>,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionToolCall {
    pub function: ChatCompletionFunctionCall,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionFunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ChatCompletionUsage {
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub total_cost: Option<f64>,
}

/// Request body for OpenAI-compatible chat (non-stream).
#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest<'a> {
    pub model: &'a str,
    pub messages: Vec<ChatMessageTurn<'a>>,
    pub temperature: f32,
    pub max_tokens: u64,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ChatMessageTurn<'a> {
    pub role: &'a str,
    pub content: &'a str,
}
