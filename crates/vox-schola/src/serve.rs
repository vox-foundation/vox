//! `vox-schola serve` — OpenAI-compatible ChatCompletion HTTP server.
//!
//! Loads a trained QLoRA adapter + base weights and serves generation requests.
//!
//! # Protocol
//!
//! - `POST /v1/chat/completions` — OpenAI Chat Completions.
//! - `POST /api/chat`, `GET /api/tags` — Ollama-shaped (MCP-friendly).
//! - `POST /api/generate` — Ollama generate (`stream: true|false`) for [`vox_ludus`] and [`vox_runtime::mens::PopuliClient`].
//! - `GET /api/version` — hints for [`vox_runtime::inference_env::probe_populi_capabilities`].
//! - `POST /api/embeddings` — not implemented (501).

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpListener;

use crate::cli::{Args, Cmd};

// ── Request / Response wire types ─────────────────────────────────────────────

/// OpenAI-compatible chat completions request.
#[derive(Debug, Deserialize)]
struct ChatRequest {
    /// Model identifier (ignored; adapter loaded at startup).
    #[allow(dead_code)]
    model: Option<String>,
    /// Conversation turns.
    messages: Vec<ChatMessage>,
    /// Max new tokens to generate.
    max_tokens: Option<usize>,
    /// Sampling temperature (0 = greedy, default: 0.7).
    temperature: Option<f64>,
    /// Top-p nucleus sampling threshold.
    top_p: Option<f64>,
    /// Optional structured output forcing.
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
}

/// Ollama-compatible `POST /api/chat` request.
#[derive(Debug, Deserialize)]
struct OllamaChatRequest {
    #[allow(dead_code)]
    model: Option<String>,
    messages: Vec<ChatMessage>,
    #[allow(dead_code)]
    stream: Option<bool>,
    #[serde(default)]
    options: OllamaOptions,
}

#[derive(Debug, Deserialize, Default)]
struct OllamaOptions {
    temperature: Option<f64>,
    num_predict: Option<i32>,
}

/// Ollama `POST /api/generate` request.
#[derive(Debug, Deserialize)]
struct OllamaGenerateRequest {
    #[serde(default)]
    model: Option<String>,
    prompt: String,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    options: OllamaGenerateOptions,
}

#[derive(Debug, Deserialize, Default)]
struct OllamaGenerateOptions {
    temperature: Option<f64>,
    num_predict: Option<u32>,
}

/// Single conversation turn.
#[derive(Debug, Deserialize, Serialize, Clone)]
struct ChatMessage {
    /// Role: system | user | assistant.
    role: String,
    /// Message text.
    content: String,
}

/// OpenAI-compatible chat completions response.
#[derive(Serialize)]
struct ChatResponse {
    id: String,
    object: String,
    choices: Vec<ChatChoice>,
    usage: UsageSummary,
}

#[derive(Serialize)]
struct ChatChoice {
    index: u32,
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Serialize)]
struct UsageSummary {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Serialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaTagModel>,
}

#[derive(Serialize)]
struct OllamaTagModel {
    name: String,
    model: String,
}

#[derive(Serialize)]
struct OllamaChatResponse {
    model: String,
    message: ChatMessage,
    done: bool,
    prompt_eval_count: usize,
    eval_count: usize,
}

/// Ollama `POST /api/generate` non-streaming response (subset; extra fields ignored by clients).
#[derive(Serialize)]
struct OllamaGenerateResponse {
    model: String,
    response: String,
    done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_eval_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eval_count: Option<usize>,
}

// ── Engine ─────────────────────────────────────────────────────────────────────

/// Inference configuration.
struct ServeConfig {
    model_dir: PathBuf,
    model_name: String,
    max_tokens: usize,
    temperature: f64,
    device: String,
    domain_router: Option<vox_populi::mens::tensor::domain_router::DomainRouter>,
}

/// Shared state for Axum handlers.
struct AppState {
    config: ServeConfig,
}

// ── Axum handler ──────────────────────────────────────────────────────────────

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> impl IntoResponse {
    let max_tokens = req.max_tokens.unwrap_or(state.config.max_tokens);
    let temperature = req.temperature.unwrap_or(state.config.temperature);
    let top_p = req.top_p;
    let json_mode = req
        .response_format
        .and_then(|v| {
            v.get("type")
                .and_then(|t| t.as_str())
                .map(|t| t == "json_object")
        })
        .unwrap_or(false);

    let prompt = build_prompt(&req.messages);
    let result = tokio::task::spawn_blocking({
        let mut model_dir = state.config.model_dir.clone();
        if let Some(req_model) = &req.model {
            if let Some(router) = &state.config.domain_router {
                if let Some(path) = router.route(req_model) {
                    if let Some(parent) = path.parent() {
                        model_dir = parent.to_path_buf();
                    }
                }
            }
        }
        let device = state.config.device.clone();
        let prompt_clone = prompt.clone();
        move || {
            generate_response(
                &model_dir,
                &prompt_clone,
                &device,
                max_tokens,
                temperature,
                top_p,
                json_mode,
            )
        }
    })
    .await;

    match result {
        Ok(Ok(text)) => {
            let prompt_tokens = prompt.split_whitespace().count();
            let completion_tokens = text.split_whitespace().count();
            Json(ChatResponse {
                id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                object: "chat.completion".into(),
                choices: vec![ChatChoice {
                    index: 0,
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: text,
                    },
                    finish_reason: "stop".into(),
                }],
                usage: UsageSummary {
                    prompt_tokens,
                    completion_tokens,
                    total_tokens: prompt_tokens + completion_tokens,
                },
            })
            .into_response()
        }
        Ok(Err(e)) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("inference error: {e}"),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("task error: {e}"),
        )
            .into_response(),
    }
}

async fn ollama_tags(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(OllamaTagsResponse {
        models: vec![OllamaTagModel {
            name: state.config.model_name.clone(),
            model: state.config.model_name.clone(),
        }],
    })
}

async fn ollama_generate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OllamaGenerateRequest>,
) -> Response {
    let max_tokens = req
        .options
        .num_predict
        .map(|v| v.max(1) as usize)
        .unwrap_or(state.config.max_tokens);
    let temperature = req.options.temperature.unwrap_or(state.config.temperature);

    let result = tokio::task::spawn_blocking({
        let mut model_dir = state.config.model_dir.clone();
        if let Some(ref req_model) = req.model {
            if let Some(router) = &state.config.domain_router {
                if let Some(path) = router.route(req_model) {
                    if let Some(parent) = path.parent() {
                        model_dir = parent.to_path_buf();
                    }
                }
            }
        }
        let device = state.config.device.clone();
        let prompt = req.prompt.clone();
        move || {
            generate_response(
                &model_dir,
                &prompt,
                &device,
                max_tokens,
                temperature,
                None,
                false,
            )
        }
    })
    .await;

    let model_name = state.config.model_name.clone();
    let text = match result {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("inference error: {e}"),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("task error: {e}"),
            )
                .into_response();
        }
    };

    let prompt_tokens = req.prompt.split_whitespace().count();
    let completion_tokens = text.split_whitespace().count();

    if req.stream {
        let ndjson = ollama_generate_ndjson_body(&model_name, &text);

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/x-ndjson")
            .body(axum::body::Body::from(ndjson))
            .unwrap_or_else(|_| {
                (StatusCode::INTERNAL_SERVER_ERROR, "body build error").into_response()
            })
    } else {
        Json(OllamaGenerateResponse {
            model: model_name,
            response: text,
            done: true,
            prompt_eval_count: Some(prompt_tokens),
            eval_count: Some(completion_tokens),
        })
        .into_response()
    }
}

async fn ollama_embeddings() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "vox-schola local inference does not implement /api/embeddings; use Ollama.app or an embedding-capable endpoint"
        })),
    )
}

async fn ollama_version(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let device_lc = state.config.device.to_lowercase();
    let notes = if device_lc.contains("cuda") {
        "schola-candle-cuda"
    } else if device_lc.contains("metal") {
        "schola-candle-metal"
    } else {
        "schola-candle"
    };
    Json(json!({
        "version": "vox-schola",
        "ollama": "compat-subset",
        "runtime": notes,
        "cuda": device_lc.contains("cuda")
    }))
    .into_response()
}

async fn ollama_chat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OllamaChatRequest>,
) -> impl IntoResponse {
    let max_tokens = req
        .options
        .num_predict
        .map(|v| v.max(1) as usize)
        .unwrap_or(state.config.max_tokens);
    let temperature = req.options.temperature.unwrap_or(state.config.temperature);
    let prompt = build_prompt(&req.messages);
    let result = tokio::task::spawn_blocking({
        let mut model_dir = state.config.model_dir.clone();
        if let Some(req_model) = &req.model {
            if let Some(router) = &state.config.domain_router {
                if let Some(path) = router.route(req_model) {
                    if let Some(parent) = path.parent() {
                        model_dir = parent.to_path_buf();
                    }
                }
            }
        }
        let device = state.config.device.clone();
        let prompt_clone = prompt.clone();
        move || {
            generate_response(
                &model_dir,
                &prompt_clone,
                &device,
                max_tokens,
                temperature,
                None,
                false,
            )
        }
    })
    .await;

    match result {
        Ok(Ok(text)) => {
            let prompt_tokens = prompt.split_whitespace().count();
            let completion_tokens = text.split_whitespace().count();
            Json(OllamaChatResponse {
                model: state.config.model_name.clone(),
                message: ChatMessage {
                    role: "assistant".into(),
                    content: text,
                },
                done: true,
                prompt_eval_count: prompt_tokens,
                eval_count: completion_tokens,
            })
            .into_response()
        }
        Ok(Err(e)) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("inference error: {e}"),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("task error: {e}"),
        )
            .into_response(),
    }
}

/// Build a Qwen-style ChatML prompt string from conversation messages.
fn build_prompt(messages: &[ChatMessage]) -> String {
    let mut s = String::new();
    for m in messages {
        s.push_str("<|im_start|>");
        s.push_str(&m.role);
        s.push('\n');
        s.push_str(&m.content);
        s.push_str("<|im_end|>\n");
    }
    s.push_str("<|im_start|>assistant\n");
    s
}

/// Blocking inference call: load model + adapter, run autoregressive generation.
///
/// The Candle inference pipeline for adapter-based generation requires:
/// 1. Load base model weights from `model_dir/` safetensors shards.
/// 2. Load LoRA adapter from `model_dir/candle_qlora_adapter.safetensors`.
/// 3. Apply adapter to the model graph.
/// 4. Tokenize the prompt with `model_dir/tokenizer.json`.
/// 5. Run autoregressive generation with a KV cache.
///
/// This function is the integration point. The actual Candle model graph is
/// implemented in `vox_populi::mens::tensor::candle_model_qwen` and the serving
/// infrastructure in `vox_populi::mens::tensor::candle_inference_serve`.
/// Build Ollama-style NDJSON for `stream: true` (`/api/generate`), one JSON object per line.
fn ollama_generate_ndjson_body(model_name: &str, text: &str) -> String {
    let mut ndjson = String::new();
    const CHUNK: usize = 24;
    let mut start = 0usize;
    while start < text.len() {
        let end = (start + CHUNK).min(text.len());
        let piece = &text[start..end];
        let line = json!({
            "model": model_name,
            "response": piece,
            "done": false
        });
        ndjson.push_str(&line.to_string());
        ndjson.push('\n');
        start = end;
    }
    let final_line = json!({
        "model": model_name,
        "response": "",
        "done": true
    });
    ndjson.push_str(&final_line.to_string());
    ndjson.push('\n');
    ndjson
}

fn generate_response(
    model_dir: &std::path::Path,
    prompt: &str,
    device: &str,
    max_new_tokens: usize,
    temperature: f64,
    top_p: Option<f64>,
    json_mode: bool,
) -> Result<String> {
    let grammar_mode = if json_mode {
        vox_constrained_gen::GrammarMode::Json
    } else {
        vox_constrained_gen::GrammarMode::None
    };
    // Resolve device
    let device_kind =
        vox_populi::mens::normalize_device(device).map_err(|e| anyhow::anyhow!("{}", e))?;
    vox_populi::mens::apply_backend_env(device_kind);

    let adapter_path = model_dir.join("candle_qlora_adapter.safetensors");
    let tokenizer_path = model_dir.join("tokenizer.json");

    if !adapter_path.is_file() {
        anyhow::bail!(
            "adapter not found at {}. Train first with: vox-schola train --model <HF_REPO>",
            adapter_path.display()
        );
    }
    if !tokenizer_path.is_file() {
        anyhow::bail!("tokenizer.json not found at {}.", tokenizer_path.display());
    }

    let mut engine = vox_populi::mens::tensor::candle_inference_serve::InferenceEngine::load(
        model_dir,
        &device_kind,
    )?;
    engine.generate(prompt, max_new_tokens, temperature, top_p, &grammar_mode)
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn run(args: Args) -> Result<()> {
    let Cmd::Serve {
        model,
        port,
        host,
        max_tokens,
        temperature,
        device,
        model_name,
    } = args.cmd
    else {
        unreachable!()
    };

    if !model.is_dir() {
        anyhow::bail!(
            "model directory not found: {}. Specify the output_dir from a completed training run.",
            model.display()
        );
    }

    let default_name = model
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or("vox-schola-local")
        .to_string();
    let model_display_name = model_name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or(default_name);

    let state = Arc::new(AppState {
        config: ServeConfig {
            domain_router: vox_populi::mens::tensor::domain_router::DomainRouter::discover(
                model.parent().unwrap_or(&model),
            )
            .ok(),
            model_dir: model.clone(),
            model_name: model_display_name.clone(),
            max_tokens,
            temperature,
            device: device.clone(),
        },
    });

    let router = Router::new()
        .route("/api/tags", get(ollama_tags))
        .route("/api/version", get(ollama_version))
        .route("/api/chat", post(ollama_chat))
        .route("/api/generate", post(ollama_generate))
        .route("/api/embeddings", post(ollama_embeddings))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state);

    let addr = format!("{host}:{port}");
    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║   Vox Train Inference Server            ║");
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!("  Model:    {}", model.display());
    eprintln!("  Endpoint: http://{addr}/v1/chat/completions");
    eprintln!(
        "  Ollama:   http://{addr}/api/chat  /api/generate  /api/tags  /api/version  (embeddings -> 501)"
    );
    eprintln!("  Model id: {model_display_name}  (set --model-name to match POPULI_MODEL)");
    eprintln!("  Max tok:  {max_tokens}");
    eprintln!("  Temp:     {temperature}");
    eprintln!();
    eprintln!("Press Ctrl+C to stop.");

    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("bind to {addr}"))?;
    axum::serve(listener, router).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ollama_generate_ndjson_body;

    #[test]
    fn ndjson_stream_ends_with_done_and_chunks_response() {
        let body = ollama_generate_ndjson_body("test-model", "abcdefghijkl");
        let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
        assert!(!lines.is_empty());
        let last: serde_json::Value =
            serde_json::from_str(lines.last().expect("last line")).unwrap();
        assert_eq!(last["done"], true);
        assert_eq!(last["model"], "test-model");
        let mut joined = String::new();
        for l in &lines {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            if let Some(s) = v["response"].as_str() {
                joined.push_str(s);
            }
        }
        assert_eq!(joined, "abcdefghijkl");
    }
}
