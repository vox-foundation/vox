use super::params::{ANTI_LAZINESS_RIDER, InlineEditParams, InlineEditResult};
use crate::llm_bridge::{
    McpChatModelResolution, McpInferRouting, clamp_http_max_output_tokens, mcp_infer_completion,
};
use crate::params::ToolResult;
use crate::server::ServerState;
use crate::tools::chat_model_resolve::resolve_chat_llm_model;
use crate::tools::chat_socrates_meta::{
    socrates_system_rider, socrates_tool_meta, spawn_socrates_telemetry,
};

/// Perform an inline edit on a range in a file.
/// The editor sends the current text; Rust queries the LLM and returns the replacement.
pub async fn inline_edit(state: &ServerState, params: InlineEditParams) -> String {
    let language = params.language.as_deref().unwrap_or("text");
    let context_before = params.context_before.as_deref().unwrap_or("");
    let context_after = params.context_after.as_deref().unwrap_or("");

    let user_prompt = format!(
        r"You are an expert {language} programmer. Edit the following code snippet as instructed.

INSTRUCTION: {prompt}

CONTEXT BEFORE (do not modify):
```{language}
{context_before}
```

CODE TO EDIT (lines {start_line}-{end_line} of file `{file}`):
```{language}
{current_text}
```

CONTEXT AFTER (do not modify):
```{language}
{context_after}
```

OUTPUT RULES:
- Output ONLY the replacement code for lines {start_line}-{end_line}.
- Do NOT include context_before or context_after.
- Do NOT wrap output in markdown fences — output raw code only.
- Preserve indentation consistent with context_before.
- Do NOT add placeholder comments or TODOs.",
        prompt = params.prompt,
        file = params.file,
        start_line = params.start_line,
        end_line = params.end_line,
        current_text = params.current_text,
    );

    let pol = state.orchestrator_config.effective_socrates_policy();
    let system_prompt = format!(
        "You are an expert inline code editor. You output ONLY replacement code, no markdown fences, no explanation.{}\n{}",
        ANTI_LAZINESS_RIDER,
        socrates_system_rider(&pol)
    );

    let resolution_template = McpChatModelResolution {
        allow_cheapest_fallback: true,
        ..Default::default()
    };
    let (model, free_only) = match resolve_chat_llm_model(
        state,
        &user_prompt,
        resolution_template.clone(),
        params.session_id.as_deref(),
    )
    .await
    {
        Ok(pair) => pair,
        Err(e) => return ToolResult::<String>::err(e).to_json(),
    };
    let pref = state.mcp_chat_model_override.read().unwrap().clone();
    let max_tokens = clamp_http_max_output_tokens(model.max_tokens);
    let temperature = 0.3_f32;
    let routing = McpInferRouting {
        user_prompt: &user_prompt,
        sticky_model_pref: pref.as_deref(),
        resolution_template,
        free_only,
        allow_cloud_ollama_fallback: true,
        user_id: params.session_id.as_deref(),
    };

    let (replacement, model_used, tokens) = match mcp_infer_completion(
        state,
        model,
        "mcp_inline_edit",
        &system_prompt,
        &routing,
        max_tokens,
        temperature,
        params.json_mode,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("LLM error: {e}")).to_json(),
    };

    let result = InlineEditResult {
        replacement: replacement.trim().to_string(),
        explanation: params.prompt.clone(),
        tokens,
        model_used,
    };

    let grounding = 0.66_f64;
    let soc = socrates_tool_meta(&pol, grounding, params.current_text.len() < 8);
    spawn_socrates_telemetry(
        state,
        "vox_inline_edit",
        soc.clone(),
        Some(result.model_used.clone()),
    );

    let mut v = serde_json::to_value(&result).unwrap_or(serde_json::Value::Null);
    if let Some(obj) = v.as_object_mut() {
        obj.insert("socrates".to_string(), soc);
    }
    ToolResult::ok(v).to_json()
}
