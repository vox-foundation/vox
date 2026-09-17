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
///
/// Wires into the `mens-candle-cuda` plugin via `MlBackend::run_inference`, which
/// loads the model directory (expects tokenizer.json, candle_qlora_adapter.safetensors /
/// merged.safetensors, adapter_manifest.json, config.json) on the first request and
/// generates from the plugin's `InferenceEngine::generate`. The plugin reloads the engine
/// on each call (stateless), so no model object is retained across requests.
#[cfg(feature = "execution-api")]
pub fn spawn_inference_worker(
    config: &ServeConfig,
    model_name: &str,
    system_prompt: &str,
) -> SyncSender<InferenceRequest> {
    let model_path = config.model_path.to_string_lossy().to_string();
    let _ = model_name;
    let _ = system_prompt;

    let (tx, rx) = std::sync::mpsc::sync_channel::<InferenceRequest>(8);
    std::thread::spawn(move || {
        // Load the plugin once; prefer metal on macos, falling back to cuda.
        let plugin_result = vox_plugin_host::cached_code_plugin("mens-candle-metal")
            .or_else(|_| vox_plugin_host::cached_code_plugin("mens-candle-cuda"));
        let plugin = match plugin_result {
            Ok(p) => p,
            Err(e) => {
                let err_msg = format!("mens-candle plugin unavailable: {e}");
                tracing::error!("{err_msg}");
                while let Ok(req) = rx.recv() {
                    if let Some(ref stx) = req.stream_tx {
                        let _ = stx.blocking_send(Err(err_msg.clone()));
                    }
                    let _ = req.reply.send(Err(err_msg.clone()));
                }
                return;
            }
        };
        let backend = match plugin.plugin.as_ml_backend().into_option() {
            Some(b) => b,
            None => {
                let err_msg = "mens-candle plugin has no MlBackend".to_string();
                tracing::error!("{err_msg}");
                while let Ok(req) = rx.recv() {
                    if let Some(ref stx) = req.stream_tx {
                        let _ = stx.blocking_send(Err(err_msg.clone()));
                    }
                    let _ = req.reply.send(Err(err_msg.clone()));
                }
                return;
            }
        };
        let handle = match backend.load_model(model_path.as_str().into()).into_result() {
            Ok(h) => h,
            Err(e) => {
                let err_msg = format!("load_model failed: {e}");
                tracing::error!("load_model({model_path}): {e}");
                while let Ok(req) = rx.recv() {
                    if let Some(ref stx) = req.stream_tx {
                        let _ = stx.blocking_send(Err(err_msg.clone()));
                    }
                    let _ = req.reply.send(Err(err_msg.clone()));
                }
                return;
            }
        };

        tracing::info!("Inference worker ready — model: {model_path}");
        while let Ok(req) = rx.recv() {
            let prompt_json = serde_json::json!({
                "prompt": req.prompt,
                "max_tokens": req.max_tokens,
                "temperature": req.temperature,
            })
            .to_string();
            let result = backend
                .run_inference(&handle, prompt_json.as_str().into())
                .into_result()
                .map_err(|e| e.to_string())
                .and_then(|resp| {
                    serde_json::from_str::<serde_json::Value>(resp.as_str())
                        .map_err(|e| e.to_string())
                        .map(|v| {
                            v.get("generated_text")
                                .and_then(|t| t.as_str())
                                .unwrap_or_default()
                                .to_string()
                        })
                });
            if let Some(ref stx) = req.stream_tx {
                match &result {
                    Ok(text) => {
                        let _ = stx.blocking_send(Ok(text.clone()));
                    }
                    Err(err) => {
                        let _ = stx.blocking_send(Err(err.clone()));
                    }
                }
            }
            let _ = req.reply.send(result);
        }

        drop(handle);
    });
    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inference_request_channel_contract() {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let req = InferenceRequest {
            prompt: "fn main() {}".to_string(),
            max_tokens: 128,
            temperature: 0.2,
            top_k: 40,
            output_mode: None,
            reply: reply_tx,
            stream_tx: None,
        };
        assert_eq!(req.max_tokens, 128);
        assert_eq!(req.prompt, "fn main() {}");
        let _ = req.reply.send(Ok("fn main() {}".to_string()));
        let res = reply_rx.blocking_recv().unwrap();
        assert_eq!(res.unwrap(), "fn main() {}");
    }
}
