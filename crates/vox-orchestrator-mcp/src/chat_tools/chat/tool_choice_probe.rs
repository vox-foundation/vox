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
    use crate::llm_bridge::tool_selection::{CHAT_TURN_PINS, INTENT_FAMILIES};
    use serde_json::{Value, json};
    use vox_orchestrator::models::ProviderType;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    const MODEL_ID: &str = "test-openrouter-model-probe";
    const DESKTOP_PROMPT: &str = "Use the Calculator app to compute 17 times 23.";

    fn reply(message: Value) -> Value {
        json!({
            "id": "chatcmpl-probe",
            "model": MODEL_ID,
            "choices": [{"index": 0, "message": message, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 11, "completion_tokens": 7},
        })
    }

    fn tool_call(name: &str) -> Value {
        reply(json!({
            "role": "assistant",
            "content": null,
            "tool_calls": [{
                "id": "call_1",
                "type": "function",
                "function": {"name": name, "arguments": "{\"app\":\"Calculator\"}"},
            }],
        }))
    }

    fn text(content: &str) -> Value {
        reply(json!({"role": "assistant", "content": content}))
    }

    /// Run the probe for `prompt` against a mock OpenRouter answering every request with
    /// `response`; returns the probe result and every request the mock received.
    #[allow(clippy::await_holding_lock)]
    async fn probe_against(
        response: ResponseTemplate,
        prompt: &str,
    ) -> (Result<FirstToolChoice, String>, Vec<Request>) {
        let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(response)
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
        {
            let handle = state.orchestrator.models_handle();
            let mut registry = handle.write().expect("models registry lock");
            registry.register(model_spec(ProviderType::OpenRouter, MODEL_ID));
        }
        *state.mcp_chat_model_override.write() = Some(MODEL_ID.to_string());

        let result = probe_first_tool_choice(&state, prompt).await;

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
        let requests = server.received_requests().await.expect("request recording");
        (result, requests)
    }

    fn body(request: &Request) -> Value {
        serde_json::from_slice(&request.body).expect("request body is JSON")
    }

    fn sent_tool_names(body: &Value) -> Vec<String> {
        body["tools"]
            .as_array()
            .expect("request must carry a tools array")
            .iter()
            .map(|t| {
                t["function"]["name"]
                    .as_str()
                    .expect("every tool has a function name")
                    .to_string()
            })
            .collect()
    }

    fn family_core_tools(id: &str) -> &'static [&'static str] {
        INTENT_FAMILIES
            .iter()
            .find(|f| f.id == id)
            .expect("family exists")
            .core_tools
    }

    /// One request, built like a real turn: routed model, system prompt, the user's
    /// words, and the selected tools. The reported choice and offer list come from that
    /// exchange, and nothing follows it (no dispatch, no second model call).
    #[tokio::test]
    async fn reports_the_structured_tool_call_from_the_tools_actually_sent() {
        let (result, requests) = probe_against(
            ResponseTemplate::new(200).set_body_json(tool_call("vox_computer_act")),
            DESKTOP_PROMPT,
        )
        .await;
        let choice = result.expect("probe succeeds against the mock");
        assert_eq!(
            requests.len(),
            1,
            "exactly one model call, no follow-up turn"
        );

        let body = body(&requests[0]);
        assert_eq!(body["model"], MODEL_ID);
        let messages = body["messages"].as_array().expect("messages array");
        assert_eq!(messages.len(), 2, "system + user only: {messages:?}");
        assert_eq!(messages[0]["role"], "system");
        assert!(
            messages[0]["content"]
                .as_str()
                .is_some_and(|s| s.len() > 100),
            "the real system prompt is sent: {:?}",
            messages[0]["content"]
        );
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], DESKTOP_PROMPT);

        let sent = sent_tool_names(&body);
        assert_eq!(
            choice.offered, sent,
            "reported offer list is what was sent, in order"
        );
        assert_eq!(
            &sent[..CHAT_TURN_PINS.len()],
            CHAT_TURN_PINS,
            "pins lead the offer"
        );
        for tool in family_core_tools("computer") {
            assert!(sent.iter().any(|s| s == tool), "{tool} not sent: {sent:?}");
        }
        assert!(
            !sent.iter().any(|s| s.starts_with("vox_browser_")),
            "browser tools sent for a desktop request: {sent:?}"
        );

        assert_eq!(choice.tool.as_deref(), Some("vox_computer_act"));
        assert_eq!(choice.model, MODEL_ID);
        assert_eq!((choice.prompt_tokens, choice.completion_tokens), (11, 7));
    }

    /// A prose answer is "no tool", and an unrelated request is sent neither family.
    #[tokio::test]
    async fn prose_reply_reports_no_tool_and_unrelated_turns_get_no_family_tools() {
        let (result, requests) = probe_against(
            ResponseTemplate::new(200).set_body_json(text("17 times 23 is 391.")),
            "What is 17 times 23?",
        )
        .await;
        let choice = result.expect("probe succeeds against the mock");
        assert_eq!(choice.tool, None);
        assert_eq!(requests.len(), 1);
        let sent = sent_tool_names(&body(&requests[0]));
        assert!(
            !sent
                .iter()
                .any(|s| s.starts_with("vox_browser_") || s.starts_with("vox_computer_")),
            "family tools sent for an arithmetic question: {sent:?}"
        );
    }

    /// Models that write the call as JSON text are counted, but only for an offered tool:
    /// naming a tool the turn never offered is not a choice of that tool.
    #[tokio::test]
    async fn text_tool_calls_count_only_for_offered_tools() {
        let offered = "```json\n{\"name\": \"vox_computer_snapshot\", \"arguments\": {\"app\": \"Calculator\"}}\n```";
        let (result, _) = probe_against(
            ResponseTemplate::new(200).set_body_json(text(offered)),
            DESKTOP_PROMPT,
        )
        .await;
        assert_eq!(
            result.expect("probe succeeds").tool.as_deref(),
            Some("vox_computer_snapshot")
        );

        let unoffered = "```json\n{\"name\": \"vox_browser_open\", \"arguments\": {\"url\": \"https://example.com\"}}\n```";
        let (result, _) = probe_against(
            ResponseTemplate::new(200).set_body_json(text(unoffered)),
            DESKTOP_PROMPT,
        )
        .await;
        assert_eq!(result.expect("probe succeeds").tool, None);
    }

    /// A failed model call must surface as an error. Reporting it as "no tool" would
    /// silently pass every negative trigger case.
    #[tokio::test]
    async fn provider_error_is_an_error_not_a_missing_tool() {
        let (result, requests) = probe_against(
            ResponseTemplate::new(500).set_body_json(json!({"error": {"message": "boom"}})),
            "What is 17 times 23?",
        )
        .await;
        assert!(!requests.is_empty(), "the probe must have called the model");
        let err = result.expect_err("a 500 must not look like a prose answer");
        assert!(
            err.contains("500"),
            "error names the provider status: {err}"
        );
    }
}
