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
    let lane = match req.lane.as_deref() {
        None => vox_eval::qwen_reasoning::Lane::Codegen,
        Some(raw) => match vox_eval::qwen_reasoning::Lane::parse(raw) {
            Ok(parsed) => parsed,
            Err(msg) => {
                return (
                    StatusCode::BAD_REQUEST,
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
        },
    };
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
            seed: req.seed,
            top_p: req.top_p,
            top_k: 40,
            output_mode: output_mode.map(String::from),
            system_prompt: req.system_prompt.clone(),
            lane,
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
    lane: vox_eval::qwen_reasoning::Lane,
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
    let last_user = messages.iter().rposition(|m| m.role == "user");
    let mut out = String::new();
    for (i, message) in messages.iter().enumerate() {
        let mut content = message.content.clone().unwrap_or_default();
        if Some(i) == last_user {
            content = vox_eval::qwen_reasoning::user_for_lane(&content, lane);
        }
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
    // The opener depends on the lane: codegen is direct-answer generation,
    // matching QLoRA training (prefilled empty think); hub/agent leave the
    // assistant turn open so the model thinks.
    out.push_str("<|im_start|>assistant\n");
    out.push_str(vox_eval::qwen_reasoning::assistant_opener_for_lane(lane));
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
    // Validate the lane before any other work: an unknown value is a JSON 400
    // (same body shape as a normal reply) and never reaches the worker.
    let lane = match req.lane.as_deref() {
        None => vox_eval::qwen_reasoning::Lane::Codegen,
        Some(raw) => match vox_eval::qwen_reasoning::Lane::parse(raw) {
            Ok(parsed) => parsed,
            Err(msg) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ChatCompletionResponse {
                        id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                        object: "chat.completion",
                        model: state.model_name.to_string(),
                        choices: vec![ChatCompletionChoice {
                            index: 0,
                            message: ChatCompletionResponseMessage {
                                role: "assistant",
                                content: Some(msg),
                                tool_calls: None,
                            },
                            finish_reason: "stop",
                        }],
                    }),
                );
            }
        },
    };
    let has_tools = req.tools.as_ref().is_some_and(|t| !t.is_empty());
    let prompt = flatten_chat_messages(&req.messages, req.tools.as_deref(), lane);

    let generate_req = GenerateRequest {
        prompt,
        max_tokens: req.max_tokens.unwrap_or(256),
        temperature: req.temperature.unwrap_or(0.7),
        seed: req.seed,
        top_p: req.top_p,
        model: req.model.clone(),
        output_mode: has_tools.then(|| "tool_args_json".to_string()),
        max_retries: 3,
        schema: has_tools.then(|| serde_json::json!({"name": "string", "arguments": "object"})),
        stream: false,
        // `/v1/chat/completions` has no system-role-carrying wire field of its
        // own yet (messages[] already carries a "system" role entry, flattened
        // into the prompt above) — no per-request override to thread here.
        system_prompt: None,
        // do_generate re-parses this; it is always Ok here (validated above).
        lane: req.lane.clone(),
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
        let flattened = flatten_chat_messages(
            &messages,
            Some(&tools),
            vox_eval::qwen_reasoning::Lane::Codegen,
        );
        assert!(flattened.contains("get_weather"));
        assert!(flattened.contains("looks up the weather"));
        assert!(flattened.starts_with("<|im_start|>system\nAvailable tools"));
        assert!(flattened.contains("be helpful<|im_end|>"));
        assert!(flattened.contains("<|im_start|>user\nwhat's the weather? /no_think<|im_end|>"));
        assert!(flattened.ends_with("<|im_start|>assistant\n<think>\n\n</think>\n\n"));
    }

    #[test]
    fn flatten_chat_messages_omits_tool_catalog_when_no_tools_offered() {
        let messages = vec![ChatCompletionMessage {
            role: "user".to_string(),
            content: Some("hi".to_string()),
        }];
        let flattened =
            flatten_chat_messages(&messages, None, vox_eval::qwen_reasoning::Lane::Codegen);
        assert!(!flattened.contains("Available tools"));
        assert_eq!(
            flattened,
            "<|im_start|>user\nhi /no_think<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
    }

    fn chat_msg(role: &str, content: &str) -> ChatCompletionMessage {
        ChatCompletionMessage {
            role: role.to_string(),
            content: Some(content.to_string()),
        }
    }

    #[test]
    fn flatten_chat_messages_hub_and_agent_open_a_bare_thinking_turn() {
        use vox_eval::qwen_reasoning::Lane;
        let messages = vec![chat_msg("user", "hi")];
        for lane in [Lane::Hub, Lane::Agent] {
            assert_eq!(
                flatten_chat_messages(&messages, None, lane),
                "<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\n",
                "lane {lane:?}"
            );
        }
        // Codegen stays today's literal.
        assert_eq!(
            flatten_chat_messages(&messages, None, Lane::Codegen),
            "<|im_start|>user\nhi /no_think<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
    }

    #[test]
    fn flatten_chat_messages_hub_strips_no_think_from_the_last_user_turn_only() {
        use vox_eval::qwen_reasoning::Lane;
        // Lane wins over user text: a trailing /no_think cannot flip hub to the direct-answer prefix.
        let with = flatten_chat_messages(&[chat_msg("user", "hi /no_think")], None, Lane::Hub);
        let without = flatten_chat_messages(&[chat_msg("user", "hi")], None, Lane::Hub);
        assert_eq!(with, without);
        // /think is kept.
        let think = flatten_chat_messages(&[chat_msg("user", "hi /think")], None, Lane::Hub);
        assert!(
            think.contains("<|im_start|>user\nhi /think<|im_end|>"),
            "{think}"
        );

        // Multi-turn: only the last user turn and the opener depend on the lane.
        let convo = vec![
            chat_msg("system", "S"),
            chat_msg("user", "a /no_think"),
            chat_msg("assistant", "b"),
            chat_msg("user", "c"),
        ];
        let marker = "<|im_start|>user\nc";
        let outs: Vec<String> = [Lane::Codegen, Lane::Hub, Lane::Agent]
            .into_iter()
            .map(|l| flatten_chat_messages(&convo, None, l))
            .collect();
        let heads: Vec<&str> = outs
            .iter()
            .map(|o| &o[..o.rfind(marker).expect("last user turn present")])
            .collect();
        assert_eq!(heads[0], heads[1]);
        assert_eq!(heads[1], heads[2]);
        assert!(outs[1].contains("<|im_start|>user\na /no_think<|im_end|>"));
        assert!(outs[1].ends_with("<|im_start|>assistant\n"));
        assert!(outs[0].ends_with("<think>\n\n</think>\n\n"));
    }

    #[test]
    fn flatten_chat_messages_codegen_keeps_user_think_and_the_empty_think_opener() {
        use vox_eval::qwen_reasoning::Lane;
        let out = flatten_chat_messages(&[chat_msg("user", "hi /think")], None, Lane::Codegen);
        assert!(
            out.contains("<|im_start|>user\nhi /think<|im_end|>"),
            "{out}"
        );
        assert!(out.ends_with("<think>\n\n</think>\n\n"));
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

    /// Like `fake_app_state`, but records every `InferenceRequest` it receives
    /// (before replying) so a test can assert on the lane and the payload.
    fn capturing_app_state(
        reply: &'static str,
    ) -> (
        AppState,
        std::sync::Arc<std::sync::Mutex<Vec<InferenceRequest>>>,
    ) {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = std::sync::Arc::clone(&captured);
        let (tx, rx) = std::sync::mpsc::sync_channel::<InferenceRequest>(8);
        std::thread::spawn(move || {
            while let Ok(mut req) = rx.recv() {
                let (dummy, _) = tokio::sync::oneshot::channel();
                let reply_tx = std::mem::replace(&mut req.reply, dummy);
                sink.lock().unwrap().push(req);
                let _ = reply_tx.send(Ok(reply.to_string()));
            }
        });
        let state = AppState {
            tx,
            model_name: std::sync::Arc::from("test-model"),
            ready: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        };
        (state, captured)
    }

    #[tokio::test]
    async fn do_generate_hub_lane_reaches_the_worker_payload() {
        let (state, captured) = capturing_app_state("a plain answer");
        let req: GenerateRequest =
            serde_json::from_value(serde_json::json!({"prompt": "hi", "lane": "hub"})).unwrap();
        let (status, Json(resp)) = do_generate(State(state), Json(req)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(resp.valid);
        let seen = captured.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].lane, vox_eval::qwen_reasoning::Lane::Hub);
        assert_eq!(
            seen[0].prompt, "hi",
            "the raw prompt is not wrapped host-side"
        );
        let v: serde_json::Value = serde_json::from_str(
            &crate::commands::ai::serve::worker::inference_payload("S", &seen[0]),
        )
        .unwrap();
        assert_eq!(v["lane"], "hub");
    }

    #[tokio::test]
    async fn do_generate_without_a_lane_serializes_codegen() {
        let (state, captured) = capturing_app_state("ok");
        let req: GenerateRequest =
            serde_json::from_value(serde_json::json!({"prompt": "hi"})).unwrap();
        let (status, _) = do_generate(State(state), Json(req)).await;
        assert_eq!(status, StatusCode::OK);
        let seen = captured.lock().unwrap();
        assert_eq!(seen[0].lane, vox_eval::qwen_reasoning::Lane::Codegen);
        let v: serde_json::Value = serde_json::from_str(
            &crate::commands::ai::serve::worker::inference_payload("S", &seen[0]),
        )
        .unwrap();
        assert_eq!(v["lane"], "codegen");
    }

    #[tokio::test]
    async fn unknown_lane_on_generate_is_400_before_the_worker() {
        for bad in ["bogus", "Hub", ""] {
            let (state, captured) = capturing_app_state("never used");
            let req: GenerateRequest =
                serde_json::from_value(serde_json::json!({"prompt": "hi", "lane": bad})).unwrap();
            let (status, Json(resp)) = do_generate(State(state), Json(req)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "lane {bad:?}");
            assert!(!resp.valid, "lane {bad:?}");
            assert!(!resp.errors.is_empty(), "lane {bad:?}");
            for name in ["codegen", "hub", "agent"] {
                assert!(resp.text.contains(name), "{:?} must name {name}", resp.text);
            }
            assert_eq!(resp.tokens_generated, 0);
            assert_eq!(
                captured.lock().unwrap().len(),
                0,
                "lane {bad:?} must not reach the worker"
            );
        }
    }

    #[tokio::test]
    async fn do_generate_repair_retries_keep_the_lane() {
        // strict_json is validated by do_generate; a non-JSON reply forces the repair loop.
        let (state, captured) = capturing_app_state("this is not json");
        let req: GenerateRequest = serde_json::from_value(serde_json::json!({
            "prompt": "hi",
            "lane": "hub",
            "output_mode": "strict_json",
            "max_retries": 3
        }))
        .unwrap();
        let (status, Json(resp)) = do_generate(State(state), Json(req)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!resp.valid);
        let seen = captured.lock().unwrap();
        assert!(
            seen.len() >= 2,
            "expected repair retries, got {}",
            seen.len()
        );
        assert!(
            seen.iter()
                .all(|r| r.lane == vox_eval::qwen_reasoning::Lane::Hub),
            "every retry must carry the request's lane"
        );
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
            seed: None,
            top_p: None,
            tools: Some(vec![serde_json::json!({
                "type": "function",
                "function": {"name": "get_weather", "description": "looks up the weather"}
            })]),
            lane: None,
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
            seed: None,
            top_p: None,
            tools: Some(vec![serde_json::json!({
                "type": "function",
                "function": {"name": "get_weather"}
            })]),
            lane: None,
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

    fn chat_request_with_lane(lane: Option<&str>) -> ChatCompletionRequest {
        ChatCompletionRequest {
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some("hi".to_string()),
            }],
            model: None,
            max_tokens: None,
            temperature: None,
            seed: None,
            top_p: None,
            tools: None,
            lane: lane.map(str::to_string),
        }
    }

    #[tokio::test]
    async fn unknown_lane_on_chat_is_400_before_the_worker() {
        for bad in ["bogus", "Agent", ""] {
            let (state, captured) = capturing_app_state("never used");
            let req = chat_request_with_lane(Some(bad));
            let (status, Json(resp)) = do_chat_completions(State(state), Json(req)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "lane {bad:?}");
            let content = resp.choices[0]
                .message
                .content
                .as_deref()
                .expect("the 400 carries the parse error as content");
            for name in ["codegen", "hub", "agent"] {
                assert!(content.contains(name), "{content:?} must name {name}");
            }
            assert_eq!(resp.choices[0].finish_reason, "stop");
            assert_eq!(resp.model, "test-model");
            assert_eq!(
                captured.lock().unwrap().len(),
                0,
                "lane {bad:?} must not reach the worker"
            );
        }
    }

    #[tokio::test]
    async fn do_chat_completions_hub_lane_reaches_the_worker_as_bare_thinking() {
        let (state, captured) = capturing_app_state("a plain answer");
        let req = chat_request_with_lane(Some("hub"));
        let (status, _) = do_chat_completions(State(state), Json(req)).await;
        assert_eq!(status, StatusCode::OK);
        let seen = captured.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].lane, vox_eval::qwen_reasoning::Lane::Hub);
        assert!(
            seen[0].prompt.ends_with("<|im_start|>assistant\n"),
            "{:?}",
            seen[0].prompt
        );
        assert!(
            !seen[0].prompt.contains("/no_think"),
            "{:?}",
            seen[0].prompt
        );
    }

    #[tokio::test]
    async fn do_chat_completions_without_a_lane_stays_codegen() {
        let (state, captured) = capturing_app_state("ok");
        let req = chat_request_with_lane(None);
        let (status, _) = do_chat_completions(State(state), Json(req)).await;
        assert_eq!(status, StatusCode::OK);
        let seen = captured.lock().unwrap();
        assert_eq!(seen[0].lane, vox_eval::qwen_reasoning::Lane::Codegen);
        assert!(
            seen[0].prompt.ends_with("<think>\n\n</think>\n\n"),
            "{:?}",
            seen[0].prompt
        );
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
            seed: None,
            top_p: None,
            tools: None,
            lane: None,
        };
        let (status, Json(resp)) = do_chat_completions(State(state), Json(req)).await;
        assert_eq!(status, StatusCode::OK);
        let message = &resp.choices[0].message;
        assert_eq!(message.content.as_deref(), Some("just a plain answer"));
        assert!(message.tool_calls.is_none());
        assert_eq!(resp.choices[0].finish_reason, "stop");
    }
}
