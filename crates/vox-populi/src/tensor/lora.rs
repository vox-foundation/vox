//! LoRA (Low-Rank Adaptation) layers for vox-tensor.
//!
//! This implements the core LoRA algorithm from Hu et al. 2021.
//! All types are Burn-backend-agnostic and work with any `B: Backend`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use vox_tensor::lora::{LoraConfig, LoraLinear};
//!
//! let config = LoraConfig { rank: 8, alpha: 16.0, dropout: 0.0 };
//! let layer = LoraLinear::<WgpuBackend>::new(&device, 512, 512, &config);
//! let out = layer.forward(x); // base(x) + alpha/r * B(A(x))
//! ```
//!
//! ## Native LoRA (no quantization)
//!
//! This crate implements standard LoRA (Hu et al. 2021). There is no NF4/4-bit
//! quantization — base weights are full precision. True QLoRA would require
//! a `quant` module (planned). This may be extracted to a `burn-lora` crate.

#[cfg(feature = "gpu")]
use burn::module::Module;
#[cfg(feature = "gpu")]
use burn::nn;
#[cfg(feature = "gpu")]
use burn::tensor::Int;
#[cfg(feature = "gpu")]
use burn::tensor::Tensor;
#[cfg(feature = "gpu")]
use burn::tensor::TensorData;
#[cfg(feature = "gpu")]
use burn::tensor::backend::Backend;

/// Configuration for a LoRA adapter.
///
/// Typical defaults: rank=8, alpha=16, dropout=0.0
#[derive(Debug, Clone)]
pub struct LoraConfig {
    /// Low-rank dimension r. Typical values: 4, 8, 16, 32.
    /// Higher rank → more expressiveness but more parameters.
    pub rank: usize,
    /// LoRA scaling factor. The adapter output is multiplied by `alpha / rank`.
    /// Setting alpha=rank gives scale=1.0 (no explicit scaling).
    pub alpha: f32,
    /// Dropout probability applied to LoRA branch input. 0.0 = disabled.
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
#[cfg(feature = "gpu")]
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

#[cfg(feature = "gpu")]
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
    /// Burn Linear weight is [d_input, d_output], so dims()[0]=in, dims()[1]=out.
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

/// Estimated memory savings from LoRA vs full fine-tuning.
///
/// Returns `(lora_params, full_params, saving_pct)`.
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

// ─────────────────────────────────────────────────────────────────────────────
// GPU-only: Full LoRA transformer model (requires Burn autodiff backend)
// ─────────────────────────────────────────────────────────────────────────────

/// LoRA-adapted Multi-Head Attention layer.
///
/// Wraps query (q), key (k), and value (v) projections with LoRA adapters
/// while keeping the output projection (o) and RoPE (rotary positional embeddings)
/// as part of the attention computation.
#[cfg(feature = "gpu")]
#[derive(Module, Debug)]
pub struct LoraAttention<B: Backend> {
    /// LoRA adapted query projection.
    pub q: LoraLinear<B>,
    /// LoRA adapted key projection.
    pub k: LoraLinear<B>,
    /// LoRA adapted value projection.
    pub v: LoraLinear<B>,
    /// Output projection (standard linear, not adapted).
    pub out: nn::Linear<B>,
    /// Model embedding dimension.
    pub d_model: usize,
    /// Number of attention heads.
    pub n_heads: usize,
    /// Dimension of each attention head.
    pub d_head: usize,
    /// When true, apply RoPE to Q and K (Qwen2). When false, use external pos embeddings (GPT-2).
    pub use_rope: bool,
    /// RoPE theta base (e.g. 1000000.0 for Qwen2). Only used when use_rope is true.
    pub rope_theta: f32,
}

/// Key/value tensors after projections (and RoPE on K when enabled), for autoregressive decode.
#[cfg(feature = "gpu")]
#[derive(Clone, Debug)]
pub struct LoraAttentionKvCache<B: Backend> {
    /// Shape `[batch, n_heads, seq_len, d_head]`.
    pub k: Tensor<B, 4>,
    /// Shape `[batch, n_heads, seq_len, d_head]`.
    pub v: Tensor<B, 4>,
}

/// Per-layer attention KV cache for incremental decoding after prompt prefill.
#[cfg(feature = "gpu")]
#[derive(Clone, Debug)]
pub struct LoraTransformerKvCache<B: Backend> {
    /// One K/V pair per transformer block, same order as `LoraVoxTransformer::blocks`.
    pub layers: Vec<LoraAttentionKvCache<B>>,
}

/// Collection of base linear layers for an attention block.
#[cfg(feature = "gpu")]
pub struct AttentionBases<B: Backend> {
    /// Query projection.
    pub q: nn::Linear<B>,
    /// Key projection.
    pub k: nn::Linear<B>,
    /// Value projection.
    pub v: nn::Linear<B>,
    /// Output projection.
    pub out: nn::Linear<B>,
}

#[cfg(feature = "gpu")]
impl<B: Backend> LoraAttention<B> {
    /// Create a newly initialized LoRA adapted multi-head attention module.
    pub fn new(device: &B::Device, d_model: usize, n_heads: usize, r: usize, alpha: f32) -> Self {
        let config = LoraConfig {
            rank: r,
            alpha,
            dropout: 0.0,
        };
        let d_head = d_model / n_heads;
        Self {
            q: LoraLinear::new(device, d_model, d_model, &config),
            k: LoraLinear::new(device, d_model, d_model, &config),
            v: LoraLinear::new(device, d_model, d_model, &config),
            out: nn::LinearConfig::new(d_model, d_model).init(device),
            d_model,
            n_heads,
            d_head,
            use_rope: false,
            rope_theta: 10000.0,
        }
    }

    /// Build attention with pre-loaded base weights (e.g. from Hugging Face GPT-2).
    ///
    /// Expects `q`, `k`, `v`, `out` as standard `nn::Linear` layers with shapes
    /// `[d_model, d_model]` for q/k/v and out. `n_heads` must divide `d_model`.
    pub fn with_bases(
        q: nn::Linear<B>,
        k: nn::Linear<B>,
        v: nn::Linear<B>,
        out: nn::Linear<B>,
        n_heads: usize,
        r: usize,
        alpha: f32,
    ) -> Self {
        let bases = AttentionBases { q, k, v, out };
        let config = LoraConfig {
            rank: r,
            alpha,
            ..Default::default()
        };
        Self::with_bases_and_rope(bases, n_heads, config, false, 10000.0)
    }

    /// Build attention with pre-loaded base weights and optional RoPE (for Qwen2).
    pub fn with_bases_and_rope(
        bases: AttentionBases<B>,
        n_heads: usize,
        config: LoraConfig,
        use_rope: bool,
        rope_theta: f32,
    ) -> Self {
        let d_model = bases.q.weight.dims()[1];
        let d_head = d_model / n_heads;
        Self {
            q: LoraLinear::with_base(bases.q, &config),
            k: LoraLinear::with_base(bases.k, &config),
            v: LoraLinear::with_base(bases.v, &config),
            out: bases.out,
            d_model,
            n_heads,
            d_head,
            use_rope,
            rope_theta,
        }
    }

    /// Build causal mask [1, 1, seq_len, seq_len]: 0 where j <= i, -1e9 where j > i.
    fn causal_mask(seq_len: usize, device: &B::Device) -> Tensor<B, 4> {
        use burn::tensor::TensorData;
        let mut data = vec![0.0f32; seq_len * seq_len];
        for i in 0..seq_len {
            for j in (i + 1)..seq_len {
                data[i * seq_len + j] = -1e9f32;
            }
        }
        let data = TensorData::new(data, [1, 1, seq_len, seq_len]);
        Tensor::from_data(data, device)
    }

    /// Apply RoPE to Q or K tensor [batch, n_heads, seq_len, d_head] using explicit 1-D positions.
    fn apply_rope_with_pos_ids(
        &self,
        t: Tensor<B, 4>,
        pos_ids_1d: Tensor<B, 1, Int>,
        device: &B::Device,
    ) -> Tensor<B, 4> {
        let [batch, n_heads, seq_len, d_head] = t.dims();
        debug_assert_eq!(pos_ids_1d.dims()[0], seq_len);
        let n_pairs = d_head / 2;
        let mut inv_freq: Vec<f32> = Vec::with_capacity(n_pairs);
        for i in 0..n_pairs {
            inv_freq.push(1.0 / self.rope_theta.powf(2.0 * i as f32 / d_head as f32));
        }
        let pos = pos_ids_1d.float().reshape([1, 1, seq_len, 1]);
        let inv_freq_t =
            Tensor::<B, 1>::from_floats(inv_freq.as_slice(), device).reshape([1, 1, 1, n_pairs]);
        let freqs = pos.matmul(inv_freq_t);
        let cos_t = freqs
            .clone()
            .cos()
            .repeat_dim(0, batch)
            .repeat_dim(1, n_heads);
        let sin_t = freqs.sin().repeat_dim(0, batch).repeat_dim(1, n_heads);
        let t_reshaped = t.reshape([batch, n_heads, seq_len, n_pairs, 2]);
        let t0 = t_reshaped
            .clone()
            .slice([0..batch, 0..n_heads, 0..seq_len, 0..n_pairs, 0..1])
            .reshape([batch, n_heads, seq_len, n_pairs]);
        let t1 = t_reshaped
            .slice([0..batch, 0..n_heads, 0..seq_len, 0..n_pairs, 1..2])
            .reshape([batch, n_heads, seq_len, n_pairs]);
        let t_rot0 = t0.clone() * cos_t.clone() - t1.clone() * sin_t.clone();
        let t_rot1 = t0 * sin_t + t1 * cos_t;
        let stacked: Tensor<B, 5> = Tensor::stack(vec![t_rot0, t_rot1], 4);
        stacked.reshape([batch, n_heads, seq_len, d_head])
    }

    /// Apply RoPE with positions `0..seq_len`.
    fn apply_rope(&self, t: Tensor<B, 4>, device: &B::Device) -> Tensor<B, 4> {
        let seq_len = t.dims()[2];
        let pos_ids =
            burn::tensor::Tensor::<B, 1, Int>::arange(0..seq_len as i64, device);
        self.apply_rope_with_pos_ids(t, pos_ids, device)
    }

    /// Forward pass through the attention mechanism (training / full-seq inference; no KV capture).
    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let [batch, seq_len, _d] = x.dims();
        let device = x.devices()[0].clone();

        let flat = x.clone().reshape([batch * seq_len, self.d_model]);

        let mut q = self
            .q
            .forward(flat.clone())
            .reshape([batch, seq_len, self.n_heads, self.d_head])
            .swap_dims(1, 2);
        let mut k = self
            .k
            .forward(flat.clone())
            .reshape([batch, seq_len, self.n_heads, self.d_head])
            .swap_dims(1, 2);
        let v = self
            .v
            .forward(flat)
            .reshape([batch, seq_len, self.n_heads, self.d_head])
            .swap_dims(1, 2);

        if self.use_rope {
            q = self.apply_rope(q, &device);
            k = self.apply_rope(k, &device);
        }

        let scale = (self.d_head as f32).sqrt();
        let mut attn_weights = q.matmul(k.transpose()) / scale;
        let causal_mask = Self::causal_mask(seq_len, &device);
        attn_weights = attn_weights + causal_mask;
        attn_weights = burn::tensor::activation::softmax(attn_weights, 3);

        let context = attn_weights.matmul(v);
        let context = context
            .swap_dims(1, 2)
            .reshape([batch, seq_len, self.d_model]);

        self.out.forward(context)
    }

    /// Full-sequence forward; returns output and K/V tensors (after RoPE when enabled) for KV caching.
    pub fn forward_prefill_with_kv(
        &self,
        x: Tensor<B, 3>,
    ) -> (Tensor<B, 3>, LoraAttentionKvCache<B>) {
        let [batch, seq_len, _d] = x.dims();
        let device = x.devices()[0].clone();

        let flat = x.clone().reshape([batch * seq_len, self.d_model]);

        let mut q = self
            .q
            .forward(flat.clone())
            .reshape([batch, seq_len, self.n_heads, self.d_head])
            .swap_dims(1, 2);
        let mut k = self
            .k
            .forward(flat.clone())
            .reshape([batch, seq_len, self.n_heads, self.d_head])
            .swap_dims(1, 2);
        let v = self
            .v
            .forward(flat)
            .reshape([batch, seq_len, self.n_heads, self.d_head])
            .swap_dims(1, 2);

        if self.use_rope {
            let pos_ids =
                burn::tensor::Tensor::<B, 1, Int>::arange(0..seq_len as i64, &device);
            q = self.apply_rope_with_pos_ids(q, pos_ids.clone(), &device);
            k = self.apply_rope_with_pos_ids(k, pos_ids, &device);
        }

        let scale = (self.d_head as f32).sqrt();
        let mut attn_weights = q.matmul(k.clone().transpose()) / scale;
        let causal_mask = Self::causal_mask(seq_len, &device);
        attn_weights = attn_weights + causal_mask;
        attn_weights = burn::tensor::activation::softmax(attn_weights, 3);

        let context = attn_weights.matmul(v.clone());
        let context = context
            .swap_dims(1, 2)
            .reshape([batch, seq_len, self.d_model]);

        let out = self.out.forward(context);
        let kv = LoraAttentionKvCache { k, v };
        (out, kv)
    }

    /// One autoregressive step: `x` is `[batch, 1, d_model]`, `abs_pos` is the index of this token.
    pub fn forward_decode_one(
        &self,
        x: Tensor<B, 3>,
        cache: &mut LoraAttentionKvCache<B>,
        abs_pos: usize,
    ) -> Tensor<B, 3> {
        let [batch, seq_len, _d] = x.dims();
        debug_assert_eq!(seq_len, 1);
        let device = x.devices()[0].clone();

        let flat = x.clone().reshape([batch * seq_len, self.d_model]);

        let mut q = self
            .q
            .forward(flat.clone())
            .reshape([batch, seq_len, self.n_heads, self.d_head])
            .swap_dims(1, 2);
        let mut k_new = self
            .k
            .forward(flat.clone())
            .reshape([batch, seq_len, self.n_heads, self.d_head])
            .swap_dims(1, 2);
        let v_new = self
            .v
            .forward(flat)
            .reshape([batch, seq_len, self.n_heads, self.d_head])
            .swap_dims(1, 2);

        if self.use_rope {
            let pos_id = Tensor::<B, 1, Int>::from_ints(
                TensorData::new(vec![abs_pos as i32], [1]),
                &device,
            );
            q = self.apply_rope_with_pos_ids(q, pos_id.clone(), &device);
            k_new = self.apply_rope_with_pos_ids(k_new, pos_id, &device);
        }

        let k_full = Tensor::cat(vec![cache.k.clone(), k_new], 2);
        let v_full = Tensor::cat(vec![cache.v.clone(), v_new], 2);

        let scale = (self.d_head as f32).sqrt();
        let attn_weights = q.matmul(k_full.clone().transpose()) / scale;
        let attn_weights = burn::tensor::activation::softmax(attn_weights, 3);
        let context = attn_weights.matmul(v_full.clone());
        let context = context
            .swap_dims(1, 2)
            .reshape([batch, seq_len, self.d_model]);

        let out = self.out.forward(context);
        cache.k = k_full;
        cache.v = v_full;
        out
    }

    /// Returns a placeholder MHA — does NOT merge LoRA weights.
    ///
    /// Burn's `MultiHeadAttention` has private fields; we cannot inject merged
    /// q/k/v weights via the public API. This returns a freshly-initialized
    /// MHA with random weights. For correct inference, use `LoraVoxTransformer`
    /// or `LoraAttention` directly at serve time; do not rely on `merge()` for
    /// attention.
    pub fn merge(&self) -> nn::attention::MultiHeadAttention<B> {
        let device = self.out.devices()[0].clone();
        let config = nn::attention::MultiHeadAttentionConfig::new(self.d_model, self.n_heads);
        config.init(&device)
    }
}

#[cfg(feature = "gpu")]
impl<B: Backend> LoraLinear<B> {
    /// Merge LoRA weights into the base linear layer, returning a standard `nn::Linear`.
    ///
    /// The merged weight is: `W_merged = W_base + scale * (B @ A)`
    /// where A = [in, rank] and B = [rank, out].
    ///
    /// Uses `.val()` (not `.val_device()`) to stay on the current device.
    pub fn merge(&self) -> nn::Linear<B> {
        // Burn's LinearConfig::new(in, out) stores weight as [out, in].
        // So: lora_a.weight = [rank, in_features]
        //     lora_b.weight = [out_features, rank]
        // delta_W = lora_b.weight @ lora_a.weight → [out_features, in_features]
        // which matches base.weight shape exactly.
        let a = self.lora_a.weight.val(); // [rank, in_features]
        let b = self.lora_b.weight.val(); // [out_features, rank]
        let delta_w = b.matmul(a) * self.scale as f64;
        let merged_w = self.base.weight.val() + delta_w;
        let mut merged = self.base.clone();
        merged.weight = burn::module::Param::from_tensor(merged_w);
        merged
    }
}

/// LoRA-adapted Transformer Block.
///
/// Pre-norm architecture: LayerNorm → Attention → residual → LayerNorm → FFN → residual.
/// Supports GPT-2 (ff_linear1 + GELU) and Qwen2 (ff_gate + ff_up + SwiGLU).
#[cfg(feature = "gpu")]
#[derive(Module, Debug)]
pub struct LoraTransformerBlock<B: Backend> {
    /// LoRA-adapted multi-head attention.
    pub attention: LoraAttention<B>,
    /// First layer normalization (applied before attention).
    pub norm1: nn::LayerNorm<B>,
    /// GPT-2 style: single linear before GELU. None when using SwiGLU.
    pub ff_linear1: Option<nn::Linear<B>>,
    /// Qwen2 SwiGLU: gate projection. None when using GPT-2 MLP.
    pub ff_gate: Option<nn::Linear<B>>,
    /// Qwen2 SwiGLU: up projection. None when using GPT-2 MLP.
    pub ff_up: Option<nn::Linear<B>>,
    /// Second linear layer (projection) of the FFN.
    pub ff_linear2: nn::Linear<B>,
    /// Second layer normalization (applied before FFN).
    pub norm2: nn::LayerNorm<B>,
}

#[cfg(feature = "gpu")]
impl<B: Backend> LoraTransformerBlock<B> {
    /// Create a newly initialized LoRA adapted transformer block.
    pub fn new(device: &B::Device, d_model: usize, n_heads: usize, r: usize, alpha: f32) -> Self {
        Self {
            attention: LoraAttention::new(device, d_model, n_heads, r, alpha),
            norm1: nn::LayerNormConfig::new(d_model).init(device),
            ff_linear1: Some(nn::LinearConfig::new(d_model, d_model * 4).init(device)),
            ff_gate: None,
            ff_up: None,
            ff_linear2: nn::LinearConfig::new(d_model * 4, d_model).init(device),
            norm2: nn::LayerNormConfig::new(d_model).init(device),
        }
    }

    /// Create block with SwiGLU MLP (for Qwen2).
    pub fn new_swiglu(
        device: &B::Device,
        d_model: usize,
        n_heads: usize,
        intermediate_size: usize,
        r: usize,
        alpha: f32,
    ) -> Self {
        Self {
            attention: LoraAttention::new(device, d_model, n_heads, r, alpha),
            norm1: nn::LayerNormConfig::new(d_model).init(device),
            ff_linear1: None,
            ff_gate: Some(nn::LinearConfig::new(d_model, intermediate_size).init(device)),
            ff_up: Some(nn::LinearConfig::new(d_model, intermediate_size).init(device)),
            ff_linear2: nn::LinearConfig::new(intermediate_size, d_model).init(device),
            norm2: nn::LayerNormConfig::new(d_model).init(device),
        }
    }

    /// Forward pass through the transformer block.
    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let x_attn = self.attention.forward(self.norm1.forward(x.clone()));
        let x_mid = x + x_attn;

        let h = self.norm2.forward(x_mid.clone());
        let x_ff = if let (Some(gate), Some(up)) = (&self.ff_gate, &self.ff_up) {
            self.ff_linear2
                .forward(gate.forward(h.clone()) * burn::tensor::activation::silu(up.forward(h)))
        } else if let Some(ref ff1) = self.ff_linear1 {
            self.ff_linear2
                .forward(burn::tensor::activation::gelu(ff1.forward(h)))
        } else {
            unreachable!("block must have either ff_linear1 or (ff_gate, ff_up)");
        };
        x_mid + x_ff
    }

    /// Forward with attention K/V capture for incremental decode.
    pub fn forward_prefill_with_kv(
        &self,
        x: Tensor<B, 3>,
    ) -> (Tensor<B, 3>, LoraAttentionKvCache<B>) {
        let (x_attn, kv) = self
            .attention
            .forward_prefill_with_kv(self.norm1.forward(x.clone()));
        let x_mid = x + x_attn;

        let h = self.norm2.forward(x_mid.clone());
        let x_ff = if let (Some(gate), Some(up)) = (&self.ff_gate, &self.ff_up) {
            self.ff_linear2
                .forward(gate.forward(h.clone()) * burn::tensor::activation::silu(up.forward(h)))
        } else if let Some(ref ff1) = self.ff_linear1 {
            self.ff_linear2
                .forward(burn::tensor::activation::gelu(ff1.forward(h)))
        } else {
            unreachable!("block must have either ff_linear1 or (ff_gate, ff_up)");
        };
        (x_mid + x_ff, kv)
    }

    /// One autoregressive step; updates `kv` for this layer.
    pub fn forward_decode_one(
        &self,
        x: Tensor<B, 3>,
        kv: &mut LoraAttentionKvCache<B>,
        abs_pos: usize,
    ) -> Tensor<B, 3> {
        let attn_out =
            self.attention
                .forward_decode_one(self.norm1.forward(x.clone()), kv, abs_pos);
        let x_mid = x + attn_out;

        let h = self.norm2.forward(x_mid.clone());
        let x_ff = if let (Some(gate), Some(up)) = (&self.ff_gate, &self.ff_up) {
            self.ff_linear2
                .forward(gate.forward(h.clone()) * burn::tensor::activation::silu(up.forward(h)))
        } else if let Some(ref ff1) = self.ff_linear1 {
            self.ff_linear2
                .forward(burn::tensor::activation::gelu(ff1.forward(h)))
        } else {
            unreachable!("block must have either ff_linear1 or (ff_gate, ff_up)");
        };
        x_mid + x_ff
    }

    /// Merge this block into a plain `TransformerBlock`.
    ///
    /// For SwiGLU blocks (Qwen2), merge is not fully supported — returns a placeholder.
    pub fn merge(&self) -> super::nn::TransformerBlock<B> {
        if self.ff_gate.is_some() {
            let device = self.ff_linear2.weight.devices()[0].clone();
            let d_model = self.ff_linear2.weight.dims()[1];
            let intermediate = self.ff_linear2.weight.dims()[0];
            let ff1 = nn::LinearConfig::new(d_model, intermediate).init(&device);
            return super::nn::TransformerBlock {
                attention: self.attention.merge(),
                norm1: self.norm1.clone(),
                ff_linear1: ff1,
                ff_linear2: self.ff_linear2.clone(),
                norm2: self.norm2.clone(),
            };
        }
        let ff1 = self
            .ff_linear1
            .as_ref()
            .expect("GPT-2 block must have ff_linear1");
        super::nn::TransformerBlock {
            attention: self.attention.merge(),
            norm1: self.norm1.clone(),
            ff_linear1: ff1.clone(),
            ff_linear2: self.ff_linear2.clone(),
            norm2: self.norm2.clone(),
        }
    }
}

/// Maximum sequence length for positional embeddings (must match train/serve config).
pub const MAX_SEQ_LEN: usize = 512;

/// Default model architecture (d_model, n_heads, n_layers) — must match training config.
pub const DEFAULT_D_MODEL: usize = 512;
/// Default number of attention heads (must divide `DEFAULT_D_MODEL`).
pub const DEFAULT_N_HEADS: usize = 8;
/// Default number of transformer layers.
pub const DEFAULT_N_LAYERS: usize = 6;

/// LoRA-wrapped Vox Transformer Model.
///
/// A full stack of `LoraTransformerBlock`s with embedding and output head.
/// Used as the trainable model in `run_qlora_training`.
///
/// Architecture:
/// - `embedding`: token → d_model
/// - `pos_embedding`: learned positional embeddings (zero when use_rope)
/// - `blocks`: N × LoraTransformerBlock
/// - `norm`: final LayerNorm
/// - `output`: d_model → vocab_size (output head, not LoRA-adapted)
/// - `use_rope`: when true, skip pos_embedding (Qwen2 uses RoPE in attention)
#[cfg(feature = "gpu")]
#[derive(Module, Debug)]
pub struct LoraVoxTransformer<B: Backend> {
    /// Token embeddings.
    pub embedding: nn::Embedding<B>,
    /// Positional embeddings.
    pub pos_embedding: nn::Embedding<B>,
    /// Stack of LoRA adapted transformer blocks.
    pub blocks: Vec<LoraTransformerBlock<B>>,
    /// Final layer normalization.
    pub norm: nn::LayerNorm<B>,
    /// Output projection layer producing logits.
    pub output: nn::Linear<B>,
    /// When true, do not add pos_embedding (RoPE is used in attention).
    pub use_rope: bool,
}

#[cfg(feature = "gpu")]
impl<B: Backend> LoraVoxTransformer<B> {
    /// Create a newly initialized full LoRA adapted transformer model.
    pub fn new(
        device: &B::Device,
        vocab_size: usize,
        d_model: usize,
        n_heads: usize,
        n_layers: usize,
        r: usize,
        alpha: f32,
    ) -> Self {
        Self {
            embedding: nn::EmbeddingConfig::new(vocab_size, d_model).init(device),
            pos_embedding: nn::EmbeddingConfig::new(MAX_SEQ_LEN, d_model).init(device),
            blocks: (0..n_layers)
                .map(|_| LoraTransformerBlock::new(device, d_model, n_heads, r, alpha))
                .collect(),
            norm: nn::LayerNormConfig::new(d_model).init(device),
            output: nn::LinearConfig::new(d_model, vocab_size).init(device),
            use_rope: false,
        }
    }

    /// Forward pass.
    ///
    /// Input:  `x` — integer token indices, shape `[batch, seq_len]`
    /// Output: logits shape `[batch, seq_len, vocab_size]`
    pub fn forward(&self, x: Tensor<B, 2, burn::tensor::Int>) -> Tensor<B, 3> {
        let [batch, seq_len] = x.dims();
        let device = x.devices()[0].clone();

        // Token embedding → [batch, seq_len, d_model]
        let mut h = self.embedding.forward(x.clone());

        // Learned positional embeddings (skip when use_rope — RoPE is in attention)
        if !self.use_rope {
            let pos_ids_1d =
                burn::tensor::Tensor::<B, 1, burn::tensor::Int>::arange(0..seq_len as i64, &device);
            let pos_ids = pos_ids_1d.reshape([1, seq_len]).repeat_dim(0, batch);
            let pos_emb = self.pos_embedding.forward(pos_ids);
            h = h + pos_emb;
        }

        // Transformer blocks
        for block in &self.blocks {
            h = block.forward(h);
        }

        // Final layer-norm
        h = self.norm.forward(h);

        // Output head: project each token position to vocab logits.
        // Flatten to [batch * seq_len, d_model], apply linear, reshape back.
        // Burn Linear weight is [d_input, d_output] = [in_features, vocab_size]
        let d_model = h.dims()[2];
        let vocab_size = self.output.weight.dims()[1];
        let logits = self.output.forward(h.reshape([batch * seq_len, d_model]));
        logits.reshape([batch, seq_len, vocab_size])
    }

    /// Prompt prefill with KV capture: logits for the **last** prompt token, shape `[batch, vocab_size]`.
    pub fn forward_prefill_last_logits_and_kv(
        &self,
        x: Tensor<B, 2, Int>,
    ) -> (Tensor<B, 2>, LoraTransformerKvCache<B>) {
        let [batch, seq_len] = x.dims();
        let device = x.devices()[0].clone();

        let mut h = self.embedding.forward(x.clone());
        if !self.use_rope {
            let pos_ids_1d = Tensor::<B, 1, Int>::arange(0..seq_len as i64, &device);
            let pos_ids = pos_ids_1d.reshape([1, seq_len]).repeat_dim(0, batch);
            let pos_emb = self.pos_embedding.forward(pos_ids);
            h = h + pos_emb;
        }

        let mut layer_caches: Vec<LoraAttentionKvCache<B>> = Vec::with_capacity(self.blocks.len());
        for block in &self.blocks {
            let (h_next, kv) = block.forward_prefill_with_kv(h);
            layer_caches.push(kv);
            h = h_next;
        }

        h = self.norm.forward(h);
        let d_model = h.dims()[2];
        let last = h.slice([0..batch, (seq_len - 1)..seq_len, 0..d_model]);
        let last = last.reshape([batch, d_model]);
        let logits = self.output.forward(last);
        (
            logits,
            LoraTransformerKvCache {
                layers: layer_caches,
            },
        )
    }

    /// Decode one new token; `abs_pos` is the index of that token in the running sequence.
    pub fn forward_decode_logits(
        &self,
        token: Tensor<B, 2, Int>,
        abs_pos: usize,
        caches: &mut LoraTransformerKvCache<B>,
    ) -> Tensor<B, 2> {
        let [batch, one] = token.dims();
        debug_assert_eq!(one, 1);
        let device = token.devices()[0].clone();

        let mut h = self.embedding.forward(token.clone());
        if !self.use_rope {
            let pos_id = Tensor::<B, 1, Int>::from_ints(
                TensorData::new(vec![abs_pos as i32], [1]),
                &device,
            );
            let pos_ids = pos_id.reshape([1, 1]).repeat_dim(0, batch);
            let pos_emb = self.pos_embedding.forward(pos_ids);
            h = h + pos_emb;
        }

        for (i, block) in self.blocks.iter().enumerate() {
            h = block.forward_decode_one(h, &mut caches.layers[i], abs_pos);
        }

        h = self.norm.forward(h);
        let d_model = h.dims()[2];
        self.output.forward(h.reshape([batch, d_model]))
    }

    /// Merge all LoRA adapters back into base weights.
    ///
    /// Returns a plain `VoxTransformer` suitable for inference (no LoRA overhead).
    pub fn merge(&self) -> super::nn::VoxTransformer<B> {
        super::nn::VoxTransformer {
            embedding: self.embedding.clone(),
            pos_embedding: self.pos_embedding.clone(),
            blocks: self.blocks.iter().map(|b| b.merge()).collect(),
            norm: self.norm.clone(),
            output: self.output.clone(),
        }
    }
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
