//! `vox-train serve` — OpenAI-compatible ChatCompletion HTTP server.
//!
//! Loads a trained QLoRA adapter + base weights and serves generation requests.
//!
//! # Protocol
//!
//! `POST /v1/chat/completions` with an OpenAI-compatible request body.
//! Response includes `choices[0].message.content` with the generated text.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
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

// ── Engine ─────────────────────────────────────────────────────────────────────

/// Inference configuration.
struct ServeConfig {
    model_dir: PathBuf,
    max_tokens: usize,
    temperature: f64,
    device: String,
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

    let prompt = build_prompt(&req.messages);
    let result = tokio::task::spawn_blocking({
        let model_dir = state.config.model_dir.clone();
        let device = state.config.device.clone();
        let prompt_clone = prompt.clone();
        move || {
            generate_response(&model_dir, &prompt_clone, &device, max_tokens, temperature, top_p)
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
                    message: ChatMessage { role: "assistant".into(), content: text },
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
/// implemented in `vox_populi::tensor::candle_model_qwen` and the serving
/// infrastructure in `vox_populi::tensor::candle_inference_serve`.
fn generate_response(
    model_dir: &std::path::Path,
    prompt: &str,
    device: &str,
    max_new_tokens: usize,
    temperature: f64,
    top_p: Option<f64>,
) -> Result<String> {
    // Resolve device
    let device_kind = vox_populi::normalize_device(device)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    vox_populi::apply_backend_env(device_kind);

    let adapter_path = model_dir.join("candle_qlora_adapter.safetensors");
    let tokenizer_path = model_dir.join("tokenizer.json");

    if !adapter_path.is_file() {
        anyhow::bail!(
            "adapter not found at {}. Train first with: vox-train train --model <HF_REPO>",
            adapter_path.display()
        );
    }
    if !tokenizer_path.is_file() {
        anyhow::bail!(
            "tokenizer.json not found at {}.",
            tokenizer_path.display()
        );
    }

    let mut engine = vox_populi::tensor::candle_inference_serve::InferenceEngine::load(model_dir, &device_kind)?;
    engine.generate(prompt, max_new_tokens, temperature, top_p)
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn run(args: Args) -> Result<()> {
    let Cmd::Serve { model, port, host, max_tokens, temperature, device } = args.cmd else {
        unreachable!()
    };

    if !model.is_dir() {
        anyhow::bail!(
            "model directory not found: {}. Specify the output_dir from a completed training run.",
            model.display()
        );
    }

    let state = Arc::new(AppState {
        config: ServeConfig {
            model_dir: model.clone(),
            max_tokens,
            temperature,
            device,
        },
    });

    let router = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state);

    let addr = format!("{host}:{port}");
    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║   Vox Train Inference Server            ║");
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!("  Model:    {}", model.display());
    eprintln!("  Endpoint: http://{addr}/v1/chat/completions");
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
