//! `QLoRA` layer implementation.
//!
//! Combines quantized base weights with trainable `LoRA` adapters.
//!
//! # Training Configuration
//!
//! **CRITICAL**: Always use BF16 compute dtype for training stability.
//! Using FP16 results in ~20% training failure rate due to numerical instability.
//!
//! # References
//! - `QLoRA` paper: <https://arxiv.org/abs/2305.14314>
//! - PEFT `prepare_model_for_kbit_training`: upcasts non-quantized modules to FP32

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use peft_rs::{Adapter, LoraConfig, LoraLayer};
use serde::{Deserialize, Serialize};

use crate::error::{QLoraError, Result};
use crate::quantization::{
    ComputeDType, QuantizationConfig, QuantizedTensor, dequantize_nf4_with_dtype,
    quantize_nf4_with_config,
};

/// Apply a `candle_nn::Linear`-style (no-bias) projection `x @ w^T` where `w` has
/// shape `[out, in]`, mirroring how peft_rs's LoRA A/B (built via `linear_no_bias`)
/// project. candle's `Linear::forward` handles rank-3 inputs by collapsing the
/// leading dims, matmul-ing, and reshaping back; we replicate that so the BF16 path
/// behaves identically to peft_rs's F32 path apart from dtype.
fn linear_no_bias_t(input: &Tensor, w: &Tensor) -> Result<Tensor> {
    let w_t = w.t()?; // [in, out]
    let out = if input.dims().len() == 3 {
        let (b, s, in_f) = input.dims3()?;
        let out_f = w_t.dim(1)?;
        input
            .reshape(&[b * s, in_f])?
            .matmul(&w_t)?
            .reshape(&[b, s, out_f])?
    } else {
        input.matmul(&w_t)?
    };
    Ok(out)
}

fn warn_cpu_fallback(device: &Device) {
    static WARN_ONCE: std::sync::Once = std::sync::Once::new();
    if matches!(device, Device::Cpu) {
        WARN_ONCE.call_once(|| {
            eprintln!(
                "qlora-rs: CPU device in use. CUDA is the intended default; enable the 'cuda' feature and use Device::cuda_if_available(0) when possible."
            );
        });
    }
}

/// Configuration for `QLoRA` training and inference.
///
/// # Compute Dtype
///
/// **CRITICAL**: The `compute_dtype` field controls numerical precision during training.
/// - `BF16`: **Required for training** - 100% stability rate
/// - `FP16`: **Do NOT use for training** - 20% failure rate due to numerical instability
/// - `FP32`: Stable but slower, useful for debugging
///
/// # Target Modules
///
/// The `target_modules` field controls which linear layers get `LoRA` adapters:
/// - Minimal: `["q_proj", "v_proj"]` - 98% of full fine-tuning quality
/// - Recommended: `["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"]`
///   - Matches full fine-tuning quality (~99.3%)
///
/// # Example
///
/// ```rust
/// use qlora_rs::QLoraConfig;
///
/// // Use preset for best stability
/// let config = QLoraConfig::preset_all_bf16(64, 16);
///
/// // Or customize
/// let config = QLoraConfig {
///     target_modules: vec!["q_proj".into(), "v_proj".into()],
///     ..QLoraConfig::preset_all_bf16(32, 8)
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QLoraConfig {
    /// `LoRA` configuration (rank, alpha, dropout).
    pub lora: LoraConfig,
    /// Quantization configuration (block size, double quant).
    pub quantization: QuantizationConfig,
    /// Target modules to apply `LoRA` to.
    /// Default: all linear layers in transformer blocks.
    #[serde(default = "default_target_modules")]
    pub target_modules: Vec<String>,
    /// Whether to cache dequantized weights (opt-in, for inference speedup).
    /// Default: false (on-the-fly dequantization saves memory).
    #[serde(default)]
    pub cache_dequantized: bool,
}

fn default_target_modules() -> Vec<String> {
    vec![
        "q_proj".into(),
        "k_proj".into(),
        "v_proj".into(),
        "o_proj".into(),
        "gate_proj".into(),
        "up_proj".into(),
        "down_proj".into(),
    ]
}

impl Default for QLoraConfig {
    /// Default configuration: BF16 compute, all linear layers targeted.
    ///
    /// **Note**: Default uses BF16 compute dtype for training stability.
    fn default() -> Self {
        Self {
            lora: LoraConfig {
                r: 64,
                alpha: 16,
                dropout: 0.05,
                ..Default::default()
            },
            quantization: QuantizationConfig {
                block_size: 64,
                double_quant: true,
                compute_dtype: ComputeDType::BF16, // CRITICAL: BF16 for stability
                ..Default::default()
            },
            target_modules: default_target_modules(),
            cache_dequantized: false, // On-the-fly by default (memory optimal)
        }
    }
}

impl QLoraConfig {
    /// Create preset targeting all linear layers with BF16 compute.
    ///
    /// **Recommended for training**. Matches `QLoRA` paper configuration.
    ///
    /// # Arguments
    /// * `r` - `LoRA` rank (typical: 8-64)
    /// * `alpha` - `LoRA` scaling factor (typical: 16-32)
    #[must_use]
    pub fn preset_all_bf16(r: usize, alpha: usize) -> Self {
        Self {
            lora: LoraConfig {
                r,
                alpha,
                dropout: 0.05,
                ..Default::default()
            },
            quantization: QuantizationConfig {
                block_size: 64,
                double_quant: true,
                compute_dtype: ComputeDType::BF16,
                ..Default::default()
            },
            target_modules: default_target_modules(),
            cache_dequantized: false,
        }
    }

    /// Create preset targeting only attention Q/V projections with BF16 compute.
    ///
    /// Memory-optimal preset: fewer trainable parameters.
    /// Achieves ~98% of full fine-tuning quality.
    #[must_use]
    pub fn preset_qv_bf16(r: usize, alpha: usize) -> Self {
        Self {
            lora: LoraConfig {
                r,
                alpha,
                dropout: 0.05,
                ..Default::default()
            },
            quantization: QuantizationConfig {
                block_size: 64,
                double_quant: true,
                compute_dtype: ComputeDType::BF16,
                ..Default::default()
            },
            target_modules: vec!["q_proj".into(), "v_proj".into()],
            cache_dequantized: false,
        }
    }

    /// Create preset for inference with weight caching enabled.
    ///
    /// Uses cached dequantization for faster inference at cost of memory.
    #[must_use]
    pub fn preset_inference(r: usize, alpha: usize) -> Self {
        Self {
            cache_dequantized: true, // Enable caching for inference speed
            ..Self::preset_all_bf16(r, alpha)
        }
    }

    /// Check if a module should have `LoRA` applied.
    #[must_use]
    pub fn is_target(&self, module_name: &str) -> bool {
        self.target_modules.iter().any(|t| module_name.contains(t))
    }

    /// Get the `LoRA` scaling factor (alpha / r).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn scale(&self) -> f64 {
        self.lora.alpha as f64 / self.lora.r as f64
    }

    /// Validate configuration for training.
    ///
    /// # Errors
    /// Returns error if configuration is invalid for training.
    pub fn validate_for_training(&self) -> Result<()> {
        if self.lora.r == 0 {
            return Err(QLoraError::InvalidConfig("LoRA rank must be > 0".into()));
        }
        if self.target_modules.is_empty() {
            return Err(QLoraError::InvalidConfig(
                "At least one target module required".into(),
            ));
        }
        // Warn about FP16 but don't error - user might know what they're doing
        if matches!(self.quantization.compute_dtype, ComputeDType::F16) {
            tracing::warn!(
                "FP16 compute dtype may cause training instability (20% failure rate). \
                 Consider using BF16 instead."
            );
        }
        Ok(())
    }
}

/// A linear layer with quantized base weights and trainable `LoRA` adapters.
///
/// # Dequantization Modes
///
/// - **On-the-fly** (default): Dequantizes during each forward pass, saves memory.
/// - **Cached** (opt-in via `cache_dequantized`): Dequantizes once, faster inference.
///
/// For training, always use on-the-fly mode (default) to save memory.
/// For inference, consider enabling caching for ~30% speedup.
pub struct QuantizedLinear {
    /// Quantized base weight (frozen, NF4 format).
    quantized_weight: QuantizedTensor,
    /// Cached dequantized weight (opt-in for inference speedup).
    cached_weight: Option<Tensor>,
    /// Compact GPU-resident NF4 `(q_data_u8, scales_f32, out_features, in_features, compute_dtype, allow_gemv)`
    /// for Metal (`0.5625 B/param` vs `2 B/param` BF16 cache). `allow_gemv` permits the
    /// inference-only `nf4_gemv` path (`apply_op3_no_bwd`); training layers keep `allow_gemv = false`
    /// and dequantize on the fly via `nf4_dequant` + differentiable `matmul`.
    gpu_nf4: Option<(Tensor, Tensor, usize, usize, DType, bool)>,
    /// Optional bias (not quantized, kept in full precision).
    bias: Option<Tensor>,
    /// `LoRA` adapter (trainable).
    lora: LoraLayer,
    /// True when `lora` holds untouched `B = 0` weights (`from_weight` with `loftq_iterations == 0`),
    /// so `x @ A^T @ B^T * scale` is identically zero and can be skipped in `forward`.
    lora_zero: bool,
    /// Device for dequantization.
    device: Device,
    /// Configuration for quantization (needed for on-the-fly dequant).
    config: QLoraConfig,
}

impl QuantizedLinear {
    /// Create a new quantized linear layer from existing weights.
    ///
    /// Uses on-the-fly dequantization by default (memory-optimal).
    /// Set `config.cache_dequantized = true` for inference speedup.
    ///
    /// # Arguments
    /// * `weight` - Full-precision weight tensor to quantize
    /// * `bias` - Optional bias tensor (kept in full precision)
    /// * `config` - `QLoRA` configuration
    /// * `device` - Device for computation
    ///
    /// # Errors
    /// Returns error if weight tensor has invalid shape or quantization fails
    pub fn from_weight(
        weight: &Tensor,
        bias: Option<Tensor>,
        config: &QLoraConfig,
        device: &Device,
    ) -> Result<Self> {
        warn_cpu_fallback(device);
        let shape = weight.shape().dims();
        if shape.len() != 2 {
            return Err(QLoraError::InvalidConfig("weight must be 2D".into()));
        }
        let (out_features, in_features) = (shape[0], shape[1]);

        // Quantize the base weight using full config
        let quantized_weight = quantize_nf4_with_config(weight, &config.quantization)?;

        // Only cache if explicitly requested. On Metal with aligned block-64 NF4, keep
        // compact NF4 bytes + F32 block scales on GPU (~0.56 B/param instead of 2 B/param
        // BF16) and dispatch fused nf4_gemv on M<=16 decode / nf4_dequant on M>16 prefill.
        let (quantized_weight, cached_weight, gpu_nf4) = if config.cache_dequantized {
            let (q, gpu_nf4) = Self::try_stage_metal_nf4(
                quantized_weight,
                out_features,
                in_features,
                config,
                device,
                true,
            )?;
            if gpu_nf4.is_some() {
                (q, None, gpu_nf4)
            } else {
                let cache = dequantize_nf4_with_dtype(
                    &q,
                    weight.device(),
                    config.quantization.compute_dtype,
                )?;
                let cache = if cache.device().same_device(device) {
                    cache
                } else {
                    cache.to_device(device)?
                };
                let mut q = q;
                q.data = Vec::new();
                q.scales = Vec::new();
                q.zero_points = None;
                (q, Some(cache.detach()), None)
            }
        } else {
            (quantized_weight, None, None)
        };

        // Create LoRA adapter
        let lora =
            LoraLayer::new_with_zeros(in_features, out_features, config.lora.clone(), device)?;

        Ok(Self {
            quantized_weight,
            cached_weight,
            gpu_nf4,
            bias,
            lora,
            lora_zero: config.lora.loftq_iterations == 0,
            device: device.clone(),
            config: config.clone(),
        })
    }

    fn compute_candle_dtype(config: &QLoraConfig) -> DType {
        match config.quantization.compute_dtype {
            ComputeDType::F32 => DType::F32,
            ComputeDType::F16 => DType::F16,
            ComputeDType::BF16 => DType::BF16,
        }
    }

    #[allow(clippy::type_complexity)]
    fn try_stage_metal_nf4(
        mut quantized_weight: QuantizedTensor,
        out_features: usize,
        in_features: usize,
        config: &QLoraConfig,
        device: &Device,
        allow_gemv: bool,
    ) -> Result<(
        QuantizedTensor,
        Option<(Tensor, Tensor, usize, usize, DType, bool)>,
    )> {
        let can_use_metal_nf4 = device.is_metal()
            && config.lora.loftq_iterations == 0
            && quantized_weight.strategy == crate::quantization::QuantizationStrategy::PerTensor
            && quantized_weight.zero_points.is_none()
            && quantized_weight.block_size == 64
            && in_features >= 64
            && in_features % 64 == 0
            && quantized_weight.data.len() == (out_features * in_features) / 2;
        if !can_use_metal_nf4 {
            return Ok((quantized_weight, None));
        }

        let scales_vec = quantized_weight.unpacked_scales();
        let q_len = quantized_weight.data.len();
        let s_len = scales_vec.len();
        let q_bytes = std::mem::take(&mut quantized_weight.data);
        quantized_weight.scales = Vec::new();
        quantized_weight.scales_quantized = None;
        quantized_weight.scales_scales = None;
        quantized_weight.zero_points = None;
        let q_gpu = Tensor::from_vec(q_bytes, q_len, device)?;
        let s_gpu = Tensor::from_vec(scales_vec, s_len, device)?;
        let compute_candle_dtype = Self::compute_candle_dtype(config);
        Ok((
            quantized_weight,
            Some((
                q_gpu,
                s_gpu,
                out_features,
                in_features,
                compute_candle_dtype,
                allow_gemv,
            )),
        ))
    }

    /// Shared NF4 quantization + optional BF16/F16 cache or compact Metal NF4 staging on `compute_device`.
    #[allow(clippy::type_complexity)]
    fn quantize_and_cache_dequantized(
        weight: &Tensor,
        config: &QLoraConfig,
        compute_device: &Device,
    ) -> Result<(
        usize,
        usize,
        QuantizedTensor,
        Option<Tensor>,
        Option<(Tensor, Tensor, usize, usize, DType, bool)>,
    )> {
        let shape = weight.shape().dims();
        if shape.len() != 2 {
            return Err(QLoraError::InvalidConfig("weight must be 2D".into()));
        }
        let (out_features, in_features) = (shape[0], shape[1]);
        warn_cpu_fallback(compute_device);

        // Quantize on the base weight's own device (CPU when the caller mmapped it there).
        let quantized_weight = quantize_nf4_with_config(weight, &config.quantization)?;

        let (quantized_weight, cached_weight, gpu_nf4) = if config.cache_dequantized {
            // Dequantize on the BASE weight's device first, then move the result to
            // the compute device so dequant temporaries never spike VRAM during build.
            let cache = dequantize_nf4_with_dtype(
                &quantized_weight,
                weight.device(),
                config.quantization.compute_dtype,
            )?;
            let cache = if cache.device().same_device(compute_device) {
                cache
            } else {
                cache.to_device(compute_device)?
            };
            let mut q = quantized_weight;
            q.data = Vec::new();
            q.scales = Vec::new();
            q.zero_points = None;
            (q, Some(cache.detach()), None)
        } else {
            let (q, gpu_nf4) = Self::try_stage_metal_nf4(
                quantized_weight,
                out_features,
                in_features,
                config,
                compute_device,
                false,
            )?;
            (q, None, gpu_nf4)
        };
        Ok((
            out_features,
            in_features,
            quantized_weight,
            cached_weight,
            gpu_nf4,
        ))
    }

    /// Create a quantized linear layer with trainable `LoRA` weights registered via `VarBuilder`.
    ///
    /// This constructor ensures `LoRA` A/B weights are tracked for gradient computation.
    /// Use this for training; use `from_weight` for inference.
    ///
    /// # Arguments
    /// * `weight` - Full-precision weight tensor to quantize
    /// * `bias` - Optional bias tensor (kept in full precision)
    /// * `config` - `QLoRA` configuration
    /// * `vb` - `VarBuilder` backed by `VarMap` for gradient tracking
    ///
    /// # Errors
    /// Returns error if weight tensor has invalid shape or quantization fails
    pub fn from_weight_with_varbuilder(
        weight: &Tensor,
        bias: Option<Tensor>,
        config: &QLoraConfig,
        vb: VarBuilder,
    ) -> Result<Self> {
        let compute_device = vb.device().clone();
        let (out_features, in_features, quantized_weight, cached_weight, gpu_nf4) =
            Self::quantize_and_cache_dequantized(weight, config, &compute_device)?;

        // Create LoRA adapter with VarBuilder for gradient tracking (on the compute device).
        let lora = LoraLayer::new(in_features, out_features, config.lora.clone(), vb)?;

        Ok(Self {
            quantized_weight,
            cached_weight,
            gpu_nf4,
            bias,
            lora,
            lora_zero: false,
            device: compute_device,
            config: config.clone(),
        })
    }

    /// Create a frozen quantized linear layer safe for training backprop through `input`.
    ///
    /// Unlike `from_weight`, when Metal GPU NF4 staging is active this keeps `allow_gemv = false`
    /// so forward uses differentiable `nf4_dequant` + `matmul` (or `cached_weight` when caching is
    /// enabled) across all sequence lengths instead of dispatching `nf4_gemv` (`apply_op3_no_bwd`)
    /// when `batch * seq <= 16`. No LoRA params are added to any `VarMap`.
    ///
    /// # Errors
    /// Returns error if weight tensor has invalid shape, quantization fails, or LoRA init fails
    pub fn from_weight_frozen(
        weight: &Tensor,
        bias: Option<Tensor>,
        config: &QLoraConfig,
        device: &Device,
    ) -> Result<Self> {
        let compute_device = device.clone();
        let (out_features, in_features, quantized_weight, cached_weight, gpu_nf4) =
            Self::quantize_and_cache_dequantized(weight, config, &compute_device)?;
        let lora =
            LoraLayer::new_with_zeros(in_features, out_features, config.lora.clone(), device)?;

        Ok(Self {
            quantized_weight,
            cached_weight,
            gpu_nf4,
            bias,
            lora,
            lora_zero: config.lora.loftq_iterations == 0,
            device: compute_device,
            config: config.clone(),
        })
    }

    /// Create a new quantized linear layer with zero-initialized quantized weights.
    ///
    /// Primarily for testing; use `from_weight` for actual models.
    ///
    /// # Errors
    /// Returns error if tensor creation or quantization fails
    pub fn new(
        in_features: usize,
        out_features: usize,
        config: &QLoraConfig,
        device: &Device,
    ) -> Result<Self> {
        let weight = Tensor::zeros(&[out_features, in_features], DType::F32, device)?;
        Self::from_weight(&weight, None, config, device)
    }

    /// Forward pass through the quantized linear layer.
    ///
    /// Computes: `output = x @ W_q^T + x @ (B @ A)^T * scaling + bias`
    ///
    /// Uses on-the-fly dequantization unless `cache_dequantized` was enabled.
    ///
    /// # Errors
    /// Returns error if tensor operations fail
    pub fn forward(&self, input: &Tensor) -> Result<Tensor> {
        let base_output =
            if let Some((q_gpu, s_gpu, out_features, in_features, w_dtype, allow_gemv)) =
                &self.gpu_nf4
            {
                let in_dtype = input.dtype();
                if input.dims().len() == 3 {
                    let (batch, seq, in_dim) = input.dims3()?;
                    let m = batch * seq;
                    let mut reshaped = input.reshape(&[m, in_dim])?;
                    if *w_dtype != in_dtype {
                        reshaped = reshaped.to_dtype(*w_dtype)?;
                    }
                    let out = if *allow_gemv && (1..=16).contains(&m) {
                        reshaped.nf4_gemv(q_gpu, s_gpu, *out_features, *in_features)?
                    } else {
                        let w = Tensor::nf4_dequant(
                            q_gpu,
                            s_gpu,
                            *out_features,
                            *in_features,
                            *w_dtype,
                        )?;
                        reshaped.matmul(&w.t()?)?
                    };
                    let out = out.reshape(&[batch, seq, *out_features])?;
                    if *w_dtype != in_dtype {
                        out.to_dtype(in_dtype)?
                    } else {
                        out
                    }
                } else {
                    let (m, _in_dim) = input.dims2()?;
                    let lhs = if *w_dtype != in_dtype {
                        input.to_dtype(*w_dtype)?
                    } else {
                        input.clone()
                    };
                    let out = if *allow_gemv && (1..=16).contains(&m) {
                        lhs.nf4_gemv(q_gpu, s_gpu, *out_features, *in_features)?
                    } else {
                        let w = Tensor::nf4_dequant(
                            q_gpu,
                            s_gpu,
                            *out_features,
                            *in_features,
                            *w_dtype,
                        )?;
                        lhs.matmul(&w.t()?)?
                    };
                    if *w_dtype != in_dtype {
                        out.to_dtype(in_dtype)?
                    } else {
                        out
                    }
                }
            } else {
                // Get dequantized weight (either from cache or on-the-fly), honoring the
                // configured compute dtype. With BF16 compute, the dequantized base weight
                // — which candle retains through the backward pass to compute input
                // gradients (the base is frozen, but d_input still needs the weight) — is
                // held at 2 bytes/param instead of 4. For a multi-billion-param model that
                // retained-weight term dominates VRAM, so this ~halves training footprint.
                let weight = if let Some(cached) = &self.cached_weight {
                    cached.clone()
                } else {
                    // On-the-fly dequantization (default, memory-optimal)
                    dequantize_nf4_with_dtype(
                        &self.quantized_weight,
                        &self.device,
                        self.config.quantization.compute_dtype,
                    )?
                };
                let weight_t = weight.t()?;

                // The base matmul runs in the weight's (compute) dtype. Cast the input to
                // match only when dtypes differ (no-op cost on the all-F32 path), then cast
                // the base output back to the input dtype so the LoRA residual + downstream
                // norms — which stay in F32 for numerical stability — remain consistent.
                let w_dtype = weight.dtype();
                let in_dtype = input.dtype();
                let base_output = if input.dims().len() == 3 {
                    // For [batch, seq, in_features], reshape to [batch * seq, in_features]
                    let (batch, seq, in_features) = input.dims3()?;
                    let mut reshaped = input.reshape(&[batch * seq, in_features])?;
                    if w_dtype != in_dtype {
                        reshaped = reshaped.to_dtype(w_dtype)?;
                    }
                    let out = reshaped.matmul(&weight_t)?;
                    // Reshape back to [batch, seq, out_features]
                    let out_features = weight_t.dim(1)?;
                    out.reshape(&[batch, seq, out_features])?
                } else {
                    // For 2D [batch, in_features], standard matmul
                    let lhs = if w_dtype != in_dtype {
                        input.to_dtype(w_dtype)?
                    } else {
                        input.clone()
                    };
                    lhs.matmul(&weight_t)?
                };
                if w_dtype != in_dtype {
                    base_output.to_dtype(in_dtype)?
                } else {
                    base_output
                }
            };

        // LoRA forward: adds x @ A^T @ B^T * scaling.
        //
        // peft_rs's `LoraLayer::forward` runs its matmuls in the LoRA params' dtype
        // (F32). Under BF16 compute the activations (`input`) are BF16, so calling it
        // directly panics with `dtype mismatch in matmul, lhs: BF16, rhs: F32`. We
        // replicate its exact math here but cast the (tiny, rank-r) A/B matrices to the
        // ACTIVATION dtype instead of casting the activation to F32. This keeps:
        //   - the trainable params + optimizer moments in F32 (stable),
        //   - the activations in BF16 (the memory win — no full-size F32 copy retained
        //     for backward; the cast tensors here are only [r,in] / [out,r]),
        //   - gradient flow to A/B intact (`to_dtype` is differentiable in candle),
        //   - the all-F32 CPU path a bitwise no-op (the casts are identities).
        // peft_rs applies no dropout in forward (the `dropout` config field is unused),
        // so there is nothing further to replicate. Scaling = alpha/r (or alpha/sqrt(r)
        // for rsLoRA) is taken verbatim from the layer.
        let output = if self.lora_zero {
            base_output
        } else {
            let (lora_a, lora_b) = self.lora.weights();
            let act_dtype = base_output.dtype();
            let a = lora_a.to_dtype(act_dtype)?; // [r, in_features]
            let b = lora_b.to_dtype(act_dtype)?; // [out_features, r]
            // x @ A^T -> [..., r]; (x @ A^T) @ B^T -> [..., out_features]
            let lora_out = linear_no_bias_t(input, &a)?;
            let lora_out = linear_no_bias_t(&lora_out, &b)?;
            let lora_out = (lora_out * self.lora.scaling())?;
            base_output.broadcast_add(&lora_out)?
        };

        // Add bias if present
        match &self.bias {
            Some(bias) => {
                if bias.dtype() != output.dtype() {
                    Ok(output.broadcast_add(&bias.to_dtype(output.dtype())?)?)
                } else {
                    Ok(output.broadcast_add(bias)?)
                }
            }
            None => Ok(output),
        }
    }

    /// Target row count when padding decode rows up to `gemm_rows` for width-invariant GEMM.
    ///
    /// Live row counts in `1..gemm_rows` pad to `gemm_rows`; `0` or `>= gemm_rows` (e.g. a
    /// long single-sequence prefill) stay unchanged.
    #[must_use]
    pub fn padded_rows(rows: usize, gemm_rows: usize) -> usize {
        if rows > 0 && rows < gemm_rows {
            gemm_rows
        } else {
            rows
        }
    }

    /// Width-invariant GEMM forward pass (`VOX_MENS_DECODE_PROTOCOL=bf16`): pads any
    /// `1..gemm_rows` input with zero rows up to `gemm_rows` before `nf4_dequant` + `matmul`
    /// and narrows back to the live rows, so every batch width `1..=gemm_rows` dispatches
    /// the exact same GEMM tile and produces bitwise-identical outputs per sequence.
    ///
    /// # Errors
    /// Returns error if tensor shapes mismatch or dequantization/matmul fails.
    pub fn forward_padded_gemm(&self, input: &Tensor, gemm_rows: usize) -> Result<Tensor> {
        let in_dtype = input.dtype();
        let (m, in_dim, out_shape) = if input.dims().len() == 3 {
            let (batch, seq, in_dim) = input.dims3()?;
            (batch * seq, in_dim, Some((batch, seq)))
        } else {
            let (m, in_dim) = input.dims2()?;
            (m, in_dim, None)
        };

        let base_output =
            if let Some((q_gpu, s_gpu, out_features, in_features, w_dtype, _allow_gemv)) =
                &self.gpu_nf4
            {
                let mut lhs = input.reshape(&[m, in_dim])?;
                if *w_dtype != in_dtype {
                    lhs = lhs.to_dtype(*w_dtype)?;
                }
                let padded_m = Self::padded_rows(m, gemm_rows);
                let padded_lhs = if padded_m > m {
                    let zeros = Tensor::zeros((padded_m - m, in_dim), lhs.dtype(), lhs.device())?;
                    Tensor::cat(&[&lhs, &zeros], 0)?
                } else {
                    lhs
                };
                let w = Tensor::nf4_dequant(q_gpu, s_gpu, *out_features, *in_features, *w_dtype)?;
                let out = padded_lhs.matmul(&w.t()?)?;
                let out = if padded_m > m {
                    out.narrow(0, 0, m)?
                } else {
                    out
                };
                let out = if let Some((batch, seq)) = out_shape {
                    out.reshape(&[batch, seq, *out_features])?
                } else {
                    out
                };
                if *w_dtype != in_dtype {
                    out.to_dtype(in_dtype)?
                } else {
                    out
                }
            } else {
                let weight = if let Some(cached) = &self.cached_weight {
                    cached.clone()
                } else {
                    dequantize_nf4_with_dtype(
                        &self.quantized_weight,
                        &self.device,
                        self.config.quantization.compute_dtype,
                    )?
                };
                let weight_t = weight.t()?;
                let w_dtype = weight.dtype();
                let out_features = weight_t.dim(1)?;
                let mut lhs = input.reshape(&[m, in_dim])?;
                if w_dtype != in_dtype {
                    lhs = lhs.to_dtype(w_dtype)?;
                }
                let padded_m = Self::padded_rows(m, gemm_rows);
                let padded_lhs = if padded_m > m {
                    let zeros = Tensor::zeros((padded_m - m, in_dim), lhs.dtype(), lhs.device())?;
                    Tensor::cat(&[&lhs, &zeros], 0)?
                } else {
                    lhs
                };
                let out = padded_lhs.matmul(&weight_t)?;
                let out = if padded_m > m {
                    out.narrow(0, 0, m)?
                } else {
                    out
                };
                let out = if let Some((batch, seq)) = out_shape {
                    out.reshape(&[batch, seq, out_features])?
                } else {
                    out
                };
                if w_dtype != in_dtype {
                    out.to_dtype(in_dtype)?
                } else {
                    out
                }
            };

        let output = if self.lora_zero {
            base_output
        } else {
            let (lora_a, lora_b) = self.lora.weights();
            let act_dtype = base_output.dtype();
            let a = lora_a.to_dtype(act_dtype)?;
            let b = lora_b.to_dtype(act_dtype)?;
            let lora_out = linear_no_bias_t(input, &a)?;
            let lora_out = linear_no_bias_t(&lora_out, &b)?;
            let lora_out = (lora_out * self.lora.scaling())?;
            base_output.broadcast_add(&lora_out)?
        };

        match &self.bias {
            Some(bias) => {
                if bias.dtype() != output.dtype() {
                    Ok(output.broadcast_add(&bias.to_dtype(output.dtype())?)?)
                } else {
                    Ok(output.broadcast_add(bias)?)
                }
            }
            None => Ok(output),
        }
    }

    /// Width-invariant F32 head projection (`D-25` / Pitfall 6): casts activations to `F32`
    /// and returns `F32` logits directly, chunking Metal `nf4_gemv` in slices of `<= 16` rows
    /// so `m > 16` neither allocates a multi-GB F32 weight buffer nor changes reduction order.
    ///
    /// # Errors
    /// Returns error if tensor shapes mismatch or projection fails.
    pub fn forward_f32_head(&self, input: &Tensor) -> Result<Tensor> {
        let input_f32 = if input.dtype() == DType::F32 {
            input.clone()
        } else {
            input.to_dtype(DType::F32)?
        };
        if let Some((q_gpu, s_gpu, out_features, in_features, _w_dtype, allow_gemv)) = &self.gpu_nf4
        {
            let (m, in_dim, out_shape) = if input_f32.dims().len() == 3 {
                let (batch, seq, in_dim) = input_f32.dims3()?;
                (batch * seq, in_dim, Some((batch, seq)))
            } else {
                let (m, in_dim) = input_f32.dims2()?;
                (m, in_dim, None)
            };
            let lhs = input_f32.reshape(&[m, in_dim])?.contiguous()?;
            let base_2d = if !*allow_gemv {
                let w = Tensor::nf4_dequant(q_gpu, s_gpu, *out_features, *in_features, DType::F32)?;
                lhs.matmul(&w.t()?)?
            } else if m <= 16 {
                lhs.nf4_gemv(q_gpu, s_gpu, *out_features, *in_features)?
            } else {
                let mut chunks = Vec::with_capacity(m.div_ceil(16));
                let mut start = 0usize;
                while start < m {
                    let take = (m - start).min(16);
                    let slice = lhs.narrow(0, start, take)?;
                    chunks.push(slice.nf4_gemv(q_gpu, s_gpu, *out_features, *in_features)?);
                    start += take;
                }
                Tensor::cat(&chunks, 0)?
            };
            let base_output = if let Some((batch, seq)) = out_shape {
                base_2d.reshape(&[batch, seq, *out_features])?
            } else {
                base_2d
            };
            let output = if self.lora_zero {
                base_output
            } else {
                let (lora_a, lora_b) = self.lora.weights();
                let a = lora_a.to_dtype(DType::F32)?;
                let b = lora_b.to_dtype(DType::F32)?;
                let lora_out = linear_no_bias_t(&input_f32, &a)?;
                let lora_out = linear_no_bias_t(&lora_out, &b)?;
                let lora_out = (lora_out * self.lora.scaling())?;
                base_output.broadcast_add(&lora_out)?
            };
            match &self.bias {
                Some(bias) => {
                    if bias.dtype() != DType::F32 {
                        Ok(output.broadcast_add(&bias.to_dtype(DType::F32)?)?)
                    } else {
                        Ok(output.broadcast_add(bias)?)
                    }
                }
                None => Ok(output),
            }
        } else {
            self.forward(&input_f32)
        }
    }

    /// Enable weight caching for faster inference.
    ///
    /// Call this after loading a trained model for inference.
    /// Not recommended for training (wastes memory).
    ///
    /// # Errors
    /// Returns error if dequantization fails.
    pub fn enable_weight_caching(&mut self) -> Result<()> {
        if self.cached_weight.is_none() && self.gpu_nf4.is_none() {
            self.cached_weight = Some(dequantize_nf4_with_dtype(
                &self.quantized_weight,
                &self.device,
                self.config.quantization.compute_dtype,
            )?);
        }
        Ok(())
    }

    /// Disable weight caching to save memory.
    pub fn disable_weight_caching(&mut self) {
        self.cached_weight = None;
    }

    /// Check if weight caching is enabled.
    #[must_use]
    pub fn is_weight_cached(&self) -> bool {
        self.cached_weight.is_some() || self.gpu_nf4.is_some()
    }

    /// Borrow the pre-dequantized dense `[out_features, in_features]` weight cache, if enabled.
    #[must_use]
    pub fn cached_weight(&self) -> Option<&Tensor> {
        self.cached_weight.as_ref()
    }

    /// Borrow the optional bias tensor attached to this linear layer.
    #[must_use]
    pub fn bias(&self) -> Option<&Tensor> {
        self.bias.as_ref()
    }

    /// True when the `LoRA` `B` matrix is untouched zeros and forward skips the adapter delta.
    #[must_use]
    pub fn is_lora_zero(&self) -> bool {
        self.lora_zero
    }

    /// Get the `QLoRA` configuration used to create this layer.
    #[must_use]
    pub fn config(&self) -> &QLoraConfig {
        &self.config
    }

    /// Get the `LoRA` adapter.
    #[must_use]
    pub fn lora(&self) -> &LoraLayer {
        &self.lora
    }

    /// Get mutable access to the `LoRA` adapter.
    pub fn lora_mut(&mut self) -> &mut LoraLayer {
        self.lora_zero = false;
        &mut self.lora
    }

    /// Get the `LoRA` A and B weight tensors.
    ///
    /// Returns (`lora_a`, `lora_b`) where:
    /// - `lora_a` has shape `[r, in_features]`
    /// - `lora_b` has shape `[out_features, r]`
    #[must_use]
    pub fn lora_weights(&self) -> (&Tensor, &Tensor) {
        self.lora.weights()
    }

    /// Get the number of trainable parameters (`LoRA` only).
    #[must_use]
    pub fn num_trainable_parameters(&self) -> usize {
        self.lora.num_parameters()
    }

    /// Get total memory usage in bytes.
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        let mut quantized_size = self.quantized_weight.size_bytes();
        if let Some((q_gpu, s_gpu, ..)) = &self.gpu_nf4 {
            quantized_size += q_gpu.elem_count() * q_gpu.dtype().size_in_bytes()
                + s_gpu.elem_count() * s_gpu.dtype().size_in_bytes();
        }
        let lora_size = self.lora.num_parameters() * 4; // f32
        let bias_size = self.bias.as_ref().map_or(0, |b| b.elem_count() * 4);
        quantized_size + lora_size + bias_size
    }
}

/// `QLoRA` adapter wrapping a model's linear layers.
pub struct QLoraLayer {
    /// Underlying quantized linear layer.
    linear: QuantizedLinear,
}

impl QLoraLayer {
    /// Create a new `QLoRA` layer.
    #[must_use]
    pub fn new(linear: QuantizedLinear) -> Self {
        Self { linear }
    }

    /// Forward pass.
    ///
    /// # Errors
    /// Returns error if the underlying linear layer forward fails
    pub fn forward(&self, input: &Tensor) -> Result<Tensor> {
        self.linear.forward(input)
    }

    /// Get a reference to the quantized base weight tensor.
    #[must_use]
    pub fn quantized_weight(&self) -> &QuantizedTensor {
        &self.linear.quantized_weight
    }

    /// Get the `LoRA` A and B weight tensors.
    ///
    /// Returns (`lora_a`, `lora_b`) where:
    /// - `lora_a` has shape `[r, in_features]`
    /// - `lora_b` has shape `[out_features, r]`
    #[must_use]
    pub fn lora_weights(&self) -> (&Tensor, &Tensor) {
        self.linear.lora_weights()
    }

    /// Get the `LoRA` scaling factor (alpha / rank).
    #[must_use]
    pub fn lora_scale(&self) -> f64 {
        self.linear.config.scale()
    }

    /// Get the device used by this layer.
    #[must_use]
    pub fn device(&self) -> &Device {
        &self.linear.device
    }

    /// Get the quantization configuration.
    #[must_use]
    pub fn config(&self) -> &QLoraConfig {
        &self.linear.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qlora_creation() {
        let config = QLoraConfig::default();
        let device = Device::Cpu;
        let layer = QuantizedLinear::new(768, 768, &config, &device);
        assert!(layer.is_ok());
    }

    #[test]
    fn test_qlora_forward_shape() {
        let config = QLoraConfig::default();
        let device = Device::Cpu;
        let layer = QuantizedLinear::new(768, 768, &config, &device).unwrap();

        let input = Tensor::zeros(&[1, 10, 768], DType::F32, &device).unwrap();
        let output = layer.forward(&input).unwrap();

        assert_eq!(output.shape().dims(), &[1, 10, 768]);
    }

    #[test]
    fn test_qlora_memory_reduction() {
        let config = QLoraConfig::default();
        let device = Device::Cpu;
        let layer = QuantizedLinear::new(4096, 4096, &config, &device).unwrap();

        // Full precision would be 4096 * 4096 * 4 = 67MB
        let full_size = 4096 * 4096 * 4;
        let actual_size = layer.memory_bytes();

        // Should be significantly smaller due to quantization
        #[allow(clippy::cast_precision_loss)]
        let ratio = f64::from(full_size) / actual_size as f64;
        assert!(ratio > 2.0, "Expected >2x reduction, got {ratio:.2}x");
    }

    // Tests for QLoraConfig presets and helper methods
    // Addresses PR #10 review comment: "No test coverage for presets and helper methods"

    #[test]
    fn test_preset_all_bf16() {
        let config = QLoraConfig::preset_all_bf16(64, 16);

        // Check LoRA config
        assert_eq!(config.lora.r, 64);
        assert_eq!(config.lora.alpha, 16);
        assert!((config.lora.dropout - 0.05).abs() < 1e-10);

        // Check quantization config
        assert!(matches!(
            config.quantization.compute_dtype,
            ComputeDType::BF16
        ));
        assert!(config.quantization.double_quant);

        // Check target modules (should be all linear layers)
        assert!(config.target_modules.contains(&"q_proj".to_string()));
        assert!(config.target_modules.contains(&"k_proj".to_string()));
        assert!(config.target_modules.contains(&"v_proj".to_string()));
        assert!(config.target_modules.contains(&"o_proj".to_string()));
        assert!(config.target_modules.contains(&"gate_proj".to_string()));

        // Should not cache weights by default (memory-optimal for training)
        assert!(!config.cache_dequantized);
    }

    #[test]
    fn test_preset_qv_bf16() {
        let config = QLoraConfig::preset_qv_bf16(32, 8);

        // Check LoRA config
        assert_eq!(config.lora.r, 32);
        assert_eq!(config.lora.alpha, 8);

        // Check target modules (should only be Q/V)
        assert_eq!(config.target_modules.len(), 2);
        assert!(config.target_modules.contains(&"q_proj".to_string()));
        assert!(config.target_modules.contains(&"v_proj".to_string()));

        // Should NOT contain other modules
        assert!(!config.target_modules.contains(&"k_proj".to_string()));
        assert!(!config.target_modules.contains(&"o_proj".to_string()));
    }

    #[test]
    fn test_preset_inference() {
        let config = QLoraConfig::preset_inference(16, 32);

        // Check LoRA config
        assert_eq!(config.lora.r, 16);
        assert_eq!(config.lora.alpha, 32);

        // Key difference: should cache weights for inference speed
        assert!(config.cache_dequantized);

        // Should still use BF16 compute
        assert!(matches!(
            config.quantization.compute_dtype,
            ComputeDType::BF16
        ));
    }

    #[test]
    fn test_is_target() {
        let config = QLoraConfig::preset_all_bf16(8, 16);

        // Should match target modules
        assert!(config.is_target("model.layer.q_proj"));
        assert!(config.is_target("transformer.blocks.0.attn.v_proj"));
        assert!(config.is_target("gate_proj"));

        // Should NOT match non-target modules
        assert!(!config.is_target("embed_tokens"));
        assert!(!config.is_target("lm_head"));
        assert!(!config.is_target("layer_norm"));
    }

    #[test]
    fn test_scale() {
        let config = QLoraConfig::preset_all_bf16(64, 16);
        let scale = config.scale();

        // scale = alpha / r = 16 / 64 = 0.25
        assert!((scale - 0.25).abs() < 1e-10);

        let config2 = QLoraConfig::preset_all_bf16(8, 32);
        let scale2 = config2.scale();

        // scale = alpha / r = 32 / 8 = 4.0
        assert!((scale2 - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_validate_for_training_success() {
        let config = QLoraConfig::preset_all_bf16(8, 16);
        assert!(config.validate_for_training().is_ok());
    }

    #[test]
    fn test_validate_for_training_zero_rank() {
        let mut config = QLoraConfig::preset_all_bf16(0, 16);
        config.lora.r = 0;

        let result = config.validate_for_training();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("rank"));
        }
    }

    #[test]
    fn test_validate_for_training_empty_targets() {
        let mut config = QLoraConfig::preset_all_bf16(8, 16);
        config.target_modules.clear();

        let result = config.validate_for_training();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("target module"));
        }
    }

    #[test]
    fn test_default_config() {
        let config = QLoraConfig::default();

        // Should use BF16 by default for training stability
        assert!(matches!(
            config.quantization.compute_dtype,
            ComputeDType::BF16
        ));

        // Should have standard LoRA defaults
        assert_eq!(config.lora.r, 64);
        assert_eq!(config.lora.alpha, 16);

        // Should target all linear layers
        assert!(!config.target_modules.is_empty());

        // Should not cache by default (memory-optimal)
        assert!(!config.cache_dequantized);
    }

    #[test]
    fn test_lora_weights() {
        let config = QLoraConfig::preset_all_bf16(8, 16);
        let device = Device::Cpu;
        let layer = QuantizedLinear::new(64, 128, &config, &device).unwrap();

        let (a_weight, b_weight) = layer.lora_weights();

        // A: [r, in_features] = [8, 64]
        assert_eq!(a_weight.dims(), &[8, 64]);

        // B: [out_features, r] = [128, 8]
        assert_eq!(b_weight.dims(), &[128, 8]);
    }
}
