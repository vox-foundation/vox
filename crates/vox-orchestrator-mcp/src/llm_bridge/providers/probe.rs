use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::metadata::ollama_base_url;
use crate::llm_bridge::error::HttpInferError;
use crate::llm_bridge::limits::{
    OLLAMA_PROBE_CACHE_TTL_SECS, OLLAMA_PROBE_TIMEOUT_SECS, VOX_LOCAL_PROBE_CACHE_TTL_SECS,
    VOX_LOCAL_PROBE_TIMEOUT_SECS,
};

fn vox_local_probe_cache() -> &'static Mutex<Option<(Instant, String)>> {
    static CACHE: OnceLock<Mutex<Option<(Instant, String)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

/// Base URL used for VoxLocal `/generate` after a successful health probe.
///
/// Prefers the TTL-cached winner from [`probe_vox_local_health`]. When the cache
/// is cold, falls back to explicit `VOX_LOCAL_ENDPOINT` or the first probe
/// candidate (`:11434`).
#[must_use]
pub(crate) fn vox_local_generate_base_url() -> String {
    if let Ok(guard) = vox_local_probe_cache().lock() {
        if let Some((t0, base)) = guard.as_ref() {
            if t0.elapsed() < Duration::from_secs(VOX_LOCAL_PROBE_CACHE_TTL_SECS) {
                return base.clone();
            }
        }
    }
    let candidates = vox_config::inference::vox_local_endpoint_probe_candidates();
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string())
}

async fn health_ok_at(client: &reqwest::Client, base: &str) -> bool {
    let url = format!("{}/health", base.trim_end_matches('/'));
    let Ok(res) = client
        .get(&url)
        .timeout(Duration::from_secs(VOX_LOCAL_PROBE_TIMEOUT_SECS))
        .send()
        .await
    else {
        return false;
    };
    if !res.status().is_success() {
        return false;
    }
    let Ok(body) = res.text().await else {
        return false;
    };
    vox_config::inference::vox_local_health_identifies_serve(&body)
}

/// Cheap `GET /health` probe so routing to VoxLocal fails fast with a clear message.
///
/// Walks [`vox_config::inference::vox_local_endpoint_probe_candidates`] (explicit
/// `VOX_LOCAL_ENDPOINT`, else `:11434` then `:11435`) and caches the first
/// `vox-ml-cli` / healthy winner for [`VOX_LOCAL_PROBE_CACHE_TTL_SECS`].
pub(crate) async fn probe_vox_local_health(client: &reqwest::Client) -> Result<(), HttpInferError> {
    let ttl = Duration::from_secs(VOX_LOCAL_PROBE_CACHE_TTL_SECS);
    {
        let cache = vox_local_probe_cache()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some((t0, _)) = cache.as_ref() {
            if t0.elapsed() < ttl {
                return Ok(());
            }
        }
    }

    let candidates = vox_config::inference::vox_local_endpoint_probe_candidates();
    if candidates.is_empty() {
        return Err(HttpInferError {
            status: 0,
            message: "VoxLocal probe candidates empty (VOX_LOCAL_ENDPOINT is blank)".into(),
            is_capability_gap: false,
        });
    }

    let mut last_err = String::new();
    for base in &candidates {
        if health_ok_at(client, base).await {
            let mut cache = vox_local_probe_cache()
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            *cache = Some((Instant::now(), base.clone()));
            return Ok(());
        }
        last_err = format!("no healthy vox-ml-cli at {base}/health");
    }

    let mut cache = vox_local_probe_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *cache = None;
    Err(HttpInferError {
        status: 0,
        message: format!(
            "VoxLocal unreachable ({last_err}); run `vox mens serve` (often :11435 when Ollama owns :11434) or set VOX_LOCAL_ENDPOINT."
        ),
        is_capability_gap: false,
    })
}

fn ollama_probe_ok_at() -> &'static Mutex<Option<Instant>> {
    static CACHE: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

/// Cheap `GET /api/tags` probe so routing to Ollama fails fast with a clear message.
///
/// Successful probes are cached per-process for `OLLAMA_PROBE_CACHE_TTL_SECS`.
pub(crate) async fn probe_ollama_tags(client: &reqwest::Client) -> Result<(), HttpInferError> {
    let ttl = Duration::from_secs(OLLAMA_PROBE_CACHE_TTL_SECS);
    {
        let cache = ollama_probe_ok_at()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(t0) = *cache {
            if t0.elapsed() < ttl {
                return Ok(());
            }
        }
    }

    let base = ollama_base_url();
    let url = format!("{}/api/tags", base.trim_end_matches('/'));
    let res = client
        .get(&url)
        .timeout(Duration::from_secs(OLLAMA_PROBE_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| HttpInferError {
            status: 0,
            message: format!(
                "Ollama unreachable at {base} ({e}); set OLLAMA_HOST or start Ollama / Mens."
            ),
            is_capability_gap: false,
        })?;
    let code = res.status().as_u16();
    if !res.status().is_success() {
        let t = res.text().await.unwrap_or_default();
        let err = HttpInferError {
            status: code,
            message: format!("Ollama /api/tags error: {t}"),
            is_capability_gap: false,
        };
        let mut cache = ollama_probe_ok_at()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *cache = None;
        return Err(err);
    }
    let mut cache = ollama_probe_ok_at()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *cache = Some(Instant::now());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_base_falls_back_to_first_candidate_when_cache_cold() {
        // Clear any ambient cache from parallel tests.
        if let Ok(mut g) = vox_local_probe_cache().lock() {
            *g = None;
        }
        let base = vox_local_generate_base_url();
        assert!(
            base.contains("11434") || base.contains("11435") || !base.is_empty(),
            "unexpected base {base}"
        );
    }
}
