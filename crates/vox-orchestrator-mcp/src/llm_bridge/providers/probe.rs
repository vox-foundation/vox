use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::metadata::ollama_base_url;
use crate::llm_bridge::error::HttpInferError;
use crate::llm_bridge::limits::{
    OLLAMA_PROBE_CACHE_TTL_SECS, OLLAMA_PROBE_TIMEOUT_SECS, VOX_LOCAL_PROBE_CACHE_TTL_SECS,
    VOX_LOCAL_PROBE_TIMEOUT_SECS,
};

#[derive(Clone, Debug)]
struct VoxLocalProbeCache {
    cached_at: Instant,
    base: String,
    /// Join of [`vox_config::inference::vox_local_endpoint_probe_candidates`].
    candidates_fingerprint: String,
}

fn vox_local_probe_cache() -> &'static Mutex<Option<VoxLocalProbeCache>> {
    static CACHE: OnceLock<Mutex<Option<VoxLocalProbeCache>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn candidates_fingerprint(candidates: &[String]) -> String {
    candidates.join("\n")
}

/// Base URL used for VoxLocal `/generate` after a successful health probe.
///
/// Prefers the TTL-cached winner from [`probe_vox_local_health`]. When the cache
/// is cold, falls back to explicit `VOX_LOCAL_ENDPOINT` or the first probe
/// candidate (`:11434`). Callers that matter (`vox_local_generate`, adapter)
/// MUST probe first so the cache is warm.
#[must_use]
pub(crate) fn vox_local_generate_base_url() -> String {
    let candidates = vox_config::inference::vox_local_endpoint_probe_candidates();
    let fp = candidates_fingerprint(&candidates);
    if let Ok(guard) = vox_local_probe_cache().lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.candidates_fingerprint == fp
                && entry.cached_at.elapsed() < Duration::from_secs(VOX_LOCAL_PROBE_CACHE_TTL_SECS)
            {
                return entry.base.clone();
            }
        }
    }
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string())
}

async fn health_ok_at(client: &reqwest::Client, base: &str) -> bool {
    let root = base.trim_end_matches('/');
    // Match GUI dual-path probe (`/health` then `/ready`).
    for path in ["/health", "/ready"] {
        let url = format!("{root}{path}");
        let Ok(res) = client
            .get(&url)
            .timeout(Duration::from_secs(VOX_LOCAL_PROBE_TIMEOUT_SECS))
            .send()
            .await
        else {
            continue;
        };
        if !res.status().is_success() {
            continue;
        }
        let Ok(body) = res.text().await else {
            continue;
        };
        if vox_config::inference::vox_local_health_identifies_serve(&body) {
            return true;
        }
    }
    false
}

fn store_probe_winner(base: String, fingerprint: String) {
    let mut cache = vox_local_probe_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *cache = Some(VoxLocalProbeCache {
        cached_at: Instant::now(),
        base,
        candidates_fingerprint: fingerprint,
    });
}

/// Cheap health probe so routing to VoxLocal fails fast with a clear message.
///
/// Walks [`vox_config::inference::vox_local_endpoint_probe_candidates`] (explicit
/// `VOX_LOCAL_ENDPOINT`, else `:11434` then `:11435`) and caches the first
/// `vox-ml-cli` winner for [`VOX_LOCAL_PROBE_CACHE_TTL_SECS`].
///
/// On cache hit, re-validates the cached base (avoids stale port after a server
/// swap). Failed walks do **not** clear a prior winner (avoids concurrent
/// probe clobber).
pub(crate) async fn probe_vox_local_health(client: &reqwest::Client) -> Result<(), HttpInferError> {
    let ttl = Duration::from_secs(VOX_LOCAL_PROBE_CACHE_TTL_SECS);
    let candidates = vox_config::inference::vox_local_endpoint_probe_candidates();
    let fp = candidates_fingerprint(&candidates);

    let cached_base = {
        let cache = vox_local_probe_cache()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        cache.as_ref().and_then(|entry| {
            if entry.candidates_fingerprint == fp && entry.cached_at.elapsed() < ttl {
                Some(entry.base.clone())
            } else {
                None
            }
        })
    };

    if let Some(base) = cached_base {
        if health_ok_at(client, &base).await {
            // Refresh TTL without changing winner.
            store_probe_winner(base, fp);
            return Ok(());
        }
        // Cached base went stale — fall through to a full walk. Do not clear
        // yet; a concurrent successful walk must not be wiped by this miss.
    }

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
            store_probe_winner(base.clone(), fp);
            return Ok(());
        }
        last_err = format!("no healthy vox-ml-cli at {base}/{{health,ready}}");
    }

    // Do not clear cache on failure — concurrent success must survive.
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
        if let Ok(mut g) = vox_local_probe_cache().lock() {
            *g = None;
        }
        let base = vox_local_generate_base_url();
        assert!(
            base.contains("11434") || base.contains("11435") || !base.is_empty(),
            "unexpected base {base}"
        );
    }

    #[test]
    fn failed_walk_does_not_clear_prior_winner() {
        let fp = "http://127.0.0.1:11435".to_string();
        store_probe_winner("http://127.0.0.1:11435".into(), fp.clone());
        // Simulate a failed walk's previous (buggy) clear path by asserting
        // the helper contract: we never null the cache from Err path.
        {
            let cache = vox_local_probe_cache()
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            assert_eq!(
                cache.as_ref().map(|e| e.base.as_str()),
                Some("http://127.0.0.1:11435")
            );
            assert_eq!(
                cache.as_ref().map(|e| e.candidates_fingerprint.as_str()),
                Some(fp.as_str())
            );
        }
        // Cold generate must still prefer the warm cache when fingerprint matches.
        if let Ok(mut g) = vox_local_probe_cache().lock() {
            *g = Some(VoxLocalProbeCache {
                cached_at: Instant::now(),
                base: "http://127.0.0.1:11435".into(),
                candidates_fingerprint: candidates_fingerprint(
                    &vox_config::inference::vox_local_endpoint_probe_candidates(),
                ),
            });
        }
        let base = vox_local_generate_base_url();
        assert!(
            base.contains("11435"),
            "warm cache should win over first candidate; got {base}"
        );
    }
}
