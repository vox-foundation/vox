//! Prefix-cache hit predictor for cost-aware routing (D7).
//!
//! Predicts whether the current request will hit the provider's prompt cache by
//! comparing prefix overlap tokens against total context tokens.
//! All logic is pure: no async, no I/O.

use serde::{Deserialize, Serialize};

/// Input signals for cache prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSignal {
    /// Tokens in the shared prefix that may already be cached.
    pub prefix_overlap_tokens: u32,
    /// Total tokens in the current context.
    pub total_context_tokens: u32,
}

/// Cache prediction outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CachePrediction {
    /// Prefix overlap ratio ≥ threshold — likely cache hit.
    Hit,
    /// Prefix overlap ratio < threshold — likely cache miss.
    Miss,
}

impl std::fmt::Display for CachePrediction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hit => write!(f, "hit"),
            Self::Miss => write!(f, "miss"),
        }
    }
}

/// Configuration. Defaults mirror contract defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePredictorConfig {
    /// Prefix overlap ratio ≥ this → predict Hit.
    pub hit_threshold: f64,
}

impl Default for CachePredictorConfig {
    fn default() -> Self {
        Self { hit_threshold: 0.70 }
    }
}

/// Pure cache predictor.
pub struct CachePredictor {
    config: CachePredictorConfig,
}

impl CachePredictor {
    pub fn new(config: CachePredictorConfig) -> Self {
        Self { config }
    }

    /// Compute overlap ratio: `prefix_overlap_tokens / total_context_tokens`.
    /// Returns 0.0 when `total_context_tokens == 0`.
    #[must_use]
    #[inline]
    pub fn overlap_ratio(signal: &CacheSignal) -> f64 {
        if signal.total_context_tokens == 0 {
            return 0.0;
        }
        (signal.prefix_overlap_tokens as f64 / signal.total_context_tokens as f64).clamp(0.0, 1.0)
    }

    /// Predict whether this request will hit the provider's prompt cache.
    #[must_use]
    #[inline]
    pub fn predict(&self, signal: &CacheSignal) -> CachePrediction {
        if Self::overlap_ratio(signal) >= self.config.hit_threshold {
            CachePrediction::Hit
        } else {
            CachePrediction::Miss
        }
    }
}

/// Metric payload emitted for each cache prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHitPredictionEvent {
    pub metric_type: &'static str,
    pub prediction: String,
    pub overlap_ratio: f64,
    pub session_id: Option<String>,
}

impl CacheHitPredictionEvent {
    pub fn new(prediction: CachePrediction, overlap_ratio: f64, session_id: Option<String>) -> Self {
        Self {
            metric_type: vox_db::research_metrics_contract::METRIC_TYPE_CACHE_HIT_PREDICTION,
            prediction: prediction.to_string(),
            overlap_ratio,
            session_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn predictor() -> CachePredictor {
        CachePredictor::new(CachePredictorConfig::default())
    }

    #[test]
    fn above_threshold_predicts_hit() {
        let p = predictor();
        let sig = CacheSignal { prefix_overlap_tokens: 700, total_context_tokens: 1000 };
        assert_eq!(p.predict(&sig), CachePrediction::Hit);
    }

    #[test]
    fn at_threshold_predicts_hit() {
        let p = predictor();
        let sig = CacheSignal { prefix_overlap_tokens: 700, total_context_tokens: 1000 };
        assert_eq!(p.predict(&sig), CachePrediction::Hit);
    }

    #[test]
    fn below_threshold_predicts_miss() {
        let p = predictor();
        let sig = CacheSignal { prefix_overlap_tokens: 500, total_context_tokens: 1000 };
        assert_eq!(p.predict(&sig), CachePrediction::Miss);
    }

    #[test]
    fn zero_total_tokens_gives_miss() {
        let p = predictor();
        let sig = CacheSignal { prefix_overlap_tokens: 100, total_context_tokens: 0 };
        assert_eq!(p.predict(&sig), CachePrediction::Miss);
    }

    #[test]
    fn cache_prediction_event_has_correct_metric_type() {
        let ev = CacheHitPredictionEvent::new(CachePrediction::Hit, 0.75, None);
        assert_eq!(ev.metric_type, "orch.cache.hit_prediction");
    }
}
