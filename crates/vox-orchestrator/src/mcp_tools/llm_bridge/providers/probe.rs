use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::metadata::ollama_base_url;
use crate::mcp_tools::llm_bridge::error::HttpInferError;
use crate::mcp_tools::llm_bridge::limits::{OLLAMA_PROBE_CACHE_TTL_SECS, OLLAMA_PROBE_TIMEOUT_SECS};

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
        })?;
    let code = res.status().as_u16();
    if !res.status().is_success() {
        let t = res.text().await.unwrap_or_default();
        let err = HttpInferError {
            status: code,
            message: format!("Ollama /api/tags error: {t}"),
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
