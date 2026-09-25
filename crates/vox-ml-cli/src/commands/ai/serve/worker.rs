//! Inference worker thread. Owns the model; communicates via mpsc + oneshot.

#[cfg(feature = "execution-api")]
use super::config::ServeConfig;
#[cfg(feature = "execution-api")]
use anyhow::Result;
#[cfg(feature = "execution-api")]
#[cfg(feature = "execution-api")]
use std::sync::Arc;
#[cfg(feature = "execution-api")]
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// Per-request system prompt override (see `GenerateRequest::system_prompt`
    /// in `schema.rs`). When present and non-empty, REPLACES the worker's
    /// startup-default system prompt for this request; otherwise the
    /// startup default is used unchanged.
    pub system_prompt: Option<String>,
    pub reply: tokio::sync::oneshot::Sender<Result<String, String>>,
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
) -> (SyncSender<InferenceRequest>, Arc<AtomicBool>) {
    let model_path = config.model_path.to_string_lossy().to_string();
    let _ = model_name;
    let system_prompt = system_prompt.to_string();
    let ready = Arc::new(AtomicBool::new(false));
    let ready_for_worker = Arc::clone(&ready);

    let (tx, rx) = std::sync::mpsc::sync_channel::<InferenceRequest>(8);
    std::thread::spawn(move || {
        let ready = ready_for_worker;
        // Load the plugin once; keep it alive for the worker's lifetime.
        let plugin_id = vox_populi::mens::select_mens_backend(
            vox_populi::mens::DeviceKind::Best,
            &vox_plugin_host::probe(),
        );
        let plugin_result = vox_plugin_host::cached_code_plugin(plugin_id);
        let plugin = match plugin_result {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("{plugin_id} plugin not found: {e}");
                ready.store(false, Ordering::SeqCst);
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
                ready.store(false, Ordering::SeqCst);
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
                ready.store(false, Ordering::SeqCst);
                while let Ok(req) = rx.recv() {
                    let _ = req.reply.send(Err(format!("load_model failed: {e}")));
                }
                return;
            }
        };
        ready.store(true, Ordering::SeqCst);

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
    (tx, ready)
}

/// Build the JSON payload sent to the ML backend's `run_inference`, carrying the
/// system prompt and the sampling parameters the request specified.
///
/// Precedence: a non-empty `req.system_prompt` (the caller's per-request
/// override) REPLACES `default_system_prompt` (the server's startup-baked
/// default) entirely — never stacked with it. An absent or empty
/// `req.system_prompt` falls back to `default_system_prompt` unchanged.
#[cfg(feature = "execution-api")]
fn inference_payload(default_system_prompt: &str, req: &InferenceRequest) -> String {
    let system_prompt = req
        .system_prompt
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(default_system_prompt);
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

#[cfg(all(test, feature = "execution-api"))]
mod tests {
    use super::{InferenceRequest, inference_payload};

    #[test]
    fn the_payload_carries_the_system_prompt_and_the_sampling_parameters() {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let req = InferenceRequest {
            prompt: "hi".into(),
            max_tokens: 8,
            temperature: 0.0,
            top_k: 40,
            output_mode: Some("strict_json".into()),
            system_prompt: None,
            reply: tx,
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

    /// (a) A non-empty per-request `system_prompt` REPLACES the server's
    /// startup default entirely — the default text must not appear anywhere
    /// in the payload sent to the worker.
    #[test]
    fn per_request_system_prompt_replaces_the_startup_default() {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let req = InferenceRequest {
            prompt: "hi".into(),
            max_tokens: 8,
            temperature: 0.0,
            top_k: 40,
            output_mode: None,
            system_prompt: Some("CALLER OVERRIDE".into()),
            reply: tx,
        };
        let v: serde_json::Value =
            serde_json::from_str(&inference_payload("STARTUP DEFAULT", &req)).unwrap();
        assert_eq!(v["system"], "CALLER OVERRIDE");
        assert_ne!(
            v["system"].as_str().unwrap(),
            "STARTUP DEFAULT",
            "override must replace, never append alongside, the startup default"
        );
        assert!(
            !v["system"].as_str().unwrap().contains("STARTUP DEFAULT"),
            "the startup default text must not leak into an overridden payload"
        );
    }

    /// (b) An absent per-request `system_prompt` falls back to the server's
    /// startup default, unchanged.
    #[test]
    fn absent_system_prompt_falls_back_to_startup_default() {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let req = InferenceRequest {
            prompt: "hi".into(),
            max_tokens: 8,
            temperature: 0.0,
            top_k: 40,
            output_mode: None,
            system_prompt: None,
            reply: tx,
        };
        let v: serde_json::Value =
            serde_json::from_str(&inference_payload("STARTUP DEFAULT", &req)).unwrap();
        assert_eq!(v["system"], "STARTUP DEFAULT");
    }

    /// (b) An empty-string per-request `system_prompt` (as opposed to
    /// absent) also falls back to the startup default — empty is treated as
    /// "no override", not as "override with nothing".
    #[test]
    fn empty_system_prompt_falls_back_to_startup_default() {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let req = InferenceRequest {
            prompt: "hi".into(),
            max_tokens: 8,
            temperature: 0.0,
            top_k: 40,
            output_mode: None,
            system_prompt: Some(String::new()),
            reply: tx,
        };
        let v: serde_json::Value =
            serde_json::from_str(&inference_payload("STARTUP DEFAULT", &req)).unwrap();
        assert_eq!(v["system"], "STARTUP DEFAULT");
    }
}
