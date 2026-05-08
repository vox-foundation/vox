//! `<TOOL_CALLS>` XML fallback for LLM providers without native function-call support.
//!
//! Moved from `vox-tools/src/mens_chat.rs` when `vox-tools` was deleted.
//! Provides `fallback_tool_system_prompt` and `parse_fallback_tools` for Mens chat
//! providers that don't support OpenAI-style native tool invocations.

use vox_capability_registry::{
    CapabilityDescriptor, CapabilityRegistry, default_registry, mens_chat_parameters,
};

fn mcp_tool_name(cap: &CapabilityDescriptor) -> &str {
    cap.invocation_forms
        .mcp_tool
        .as_deref()
        .unwrap_or(cap.capability_id.as_str())
}

fn direct_mens_chat_capabilities(
    registry: &CapabilityRegistry,
) -> impl Iterator<Item = &CapabilityDescriptor> + '_ {
    registry.mens_chat_capabilities()
}

/// System prompt block instructing the LLM to output tools via a JSON block if native tools are unsupported.
#[must_use]
pub fn fallback_tool_system_prompt() -> String {
    let registry = default_registry();
    let mut instructions = String::from(
        "You have access to the following tools. To use a tool, output a JSON array of tool calls \
        inside a `<TOOL_CALLS>` XML block. Format: `<TOOL_CALLS>[{\"name\": \"tool_name\", \
        \"arguments\": {\"arg\": \"value\"}}]</TOOL_CALLS>`. Do not write anything else inside \
        the block.\n\n",
    );
    for cap in direct_mens_chat_capabilities(&registry) {
        let name = mcp_tool_name(cap);
        let params = mens_chat_parameters(&cap.capability_id);
        let params_json = serde_json::to_string_pretty(&params).unwrap_or_default();
        instructions.push_str(&format!(
            "Tool: {}\nDescription: {}\nParameters: {}\n\n",
            name, cap.description, params_json
        ));
    }
    instructions
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

/// Parses `<TOOL_CALLS>` XML blocks from raw text into native [`ToolCall`] objects.
#[must_use]
pub fn parse_fallback_tools(content: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    let start_tag = "<TOOL_CALLS>";
    let end_tag = "</TOOL_CALLS>";

    let mut current_idx = 0;
    while let Some(start_idx) = content[current_idx..].find(start_tag) {
        let absolute_start = current_idx + start_idx + start_tag.len();
        if let Some(end_idx) = content[absolute_start..].find(end_tag) {
            let json_str = &content[absolute_start..absolute_start + end_idx];
            if let Ok(parsed_arr) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
                for (i, v) in parsed_arr.into_iter().enumerate() {
                    if let (Some(name), Some(args)) =
                        (v.get("name").and_then(|n| n.as_str()), v.get("arguments"))
                    {
                        let unix_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis();
                        let id = format!("call_{}_{}", unix_ms, i);
                        calls.push(ToolCall {
                            id,
                            name: name.to_string(),
                            arguments: Some(args.to_string()),
                        });
                    }
                }
            }
            current_idx = absolute_start + end_idx + end_tag.len();
        } else {
            break;
        }
    }
    calls
}
