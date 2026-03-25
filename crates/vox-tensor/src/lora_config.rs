//! Pure-Rust LoRA configuration types — always compiled, no `gpu` feature required.
//!
//! [`LoraConfig`] and [`lora_memory_estimate`] are the workspace SSOT for LoRA adapter
//! configuration. They carry no Burn or GPU dependency and may be used freely in
//! training-config parsing, logging, or CLI surfaces.
//!
//! GPU-dependent types (`LoraLinear`, `LoraVoxTransformer`, etc.) live in [`crate::lora`]
//! and require the `gpu` feature.

/// Configuration for a LoRA adapter.
///
/// Typical defaults: rank=8, alpha=16, dropout=0.0
///
/// ## Example
/// ```rust
/// use vox_tensor::LoraConfig;
/// let cfg = LoraConfig::default();
/// assert_eq!(cfg.rank, 8);
/// ```
#[derive(Debug, Clone)]
pub struct LoraConfig {
    /// Low-rank dimension r. Typical values: 4, 8, 16, 32.
    /// Higher rank → more expressiveness but more trainable parameters.
    pub rank: usize,
    /// LoRA scaling factor applied as `alpha / rank`.
    /// Setting `alpha == rank` gives a scale of 1.0 (no explicit amplification).
    pub alpha: f32,
    /// Dropout probability applied to the LoRA branch input. `0.0` disables dropout.
    pub dropout: f32,
}

impl Default for LoraConfig {
    fn default() -> Self {
        Self {
            rank: 8,
            alpha: 16.0,
            dropout: 0.0,
        }
    }
}

/// Estimated memory savings from LoRA vs full fine-tuning.
///
/// Returns `(lora_params, full_params, saving_pct)` where:
/// - `lora_params` = `in_features * rank + rank * out_features`
/// - `full_params` = `in_features * out_features`
/// - `saving_pct` = percentage saved (0–100)
///
/// ## Example
/// ```rust
/// use vox_tensor::lora_memory_estimate;
/// let (lora, full, pct) = lora_memory_estimate(512, 512, 8);
/// assert!(pct > 90.0);
/// ```
pub fn lora_memory_estimate(
    in_features: usize,
    out_features: usize,
    rank: usize,
) -> (usize, usize, f32) {
    let lora = in_features * rank + rank * out_features;
    let full = in_features * out_features;
    let saving = 1.0 - (lora as f32 / full as f32);
    (lora, full, saving * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lora_config_defaults() {
        let cfg = LoraConfig::default();
        assert_eq!(cfg.rank, 8);
        assert_eq!(cfg.alpha, 16.0);
        assert_eq!(cfg.dropout, 0.0);
    }

    #[test]
    fn lora_scale_calculation() {
        let cfg = LoraConfig {
            rank: 8,
            alpha: 16.0,
            dropout: 0.0,
        };
        let expected_scale = cfg.alpha / cfg.rank as f32;
        assert!((expected_scale - 2.0).abs() < 1e-6);
    }

    #[test]
    fn lora_memory_estimate_reasonable() {
        let (lora, full, saving_pct) = lora_memory_estimate(512, 512, 8);
        assert_eq!(full, 512 * 512);
        assert_eq!(lora, 512 * 8 + 8 * 512);
        assert!(
            saving_pct > 90.0,
            "Expected >90% saving, got {saving_pct:.1}%"
        );
    }
}
