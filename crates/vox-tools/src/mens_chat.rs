//! OpenAI-style tool definitions and execution for Mens chat.
//!
//! Definitions come from [`vox_capability_registry`] filtered by [`super::DirectToolExecutor`] support.

use serde_json::json;
use vox_capability_registry::{
    CapabilityDescriptor, CapabilityRegistry, capability_to_openai_function, default_registry,
    mens_chat_parameters,
};

use super::DirectToolExecutor;

fn mcp_tool_name(cap: &CapabilityDescriptor) -> &str {
    cap.invocation_forms
        .mcp_tool
        .as_deref()
        .unwrap_or(cap.capability_id.as_str())
}

fn direct_mens_chat_capabilities(
    registry: &CapabilityRegistry,
) -> impl Iterator<Item = &CapabilityDescriptor> + '_ {
    registry
        .mens_chat_capabilities()
        .filter(|cap| DirectToolExecutor::supports(mcp_tool_name(cap)))
}

/// Tool definitions for OpenRouter / Ollama / OpenAI-compatible function calling.
#[must_use]
pub fn chat_tool_definitions() -> Vec<serde_json::Value> {
    let registry = default_registry();
    direct_mens_chat_capabilities(&registry)
        .map(|cap| {
            let name = mcp_tool_name(cap);
            let params = mens_chat_parameters(&cap.capability_id);
            capability_to_openai_function(name, &cap.description, params)
        })
        .collect()
}

/// Run model-returned tool calls through the allowlisted executor.
pub fn execute_tool_calls(tool_calls: &[ToolCall]) -> Vec<(String, String)> {
    let executor = DirectToolExecutor::default();
    let registry = default_registry();
    let allowed: std::collections::HashSet<String> = direct_mens_chat_capabilities(&registry)
        .map(|cap| mcp_tool_name(cap).to_string())
        .collect();
    let mut results = Vec::with_capacity(tool_calls.len());
    for tc in tool_calls {
        if !allowed.contains(tc.name.as_str()) {
            results.push((
                tc.id.clone(),
                serde_json::to_string_pretty(&json!({
                    "success": false,
                    "error": format!("Tool '{}' is not in the chat allowlist", tc.name)
                }))
                .unwrap_or_default(),
            ));
            continue;
        }
        let args = tc
            .arguments
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::Value::Object(Default::default()));
        let result = executor.execute(&tc.name, args);
        results.push((
            tc.id.clone(),
            result.unwrap_or_else(|e| {
                serde_json::to_string_pretty(&json!({
                    "success": false,
                    "error": e.to_string()
                }))
                .unwrap_or_default()
            }),
        ));
    }
    results
}

/// Parsed tool call from a provider response.
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// Provider-assigned id (tool_call_id / tool_use id).
    pub id: String,
    /// MCP tool name (e.g. `vox_oratio_transcribe`).
    pub name: String,
    /// JSON arguments string from the model.
    pub arguments: Option<String>,
}
