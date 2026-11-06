//! The provider wire: OpenAI-compatible chat / streaming / embeddings egress.
//! Ported from `vox-actor-runtime/src/llm/{chat,stream,embed}.rs`, reading from a
//! resolved [`EgressRequest`] instead of `LlmConfig` and returning structured results.

use std::time::Instant;

use serde::Serialize;

use crate::{
    ChatDeltaEventStream, ChatMessage, ChatParams, ChatStream, EgressChatResponse, EgressError,
    EgressRequest, EgressToolCall, LlmContentPart, throttle,
};

#[derive(Serialize)]
struct OpenAiToolFunction<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    parameters: &'a serde_json::Value,
}

#[derive(Serialize)]
struct WireCacheControl {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Serialize)]
struct OpenAiTool<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    function: OpenAiToolFunction<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<WireCacheControl>,
}

/// Outbound shape for `function.arguments` — a JSON-encoded **string**, the inverse of
/// `EgressToolCall::arguments`'s eagerly-parsed `serde_json::Value` (see 1.3a). We
/// re-serialize here rather than changing `EgressToolCall`, which stays the shared
/// inbound/outbound type per the task's instruction to reuse it.
#[derive(Serialize)]
struct WireToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct WireToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    function: WireToolCallFunction,
}

impl From<&EgressToolCall> for WireToolCall {
    fn from(tc: &EgressToolCall) -> Self {
        WireToolCall {
            id: tc.id.clone(),
            kind: "function",
            function: WireToolCallFunction {
                name: tc.name.clone(),
                // Best-effort: if `arguments` somehow fails to re-serialize (it came
                // from `serde_json::Value`, so in practice this never fails), fall
                // back to an empty JSON object string rather than panicking.
                arguments: serde_json::to_string(&tc.arguments)
                    .unwrap_or_else(|_| "{}".to_string()),
            },
        }
    }
}

/// Per-message wire shape mirroring `ChatMessage` but with `tool_calls` re-serialized
/// to the outbound `WireToolCall` shape (JSON-string arguments) instead of the parsed
/// `Value` shape `ChatMessage`/`EgressToolCall` carry for caller convenience.
#[derive(Serialize)]
struct WireMessage<'a> {
    role: &'a str,
    content: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<WireToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
}

impl<'a> From<&'a ChatMessage> for WireMessage<'a> {
    fn from(m: &'a ChatMessage) -> Self {
        WireMessage {
            role: &m.role,
            content: wire_content(m),
            tool_calls: m
                .tool_calls
                .as_ref()
                .map(|calls| calls.iter().map(WireToolCall::from).collect()),
            tool_call_id: m.tool_call_id.as_deref(),
            name: m.name.as_deref(),
        }
    }
}

fn wire_content(m: &ChatMessage) -> serde_json::Value {
    match m.content_parts.as_ref() {
        Some(parts) if !parts.is_empty() => multimodal_content(&m.content, parts),
        _ => serde_json::Value::String(m.content.clone()),
    }
}

fn multimodal_content(text: &str, parts: &[LlmContentPart]) -> serde_json::Value {
    let mut content = vec![serde_json::json!({
        "type": "text",
        "text": text,
    })];
    for part in parts {
        match part {
            LlmContentPart::Text { .. } => {}
            LlmContentPart::ImageUrl { image_url } => content.push(serde_json::json!({
                "type": "image_url",
                "image_url": { "url": image_url.url },
            })),
        }
    }
    serde_json::Value::Array(content)
}

fn append_tool_images<'a>(wire: &mut Vec<WireMessage<'a>>, parts: &[&LlmContentPart]) {
    if parts.is_empty() {
        return;
    }
    let mut content = vec![serde_json::json!({
        "type": "text",
        "text": "Latest browser frame from the preceding tool results.",
    })];
    for part in parts {
        match part {
            LlmContentPart::Text { .. } => {}
            LlmContentPart::ImageUrl { image_url } => content.push(serde_json::json!({
                "type": "image_url",
                "image_url": { "url": image_url.url },
            })),
        }
    }
    wire.push(WireMessage {
        role: "user",
        content: serde_json::Value::Array(content),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    });
}

fn wire_messages(messages: &[ChatMessage]) -> Vec<WireMessage<'_>> {
    let mut wire = Vec::with_capacity(messages.len());
    let mut pending_tool_parts = Vec::new();
    for message in messages {
        if message.role != "tool" {
            append_tool_images(&mut wire, &pending_tool_parts);
            pending_tool_parts.clear();
        }
        let has_image = message.content_parts.as_ref().is_some_and(|parts| {
            parts
                .iter()
                .any(|part| matches!(part, LlmContentPart::ImageUrl { .. }))
        });
        if message.role == "tool" && has_image {
            // OpenAI-compatible tool messages only accept text content. Preserve the
            // tool-call response, then provide its pixels in a user multimodal turn.
            wire.push(WireMessage {
                role: &message.role,
                content: serde_json::Value::String(message.content.clone()),
                tool_calls: message
                    .tool_calls
                    .as_ref()
                    .map(|calls| calls.iter().map(WireToolCall::from).collect()),
                tool_call_id: message.tool_call_id.as_deref(),
                name: message.name.as_deref(),
            });
            pending_tool_parts.extend(
                message
                    .content_parts
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .filter(|part| matches!(part, LlmContentPart::ImageUrl { .. })),
            );
        } else {
            wire.push(WireMessage::from(message));
        }
    }
    append_tool_images(&mut wire, &pending_tool_parts);
    wire
}

fn supports_anthropic_cache_breakpoints(model: &str, base_url: &str) -> bool {
    let m = model.trim().to_ascii_lowercase();
    let u = base_url.trim().to_ascii_lowercase();
    m.starts_with("anthropic/")
        || m.starts_with("claude-")
        || m.contains("/claude-")
        || u.contains("anthropic.com")
}

fn attach_ephemeral_cache_control(content: &mut serde_json::Value) {
    match content {
        serde_json::Value::String(text) if !text.is_empty() => {
            let taken = std::mem::take(text);
            *content = serde_json::json!([{
                "type": "text",
                "text": taken,
                "cache_control": { "type": "ephemeral" }
            }]);
        }
        serde_json::Value::Array(parts) => {
            if let Some(last_text) = parts
                .iter_mut()
                .rev()
                .find(|p| p.get("type").and_then(|t| t.as_str()) == Some("text"))
                && let Some(obj) = last_text.as_object_mut()
            {
                obj.insert(
                    "cache_control".to_string(),
                    serde_json::json!({ "type": "ephemeral" }),
                );
            }
        }
        _ => {}
    }
}

fn wire_content_is_non_empty(content: &serde_json::Value) -> bool {
    match content {
        serde_json::Value::String(text) => !text.is_empty(),
        serde_json::Value::Array(parts) => parts.iter().any(|p| {
            p.get("type").and_then(|t| t.as_str()) == Some("text")
                && p.get("text")
                    .and_then(|t| t.as_str())
                    .is_some_and(|s| !s.is_empty())
        }),
        _ => false,
    }
}

fn wire_messages_for_request<'a>(
    req: &EgressRequest,
    messages: &'a [ChatMessage],
) -> Vec<WireMessage<'a>> {
    let mut wire = wire_messages(messages);
    if !supports_anthropic_cache_breakpoints(&req.model, &req.base_url) {
        return wire;
    }
    if let Some(first) = wire.first_mut()
        && first.role == "system"
    {
        attach_ephemeral_cache_control(&mut first.content);
    }

    let first_tool_idx = wire
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, m)| m.role == "tool")
        .map(|(i, _)| i);

    let mut pre_tool_bp_idx = None;
    if let Some(ft_idx) = first_tool_idx {
        for idx in (1..ft_idx).rev() {
            if wire[idx].role != "tool" && wire_content_is_non_empty(&wire[idx].content) {
                pre_tool_bp_idx = Some(idx);
                break;
            }
        }
        if let Some(idx) = pre_tool_bp_idx {
            attach_ephemeral_cache_control(&mut wire[idx].content);
        }
    }

    if wire.len() >= 3 {
        let mut tail_idx = None;
        if first_tool_idx.is_some() {
            for idx in (1..wire.len()).rev() {
                if Some(idx) != pre_tool_bp_idx && wire_content_is_non_empty(&wire[idx].content) {
                    tail_idx = Some(idx);
                    break;
                }
            }
        } else {
            for idx in (1..=wire.len() - 2).rev() {
                if wire[idx].role != "tool" && wire_content_is_non_empty(&wire[idx].content) {
                    tail_idx = Some(idx);
                    break;
                }
            }
        }
        if let Some(idx) = tail_idx {
            attach_ephemeral_cache_control(&mut wire[idx].content);
        }
    }
    wire
}

#[cfg(test)]
pub(crate) fn wire_content_json(m: &ChatMessage) -> serde_json::Value {
    wire_content(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_image_parts_produce_valid_tool_then_user_messages() {
        let message = ChatMessage {
            role: "tool".into(),
            content: "{}".into(),
            tool_calls: None,
            tool_call_id: Some("call-1".into()),
            name: None,
            content_parts: Some(vec![LlmContentPart::ImageUrl {
                image_url: crate::LlmImageUrl {
                    url: "data:image/png;base64,aaa".into(),
                },
            }]),
        };

        let wire = wire_messages(std::slice::from_ref(&message));
        let json = serde_json::to_value(wire).expect("serialize");
        let messages = json.as_array().expect("messages");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "tool");
        assert_eq!(messages[0]["content"], "{}");
        assert_eq!(messages[0]["tool_call_id"], "call-1");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"][0]["type"], "text");
        assert_eq!(messages[1]["content"][1]["type"], "image_url");
        assert!(messages[1].get("tool_call_id").is_none());
    }

    #[test]
    fn multimodal_user_turn_follows_all_consecutive_tool_responses() {
        let messages = vec![
            ChatMessage {
                role: "tool".into(),
                content: "first".into(),
                tool_calls: None,
                tool_call_id: Some("call-1".into()),
                name: None,
                content_parts: Some(vec![LlmContentPart::ImageUrl {
                    image_url: crate::LlmImageUrl {
                        url: "data:image/png;base64,aaa".into(),
                    },
                }]),
            },
            ChatMessage {
                role: "tool".into(),
                content: "second".into(),
                tool_calls: None,
                tool_call_id: Some("call-2".into()),
                name: None,
                content_parts: None,
            },
        ];

        let json = serde_json::to_value(wire_messages(&messages)).expect("serialize");
        assert_eq!(json[0]["role"], "tool");
        assert_eq!(json[1]["role"], "tool");
        assert_eq!(json[2]["role"], "user");
    }

    #[test]
    fn build_request_includes_stream_options_when_streaming() {
        let req = EgressRequest {
            base_url: "https://openrouter.ai/api/v1/chat/completions".into(),
            api_key: "secret".into(),
            model: "openai/gpt-5.4".into(),
            headers: vec![],
            throttle_key: "openrouter".into(),
            max_concurrent: 4,
            timeout_ms: None,
        };
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: "hi".into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            content_parts: None,
        }];
        let params = ChatParams::default();
        let streaming = serde_json::to_value(build_request(&req, &params, &messages, true))
            .expect("serialize streaming");
        assert_eq!(
            streaming["stream_options"],
            serde_json::json!({"include_usage": true})
        );

        let unary = serde_json::to_value(build_request(&req, &params, &messages, false))
            .expect("serialize unary");
        assert!(unary.get("stream_options").is_none());
    }

    #[test]
    fn build_request_injects_anthropic_cache_breakpoints_and_preserves_plain_openai_strings() {
        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: "You are Vox.".into(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                content_parts: None,
            },
            ChatMessage {
                role: "user".into(),
                content: "turn 1".into(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                content_parts: None,
            },
            ChatMessage {
                role: "assistant".into(),
                content: "reply 1".into(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                content_parts: None,
            },
            ChatMessage {
                role: "user".into(),
                content: "turn 2".into(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                content_parts: None,
            },
        ];
        let params = ChatParams::default();

        let anthropic_req = EgressRequest {
            base_url: "https://openrouter.ai/api/v1/chat/completions".into(),
            api_key: "secret".into(),
            model: "anthropic/claude-sonnet-4.6".into(),
            headers: vec![],
            throttle_key: "openrouter".into(),
            max_concurrent: 4,
            timeout_ms: None,
        };
        let anthropic_json =
            serde_json::to_value(build_request(&anthropic_req, &params, &messages, false))
                .expect("serialize anthropic");
        assert_eq!(
            anthropic_json["messages"][0]["content"][0]["cache_control"],
            serde_json::json!({"type": "ephemeral"})
        );
        assert_eq!(
            anthropic_json["messages"][2]["content"][0]["cache_control"],
            serde_json::json!({"type": "ephemeral"})
        );
        assert_eq!(
            anthropic_json["messages"][3]["content"],
            serde_json::json!("turn 2")
        );

        let openai_req = EgressRequest {
            model: "openai/gpt-5.4".into(),
            ..anthropic_req.clone()
        };
        let openai_json =
            serde_json::to_value(build_request(&openai_req, &params, &messages, false))
                .expect("serialize openai");
        assert_eq!(
            openai_json["messages"][0]["content"],
            serde_json::json!("You are Vox.")
        );
        assert_eq!(
            openai_json["messages"][2]["content"],
            serde_json::json!("reply 1")
        );

        // Multi-iteration tool-calling loop: system, user, empty assistant(tool_calls), tool_1, empty assistant(tool_calls), tool_2
        let tool_defs = vec![
            crate::ToolDef {
                name: "vox_read_file".into(),
                description: Some("Read file".into()),
                parameters: serde_json::json!({"type": "object"}),
            },
            crate::ToolDef {
                name: "vox_edit_file".into(),
                description: Some("Edit file".into()),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let tool_params = ChatParams {
            tools: Some(&tool_defs),
            ..Default::default()
        };
        let loop_messages = vec![
            ChatMessage {
                role: "system".into(),
                content: "You are Vox.".into(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                content_parts: None,
            },
            ChatMessage {
                role: "user".into(),
                content: "inspect and edit".into(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                content_parts: None,
            },
            ChatMessage {
                role: "assistant".into(),
                content: String::new(),
                tool_calls: Some(vec![EgressToolCall {
                    id: "c1".into(),
                    name: "vox_read_file".into(),
                    arguments: serde_json::json!({"path": "a.rs"}),
                }]),
                tool_call_id: None,
                name: None,
                content_parts: None,
            },
            ChatMessage {
                role: "tool".into(),
                content: "fn a() {}".into(),
                tool_calls: None,
                tool_call_id: Some("c1".into()),
                name: Some("vox_read_file".into()),
                content_parts: None,
            },
            ChatMessage {
                role: "assistant".into(),
                content: String::new(),
                tool_calls: Some(vec![EgressToolCall {
                    id: "c2".into(),
                    name: "vox_edit_file".into(),
                    arguments: serde_json::json!({"path": "a.rs"}),
                }]),
                tool_call_id: None,
                name: None,
                content_parts: None,
            },
            ChatMessage {
                role: "tool".into(),
                content: "edited ok".into(),
                tool_calls: None,
                tool_call_id: Some("c2".into()),
                name: Some("vox_edit_file".into()),
                content_parts: None,
            },
        ];
        let loop_json = serde_json::to_value(build_request(
            &anthropic_req,
            &tool_params,
            &loop_messages,
            false,
        ))
        .expect("serialize tool loop");
        // Breakpoint 1: last tool definition
        assert!(loop_json["tools"][0].get("cache_control").is_none());
        assert_eq!(
            loop_json["tools"][1]["cache_control"],
            serde_json::json!({"type": "ephemeral"})
        );
        // Breakpoint 2: system message
        assert_eq!(
            loop_json["messages"][0]["content"][0]["cache_control"],
            serde_json::json!({"type": "ephemeral"})
        );
        // Breakpoint 3: pre-tool user prompt (messages[1])
        assert_eq!(
            loop_json["messages"][1]["content"][0]["cache_control"],
            serde_json::json!({"type": "ephemeral"})
        );
        // Breakpoint 4: latest tool result at tail (messages[5])
        assert_eq!(
            loop_json["messages"][5]["content"][0]["cache_control"],
            serde_json::json!({"type": "ephemeral"})
        );
    }

    #[test]
    fn t3_wire_options_normalizes_reasoning_effort_and_openrouter_online_suffix() {
        let req = EgressRequest {
            base_url: "https://openrouter.ai/api/v1/chat/completions".into(),
            api_key: "secret".into(),
            model: "anthropic/claude-sonnet-4.6".into(),
            headers: vec![],
            throttle_key: "openrouter".into(),
            max_concurrent: 4,
            timeout_ms: None,
        };
        let opts = T3WireOptions {
            reasoning_effort: Some(ReasoningEffortWire::High),
            include_web_search: true,
            search_limit: Some(1),
        };
        let normalized = normalize_egress_request_for_t3_options(&req, &opts);
        assert_eq!(normalized.model, "anthropic/claude-sonnet-4.6:online");

        // Idempotent if already `:online`
        let again = normalize_egress_request_for_t3_options(&normalized, &opts);
        assert_eq!(again.model, "anthropic/claude-sonnet-4.6:online");

        let messages = vec![ChatMessage {
            role: "user".into(),
            content: "search and reason".into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            content_parts: None,
        }];
        let payload = build_request_with_t3_options(
            &normalized,
            &ChatParams::default(),
            &messages,
            true,
            &opts,
        );
        assert_eq!(payload["model"], "anthropic/claude-sonnet-4.6:online");
        assert_eq!(payload["reasoning_effort"], "high");
    }
}

#[derive(Serialize)]
pub(crate) struct StreamOptions {
    pub include_usage: bool,
}

/// Wire-level reasoning effort knob (`low | medium | high`) extracted from `t3router` & `thom-chat` (`VOX-ROUTER-T3-02`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffortWire {
    Low,
    Medium,
    High,
}

impl ReasoningEffortWire {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// Optional T3 Chat / OpenRouter turn controls (`reasoning_effort`, `include_web_search`, `search_limit`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct T3WireOptions {
    pub reasoning_effort: Option<ReasoningEffortWire>,
    pub include_web_search: bool,
    pub search_limit: Option<u32>,
}

/// Normalize an [`EgressRequest`] for [`T3WireOptions`]:
/// when `include_web_search` is true on an OpenRouter endpoint (`openrouter.ai`), appends the `:online`
/// model suffix if not already present (matching `thom-chat` `/api/generate-message` behavior).
#[must_use]
pub fn normalize_egress_request_for_t3_options(
    req: &EgressRequest,
    opts: &T3WireOptions,
) -> EgressRequest {
    let mut out = req.clone();
    if opts.include_web_search
        && out.base_url.to_ascii_lowercase().contains("openrouter.ai")
        && !out.model.ends_with(":online")
    {
        out.model = format!("{}:online", out.model);
    }
    out
}

#[derive(Serialize)]
pub(crate) struct OpenAiChatRequest<'a> {
    model: &'a str,
    messages: Vec<WireMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<&'static str>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<StreamOptions>,
}

pub(crate) fn build_request<'a>(
    req: &'a EgressRequest,
    params: &'a ChatParams<'a>,
    messages: &'a [ChatMessage],
    stream: bool,
) -> OpenAiChatRequest<'a> {
    build_request_inner(req, params, messages, stream, None)
}

/// Build and serialize an OpenAI-compatible wire payload with [`T3WireOptions`] (`VOX-ROUTER-T3-02`).
#[must_use]
pub fn build_request_with_t3_options<'a>(
    req: &'a EgressRequest,
    params: &'a ChatParams<'a>,
    messages: &'a [ChatMessage],
    stream: bool,
    t3_opts: &T3WireOptions,
) -> serde_json::Value {
    let raw = build_request_inner(req, params, messages, stream, Some(t3_opts));
    serde_json::to_value(raw).unwrap_or(serde_json::Value::Null)
}

fn build_request_inner<'a>(
    req: &'a EgressRequest,
    params: &'a ChatParams<'a>,
    messages: &'a [ChatMessage],
    stream: bool,
    t3_opts: Option<&T3WireOptions>,
) -> OpenAiChatRequest<'a> {
    let cache_enabled = supports_anthropic_cache_breakpoints(&req.model, &req.base_url);
    let tools = params.tools.map(|ts| {
        let len = ts.len();
        ts.iter()
            .enumerate()
            .map(|(idx, t)| OpenAiTool {
                kind: "function",
                function: OpenAiToolFunction {
                    name: &t.name,
                    description: t.description.as_deref(),
                    parameters: &t.parameters,
                },
                cache_control: (cache_enabled && idx + 1 == len)
                    .then_some(WireCacheControl { kind: "ephemeral" }),
            })
            .collect()
    });
    OpenAiChatRequest {
        model: &req.model,
        messages: wire_messages_for_request(req, messages),
        temperature: params.temperature,
        top_p: params.top_p,
        seed: params.seed,
        max_tokens: params.max_tokens,
        response_format: params.response_format,
        tools,
        tool_choice: params.tool_choice,
        reasoning_effort: t3_opts
            .and_then(|o| o.reasoning_effort)
            .map(ReasoningEffortWire::as_str),
        stream,
        stream_options: stream.then_some(StreamOptions {
            include_usage: true,
        }),
    }
}

fn apply_auth_headers(
    mut http: reqwest::RequestBuilder,
    req: &EgressRequest,
) -> reqwest::RequestBuilder {
    if !req.api_key.is_empty() {
        http = http.bearer_auth(&req.api_key);
    }
    for (name, value) in &req.headers {
        http = http.header(name, value);
    }
    http
}

/// Non-streaming OpenAI-compatible chat completion.
pub async fn chat_once(
    req: &EgressRequest,
    messages: &[ChatMessage],
    params: &ChatParams<'_>,
) -> Result<EgressChatResponse, EgressError> {
    let client = vox_http_client::client();
    let _permit = throttle::acquire_permit(&req.throttle_key, req.max_concurrent).await;

    let body = build_request(req, params, messages, false);
    let mut http = apply_auth_headers(client.post(&req.base_url).json(&body), req);
    if let Some(ms) = req.timeout_ms {
        http = http.timeout(std::time::Duration::from_millis(ms));
    }

    let start = Instant::now();
    let res = http
        .send()
        .await
        .map_err(|e| EgressError::Http(e.to_string()))?;
    let status = res.status();
    if status.as_u16() == 429 {
        let retry_after = throttle::retry_after_from_headers(res.headers());
        throttle::on_rate_limited(&req.throttle_key, retry_after);
        return Err(EgressError::RateLimited { retry_after });
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(EgressError::Status {
            code: status.as_u16(),
            body,
        });
    }
    let header_cost = res
        .headers()
        .get("x-response-cost")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok());
    let latency_ms = start.elapsed().as_millis() as u64;
    let json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| EgressError::Decode(e.to_string()))?;
    throttle::on_success(&req.throttle_key);

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let usage = &json["usage"];
    let prompt_tokens = usage["prompt_tokens"].as_u64().unwrap_or(0) as u32;
    let completion_tokens = usage["completion_tokens"].as_u64().unwrap_or(0) as u32;
    // Cached tokens: prefer `cache_read_input_tokens`, else `prompt_tokens_details.cached_tokens`.
    let cache_read_tokens = usage["cache_read_input_tokens"]
        .as_u64()
        .or_else(|| usage["prompt_tokens_details"]["cached_tokens"].as_u64())
        .unwrap_or(0) as u32;
    // Provider-reported cost from the body (`total_cost`/`cost`), else the header.
    let cost_usd = usage["total_cost"]
        .as_f64()
        .or_else(|| usage["cost"].as_f64())
        .or(header_cost);
    let model = json["model"].as_str().unwrap_or(&req.model).to_string();
    let tool_calls = parse_tool_calls(&json["choices"][0]["message"]["tool_calls"]);
    Ok(EgressChatResponse {
        content,
        prompt_tokens,
        completion_tokens,
        cache_read_tokens,
        model,
        cost_usd,
        latency_ms,
        tool_calls,
    })
}

/// Parse `message.tool_calls` (an array or absent/null) into `EgressToolCall`s.
/// Returns `None` when the field is absent/null/not-an-array (the common case for
/// callers that pass no tools), so this never changes behavior for existing callers.
fn parse_tool_calls(value: &serde_json::Value) -> Option<Vec<EgressToolCall>> {
    let arr = value.as_array()?;
    if arr.is_empty() {
        return None;
    }
    let calls: Vec<EgressToolCall> = arr
        .iter()
        .filter_map(|tc| {
            // Provider omitted `id`: default to empty string rather than dropping the
            // call — correlating (or rejecting) a call with no id is left to the
            // tool-dispatch loop (a separate task), not this pure wire-parsing layer.
            let id = tc["id"].as_str().unwrap_or_default().to_string();
            // Entries with no `function.name` are dropped via `?` (intentional): a
            // tool call this crate can't even name isn't actionable by any caller, so
            // we silently skip it rather than surfacing a partially-populated/garbage
            // call — see `tool_calls_entry_missing_name_is_dropped` for the locked-in
            // behavior.
            let name = tc["function"]["name"].as_str()?.to_string();
            let arguments = tc["function"]["arguments"]
                .as_str()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::Value::Null);
            Some(EgressToolCall {
                id,
                name,
                arguments,
            })
        })
        .collect();
    // If every entry was dropped (e.g. all missing `function.name`), report `None`
    // rather than `Some(vec![])` — an empty vec would misleadingly read as "tools were
    // requested but the model called none", when really nothing usable was parsed.
    if calls.is_empty() { None } else { Some(calls) }
}

/// Streaming OpenAI-compatible chat completion. Yields content deltas. Ported from
/// `vox-actor-runtime/src/llm/stream.rs` (same `vox_openai::sse` line framing).
pub async fn stream_once(
    req: &EgressRequest,
    messages: &[ChatMessage],
    params: &ChatParams<'_>,
) -> Result<(ChatStream, Option<f64>), EgressError> {
    use async_stream::stream;
    use futures_util::StreamExt;

    let client = vox_http_client::client();
    // Hold a permit across the whole stream lifetime by acquiring before the request;
    // the permit drops when the returned stream is dropped (it is moved into the closure).
    let permit = throttle::acquire_permit(&req.throttle_key, req.max_concurrent).await;

    let body = build_request(req, params, messages, true);
    let body_str = serde_json::to_string(&body).map_err(|e| EgressError::Decode(e.to_string()))?;
    let mut http = client
        .post(&req.base_url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .body(body_str);
    if !req.api_key.is_empty() {
        http = http.bearer_auth(&req.api_key);
    }
    for (name, value) in &req.headers {
        http = http.header(name, value);
    }

    let res = http
        .send()
        .await
        .map_err(|e| EgressError::Http(e.to_string()))?;
    let status = res.status();
    if status.as_u16() == 429 {
        let retry_after = throttle::retry_after_from_headers(res.headers());
        throttle::on_rate_limited(&req.throttle_key, retry_after);
        return Err(EgressError::RateLimited { retry_after });
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(EgressError::Status {
            code: status.as_u16(),
            body,
        });
    }
    throttle::on_success(&req.throttle_key);

    // Provider-reported cost from the response header (available before the body streams).
    let cost_usd = res
        .headers()
        .get("x-response-cost")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok());

    let byte_stream = res.bytes_stream();
    let out = stream! {
        // Keep the throttle permit alive for the duration of the stream.
        let _permit = permit;
        use vox_openai::sse::{sse_data_line_delta, Utf8LineBuffer};
        let mut buf = Utf8LineBuffer::new();
        futures_util::pin_mut!(byte_stream);
        while let Some(chunk_res) = byte_stream.next().await {
            match chunk_res {
                Ok(bytes) => {
                    let mut emitted: Vec<String> = Vec::new();
                    buf.push_lossy_bytes(&bytes, |line| {
                        if let Some(s) = sse_data_line_delta(line) {
                            emitted.push(s);
                        }
                    });
                    for s in emitted {
                        yield Ok(s);
                    }
                }
                Err(e) => {
                    yield Err(EgressError::Http(e.to_string()));
                    return;
                }
            }
        }
        let mut tail: Vec<String> = Vec::new();
        buf.flush_trailing(|line| {
            if let Some(s) = sse_data_line_delta(line) {
                tail.push(s);
            }
        });
        for s in tail {
            yield Ok(s);
        }
    };
    Ok((Box::pin(out), cost_usd))
}

/// Single-pass streaming OpenAI-compatible chat completion that yields both `delta.content` and
/// `delta.tool_calls` fragments via [`vox_openai::sse::SseDeltaEvent`]. Also handles the fallback
/// case where a provider/mock responds with a non-SSE JSON `chat.completion` body despite
/// `stream: true`.
pub async fn stream_events_once(
    req: &EgressRequest,
    messages: &[ChatMessage],
    params: &ChatParams<'_>,
) -> Result<(ChatDeltaEventStream, Option<f64>), EgressError> {
    use async_stream::stream;
    use futures_util::StreamExt;
    use vox_openai::sse::{SseDeltaEvent, Utf8LineBuffer, sse_data_line_delta_event};

    let client = vox_http_client::client();
    let permit = throttle::acquire_permit(&req.throttle_key, req.max_concurrent).await;

    let body = build_request(req, params, messages, true);
    let body_str = serde_json::to_string(&body).map_err(|e| EgressError::Decode(e.to_string()))?;
    let mut http = client
        .post(&req.base_url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .body(body_str);
    if !req.api_key.is_empty() {
        http = http.bearer_auth(&req.api_key);
    }
    for (name, value) in &req.headers {
        http = http.header(name, value);
    }

    let res = http
        .send()
        .await
        .map_err(|e| EgressError::Http(e.to_string()))?;
    let status = res.status();
    if status.as_u16() == 429 {
        let retry_after = throttle::retry_after_from_headers(res.headers());
        throttle::on_rate_limited(&req.throttle_key, retry_after);
        return Err(EgressError::RateLimited { retry_after });
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(EgressError::Status {
            code: status.as_u16(),
            body,
        });
    }
    throttle::on_success(&req.throttle_key);

    let cost_usd = res
        .headers()
        .get("x-response-cost")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok());

    let byte_stream = res.bytes_stream();
    let out = stream! {
        let _permit = permit;
        let mut buf = Utf8LineBuffer::new();
        futures_util::pin_mut!(byte_stream);
        while let Some(chunk_res) = byte_stream.next().await {
            match chunk_res {
                Ok(bytes) => {
                    let mut emitted: Vec<SseDeltaEvent> = Vec::new();
                    buf.push_lossy_bytes(&bytes, |line| {
                        if let Some(ev) = sse_data_line_delta_event(line) {
                            emitted.push(ev);
                        }
                    });
                    for ev in emitted {
                        yield Ok(ev);
                    }
                }
                Err(e) => {
                    yield Err(EgressError::Http(e.to_string()));
                    return;
                }
            }
        }
        let mut tail: Vec<SseDeltaEvent> = Vec::new();
        buf.flush_trailing(|line| {
            if let Some(ev) = sse_data_line_delta_event(line) {
                tail.push(ev);
            }
        });
        for ev in tail {
            yield Ok(ev);
        }
    };
    Ok((Box::pin(out), cost_usd))
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

/// Embeddings POST. `req.base_url` must be the embeddings endpoint (caller-resolved).
/// Ported from `vox-actor-runtime/src/llm/embed.rs`.
pub async fn embed_once(req: &EgressRequest, text: &str) -> Result<Vec<f32>, EgressError> {
    let client = vox_http_client::client();
    let _permit = throttle::acquire_permit(&req.throttle_key, req.max_concurrent).await;

    let body = EmbedRequest {
        model: &req.model,
        input: text,
    };
    let mut http = apply_auth_headers(client.post(&req.base_url).json(&body), req);
    if let Some(ms) = req.timeout_ms {
        http = http.timeout(std::time::Duration::from_millis(ms));
    }
    let res = http
        .send()
        .await
        .map_err(|e| EgressError::Http(e.to_string()))?;
    let status = res.status();
    if status.as_u16() == 429 {
        let retry_after = throttle::retry_after_from_headers(res.headers());
        throttle::on_rate_limited(&req.throttle_key, retry_after);
        return Err(EgressError::RateLimited { retry_after });
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(EgressError::Status {
            code: status.as_u16(),
            body,
        });
    }
    throttle::on_success(&req.throttle_key);
    let json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| EgressError::Decode(e.to_string()))?;
    let vector = json["data"][0]["embedding"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect()
        })
        .unwrap_or_default();
    Ok(vector)
}
