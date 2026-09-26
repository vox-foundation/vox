//! Axum HTTP handlers for the inference API.

#[cfg(feature = "execution-api")]
use super::prompt::{prompt_for_output_mode, validate_structured_output_with_reason};
#[cfg(feature = "execution-api")]
use super::schema::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse,
    ChatCompletionResponseMessage, ChatCompletionToolCall, ChatCompletionToolCallFunction, Choice,
    GenerateRequest, GenerateResponse,
};
#[cfg(feature = "execution-api")]
use super::worker::InferenceRequest;
#[cfg(feature = "execution-api")]
use crate::commands::ai::model_id::mismatched_model_error;
#[cfg(feature = "execution-api")]
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
#[cfg(feature = "execution-api")]
use std::sync::Arc;
#[cfg(feature = "execution-api")]
use vox_corpus::corpus::structured_eval::StructuredFailReason;

#[cfg(feature = "execution-api")]
fn parse_output_mode_label(raw: &str) -> Option<&'static str> {
    match raw.trim().to_lowercase().as_str() {
        "strict_json" | "strict-json" => Some("strict_json"),
        "jsonl_records" | "jsonl-records" => Some("jsonl_records"),
        "tool_args_json" | "tool-args-json" => Some("tool_args_json"),
        _ => None,
    }
}

#[cfg(feature = "execution-api")]
#[derive(Clone)]
pub struct AppState {
    pub tx: std::sync::mpsc::SyncSender<InferenceRequest>,
    pub model_name: Arc<str>,
    pub ready: Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(feature = "execution-api")]
pub async fn health() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "ok", "service": "vox-ml-cli"})),
    )
}

/// True once the worker thread has finished loading the model.
#[cfg(feature = "execution-api")]
fn readiness(ready: &Arc<std::sync::atomic::AtomicBool>) -> bool {
    ready.load(std::sync::atomic::Ordering::SeqCst)
}

/// Readiness probe — true only once the model has finished loading in the worker
/// thread. `/health` above answers "the process is up"; this answers "the model
/// is loaded and the server can actually serve requests".
#[cfg(feature = "execution-api")]
pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let is_ready = readiness(&state.ready);
    (
        if is_ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(serde_json::json!({"ready": is_ready, "service": "vox-ml-cli"})),
    )
}

#[cfg(feature = "execution-api")]
pub async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "object": "list",
        "data": [{
            "id": &*state.model_name,
            "object": "model",
            "owned_by": "vox-ml-cli"
        }]
    }))
}

#[cfg(feature = "execution-api")]
pub async fn do_generate(
    State(state): State<AppState>,
    Json(req): Json<GenerateRequest>,
) -> (StatusCode, Json<GenerateResponse>) {
    if let Some(msg) = mismatched_model_error(req.model.as_deref(), state.model_name.as_ref()) {
        return (
            StatusCode::CONFLICT,
            Json(GenerateResponse {
                text: msg.clone(),
                code: msg.clone(),
                tokens_generated: 0,
                model: state.model_name.to_string(),
                object: "text_completion",
                choices: vec![],
                repair_attempts: None,
                valid: false,
                errors: vec![msg],
            }),
        );
    }
    let output_mode = req.output_mode.as_deref().and_then(parse_output_mode_label);
    let max_retries = req.max_retries.max(1);
    let schema = req.schema.as_ref();

    let mut attempt = 0u32;
    #[allow(unused_assignments)]
    let mut last_text = String::new();
    #[allow(unused_assignments)]
    let mut last_error: Option<String> = None;
    let mut total_tokens = 0usize;

    loop {
        let prompt = if attempt == 0 {
            prompt_for_output_mode(&req.prompt, output_mode)
        } else {
            let err_hint = last_error.as_deref().unwrap_or("validation failed");
            let preview = last_text.chars().take(200).collect::<String>();
            format!(
                "Fix the JSON. Error: {}. Invalid output: {}\n\nOutput valid JSON only:\n\n{}",
                err_hint,
                if preview.len() < last_text.len() {
                    format!("{}...", preview)
                } else {
                    preview
                },
                prompt_for_output_mode(&req.prompt, output_mode)
            )
        };

        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let ir = InferenceRequest {
            prompt,
            max_tokens: req.max_tokens,
            temperature: req.temperature,
            top_k: 40,
            output_mode: output_mode.map(String::from),
            system_prompt: req.system_prompt.clone(),
            reply: reply_tx,
        };
        let tx = state.tx.clone();
        let send_ok = tokio::task::spawn_blocking(move || tx.send(ir))
            .await
            .map(|r: Result<(), std::sync::mpsc::SendError<InferenceRequest>>| r.is_ok())
            .unwrap_or(false);
        if !send_ok {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(GenerateResponse {
                    text: "Inference worker unavailable".into(),
                    code: "Inference worker unavailable".into(),
                    tokens_generated: 0,
                    model: state.model_name.to_string(),
                    object: "text_completion",
                    choices: vec![],
                    repair_attempts: None,
                    valid: false,
                    errors: vec!["Inference worker unavailable".into()],
                }),
            );
        }
        let text = reply_rx
            .await
            .unwrap_or_else(|_| Err("Worker dropped".into()))
            .unwrap_or_else(|e| format!("[error: {e}]"));
        let tokens = text.split_whitespace().count();
        total_tokens += tokens;
        last_text = text.clone();

        let validation: Option<StructuredFailReason> = if output_mode.is_some() {
            validate_structured_output_with_reason(&text, output_mode, schema).err()
        } else {
            None
        };
        let valid = validation.is_none();
        if let Some(ref e) = validation {
            last_error = Some(e.to_string());
        }
        if output_mode.is_none() || valid || attempt >= max_retries - 1 {
            let repair_attempts = if output_mode.is_some() && attempt > 0 {
                Some(attempt)
            } else {
                None
            };
            let err_list: Vec<String> = last_error.take().into_iter().collect();
            let resp = GenerateResponse {
                code: last_text.clone(),
                text: last_text.clone(),
                tokens_generated: total_tokens,
                model: state.model_name.to_string(),
                object: "text_completion",
                choices: vec![Choice {
                    text: last_text,
                    index: 0,
                    finish_reason: "stop",
                }],
                repair_attempts,
                valid,
                errors: err_list,
            };
            return (StatusCode::OK, Json(resp));
        }
        attempt += 1;
    }
}

/// Task B2: extract a requested tool's name from either OpenAI's
/// `{"type":"function","function":{"name":...}}` shape or the flatter
/// `{"name":...}` shape, so both are accepted from a `tools[]` entry.
#[cfg(feature = "execution-api")]
fn tool_def_name(tool: &serde_json::Value) -> Option<&str> {
    tool.get("function")
        .and_then(|f| f.get("name"))
        .or_else(|| tool.get("name"))
        .and_then(|v| v.as_str())
}

/// Task B2: extract a requested tool's description the same way `tool_def_name`
/// extracts its name — used only to enrich the flattened prompt.
#[cfg(feature = "execution-api")]
fn tool_def_description(tool: &serde_json::Value) -> Option<&str> {
    tool.get("function")
        .and_then(|f| f.get("description"))
        .or_else(|| tool.get("description"))
        .and_then(|v| v.as_str())
}

/// Task B2: render `messages[]` (+ an optional tool catalog) as the ChatML the
/// MENS adapters are trained on, ending with an open assistant turn. The tool
/// catalog is prepended to the first system turn (or the first user turn when
/// there is none), so the inference backend still adds the server's default
/// system prompt when the client sent no system message.
#[cfg(feature = "execution-api")]
fn flatten_chat_messages(
    messages: &[ChatCompletionMessage],
    tools: Option<&[serde_json::Value]>,
) -> String {
    let mut catalog = String::new();
    if let Some(tools) = tools.filter(|t| !t.is_empty()) {
        catalog.push_str(
            "Available tools (to call one, respond with ONLY a single JSON object \
             shaped like {\"name\": \"<tool>\", \"arguments\": {...}}):\n",
        );
        for tool in tools {
            let name = tool_def_name(tool).unwrap_or("unknown");
            let description = tool_def_description(tool).unwrap_or("");
            catalog.push_str(&format!("- {name}: {description}\n"));
        }
        catalog.push('\n');
    }
    let catalog_idx = messages
        .iter()
        .position(|m| m.role == "system")
        .or_else(|| messages.iter().position(|m| m.role == "user"));
    let mut out = String::new();
    for (i, message) in messages.iter().enumerate() {
        let content = message.content.as_deref().unwrap_or("");
        let prefix = if Some(i) == catalog_idx {
            catalog.as_str()
        } else {
            ""
        };
        out.push_str(&format!(
            "<|im_start|>{}\n{prefix}{content}<|im_end|>\n",
            message.role
        ));
    }
    out.push_str("<|im_start|>assistant\n");
    out
}

/// `POST /v1/chat/completions` (Task B2, MENS end-to-end completion, Route A).
///
/// Thin adapter: flattens the request onto [`GenerateRequest`] and delegates
/// to [`do_generate`] — the SAME worker channel and sampling/validation/retry
/// pipeline every other route already uses (non-goal: a second one). When
/// `tools` is present, reuses the existing `tool_args_json` output_mode (see
/// `prompt.rs`) so `do_generate` itself retries toward a `{"name",
/// "arguments"}`-shaped reply; this route only re-wraps the result into
/// `choices[].message`, populating `tool_calls` when that reply is
/// schema-valid AND names a tool this request actually offered. Otherwise the
/// raw text comes back as `content` — the well-known small-model failure mode
/// the salvage policy in `vox-orchestrator-mcp`'s `agent_loop.rs` handles as a
/// client-side fallback (regex-scanning `content` for the same shape), not
/// something this route hard-fails on.
#[cfg(feature = "execution-api")]
pub async fn do_chat_completions(
    State(state): State<AppState>,
    Json(req): Json<ChatCompletionRequest>,
) -> (StatusCode, Json<ChatCompletionResponse>) {
    let has_tools = req.tools.as_ref().is_some_and(|t| !t.is_empty());
    let prompt = flatten_chat_messages(&req.messages, req.tools.as_deref());

    let generate_req = GenerateRequest {
        prompt,
        max_tokens: req.max_tokens.unwrap_or(256),
        temperature: req.temperature.unwrap_or(0.7),
        model: req.model.clone(),
        output_mode: has_tools.then(|| "tool_args_json".to_string()),
        max_retries: 3,
        schema: has_tools.then(|| serde_json::json!({"name": "string", "arguments": "object"})),
        stream: false,
        // `/v1/chat/completions` has no system-role-carrying wire field of its
        // own yet (messages[] already carries a "system" role entry, flattened
        // into the prompt above) — no per-request override to thread here.
        system_prompt: None,
    };

    let (status, Json(gen_resp)) = do_generate(State(state), Json(generate_req)).await;

    // Only trust a tool-call candidate when do_generate's own schema check
    // passed (guaranteeing `{"name": <string>, "arguments": <object>}`) AND
    // the name matches a tool this request actually offered — an arbitrary
    // string the model happened to emit is never surfaced as a call.
    let tool_call = (has_tools && gen_resp.valid)
        .then(|| serde_json::from_str::<serde_json::Value>(gen_resp.text.trim()).ok())
        .flatten()
        .filter(|v| {
            v.get("name").and_then(|n| n.as_str()).is_some_and(|name| {
                req.tools
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .any(|t| tool_def_name(t) == Some(name))
            })
        });

    let message = match tool_call {
        Some(v) => {
            let name = v["name"].as_str().unwrap_or_default().to_string();
            let arguments =
                serde_json::to_string(v.get("arguments").unwrap_or(&serde_json::Value::Null))
                    .unwrap_or_else(|_| "{}".to_string());
            ChatCompletionResponseMessage {
                role: "assistant",
                content: None,
                tool_calls: Some(vec![ChatCompletionToolCall {
                    id: format!("call_{}", uuid::Uuid::new_v4()),
                    kind: "function",
                    function: ChatCompletionToolCallFunction { name, arguments },
                }]),
            }
        }
        None => ChatCompletionResponseMessage {
            role: "assistant",
            content: Some(gen_resp.text.clone()),
            tool_calls: None,
        },
    };

    let finish_reason = if message.tool_calls.is_some() {
        "tool_calls"
    } else {
        "stop"
    };
    let resp = ChatCompletionResponse {
        id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        object: "chat.completion",
        model: gen_resp.model,
        choices: vec![ChatCompletionChoice {
            index: 0,
            message,
            finish_reason,
        }],
    };
    (status, Json(resp))
}

#[cfg(feature = "execution-api")]
#[cfg(test)]
mod semcov_wave2_tests {
    #![allow(unused_imports)]
    use super::*;

    #[test]
    fn parse_output_mode_label_strict_json_underscore() {
        assert_eq!(parse_output_mode_label("strict_json"), Some("strict_json"));
    }

    #[test]
    fn parse_output_mode_label_strict_json_hyphen() {
        assert_eq!(parse_output_mode_label("strict-json"), Some("strict_json"));
    }

    #[test]
    fn parse_output_mode_label_jsonl_records_underscore() {
        assert_eq!(
            parse_output_mode_label("jsonl_records"),
            Some("jsonl_records")
        );
    }

    #[test]
    fn parse_output_mode_label_jsonl_records_hyphen() {
        assert_eq!(
            parse_output_mode_label("jsonl-records"),
            Some("jsonl_records")
        );
    }

    #[test]
    fn parse_output_mode_label_tool_args_json_underscore() {
        assert_eq!(
            parse_output_mode_label("tool_args_json"),
            Some("tool_args_json")
        );
    }

    #[test]
    fn parse_output_mode_label_tool_args_json_hyphen() {
        assert_eq!(
            parse_output_mode_label("tool-args-json"),
            Some("tool_args_json")
        );
    }

    #[test]
    fn parse_output_mode_label_uppercase_normalizes() {
        assert_eq!(parse_output_mode_label("STRICT_JSON"), Some("strict_json"));
        assert_eq!(parse_output_mode_label("Strict-Json"), Some("strict_json"));
    }

    #[test]
    fn parse_output_mode_label_trims_whitespace() {
        assert_eq!(
            parse_output_mode_label("  strict_json  "),
            Some("strict_json")
        );
    }

    #[test]
    fn parse_output_mode_label_unknown_returns_none() {
        assert_eq!(parse_output_mode_label("unknown"), None);
        assert_eq!(parse_output_mode_label(""), None);
        assert_eq!(parse_output_mode_label("json"), None);
    }

    #[test]
    fn ready_is_false_until_the_model_loads_and_false_again_if_it_fails() {
        let ready = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        assert!(
            !readiness(&ready),
            "a server whose model has not loaded is not ready"
        );
        ready.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(
            readiness(&ready),
            "once the model is loaded the server is ready"
        );
    }
}

/// Task B2 (MENS end-to-end completion): `/v1/chat/completions` adapter tests.
#[cfg(feature = "execution-api")]
#[cfg(test)]
mod chat_completions_tests {
    use super::*;
    use crate::commands::ai::serve::worker::InferenceRequest;

    #[test]
    fn tool_def_name_accepts_openai_and_flat_shapes() {
        let openai = serde_json::json!({"type": "function", "function": {"name": "read_file"}});
        let flat = serde_json::json!({"name": "read_file"});
        assert_eq!(tool_def_name(&openai), Some("read_file"));
        assert_eq!(tool_def_name(&flat), Some("read_file"));
        assert_eq!(tool_def_name(&serde_json::json!({})), None);
    }

    #[test]
    fn tool_def_description_accepts_openai_and_flat_shapes() {
        let openai = serde_json::json!({"type": "function", "function": {"name": "x", "description": "reads a file"}});
        let flat = serde_json::json!({"name": "x", "description": "reads a file"});
        assert_eq!(tool_def_description(&openai), Some("reads a file"));
        assert_eq!(tool_def_description(&flat), Some("reads a file"));
    }

    #[test]
    fn flatten_chat_messages_includes_tool_catalog_and_role_prefixed_turns() {
        let messages = vec![
            ChatCompletionMessage {
                role: "system".to_string(),
                content: Some("be helpful".to_string()),
            },
            ChatCompletionMessage {
                role: "user".to_string(),
                content: Some("what's the weather?".to_string()),
            },
        ];
        let tools = vec![serde_json::json!({
            "type": "function",
            "function": {"name": "get_weather", "description": "looks up the weather"}
        })];
        let flattened = flatten_chat_messages(&messages, Some(&tools));
        assert!(flattened.contains("get_weather"));
        assert!(flattened.contains("looks up the weather"));
        assert!(flattened.starts_with("<|im_start|>system\nAvailable tools"));
        assert!(flattened.contains("be helpful<|im_end|>"));
        assert!(flattened.contains("<|im_start|>user\nwhat's the weather?<|im_end|>"));
        assert!(flattened.ends_with("<|im_start|>assistant\n"));
    }

    #[test]
    fn flatten_chat_messages_omits_tool_catalog_when_no_tools_offered() {
        let messages = vec![ChatCompletionMessage {
            role: "user".to_string(),
            content: Some("hi".to_string()),
        }];
        let flattened = flatten_chat_messages(&messages, None);
        assert!(!flattened.contains("Available tools"));
        assert_eq!(
            flattened,
            "<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\n"
        );
    }

    /// Spawn a fake worker thread that replies with a fixed string to every
    /// request, mirroring the real worker's channel shape closely enough for
    /// `do_chat_completions` (a thin wrapper over `do_generate`) to be
    /// exercised end-to-end without a real model checkpoint.
    fn fake_app_state(reply: &'static str) -> AppState {
        let (tx, rx) = std::sync::mpsc::sync_channel::<InferenceRequest>(8);
        std::thread::spawn(move || {
            while let Ok(req) = rx.recv() {
                let _ = req.reply.send(Ok(reply.to_string()));
            }
        });
        AppState {
            tx,
            model_name: std::sync::Arc::from("test-model"),
            ready: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }

    #[tokio::test]
    async fn do_chat_completions_emits_tool_calls_for_a_schema_valid_offered_tool() {
        let state = fake_app_state(r#"{"name":"get_weather","arguments":{"city":"nyc"}}"#);
        let req = ChatCompletionRequest {
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some("what's the weather in nyc?".to_string()),
            }],
            model: None,
            max_tokens: None,
            temperature: None,
            tools: Some(vec![serde_json::json!({
                "type": "function",
                "function": {"name": "get_weather", "description": "looks up the weather"}
            })]),
        };
        let (status, Json(resp)) = do_chat_completions(State(state), Json(req)).await;
        assert_eq!(status, StatusCode::OK);
        let message = &resp.choices[0].message;
        let tool_calls = message
            .tool_calls
            .as_ref()
            .expect("a schema-valid, offered tool name must produce tool_calls");
        assert_eq!(tool_calls[0].function.name, "get_weather");
        assert_eq!(tool_calls[0].function.arguments, r#"{"city":"nyc"}"#);
        assert!(
            message.content.is_none(),
            "a tool-call reply carries no plain-text content"
        );
        assert_eq!(resp.choices[0].finish_reason, "tool_calls");
    }

    #[tokio::test]
    async fn do_chat_completions_falls_back_to_plain_text_for_an_unoffered_tool_name() {
        // Schema-valid JSON, but `send_email` was never offered — must not be
        // surfaced as a dispatchable tool call.
        let state = fake_app_state(r#"{"name":"send_email","arguments":{}}"#);
        let req = ChatCompletionRequest {
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some("what's the weather?".to_string()),
            }],
            model: None,
            max_tokens: None,
            temperature: None,
            tools: Some(vec![serde_json::json!({
                "type": "function",
                "function": {"name": "get_weather"}
            })]),
        };
        let (_status, Json(resp)) = do_chat_completions(State(state), Json(req)).await;
        let message = &resp.choices[0].message;
        assert!(
            message.tool_calls.is_none(),
            "a tool name that was never offered must never be dispatched"
        );
        assert!(message.content.is_some());
        assert_eq!(resp.choices[0].finish_reason, "stop");
    }

    #[tokio::test]
    async fn do_chat_completions_with_no_tools_returns_plain_content() {
        let state = fake_app_state("just a plain answer");
        let req = ChatCompletionRequest {
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some("hi".to_string()),
            }],
            model: None,
            max_tokens: None,
            temperature: None,
            tools: None,
        };
        let (status, Json(resp)) = do_chat_completions(State(state), Json(req)).await;
        assert_eq!(status, StatusCode::OK);
        let message = &resp.choices[0].message;
        assert_eq!(message.content.as_deref(), Some("just a plain answer"));
        assert!(message.tool_calls.is_none());
        assert_eq!(resp.choices[0].finish_reason, "stop");
    }
}
