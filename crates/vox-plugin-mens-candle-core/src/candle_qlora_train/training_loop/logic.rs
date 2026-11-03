//! Logic bits for training loop.
//!
//! Canonical home for `trajectory_weight_for_pair`, previously byte-identical
//! between `vox-plugin-mens-candle-metal`'s `logic.rs` (which held only this
//! function) and `vox-plugin-mens-candle-cuda`'s `logic.rs` (which also holds
//! `checkpointed_backward_step`, a CUDA-only gradient-checkpointing addition
//! not ported to Metal — that function stays in the CUDA plugin, which
//! re-exports this one alongside it.

use crate::config::LoraTrainingConfig;
use vox_tensor::data::TrainingPair;

pub fn trajectory_weight_for_pair(pair: &TrainingPair, config: &LoraTrainingConfig) -> (f64, bool) {
    // The mixer emits each row once and stamps the source `weight:` as
    // `mix_weight` for the trainer to honor; applied as a per-row loss weight.
    let mut weight = pair
        .mix_weight
        .filter(|w| w.is_finite() && *w >= 0.0)
        .unwrap_or(1.0);
    if !config.trajectory_weighting_enabled {
        return clamp_weight(weight);
    }
    if let Some(category) = pair.category.as_deref() {
        let c = category.to_ascii_lowercase();
        if c.contains("tool_trace") || c.contains("trajectory") {
            weight *= config.trajectory_tool_trace_boost.max(0.0) as f64;
        }
        if c.contains("fail") || c.contains("error") {
            weight *= config.trajectory_failure_category_boost.max(0.0) as f64;
        }
    }
    if let (Some(floor), Some(rating)) = (config.trajectory_quality_floor, pair.rating)
        && rating >= floor
    {
        weight *= config.trajectory_quality_boost.max(0.0) as f64;
    }
    clamp_weight(weight)
}

fn clamp_weight(weight: f64) -> (f64, bool) {
    if !weight.is_finite() {
        return (1.0, true);
    }
    const MAX_TRAJECTORY_WEIGHT: f64 = 8.0;
    let clamped = weight.clamp(0.0, MAX_TRAJECTORY_WEIGHT);
    let was_clamped = (clamped - weight).abs() > f64::EPSILON;
    (clamped, was_clamped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_weight_scales_loss_even_without_trajectory_weighting() {
        let cfg = LoraTrainingConfig::default();
        assert!(!cfg.trajectory_weighting_enabled);
        let pair = TrainingPair {
            mix_weight: Some(6.0),
            ..Default::default()
        };
        assert_eq!(trajectory_weight_for_pair(&pair, &cfg), (6.0, false));
        assert_eq!(
            trajectory_weight_for_pair(&TrainingPair::default(), &cfg),
            (1.0, false)
        );
        let huge = TrainingPair {
            mix_weight: Some(50.0),
            ..Default::default()
        };
        assert_eq!(trajectory_weight_for_pair(&huge, &cfg), (8.0, true));
    }
}
