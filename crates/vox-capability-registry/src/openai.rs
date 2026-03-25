//! OpenAI-style tool JSON helpers.

/// JSON Schema `parameters` for OpenAI-style tool calling (`type` must be `"object"`).
#[must_use]
pub fn mens_chat_parameters(capability_id: &str) -> serde_json::Value {
    match capability_id {
        "oratio.transcribe" => serde_json::from_str(
            r#"{"type":"object","properties":{"path":{"type":"string","description":"Workspace-relative or absolute path to an audio or transcript file"},"language_hint":{"type":"string"},"profile":{"type":"string","enum":["conservative","balanced","aggressive"]},"debug_parser_payload":{"type":"boolean"}},"required":["path"]}"#,
        )
        .unwrap_or_else(|_| serde_json::json!({"type":"object"})),
        "oratio.status" => serde_json::from_str(
            r#"{"type":"object","additionalProperties":false}"#,
        )
        .unwrap_or_else(|_| serde_json::json!({"type":"object"})),
        "oratio.listen" => serde_json::from_str(
            r#"{"type":"object","properties":{"path":{"type":"string"},"session_id":{"type":"string"},"timeout_ms":{"type":"integer","minimum":1},"max_duration_ms":{"type":"integer","minimum":1},"language_hint":{"type":"string"},"profile":{"type":"string","enum":["conservative","balanced","aggressive"]},"route_mode":{"type":"string","enum":["none","tool","chat","orchestrator"]},"debug_parser_payload":{"type":"boolean"},"emit_asr_refine_path":{"type":"string"},"llm_refinement":{"type":"boolean"},"llm_min_det_confidence":{"type":"number"},"llm_max_output_tokens":{"type":"integer","minimum":1}},"required":["path"]}"#,
        )
        .unwrap_or_else(|_| serde_json::json!({"type":"object"})),
        _ => serde_json::json!({"type":"object"}),
    }
}

/// Build one OpenAI-compatible tool definition entry.
#[must_use]
pub fn capability_to_openai_function(
    name: &str,
    description: &str,
    parameters: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": parameters,
        }
    })
}
