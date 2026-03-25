//! Inference worker thread. Owns the model; communicates via mpsc + oneshot.

#[cfg(feature = "execution-api")]
use super::config::ServeConfig;
#[cfg(feature = "execution-api")]
use anyhow::Result;
#[cfg(feature = "execution-api")]
#[cfg(feature = "execution-api")]
use std::sync::mpsc::SyncSender;

/// Internal message sent from Axum handlers to the inference worker thread.
#[cfg(feature = "execution-api")]
#[allow(dead_code)]
pub struct InferenceRequest {
    pub prompt: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_k: usize,
    pub output_mode: Option<String>,
    pub reply: tokio::sync::oneshot::Sender<Result<String, String>>,
    pub stream_tx: Option<tokio::sync::mpsc::Sender<Result<String, String>>>,
}

/// Spawn the inference worker thread and return the channel sender.
#[cfg(feature = "execution-api")]
pub fn spawn_inference_worker(
    config: &ServeConfig,
    model_name: &str,
    system_prompt: &str,
) -> SyncSender<InferenceRequest> {
    let _ = config;
    let _ = model_name;
    let _ = system_prompt;

    let (tx, rx) = std::sync::mpsc::sync_channel::<InferenceRequest>(8);
    std::thread::spawn(move || {
        while let Ok(req) = rx.recv() {
            let _ = req.reply.send(Err("Native burn inference was removed. Use a Candle backend runner or standalone serving pipeline.".to_string()));
        }
    });
    tx
}
