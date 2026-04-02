//! Similarity-graph propagation over model trust rollups (Markov smoothing / “web of trust” lite).

use std::collections::HashMap;

use serde::Serialize;

use crate::store::{StoreError, TrustRollupEntry};
use crate::{TrustObservationInput, VoxDb};

/// One row after graph-based smoothing within shared `domain` cliques.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TrustPropagatedScore {
    pub entity_id: String,
    pub domain: String,
    pub raw_score: f64,
    pub propagated_score: f64,
}

/// Domain-local affinity propagation: each node pulls toward neighbours weighted by `score_i * score_j`.
///
/// Iterates `iterations` steps; `damping` in `(0,1)` blends toward neighbour mass vs retaining raw score.
pub fn propagate_trust_rollups_domain_cliques(
    rollups: &[TrustRollupEntry],
    damping: f64,
    iterations: usize,
) -> Vec<TrustPropagatedScore> {
    let d = damping.clamp(0.01, 0.99);
    let iters = iterations.clamp(1, 256);
    #[derive(Clone)]
    struct Node {
        entity_id: String,
        domain: String,
        raw: f64,
    }
    let mut best: HashMap<(String, String), f64> = HashMap::new();
    for r in rollups {
        let k = (r.entity_id.clone(), r.domain.clone());
        let sc = r.score.clamp(0.0, 1.0);
        best.entry(k).and_modify(|v| *v = v.max(sc)).or_insert(sc);
    }
    let nodes: Vec<Node> = best
        .into_iter()
        .map(|((entity_id, domain), raw)| Node {
            entity_id,
            domain,
            raw,
        })
        .collect();
    if nodes.is_empty() {
        return Vec::new();
    }
    let mut prop: HashMap<(String, String), f64> = HashMap::new();
    for n in &nodes {
        prop.insert((n.entity_id.clone(), n.domain.clone()), n.raw);
    }
    for _ in 0..iters {
        let mut next = HashMap::new();
        for n in &nodes {
            let mut num = 0.0_f64;
            let mut den = 0.0_f64;
            for o in &nodes {
                if o.domain != n.domain {
                    continue;
                }
                let sim = (n.raw * o.raw).max(1e-9);
                let op = prop
                    .get(&(
                        o.entity_id.as_str().to_string(),
                        o.domain.as_str().to_string(),
                    ))
                    .copied()
                    .unwrap_or(o.raw);
                num += sim * op;
                den += sim;
            }
            let blended = if den > 1e-12 { num / den } else { n.raw };
            let new_v = (1.0 - d) * n.raw + d * blended;
            next.insert(
                (n.entity_id.clone(), n.domain.clone()),
                new_v.clamp(0.0, 1.0),
            );
        }
        prop = next;
    }
    nodes
        .into_iter()
        .map(|n| {
            let p = prop
                .get(&(n.entity_id.clone(), n.domain.clone()))
                .copied()
                .unwrap_or(n.raw);
            TrustPropagatedScore {
                entity_id: n.entity_id,
                domain: n.domain,
                raw_score: n.raw,
                propagated_score: p,
            }
        })
        .collect()
}

impl VoxDb {
    /// Load model rollups for `dimension` + `repository_id`, run [`propagate_trust_rollups_domain_cliques`],
    /// optionally persist `*_propagated` observations.
    pub async fn trust_propagate_model_rollups(
        &self,
        repository_id: &str,
        dimension: &str,
        damping: f64,
        iterations: u32,
        persist: bool,
    ) -> Result<Vec<TrustPropagatedScore>, StoreError> {
        let rows = self
            .list_trust_rollups(
                Some("model"),
                Some(dimension),
                None,
                Some(repository_id),
                10_000,
            )
            .await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let out = propagate_trust_rollups_domain_cliques(&rows, damping, iterations as usize);
        if persist {
            let dim_p = format!("{dimension}_propagated");
            for t in &out {
                let meta = serde_json::json!({
                    "base_dimension": dimension,
                    "damping": damping,
                    "iterations": iterations,
                    "propagation": "domain_clique_affinity",
                });
                let meta_s = serde_json::to_string(&meta)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                self.record_trust_observation(TrustObservationInput {
                    entity_type: "model",
                    entity_id: t.entity_id.as_str(),
                    dimension: dim_p.as_str(),
                    domain: Some(t.domain.as_str()),
                    task_class: Some("trust_propagation"),
                    provider: None,
                    model_id: None,
                    repository_id: Some(repository_id),
                    source_kind: Some("trust_propagation"),
                    observation_value: t.propagated_score,
                    confidence_weight: 1.0,
                    sample_size: 1,
                    artifact_ref: Some("trust_propagate_model_rollups"),
                    metadata_json: Some(meta_s.as_str()),
                    ewma_alpha: 0.08,
                })
                .await?;
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::TrustRollupEntry;

    #[test]
    fn propagation_pulls_low_toward_high_in_same_domain() {
        let rows = vec![
            TrustRollupEntry {
                entity_type: "model".into(),
                entity_id: "a".into(),
                dimension: "factuality".into(),
                domain: "d1".into(),
                task_class: "".into(),
                provider: "".into(),
                model_id: "".into(),
                repository_id: "r".into(),
                score: 0.9,
                sample_size: 5,
                ewma_alpha: 0.1,
                updated_at_ms: 0,
            },
            TrustRollupEntry {
                entity_type: "model".into(),
                entity_id: "b".into(),
                dimension: "factuality".into(),
                domain: "d1".into(),
                task_class: "".into(),
                provider: "".into(),
                model_id: "".into(),
                repository_id: "r".into(),
                score: 0.2,
                sample_size: 5,
                ewma_alpha: 0.1,
                updated_at_ms: 0,
            },
        ];
        let out = propagate_trust_rollups_domain_cliques(&rows, 0.85, 24);
        let b = out.iter().find(|x| x.entity_id == "b").unwrap();
        assert!(b.propagated_score > b.raw_score);
    }
}
