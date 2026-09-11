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
/// Wires into the host-selected Candle plugin via `MlBackend::run_inference`, which
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
    let system_prompt = system_prompt.to_string();

    let (tx, rx) = std::sync::mpsc::sync_channel::<InferenceRequest>(8);
    std::thread::spawn(move || {
        // Load the plugin once; keep it alive for the worker's lifetime.
        let plugin_id = match resolve_ml_backend_plugin(&vox_plugin_host::probe()) {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("no ML backend plugin matches this host: {e}");
                while let Ok(req) = rx.recv() {
                    let _ = req
                        .reply
                        .send(Err(format!("no ML backend plugin matches this host: {e}")));
                }
                return;
            }
        };
        let plugin_result = vox_plugin_host::cached_code_plugin(plugin_id);
        let plugin = match plugin_result {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("{plugin_id} plugin not found: {e}");
                while let Ok(req) = rx.recv() {
                    let _ = req
                        .reply
                        .send(Err(format!("{plugin_id} plugin unavailable: {e}")));
                }
                return;
            }
        };
        let backend = match plugin.plugin.as_ml_backend().into_option() {
            Some(b) => b,
            None => {
                tracing::error!("{plugin_id} plugin has no MlBackend");
                while let Ok(req) = rx.recv() {
                    let _ = req.reply.send(Err(format!("{plugin_id} has no MlBackend")));
                }
                return;
            }
        };
        let handle = match backend.load_model(model_path.as_str().into()).into_result() {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("load_model({model_path}): {e}");
                while let Ok(req) = rx.recv() {
                    let _ = req.reply.send(Err(format!("load_model failed: {e}")));
                }
                return;
            }
        };

        tracing::info!("Inference worker ready — model: {model_path}");
        while let Ok(req) = rx.recv() {
            let prompt_json = inference_payload(&system_prompt, &req);
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
            let _ = req.reply.send(result);
        }

        drop(handle);
    });
    tx
}

/// Build the JSON payload sent to the ML backend's `run_inference`, carrying the
/// trained system prompt and the sampling parameters the request specified.
#[cfg(feature = "execution-api")]
fn inference_payload(system_prompt: &str, req: &InferenceRequest) -> String {
    serde_json::json!({
        "system": system_prompt,
        "prompt": req.prompt,
        "max_tokens": req.max_tokens,
        "temperature": req.temperature,
        "top_k": req.top_k,
        "output_mode": req.output_mode,
    })
    .to_string()
}

#[cfg(feature = "execution-api")]
fn resolve_ml_backend_plugin(
    capabilities: &vox_plugin_host::CapabilitySet,
) -> Result<&'static str, vox_plugin_host::errors::LoadError> {
    vox_plugin_host::resolve_extension_point(
        "MlBackend",
        crate::commands::schola::merge_qlora::ML_BACKEND_CANDIDATES,
        capabilities,
    )
}

#[cfg(all(test, feature = "execution-api"))]
mod tests {
    use super::{InferenceRequest, inference_payload, resolve_ml_backend_plugin};

    #[test]
    fn metal_capability_selects_metal_serve_plugin() {
        let capabilities =
            vox_plugin_host::CapabilitySet::from_tags(["cpu-only", "apple-silicon", "metal"]);
        assert_eq!(
            resolve_ml_backend_plugin(&capabilities).unwrap(),
            "mens-candle-metal"
        );
    }

    #[test]
    fn the_payload_carries_the_system_prompt_and_the_sampling_parameters() {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let req = InferenceRequest {
            prompt: "hi".into(),
            max_tokens: 8,
            temperature: 0.0,
            top_k: 40,
            output_mode: Some("strict_json".into()),
            reply: tx,
            stream_tx: None,
        };
        let v: serde_json::Value =
            serde_json::from_str(&inference_payload("YOU ARE VOX", &req)).unwrap();
        assert_eq!(
            v["system"], "YOU ARE VOX",
            "the trained prompt format must reach the model"
        );
        assert_eq!(v["top_k"], 40);
        assert_eq!(
            v["output_mode"], "strict_json",
            "the JSON repair loop retries a model that was never asked for JSON"
        );
    }
}
