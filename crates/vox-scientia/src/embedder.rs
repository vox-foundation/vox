//! L3 shared `Embedder` trait, `CachedLlmEmbedder`, and deterministic `LocalHashEmbedder`.
//!
//! Routes external embeddings exclusively through `vox_actor_runtime::llm::llm_embed`,
//! caches float vectors in `VoxDb` (`scientia_embedding_cache`) keyed via `vox_crypto`,
//! and provides a zero-network pure-Rust feature-hashed fallback (`DISC-01`).

use vox_actor_runtime::llm::{LlmConfig, llm_embed};
use vox_actor_runtime::{ActivityOptions, ActivityResult};
use vox_db::VoxDb;

/// Async embedding provider seam shared across CLI, MCP, orchestrator, and publisher.
#[async_trait::async_trait]
pub trait Embedder: Send + Sync {
    /// Embed `text`; returns `None` on empty input or unrecoverable provider failure.
    async fn embed(&self, text: &str) -> Option<Vec<f32>>;
}

/// Cosine similarity in `[-1.0, 1.0]`; returns `0.0` for empty, mismatched, or zero-norm vectors.
#[must_use]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0_f64;
    let mut norm_a_sq = 0.0_f64;
    let mut norm_b_sq = 0.0_f64;
    for (&x, &y) in a.iter().zip(b.iter()) {
        let xf = f64::from(x);
        let yf = f64::from(y);
        dot += xf * yf;
        norm_a_sq += xf * xf;
        norm_b_sq += yf * yf;
    }
    let denom = norm_a_sq.sqrt() * norm_b_sq.sqrt();
    if denom == 0.0 || !denom.is_finite() {
        return 0.0;
    }
    (dot / denom).clamp(-1.0, 1.0)
}

/// Deterministic SHA-256 hex cache key over `model + NUL + text` via `vox_crypto`.
#[must_use]
pub fn embed_cache_key(text: &str, model: &str) -> String {
    let mut payload = Vec::with_capacity(model.len() + 1 + text.len());
    payload.extend_from_slice(model.as_bytes());
    payload.push(0);
    payload.extend_from_slice(text.as_bytes());
    vox_crypto::hex_encode(&vox_crypto::interop_sha256(&payload))
}

fn accumulate_hashed_feature(acc: &mut [f64], tag: u8, token: &[u8], weight: f64) {
    let mut buf = Vec::with_capacity(1 + token.len());
    buf.push(tag);
    buf.extend_from_slice(token);
    let h = vox_crypto::fast_hash(&buf);
    let idx = (h as usize) % acc.len();
    let sign = if (h >> 63) == 0 { 1.0 } else { -1.0 };
    acc[idx] += sign * weight;
}

/// Deterministic pure-Rust feature-hashing embedder over lowercase word unigrams,
/// word bigrams, and character 3-grams using `vox_crypto::fast_hash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalHashEmbedder {
    pub dims: usize,
}

impl LocalHashEmbedder {
    pub const MODEL_ID: &'static str = "vox-local-hash-v1";
    pub const DEFAULT_DIMS: usize = 256;

    #[must_use]
    pub const fn new(dims: usize) -> Self {
        Self { dims }
    }

    #[must_use]
    pub fn embed_sync(&self, text: &str) -> Option<Vec<f32>> {
        let trimmed = text.trim();
        if self.dims == 0 || trimmed.is_empty() {
            return None;
        }
        let lower = trimmed.to_lowercase();
        let words: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();

        let mut acc = vec![0.0_f64; self.dims];
        if words.is_empty() {
            for b in lower.as_bytes().chunks(3) {
                accumulate_hashed_feature(&mut acc, b'r', b, 1.0);
            }
        } else {
            for &w in &words {
                let weight = if w.chars().count() >= 3 { 2.0 } else { 0.2 };
                accumulate_hashed_feature(&mut acc, b'u', w.as_bytes(), weight);
                let chars: Vec<char> = w.chars().collect();
                for gram in chars.windows(3) {
                    let s: String = gram.iter().collect();
                    accumulate_hashed_feature(&mut acc, b'c', s.as_bytes(), 0.35);
                }
            }
            for pair in words.windows(2) {
                let bigram = format!("{} {}", pair[0], pair[1]);
                accumulate_hashed_feature(&mut acc, b'b', bigram.as_bytes(), 1.5);
            }
        }

        let mut norm = acc.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm == 0.0 || !norm.is_finite() {
            acc[0] = 1.0;
            norm = 1.0;
        }
        Some(acc.into_iter().map(|v| (v / norm) as f32).collect())
    }
}

impl Default for LocalHashEmbedder {
    fn default() -> Self {
        Self {
            dims: Self::DEFAULT_DIMS,
        }
    }
}

#[async_trait::async_trait]
impl Embedder for LocalHashEmbedder {
    async fn embed(&self, text: &str) -> Option<Vec<f32>> {
        self.embed_sync(text)
    }
}

/// Active backend for [`CachedLlmEmbedder`].
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum EmbedderBackend {
    Llm(LlmConfig),
    LocalFallback(LocalHashEmbedder),
}

/// `VoxDb`-cached embedder backed by `vox_actor_runtime::llm::llm_embed` or [`LocalHashEmbedder`].
pub struct CachedLlmEmbedder<'a> {
    pub db: &'a VoxDb,
    pub backend: EmbedderBackend,
    pub options: ActivityOptions,
}

impl<'a> CachedLlmEmbedder<'a> {
    #[must_use]
    pub fn new(db: &'a VoxDb, config: LlmConfig) -> Self {
        Self {
            db,
            backend: EmbedderBackend::Llm(config),
            options: ActivityOptions::default(),
        }
    }

    #[must_use]
    pub fn from_env(db: &'a VoxDb) -> Option<Self> {
        vox_search::embedding_env::embedding_config_from_env().map(|cfg| Self::new(db, cfg))
    }

    #[must_use]
    pub fn from_env_or_local_fallback(db: &'a VoxDb) -> Self {
        Self::from_env(db).unwrap_or(Self {
            db,
            backend: EmbedderBackend::LocalFallback(LocalHashEmbedder::default()),
            options: ActivityOptions::default(),
        })
    }

    #[must_use]
    pub fn is_local_fallback(&self) -> bool {
        matches!(self.backend, EmbedderBackend::LocalFallback(_))
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        match &self.backend {
            EmbedderBackend::Llm(cfg) => cfg.model.as_str(),
            EmbedderBackend::LocalFallback(_) => LocalHashEmbedder::MODEL_ID,
        }
    }
}

#[async_trait::async_trait]
impl<'a> Embedder for CachedLlmEmbedder<'a> {
    async fn embed(&self, text: &str) -> Option<Vec<f32>> {
        if text.trim().is_empty() {
            return None;
        }
        let model_id = self.model_id();
        let key = embed_cache_key(text, model_id);
        if let Ok(Some(vec)) = self.db.get_cached_embedding(&key).await {
            return Some(vec);
        }
        let vec = match &self.backend {
            EmbedderBackend::Llm(cfg) => match llm_embed(&self.options, text, cfg.clone()).await {
                ActivityResult::Ok(Ok(v)) => v,
                ActivityResult::Ok(Err(_))
                | ActivityResult::Failed(_)
                | ActivityResult::Cancelled => {
                    return None;
                }
            },
            EmbedderBackend::LocalFallback(local) => local.embed_sync(text)?,
        };
        let _ = self.db.put_cached_embedding(&key, model_id, &vec).await;
        Some(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_db::DbConfig;

    #[test]
    fn embed_cache_key_matches_known_sha256_vector_and_separates_boundary() {
        // Empty model + NUL + empty text == SHA-256([0x00])
        assert_eq!(
            embed_cache_key("", ""),
            "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
        );
        let k1 = embed_cache_key("bc", "a");
        let k2 = embed_cache_key("c", "ab");
        assert_eq!(k1.len(), 64);
        assert_ne!(k1, k2);
    }

    #[test]
    fn cosine_similarity_handles_edge_cases_and_unit_bounds() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[1.0, 2.0], &[1.0]), 0.0);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 2.0]), 0.0);
        assert!((cosine_similarity(&[0.6, 0.8], &[0.6, 0.8]) - 1.0).abs() < 1e-9);
        assert!((cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]) + 1.0).abs() < 1e-9);
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-9);
    }

    #[tokio::test]
    async fn local_hash_embedder_determinism_unit_norm_and_discrimination() {
        let emb = LocalHashEmbedder::default();
        assert_eq!(emb.embed("   ").await, None);
        assert_eq!(LocalHashEmbedder::new(0).embed("valid").await, None);

        let a1 = emb
            .embed("Retrieval-Augmented Claim Verification in Scholarly Synthesis")
            .await
            .expect("vec a1");
        let a2 = emb
            .embed("Retrieval-Augmented Claim Verification in Scholarly Synthesis")
            .await
            .expect("vec a2");
        assert_eq!(a1, a2);
        assert_eq!(a1.len(), 256);
        let norm: f64 = a1
            .iter()
            .map(|x| f64::from(*x) * f64::from(*x))
            .sum::<f64>()
            .sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "expected unit norm, got {norm}");
        assert!((cosine_similarity(&a1, &a2) - 1.0).abs() < 1e-6);

        let related = emb
            .embed("Scholarly Claim Verification Using Retrieval-Augmented Evidence")
            .await
            .expect("related vec");
        let unrelated = emb
            .embed("Topological Superconductivity in Twisted Bilayer Graphene")
            .await
            .expect("unrelated vec");
        let sim_related = cosine_similarity(&a1, &related);
        let sim_unrelated = cosine_similarity(&a1, &unrelated);
        assert!(
            sim_related > sim_unrelated + 0.25,
            "expected overlapping titles ({sim_related}) >> unrelated ({sim_unrelated})"
        );
    }

    #[tokio::test]
    async fn cached_llm_embedder_round_trips_through_voxdb_embedding_cache() {
        let db = VoxDb::connect(DbConfig::Memory)
            .await
            .expect("open memory db");
        let text = "Deterministic novelty cache probe for SCIENTIA v7.6";

        // 1. Exercise LocalFallback cache miss -> write-back -> second-call cache hit.
        let local_cached = CachedLlmEmbedder {
            db: &db,
            backend: EmbedderBackend::LocalFallback(LocalHashEmbedder::default()),
            options: ActivityOptions::default(),
        };
        let key_local = embed_cache_key(text, local_cached.model_id());
        assert!(db.get_cached_embedding(&key_local).await.unwrap().is_none());

        let first = local_cached.embed(text).await.expect("first embed");
        assert_eq!(first.len(), 256);
        assert!(db.get_cached_embedding(&key_local).await.unwrap().is_some());

        let sentinel = vec![0.25_f32, 0.5, -0.75, 1.0];
        db.put_cached_embedding(&key_local, local_cached.model_id(), &sentinel)
            .await
            .expect("seed sentinel");
        let second = local_cached.embed(text).await.expect("second embed hit");
        assert_eq!(second, sentinel);

        // 2. Also verify `from_env_or_local_fallback` hits `scientia_embedding_cache` offline.
        let auto = CachedLlmEmbedder::from_env_or_local_fallback(&db);
        let auto_text = "Second cache probe via from_env_or_local_fallback";
        let auto_key = embed_cache_key(auto_text, auto.model_id());
        if auto.is_local_fallback() {
            let v1 = auto.embed(auto_text).await.expect("auto local miss->write");
            assert_eq!(v1.len(), 256);
        }
        db.put_cached_embedding(&auto_key, auto.model_id(), &sentinel)
            .await
            .expect("seed auto sentinel");
        assert_eq!(
            auto.embed(auto_text).await.expect("auto cache hit"),
            sentinel
        );
    }
}
