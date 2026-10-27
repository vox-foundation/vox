//! First-tool-choice probe for live tool-trigger evals.
//!
//! Builds one chat turn exactly as `chat_message` does (routed model, system prompt,
//! [`super::agent_loop::chat_turn_context`] tool selection), makes a single model call, and
//! reports which tool the model reached for first. Nothing is dispatched: a trigger eval
//! must never drive the real browser or desktop, and the first choice is what tool
//! descriptions and selection actually control.

use vox_actor_runtime::ActivityOptions;
use vox_actor_runtime::llm::{LlmChatMessage, LlmToolDef, llm_chat};
use vox_mcp_registry::TOOL_REGISTRY;

use super::agent_loop::{
    chat_turn_context, llm_tool_def, model_spec_to_llm_config, salvage_tool_call_from_text,
};
use super::message::{effective_model_pref, resolution_for_tier};
use crate::chat_tools::build_system_prompt_with_skill;
use crate::chat_tools::params::ANTI_LAZINESS_RIDER;
use crate::llm_bridge::McpChatModelResolution;
use crate::llm_bridge::tool_selection::select_tools_for_turn;
use crate::server_state::ServerState;

/// What the model chose as its first action for one request.
#[derive(Debug, Clone)]
pub struct FirstToolChoice {
    /// Registry id of the routed model.
    pub model: String,
    /// Tools offered to the model for this turn, in offer order.
    pub offered: Vec<String>,
    /// The first tool the model called, or `None` when it answered in prose.
    pub tool: Option<String>,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Run one routed model call for `prompt` with the turn's real offered tools and return
/// the model's first tool choice without executing it.
pub async fn probe_first_tool_choice(
    state: &ServerState,
    prompt: &str,
) -> Result<FirstToolChoice, String> {
    let global_pref = crate::sync_poison::poison_rw_read(
        state.mcp_chat_model_override.read(),
        "mcp_chat_model_override",
    )
    .map_err(|e| e.to_string())?
    .clone();
    let pref = effective_model_pref(None, global_pref.as_deref());
    let choice = crate::llm_bridge::resolve_mcp_chat_model_with_rationale(
        state,
        prompt,
        pref.as_deref(),
        resolution_for_tier(
            None,
            McpChatModelResolution {
                capability_prompt: Some(prompt.to_string()),
                ..Default::default()
            },
        ),
        None,
    )
    .await?;
    let mut config = model_spec_to_llm_config(&choice.model).ok_or_else(|| {
        format!(
            "model {} has no direct chat mapping for tool calling",
            choice.model.id
        )
    })?;
    let system_prompt = format!(
        "{}\n\n{}",
        build_system_prompt_with_skill(state, None, None, None, &[]).await,
        ANTI_LAZINESS_RIDER
    );
    let ctx = chat_turn_context(state, None, None, prompt);
    let tool_defs: Vec<LlmToolDef> =
        select_tools_for_turn(TOOL_REGISTRY, &state.skill_registry, &ctx)
            .into_iter()
            .map(llm_tool_def)
            .collect();
    let offered = tool_defs.iter().map(|t| t.name.clone()).collect();
    config.tools = (!tool_defs.is_empty()).then(|| tool_defs.clone());
    let messages = vec![
        LlmChatMessage {
            role: "system".into(),
            content: system_prompt,
            ..Default::default()
        },
        LlmChatMessage {
            role: "user".into(),
            content: prompt.to_string(),
            ..Default::default()
        },
    ];
    let resp = match llm_chat(&ActivityOptions::new(), messages, config).await {
        vox_actor_runtime::ActivityResult::Ok(Ok(r)) => r,
        vox_actor_runtime::ActivityResult::Ok(Err(e)) => return Err(e),
        vox_actor_runtime::ActivityResult::Failed(e) => return Err(format!("{e:?}")),
        vox_actor_runtime::ActivityResult::Cancelled => {
            return Err("llm_chat activity cancelled".to_string());
        }
    };
    let tool = resp
        .tool_calls
        .as_ref()
        .and_then(|calls| calls.first())
        .map(|call| call.name.clone())
        .or_else(|| {
            salvage_tool_call_from_text(&resp.content, &tool_defs)
                .and_then(|v| v["name"].as_str().map(str::to_string))
        });
    Ok(FirstToolChoice {
        model: choice.model.id.clone(),
        offered,
        tool,
        prompt_tokens: resp.prompt_tokens,
        completion_tokens: resp.completion_tokens,
    })
}

#[cfg(test)]
#[allow(unsafe_code)] // env var mutation under the crate-wide CHAT_MESSAGE_ENV_LOCK
mod tests {
    use super::super::agent_loop::CHAT_MESSAGE_ENV_LOCK;
    use super::super::message::tests::{model_spec, test_state};
    use super::*;
    use vox_orchestrator::models::ProviderType;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn computer_act_call() -> serde_json::Value {
        serde_json::json!({
            "id": "chatcmpl-probe",
            "model": "test-model",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "vox_computer_act",
                            "arguments": "{\"app\":\"Calculator\",\"kind\":\"click\"}",
                        },
                    }],
                },
                "finish_reason": "tool_calls",
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5},
        })
    }

    /// The probe reports the model's first tool from the turn's real offered set and
    /// stops there: exactly one model request, so the call is never dispatched and no
    /// follow-up turn runs.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn probe_reports_the_first_tool_without_dispatching_it() {
        let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(computer_act_call()))
            .expect(1)
            .mount(&server)
            .await;
        let prev_base = std::env::var("OPENROUTER_BASE_URL").ok();
        let prev_key = std::env::var("OPENROUTER_API_KEY").ok();
        unsafe {
            std::env::set_var("OPENROUTER_BASE_URL", server.uri());
            std::env::set_var("OPENROUTER_API_KEY", "test-key");
        }
        vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

        let state = test_state();
        let model_id = "test-openrouter-model-probe";
        {
            let handle = state.orchestrator.models_handle();
            let mut registry = handle.write().expect("models registry lock");
            registry.register(model_spec(ProviderType::OpenRouter, model_id));
        }
        *state.mcp_chat_model_override.write() = Some(model_id.to_string());

        let result =
            probe_first_tool_choice(&state, "Use the Calculator app to compute 17 times 23.").await;

        unsafe {
            match prev_base {
                Some(v) => std::env::set_var("OPENROUTER_BASE_URL", v),
                None => std::env::remove_var("OPENROUTER_BASE_URL"),
            }
            match prev_key {
                Some(v) => std::env::set_var("OPENROUTER_API_KEY", v),
                None => std::env::remove_var("OPENROUTER_API_KEY"),
            }
        }
        vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

        let choice = result.expect("probe should succeed against the mock model");
        assert_eq!(choice.model, model_id);
        assert_eq!(choice.tool.as_deref(), Some("vox_computer_act"));
        assert!(
            choice.offered.iter().any(|t| t == "vox_computer_snapshot"),
            "a desktop request must be offered the desktop family: {:?}",
            choice.offered
        );
        assert!(
            !choice.offered.iter().any(|t| t.starts_with("vox_browser_")),
            "a desktop request must not be offered browser tools: {:?}",
            choice.offered
        );
    }
}
