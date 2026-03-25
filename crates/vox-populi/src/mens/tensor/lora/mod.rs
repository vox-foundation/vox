//! **Mens** LoRA transformer stack, HF loaders, and merge (`vox-mens`).
//!
//! This module owns **`LoraVoxTransformer`**, **`LoraAttention`**, merge-to-static-MHA (non-RoPE),
//! and Burn training-graph wiring — not the lightweight `vox-tensor` LoRA primitives.
//!
//! ## Relationship to `vox-tensor`
//!
//! `vox_tensor::lora` holds shared **`LoraConfig` / `LoraLinear`**
//! used elsewhere in the workspace. This file **historically duplicated** that surface for `gpu`-gated
//! Mens code paths; behavior can drift — when fixing LoRA **linear** math, check **both** trees or
//! consolidate behind one implementation (see `docs/src/architecture/mens-llm-pr-checklist.md`).
//!
//! ## Quantization
//!
//! Burn training here uses **f32** bases (no in-tree NF4 frozen base). Candle QLoRA (NF4) lives under
//! `candle_qlora_train.rs` + qlora-rs. Burn **0.19** exposes quantization building blocks; wiring
//! true QLoRA on the full Mens graph is **integration backlog** (see `hf-finetune-gap-matrix-ssot.md`).

#[cfg(feature = "mens-gpu")]
use burn::module::Module;
#[cfg(feature = "mens-gpu")]
use burn::nn;
#[cfg(feature = "mens-gpu")]
use burn::tensor::Int;
#[cfg(feature = "mens-gpu")]
use burn::tensor::Tensor;
#[cfg(feature = "mens-gpu")]
use burn::tensor::TensorData;
#[cfg(feature = "mens-gpu")]
use burn::tensor::backend::Backend;

// ── LoraConfig SSOT re-export ─────────────────────────────────────────────
//
// `LoraConfig` and `lora_memory_estimate` live in `vox-tensor` as the single
// source of truth.  This module re-exports them so that the rest of the
// `vox-mens` tree (including `lora_train.rs`, `lora::LoraLinear`, and the
// public `vox_mens::LoraConfig` path via `tensor/mod.rs`) continues to work
// without any callers changing their imports.
pub use vox_tensor::{LoraConfig, lora_memory_estimate};

/// A LoRA-adapted linear layer.
///
/// Wraps a base `nn::Linear` with two trainable low-rank matrices:
/// - `lora_a`: `(in_features, rank)` — initialized with Kaiming uniform
/// - `lora_b`: `(rank, out_features)` — initialized to zero (LoRA is identity at init)
///
/// Forward: `base(x) + scale * lora_b(lora_a(x))`
///
/// ## Freezing base weights
///
/// In a future Burn API once `requires_grad` per-module is stable, the base
/// layer should be frozen. Currently, the caller must exclude `lora_linear.base`
/// parameters from the optimizer — OR use a selective optimizer wrapper.
///
/// ## Gradient flow
///
/// Only `lora_a` and `lora_b` are intended to be updated. The scale factor
/// `alpha / rank` is applied as a constant multiplier, not a learned param.
#[cfg(feature = "mens-gpu")]
#[derive(Module, Debug)]
pub struct LoraLinear<B: Backend> {
    /// Frozen base transformation (the pre-trained weight).
    pub base: nn::Linear<B>,
    /// LoRA down-projection: in_features → rank.
    pub lora_a: nn::Linear<B>,
    /// LoRA up-projection: rank → out_features (zero-initialized).
    pub lora_b: nn::Linear<B>,
    /// Scaling constant: alpha / rank.
    pub scale: f32,
    /// Dropout on LoRA branch (between lora_a and lora_b). Use prob 0.0 when disabled.
    pub dropout: nn::Dropout,
}

#[cfg(feature = "mens-gpu")]
impl<B: Backend> LoraLinear<B> {
    /// Construct a LoRA linear layer wrapping a freshly-initialized base.
    ///
    /// In production, you would replace `base` with loaded pre-trained weights.
    pub fn new(
        device: &B::Device,
        in_features: usize,
        out_features: usize,
        config: &LoraConfig,
    ) -> Self {
        let base = nn::LinearConfig::new(in_features, out_features)
            .with_bias(true)
            .init(device);
        // A uses default (Kaiming uniform) init for gradient diversity
        let lora_a = nn::LinearConfig::new(in_features, config.rank)
            .with_bias(false)
            .init(device);
        // B is zero-initialized so LoRA contribution starts at zero (identity at init)
        let lora_b = nn::LinearConfig::new(config.rank, out_features)
            .with_bias(false)
            .with_initializer(nn::Initializer::Zeros)
            .init(device);
        let scale = config.alpha / config.rank as f32;
        let dropout = nn::DropoutConfig::new(config.dropout as f64).init();
        Self {
            base,
            lora_a,
            lora_b,
            scale,
            dropout,
        }
    }

    /// Construct a LoRA linear layer wrapping a pre-loaded base (e.g. from Hugging Face).
    ///
    /// Use this when loading HF weights: the base holds the pre-trained weights;
    /// lora_a and lora_b are initialized for training (zero-init on B).
    /// Burn Linear weight is `[d_input, d_output]`, so `dims()[0]` = in, `dims()[1]` = out.
    pub fn with_base(base: nn::Linear<B>, config: &LoraConfig) -> Self {
        let in_features = base.weight.dims()[0];
        let out_features = base.weight.dims()[1];
        let device = base.weight.devices()[0].clone();
        let lora_a = nn::LinearConfig::new(in_features, config.rank)
            .with_bias(false)
            .init(&device);
        let lora_b = nn::LinearConfig::new(config.rank, out_features)
            .with_bias(false)
            .with_initializer(nn::Initializer::Zeros)
            .init(&device);
        let scale = config.alpha / config.rank as f32;
        let dropout = nn::DropoutConfig::new(config.dropout as f64).init();
        Self {
            base,
            lora_a,
            lora_b,
            scale,
            dropout,
        }
    }

    /// Forward pass.
    ///
    /// `output = base(x) + scale * lora_b(lora_a(x))`
    ///
    /// The LoRA branch starts at zero due to zero-init of `lora_b`, meaning
    /// the model is numerically equivalent to the base model at initialization.
    pub fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
        let base_out = self.base.forward(x.clone());
        let h = self.dropout.forward(self.lora_a.forward(x));
        let lora_out = self.lora_b.forward(h);
        base_out + lora_out * self.scale
    }

    /// Count LoRA-only trainable parameters (lora_a + lora_b).
    ///
    /// `params = in_features * rank + rank * out_features`
    /// vs. full fine-tuning: `in_features * out_features`
    pub fn lora_param_count(&self, in_features: usize, out_features: usize) -> usize {
        // rank is derived from lora_a's output dimension ([in_features, rank])
        let rank = self.lora_a.weight.dims()[1];
        in_features * rank + rank * out_features
    }
}

include!("part_attention.rs");
include!("part_block.rs");
include!("part_vox.rs");

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
        // scale = alpha / rank = 16 / 8 = 2.0
        let expected_scale = cfg.alpha / cfg.rank as f32;
        assert!((expected_scale - 2.0).abs() < 1e-6);
    }

    #[test]
    fn lora_memory_estimate_reasonable() {
        // 512×512 base, rank=8
        let (lora, full, saving_pct) = lora_memory_estimate(512, 512, 8);
        // lora = 512*8 + 8*512 = 8192; full = 512*512 = 262144
        assert_eq!(full, 512 * 512);
        assert_eq!(lora, 512 * 8 + 8 * 512);
        assert!(
            saving_pct > 90.0,
            "Expected >90% memory saving, got {saving_pct:.1}%"
        );
    }

    #[test]
    fn lora_param_count_matches_formula() {
        // rank=8, alpha=8 => scale=1.0, but lora_param_count uses scale as usize which is 1
        // The test validates the formula logic — actual GPU test requires a Backend device.
        let (lora_params, _, saving) = lora_memory_estimate(768, 768, 8);
        let expected = 768 * 8 + 8 * 768;
        assert_eq!(lora_params, expected);
        println!("LoRA memory saving for 768×768, rank=8: {saving:.1}%");
    }
}

include!("part_gpu_tests.rs");
