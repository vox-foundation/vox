//! Real semantic similarity for prior-art scoring.
//!
//! Re-exports the canonical L3 [`Embedder`] trait, [`cosine_similarity`], and
//! [`embed_cache_key`] from [`vox_scientia::embedder`]. On embed failure
//! callers receive `None` — absence is propagated, never replaced with a
//! fake score.

pub use vox_scientia::embedder::{Embedder, cosine_similarity, embed_cache_key};

/// True when `VOX_SCIENTIA_REQUIRE_EMBEDDER` is set to a truthy value (`1`, `true`, `yes`, `on`).
#[must_use]
pub fn scientia_require_embedder_env_enabled() -> bool {
    std::env::var("VOX_SCIENTIA_REQUIRE_EMBEDDER")
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

/// Fail fast when online prior-art fetch would run without an embedder while
/// [`scientia_require_embedder_env_enabled`] is active.
///
/// Offline paths and unset env skip the check so deterministic tests stay usable.
pub fn require_embedder_for_online_novelty(
    offline: bool,
    embedder_available: bool,
) -> anyhow::Result<()> {
    if offline || !scientia_require_embedder_env_enabled() {
        return Ok(());
    }
    if embedder_available {
        return Ok(());
    }
    anyhow::bail!(
        "online novelty requires an embedder when VOX_SCIENTIA_REQUIRE_EMBEDDER=1; \
         configure an embedding provider (see vox-search embedding env) or pass --offline"
    )
}

/// Enrich hits' `semantic_score` with cosine(query, hit title) where embeddings
/// are available; leaves `semantic_score: None` where they are not.
pub async fn enrich_semantic_scores(
    query_text: &str,
    hits: &mut [crate::scientia_finding_ledger::NormalizedPriorArtHit],
    embedder: &dyn Embedder,
) {
    if hits.is_empty() {
        return;
    }
    let Some(query_vec) = embedder.embed(query_text).await else {
        return;
    };
    for hit in hits.iter_mut() {
        if hit.title.is_empty() {
            continue;
        }
        if let Some(hit_vec) = embedder.embed(&hit.title).await {
            hit.semantic_score = Some(cosine_similarity(&query_vec, &hit_vec));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    struct EnvVarGuard {
        key: &'static str,
        prior: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prior = std::env::var(key).ok();
            // SAFETY: test-only; single-threaded unit test module.
            unsafe { std::env::set_var(key, value) };
            Self { key, prior }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.prior {
                Some(v) => unsafe { std::env::set_var(self.key, v) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    #[test]
    fn require_embedder_fails_when_flag_set_and_no_embedder() {
        let _lock = ENV_TEST_LOCK.lock().expect("env test lock");
        let _guard = EnvVarGuard::set("VOX_SCIENTIA_REQUIRE_EMBEDDER", "1");
        let err = require_embedder_for_online_novelty(false, false).unwrap_err();
        assert!(err.to_string().contains("VOX_SCIENTIA_REQUIRE_EMBEDDER"));
    }

    #[test]
    fn require_embedder_skipped_when_offline() {
        let _lock = ENV_TEST_LOCK.lock().expect("env test lock");
        let _guard = EnvVarGuard::set("VOX_SCIENTIA_REQUIRE_EMBEDDER", "1");
        require_embedder_for_online_novelty(true, false).expect("offline bypasses guard");
    }

    #[test]
    fn require_embedder_skipped_when_flag_unset() {
        let _lock = ENV_TEST_LOCK.lock().expect("env test lock");
        let _guard = EnvVarGuard::set("VOX_SCIENTIA_REQUIRE_EMBEDDER", "0");
        require_embedder_for_online_novelty(false, false).expect("flag off bypasses guard");
    }

    #[test]
    fn identical_vectors_score_one() {
        let v = vec![0.5_f32, 0.5, 0.5, 0.5];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn orthogonal_vectors_score_zero() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![0.0_f32, 1.0];
        assert!((cosine_similarity(&a, &b)).abs() < 1e-9);
    }

    #[test]
    fn zero_vector_scores_zero_no_nan() {
        let zero = vec![0.0_f32, 0.0, 0.0];
        let v = vec![1.0_f32, 2.0, 3.0];
        let s = cosine_similarity(&zero, &v);
        assert!(!s.is_nan(), "must not be NaN");
        assert_eq!(s, 0.0);
    }

    #[test]
    fn cache_key_stable() {
        let k1 = embed_cache_key("hello world", "model-a");
        let k2 = embed_cache_key("hello world", "model-a");
        assert_eq!(k1, k2, "key must be stable");
        assert_eq!(k1.len(), 64, "sha256 hex must be 64 chars");
    }

    #[test]
    fn cache_key_model_scoped() {
        let k1 = embed_cache_key("same text", "model-a");
        let k2 = embed_cache_key("same text", "model-b");
        assert_ne!(k1, k2, "different models must produce different keys");
    }
}
