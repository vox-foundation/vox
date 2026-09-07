//! LLM settings bridge: the `[llm]` config SSOT (concurrency + retry) and a
//! presence check for the OpenRouter key.
//!
//! Reads/writes go through `VoxConfig` (which persists the `[llm]` table to
//! `~/.vox/config.toml`). The throttle in `vox-actor-runtime::llm::throttle`
//! reads these same values via `VoxConfig::load()`, so the GUI and the egress
//! path share one source of truth.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct LlmConfigDto {
    pub max_concurrent_requests: usize,
    pub openrouter_max_concurrent: Option<usize>,
    pub openai_max_concurrent: Option<usize>,
    pub retry_max_attempts: u32,
}

#[tauri::command]
pub async fn get_llm_config() -> Result<LlmConfigDto, String> {
    let cfg = vox_config::VoxConfig::load();
    Ok(LlmConfigDto {
        max_concurrent_requests: cfg.llm_max_concurrent_requests,
        openrouter_max_concurrent: cfg.llm_openrouter_max_concurrent,
        openai_max_concurrent: cfg.llm_openai_max_concurrent,
        retry_max_attempts: cfg.llm_retry_max_attempts,
    })
}

#[tauri::command]
pub async fn set_llm_config(config: serde_json::Value) -> Result<(), String> {
    let mut cfg = vox_config::VoxConfig::load();
    if let Some(v) = config.get("maxConcurrentRequests").and_then(|v| v.as_u64()) {
        cfg.llm_max_concurrent_requests = (v as usize).clamp(1, 256);
    }
    if let Some(v) = config.get("openrouterMaxConcurrent") {
        cfg.llm_openrouter_max_concurrent = v.as_u64().map(|n| (n as usize).clamp(1, 256));
    }
    if let Some(v) = config.get("openaiMaxConcurrent") {
        cfg.llm_openai_max_concurrent = v.as_u64().map(|n| (n as usize).clamp(1, 256));
    }
    if let Some(v) = config.get("retryMaxAttempts").and_then(|v| v.as_u64()) {
        cfg.llm_retry_max_attempts = (v as u32).clamp(0, 10);
    }
    // Persists the [llm] table to ~/.vox/config.toml (merge-write).
    cfg.save().map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct OpenRouterKeyStatusDto {
    pub configured: bool,
}

/// Presence check for the OpenRouter key (does not call the network — the GUI
/// only needs to know whether a key is set so it can prompt the user otherwise).
#[tauri::command]
pub async fn openrouter_key_status() -> Result<OpenRouterKeyStatusDto, String> {
    let configured =
        vox_secrets::resolve_secret(vox_secrets::SecretId::OpenRouterApiKey).is_present();
    Ok(OpenRouterKeyStatusDto { configured })
}

#[derive(Debug, Serialize)]
pub struct ProviderStatusDto {
    /// Debug-format provider name, e.g. "OpenRouter", "Ollama".
    pub provider: String,
    pub key_present: bool,
    pub is_local: bool,
    /// Some(reachable) from the cached local probe; None for cloud providers.
    pub local_reachable: Option<bool>,
    /// Model names the local server reported (empty for cloud providers).
    pub local_models: Vec<String>,
}

/// Ollama / PopuliMesh share the `:11434` probe. VoxLocal is a different
/// server (`VOX_LOCAL_ENDPOINT`) — do not mark it unreachable when Ollama is down.
fn local_availability(
    p: vox_orchestrator::models::ProviderType,
    ollama_reachable: bool,
    ollama_models: &[String],
    vox_local_reachable: Option<bool>,
    vox_local_models: &[String],
) -> (bool, Option<bool>, Vec<String>) {
    use vox_orchestrator::models::ProviderType;
    match p {
        ProviderType::Ollama | ProviderType::PopuliMesh => {
            (true, Some(ollama_reachable), ollama_models.to_vec())
        }
        ProviderType::VoxLocal => (true, vox_local_reachable, vox_local_models.to_vec()),
        _ => (false, None, Vec::new()),
    }
}

fn vox_local_endpoint_base() -> String {
    std::env::var("VOX_LOCAL_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn parse_vox_local_model_ids(body: &serde_json::Value) -> Vec<String> {
    body.get("data")
        .and_then(|d| d.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("id").and_then(|id| id.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

async fn probe_vox_local() -> (Option<bool>, Vec<String>) {
    let base = vox_local_endpoint_base();
    let client = vox_http_client::client();
    let timeout = std::time::Duration::from_secs(2);
    let healthy = {
        let health = client
            .get(format!("{base}/health"))
            .timeout(timeout)
            .send()
            .await;
        match health {
            Ok(resp) if resp.status().is_success() => true,
            _ => {
                let ready = client
                    .get(format!("{base}/ready"))
                    .timeout(timeout)
                    .send()
                    .await;
                matches!(ready, Ok(resp) if resp.status().is_success())
            }
        }
    };
    if !healthy {
        return (Some(false), Vec::new());
    }
    let models = match client
        .get(format!("{base}/v1/models"))
        .timeout(timeout)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp
            .json::<serde_json::Value>()
            .await
            .ok()
            .map(|body| parse_vox_local_model_ids(&body))
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    (Some(true), models)
}

/// Per-backend availability (B9): credential presence for every candidate
/// provider + live local-server health from the shared TTL-cached probe.
#[tauri::command]
pub async fn inference_provider_status() -> Result<Vec<ProviderStatusDto>, String> {
    let statuses = vox_orchestrator::models::key_guard::inference_provider_statuses();
    let base = vox_config::inference::local_ollama_populi_base_url();
    let probe = vox_actor_runtime::inference_env::probe_populi_capabilities_cached(
        &base,
        vox_config::timeouts::D_15S,
    )
    .await;
    let (vox_local_reachable, vox_local_models) = probe_vox_local().await;
    Ok(statuses
        .into_iter()
        .map(|(p, key_present)| {
            let provider = format!("{p:?}");
            let (is_local, local_reachable, local_models) = local_availability(
                p,
                probe.reachable,
                &probe.model_names,
                vox_local_reachable,
                &vox_local_models,
            );
            ProviderStatusDto {
                provider,
                key_present,
                is_local,
                local_reachable,
                local_models,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_status_dto_serializes_shape_frontend_expects() {
        let dto = ProviderStatusDto {
            provider: "Anthropic".into(),
            key_present: false,
            is_local: false,
            local_reachable: None,
            local_models: vec![],
        };
        let j = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(j["provider"], "Anthropic");
        assert_eq!(j["key_present"], false);
        assert!(j["local_reachable"].is_null());
    }

    #[test]
    fn vox_local_is_not_gated_on_the_ollama_probe() {
        use vox_orchestrator::models::ProviderType;
        let (is_local, reachable, models) = local_availability(
            ProviderType::VoxLocal,
            false,
            &["llama3".into()],
            Some(true),
            &["e2e-smoke-metal".into()],
        );
        assert!(is_local);
        assert_eq!(reachable, Some(true));
        assert_eq!(models, vec!["e2e-smoke-metal".to_string()]);

        let (down_local, down_reach, down_models) = local_availability(
            ProviderType::VoxLocal,
            true,
            &["llama3".into()],
            Some(false),
            &[],
        );
        assert!(down_local);
        assert_eq!(down_reach, Some(false));
        assert!(down_models.is_empty());

        let (ollama_local, ollama_reach, ollama_models) = local_availability(
            ProviderType::Ollama,
            false,
            &["llama3".into()],
            Some(true),
            &["e2e-smoke-metal".into()],
        );
        assert!(ollama_local);
        assert_eq!(ollama_reach, Some(false));
        assert_eq!(ollama_models, vec!["llama3".to_string()]);
    }

    #[test]
    fn parse_vox_local_model_ids_reads_openai_list_shape() {
        let body = serde_json::json!({
            "object": "list",
            "data": [
                {"id": "e2e-smoke-metal", "object": "model"},
                {"id": "run-b", "object": "model"}
            ]
        });
        assert_eq!(
            parse_vox_local_model_ids(&body),
            vec!["e2e-smoke-metal".to_string(), "run-b".to_string()]
        );
    }
}
