//! Embedding calls used by hybrid memory retrieval and `vox-db`-backed vector search.

use std::sync::Arc;

use vox_actor_runtime::llm::{LlmConfig, llm_embed};
use vox_actor_runtime::{ActivityOptions, ActivityResult};
use vox_db::VoxDb;

/// Generate query vectors and optionally persist rows into `embeddings`.
pub struct EmbeddingService {
    db: Arc<VoxDb>,
    config: LlmConfig,
}

impl EmbeddingService {
    /// New service bound to a [`vox_db::VoxDb`] handle and provider configuration.
    pub fn new(db: Arc<VoxDb>, config: LlmConfig) -> Self {
        Self { db, config }
    }

    /// Generate an embedding for text and store it in the database.
    pub async fn embed_and_store(
        &self,
        source_type: &str,
        source_id: &str,
        text: &str,
        metadata: Option<&str>,
    ) -> Result<i64, String> {
        let options = ActivityOptions::default();
        let vector = match llm_embed(&options, text, self.config.clone()).await {
            ActivityResult::Ok(Ok(v)) => v,
            ActivityResult::Ok(Err(e)) => return Err(format!("Embedding API error: {}", e)),
            ActivityResult::Failed(e) => return Err(format!("Embedding failed: {}", e)),
            ActivityResult::Cancelled => return Err("Embedding cancelled".to_string()),
        };

        self.db
            .store_embedding(
                source_type,
                source_id,
                &self.config.model,
                &vector,
                metadata,
                None,
            )
            .await
            .map_err(|e| format!("Database error: {}", e))
    }

    /// Generate a vector for a search query.
    pub async fn embed_query(&self, query: &str) -> Result<Vec<f32>, String> {
        let options = ActivityOptions::default();
        match llm_embed(&options, query, self.config.clone()).await {
            ActivityResult::Ok(Ok(v)) => Ok(v),
            ActivityResult::Ok(Err(e)) => Err(format!("Embedding API error: {}", e)),
            ActivityResult::Failed(e) => Err(format!("Embedding failed: {}", e)),
            ActivityResult::Cancelled => Err("Embedding cancelled".to_string()),
        }
    }

    /// Underlying model configuration (for telemetry).
    #[must_use]
    pub fn llm_config(&self) -> &LlmConfig {
        &self.config
    }

    /// Shared database handle.
    #[must_use]
    pub fn db(&self) -> &Arc<VoxDb> {
        &self.db
    }

    /// Vector space key `(model_id, revision, dim)` for this service (`EMB-02`).
    #[must_use]
    pub fn vector_space_key(&self, dim: usize) -> VectorSpaceKey {
        VectorSpaceKey::from_model_spec(&self.config.model, dim)
    }
}

/// Identity of an embedding vector space (`EMB-02`): vectors from different models, revisions,
/// or dimensions must never be compared or mixed in a single similarity query.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VectorSpaceKey {
    pub model_id: String,
    pub revision: String,
    pub dim: usize,
}

impl VectorSpaceKey {
    /// Parse `"model_id@revision"` (or default revision `"default"` when `@` is absent).
    #[must_use]
    pub fn from_model_spec(spec: &str, dim: usize) -> Self {
        match spec.split_once('@') {
            Some((m, r)) if !r.trim().is_empty() => Self {
                model_id: m.trim().to_string(),
                revision: r.trim().to_string(),
                dim,
            },
            _ => Self {
                model_id: spec.trim().to_string(),
                revision: "default".to_string(),
                dim,
            },
        }
    }
}

/// Refuse similarity comparisons across mismatched `(model_id, revision, dim)` spaces (`EMB-02`).
pub fn validate_vector_space(
    expected: &VectorSpaceKey,
    actual: &VectorSpaceKey,
) -> Result<(), String> {
    if expected != actual {
        return Err(format!(
            "vector space mismatch: expected ({} @ {}, dim={}) but got ({} @ {}, dim={}); reindex required",
            expected.model_id,
            expected.revision,
            expected.dim,
            actual.model_id,
            actual.revision,
            actual.dim
        ));
    }
    Ok(())
}

/// Compute cosine similarity between two vectors only after verifying their `VectorSpaceKey`s
/// and dimensions match (`EMB-02`).
pub fn cosine_similarity_checked(
    expected_space: &VectorSpaceKey,
    query_vec: &[f32],
    actual_space: &VectorSpaceKey,
    doc_vec: &[f32],
) -> Result<f32, String> {
    validate_vector_space(expected_space, actual_space)?;
    if query_vec.len() != expected_space.dim || doc_vec.len() != actual_space.dim {
        return Err(format!(
            "vector dimension mismatch: expected dim={} but got query_len={} doc_len={}",
            expected_space.dim,
            query_vec.len(),
            doc_vec.len()
        ));
    }
    let mut dot = 0.0_f32;
    let mut norm_q = 0.0_f32;
    let mut norm_d = 0.0_f32;
    for (&q, &d) in query_vec.iter().zip(doc_vec.iter()) {
        dot += q * d;
        norm_q += q * q;
        norm_d += d * d;
    }
    let denom = norm_q.sqrt() * norm_d.sqrt();
    if denom <= 1e-8 {
        Ok(0.0)
    } else {
        Ok(dot / denom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_similarity_across_different_models_or_dims() {
        let space_a =
            VectorSpaceKey::from_model_spec("Alibaba-NLP/gte-modernbert-base@refs/pr/1", 4);
        let space_b = VectorSpaceKey::from_model_spec("text-embedding-3-small@default", 4);
        let space_dim =
            VectorSpaceKey::from_model_spec("Alibaba-NLP/gte-modernbert-base@refs/pr/1", 8);
        let v1 = [1.0, 0.0, 0.0, 0.0];
        let v2 = [1.0, 0.0, 0.0, 0.0];
        assert!(cosine_similarity_checked(&space_a, &v1, &space_a, &v2).is_ok());
        assert!(cosine_similarity_checked(&space_a, &v1, &space_b, &v2).is_err());
        assert!(cosine_similarity_checked(&space_a, &v1, &space_dim, &v2).is_err());
    }
}
