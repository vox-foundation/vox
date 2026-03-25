// ─────────────────────────────────────────────────────────────────────────────
// GPU-only: Full LoRA transformer model (requires Burn autodiff backend)
// ─────────────────────────────────────────────────────────────────────────────

/// LoRA-adapted Multi-Head Attention layer.
///
/// Wraps query (q), key (k), and value (v) projections with LoRA adapters
/// while keeping the output projection (o) and RoPE (rotary positional embeddings)
/// as part of the attention computation.
///
/// **`merge()`**: only valid when **`use_rope` is `false`** (e.g. GPT-2 with learned
/// absolute positions). RoPE-augmented Q/K cannot be folded into static `Linear`
/// weights for Burn `MultiHeadAttention` without a dedicated RoPE-aware export.
#[cfg(feature = "mens-gpu")]
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
#[cfg(feature = "mens-gpu")]
#[derive(Clone, Debug)]
pub struct LoraAttentionKvCache<B: Backend> {
    /// Shape `[batch, n_heads, seq_len, d_head]`.
    pub k: Tensor<B, 4>,
    /// Shape `[batch, n_heads, seq_len, d_head]`.
    pub v: Tensor<B, 4>,
}

/// Per-layer attention KV cache for incremental decoding after prompt prefill.
#[cfg(feature = "mens-gpu")]
#[derive(Clone, Debug)]
pub struct LoraTransformerKvCache<B: Backend> {
    /// One K/V pair per transformer block, same order as `LoraVoxTransformer::blocks`.
    pub layers: Vec<LoraAttentionKvCache<B>>,
}

/// Collection of base linear layers for an attention block.
#[cfg(feature = "mens-gpu")]
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

#[cfg(feature = "mens-gpu")]
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
        let pos_ids = burn::tensor::Tensor::<B, 1, Int>::arange(0..seq_len as i64, device);
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
            let pos_ids = burn::tensor::Tensor::<B, 1, Int>::arange(0..seq_len as i64, &device);
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
            let pos_id =
                Tensor::<B, 1, Int>::from_ints(TensorData::new(vec![abs_pos as i32], [1]), &device);
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

    /// Merge LoRA into Q/K/V projections and return Burn `MultiHeadAttention` with the same
    /// merged `Linear` weights as [`LoraLinear::merge`].
    ///
    /// **RoPE:** Not supported — `use_rope` must be `false` (e.g. GPT-2 with learned positions).
    /// Merged MHA uses Burn's attention path; use [`super::burn_stack::TransformerBlock`] with
    /// [`MhaInput::mask_attn`](nn::attention::MhaInput::mask_attn) and
    /// [`generate_autoregressive_mask`](nn::attention::generate_autoregressive_mask) for causal LM.
    pub fn merge(&self) -> nn::attention::MultiHeadAttention<B> {
        assert!(
            !self.use_rope,
            "LoraAttention::merge requires use_rope=false (RoPE path cannot be represented as static Q/K/V linears)"
        );
        let device = self.out.devices()[0].clone();
        let mut mha = nn::attention::MultiHeadAttentionConfig::new(self.d_model, self.n_heads)
            .with_dropout(0.0)
            .with_min_float(-1e9f64)
            .init(&device);
        mha.query = self.q.merge();
        mha.key = self.k.merge();
        mha.value = self.v.merge();
        mha.output = self.out.clone();
        mha
    }
}

#[cfg(feature = "mens-gpu")]
impl<B: Backend> LoraLinear<B> {
    /// Merge LoRA weights into the base linear layer, returning a standard `nn::Linear`.
    ///
    /// The merged weight is: `W_merged = W_base + scale * (B @ A)`
    /// where A = [in, rank] and B = [rank, out].
    ///
    /// Uses `.val()` (not `.val_device()`) to stay on the current device.
    pub fn merge(&self) -> nn::Linear<B> {
        // Burn `linear()` uses `input.matmul(weight)` with weight `[d_input, d_output]`.
        // lora_a: [in_features, rank], lora_b: [rank, out_features]
        // delta: [in_features, rank] @ [rank, out_features] → [in_features, out_features]
        let a = self.lora_a.weight.val();
        let b = self.lora_b.weight.val();
        let delta_w = a.matmul(b) * self.scale as f64;
        let merged_w = self.base.weight.val() + delta_w;
        let mut merged = self.base.clone();
        merged.weight = burn::module::Param::from_tensor(merged_w);
        merged
    }
}

