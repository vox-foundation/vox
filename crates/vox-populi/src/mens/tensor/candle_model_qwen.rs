//! Candle Qwen2 transformer block implementation (training + inference).
//!
//! Pairs with `qlora-rs` for LoRA-trainable NF4 projections.
//!
//! ## Key correctness properties
//!
//! - **Causal mask**: every forward call applies a strict lower-triangular mask so
//!   tokens cannot attend to future positions (autoregressive invariant).
//! - **GQA (Grouped Query Attention)**: K/V heads are independently configurable
//!   via `n_kv_heads`. When `n_kv_heads < n_heads`, K and V are repeat-interleaved
//!   via [`repeat_kv`] to match the query head count before the attention product.
//! - **RoPE**: applied when `inv_freq` is loaded from the safetensors shard.

use candle_core::{DType, Device, Result, Tensor};
use candle_nn::{Module, RmsNorm};
use qlora_rs::qlora::QuantizedLinear;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build a lower-triangular (causal) additive attention mask.
///
/// Shape: `[1, 1, seq_len, seq_len]`.  
/// Positions on/below the diagonal are `0.0` (attend freely); positions above
/// the diagonal are `f32::NEG_INFINITY` so softmax collapses them to zero.
fn causal_mask(seq_len: usize, device: &Device) -> Result<Tensor> {
    // Build as a flat f32 vector: 0.0 for lower-tri, -inf for upper-tri
    let mut data = vec![0.0f32; seq_len * seq_len];
    for row in 0..seq_len {
        for col in (row + 1)..seq_len {
            data[row * seq_len + col] = f32::NEG_INFINITY;
        }
    }
    Tensor::from_vec(data, (1, 1, seq_len, seq_len), device)
}

/// Repeat-interleave K/V tensors to match the query head count (GQA).
///
/// `x` has shape `[batch, n_kv_heads, seq, head_dim]`.
/// Returns `[batch, n_kv_heads * n_rep, seq, head_dim]`.
fn repeat_kv(x: &Tensor, n_rep: usize) -> Result<Tensor> {
    if n_rep == 1 {
        return Ok(x.clone());
    }
    let (b, n_kv, seq, hd) = x.dims4()?;
    x.unsqueeze(2)?
        .expand((b, n_kv, n_rep, seq, hd))?
        .reshape((b, n_kv * n_rep, seq, hd))
}

/// Apply rotary position embeddings to `x` offset starting at `pos`.
fn rotate_half(x: &Tensor) -> Result<Tensor> {
    let last_dim = x.dim(candle_core::D::Minus1)?;
    let x1 = x.narrow(candle_core::D::Minus1, 0, last_dim / 2)?;
    let x2 = x.narrow(candle_core::D::Minus1, last_dim / 2, last_dim / 2)?;
    Tensor::cat(&[&x2.neg()?, &x1], candle_core::D::Minus1)
}

// ── Attention ─────────────────────────────────────────────────────────────────

/// Multi-head (or grouped-query) self-attention with causal masking and RoPE.
pub struct Qwen2Attention {
    /// Query projection — NF4 quantized + trainable LoRA.
    pub q_proj: QuantizedLinear,
    /// Key projection — NF4 quantized + trainable LoRA.
    pub k_proj: QuantizedLinear,
    /// Value projection — NF4 quantized + trainable LoRA.
    pub v_proj: QuantizedLinear,
    /// Output projection — NF4 quantized + trainable LoRA.
    pub o_proj: QuantizedLinear,
    /// Total query heads.
    pub n_heads: usize,
    /// Key/value head count (≤ `n_heads`; = `n_heads` for MHA, < for GQA).
    pub n_kv_heads: usize,
    /// Head dimension (`d_model / n_heads`).
    pub head_dim: usize,
}

impl Qwen2Attention {
    /// Causal masked self-attention forward with optional KV cache.
    ///
    /// `pos`: position offset for RoPE.
    /// `kv_cache`: optional mutable KV cache for this layer.
    pub fn forward(
        &self,
        x: &Tensor,
        pos: usize,
        inv_freq: Option<&Tensor>,
        kv_cache: Option<&mut (Tensor, Tensor)>,
    ) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        let device = x.device();

        // Linear projections → [batch, seq, heads * head_dim]
        let q = self
            .q_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        let k = self
            .k_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        let v = self
            .v_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;

        // Reshape to [batch, heads, seq, head_dim] for attention
        let q = q
            .reshape((b, seq_len, self.n_heads, self.head_dim))?
            .transpose(1, 2)?;
        let k = k
            .reshape((b, seq_len, self.n_kv_heads, self.head_dim))?
            .transpose(1, 2)?;
        let v = v
            .reshape((b, seq_len, self.n_kv_heads, self.head_dim))?
            .transpose(1, 2)?;

        // Apply RoPE if frequency table provided
        let (q, k) = if let Some(inv_freq) = inv_freq {
            self.apply_rotary_emb(&q, &k, inv_freq, pos)?
        } else {
            (q, k)
        };

        // Cache update
        let (k, v) = if let Some((k_prev, v_prev)) = kv_cache {
            let k = Tensor::cat(&[&*k_prev, &k], 2)?;
            let v = Tensor::cat(&[&*v_prev, &v], 2)?;
            *k_prev = k.clone();
            *v_prev = v.clone();
            (k, v)
        } else {
            (k, v)
        };

        // GQA: expand K/V to match query head count
        let n_rep = self.n_heads / self.n_kv_heads;
        let k = repeat_kv(&k, n_rep)?;
        let v = repeat_kv(&v, n_rep)?;

        // Scaled dot-product: [batch, n_heads, seq, seq]
        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let att = (q.contiguous()?.matmul(&k.transpose(2, 3)?.contiguous()?)? * scale)?;

        // --- Causal mask ---
        if seq_len > 1 {
            let mask = causal_mask(seq_len, device)?;
            let att = att.broadcast_add(&mask)?;
            let att = candle_nn::ops::softmax(&att, candle_core::D::Minus1)?;
            let y = att.matmul(&v.contiguous()?)?;
            let y = y.transpose(1, 2)?.contiguous()?.reshape((
                b,
                seq_len,
                self.n_heads * self.head_dim,
            ))?;
            self.o_proj
                .forward(&y)
                .map_err(|e| candle_core::Error::Msg(e.to_string()))
        } else {
            // Inference (single token): no mask needed
            let att = candle_nn::ops::softmax(&att, candle_core::D::Minus1)?;
            let y = att.matmul(&v.contiguous()?)?;
            let y = y.transpose(1, 2)?.contiguous()?.reshape((
                b,
                seq_len,
                self.n_heads * self.head_dim,
            ))?;
            self.o_proj
                .forward(&y)
                .map_err(|e| candle_core::Error::Msg(e.to_string()))
        }
    }

    fn apply_rotary_emb(
        &self,
        q: &Tensor,
        k: &Tensor,
        inv_freq: &Tensor,
        pos: usize,
    ) -> Result<(Tensor, Tensor)> {
        let (_b, _n_heads, seq_len, head_dim) = q.dims4()?;
        let device = q.device();
        let t = Tensor::arange(pos as u32, (pos + seq_len) as u32, device)?
            .to_dtype(DType::F32)?
            .reshape((seq_len, 1))?;
        let freqs = t.matmul(&inv_freq.reshape((1, inv_freq.elem_count()))?)?;
        let freqs = Tensor::cat(&[&freqs, &freqs], 1)?; // [seq, head_dim]
        let cos = freqs.cos()?.reshape((1, 1, seq_len, head_dim))?;
        let sin = freqs.sin()?.reshape((1, 1, seq_len, head_dim))?;
        let q_embed = (q.broadcast_mul(&cos)? + rotate_half(q)?.broadcast_mul(&sin)?)?;
        let k_embed = (k.broadcast_mul(&cos)? + rotate_half(k)?.broadcast_mul(&sin)?)?;
        Ok((q_embed, k_embed))
    }
}

// ── MLP ───────────────────────────────────────────────────────────────────────

/// SwiGLU feed-forward network: `down(silu(gate(x)) * up(x))`.
pub struct Qwen2MLP {
    /// Gate projection — NF4 quantized + trainable LoRA.
    pub gate_proj: QuantizedLinear,
    /// Up projection — NF4 quantized + trainable LoRA.
    pub up_proj: QuantizedLinear,
    /// Down projection — NF4 quantized + trainable LoRA.
    pub down_proj: QuantizedLinear,
}

impl Qwen2MLP {
    /// SwiGLU forward.
    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let lhs = candle_nn::ops::silu(
            &self
                .gate_proj
                .forward(x)
                .map_err(|e| candle_core::Error::Msg(e.to_string()))?,
        )?;
        let rhs = self
            .up_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        self.down_proj
            .forward(&(lhs * rhs)?)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }
}

// ── Transformer layer ─────────────────────────────────────────────────────────

/// A single Qwen2 decoder layer: `x + attn(rms1(x)) + mlp(rms2(x + attn(rms1(x))))`.
pub struct Qwen2Layer {
    /// Pre-attention RMSNorm.
    pub input_layernorm: RmsNorm,
    /// Multi-head / grouped-query self-attention.
    pub self_attn: Qwen2Attention,
    /// Pre-MLP RMSNorm.
    pub post_attention_layernorm: RmsNorm,
    /// SwiGLU MLP.
    pub mlp: Qwen2MLP,
    /// Rotary frequency table loaded from the model shard (optional for models without it).
    pub inv_freq: Option<Tensor>,
}

impl Qwen2Layer {
    /// Full pre-norm decoder block forward with optional KV cache.
    pub fn forward(
        &self,
        x: &Tensor,
        pos: usize,
        kv_cache: Option<&mut (Tensor, Tensor)>,
    ) -> Result<Tensor> {
        // Attention sub-layer with residual
        let residual = x;
        let h = self.input_layernorm.forward(x)?;
        let h = self
            .self_attn
            .forward(&h, pos, self.inv_freq.as_ref(), kv_cache)?;
        let x = (residual + h)?;

        // MLP sub-layer with residual
        let residual = &x;
        let h = self.post_attention_layernorm.forward(&x)?;
        let h = self.mlp.forward(&h)?;
        residual + h
    }
}

// ── Full Model ────────────────────────────────────────────────────────────────

/// Full Qwen2 decoder model: embeddings → N layers → final norm → LM head.
pub struct Qwen2Model {
    /// Frozen mmap'd word embeddings (`model.embed_tokens.weight`).
    pub embed_tokens: Tensor,
    /// Decoder layers.
    pub layers: Vec<Qwen2Layer>,
    /// Final RMSNorm before LM head.
    pub norm: RmsNorm,
    /// Language model head — NF4 quantized + trainable LoRA.
    pub lm_head: QuantizedLinear,
}

impl Qwen2Model {
    /// Full causal language model forward (training).
    ///
    /// Returns logits of shape `[batch, seq_len, vocab_size]`.
    pub fn forward(&self, input_ids: &Tensor) -> Result<Tensor> {
        let (b, seq_len) = input_ids.dims2()?;
        let d_model = self.embed_tokens.dim(1)?;

        // Token embeddings
        let ids = input_ids.flatten_all()?;
        let mut x = self
            .embed_tokens
            .index_select(&ids, 0)?
            .reshape((b, seq_len, d_model))?;

        // Decoder layers (full-sequence training: pos=0)
        for layer in &self.layers {
            x = layer.forward(&x, 0, None)?;
        }

        // Final norm → LM head
        let x = self.norm.forward(&x)?;
        self.lm_head
            .forward(&x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }

    /// Full causal language model forward with KV cache (inference).
    pub fn forward_with_cache(
        &self,
        input_ids: &Tensor,
        pos: usize,
        cache: &mut ForwardCache,
    ) -> Result<Tensor> {
        let (b, seq_len) = input_ids.dims2()?;
        let d_model = self.embed_tokens.dim(1)?;

        // Token embeddings
        let ids = input_ids.flatten_all()?;
        let mut x = self
            .embed_tokens
            .index_select(&ids, 0)?
            .reshape((b, seq_len, d_model))?;

        // Decoder layers
        for (i, layer) in self.layers.iter().enumerate() {
            x = layer.forward(&x, pos, Some(&mut cache.kv[i]))?;
        }

        // Final norm → LM head
        let x = self.norm.forward(&x)?;
        self.lm_head
            .forward(&x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }
}

// ── Qwen3.5 Hybrid attention/model ───────────────────────────────────────────

/// qwen3_5 linear-attention block using combined QKV projection.
pub struct Qwen35LinearAttention {
    /// Combined QKV projection (`key_dim + key_dim + value_dim`, d_model).
    pub qkv_proj: QuantizedLinear,
    /// Gating projection (`in_proj_z`).
    pub z_proj: QuantizedLinear,
    /// Beta projection (`in_proj_b`).
    pub b_proj: QuantizedLinear,
    /// A/dt projection (`in_proj_a`).
    pub a_proj: QuantizedLinear,
    /// Output projection.
    pub out_proj: QuantizedLinear,
    /// Depthwise causal conv weights from `linear_attn.conv1d.weight`, shape [conv_dim, kernel].
    pub conv_weight: Tensor,
    /// Per-head delta-rule parameters from HF weights.
    pub dt_bias: Tensor,
    pub a_log: Tensor,
    /// RMSNorm-gated output stage.
    pub norm: RmsNorm,
    pub num_k_heads: usize,
    pub num_v_heads: usize,
    pub head_k_dim: usize,
    pub head_v_dim: usize,
}

impl Qwen35LinearAttention {
    fn repeat_heads_bshd(x: &Tensor, n_rep: usize) -> Result<Tensor> {
        if n_rep == 1 {
            return Ok(x.clone());
        }
        let (b, s, h, d) = x.dims4()?;
        x.unsqueeze(3)?
            .expand((b, s, h, n_rep, d))?
            .reshape((b, s, h * n_rep, d))
    }

    fn l2norm_last(x: &Tensor, eps: f64) -> Result<Tensor> {
        let d = x.dim(candle_core::D::Minus1)?;
        let sq = x.broadcast_mul(x)?;
        let sq = sq.sum_keepdim(candle_core::D::Minus1)?;
        let inv = (sq / (d as f64))?.broadcast_add(&Tensor::new(eps as f32, x.device())?)?;
        let inv = inv.sqrt()?.recip()?;
        x.broadcast_mul(&inv)
    }

    /// Depthwise causal conv + SiLU over `[batch, seq, channels]`.
    fn causal_depthwise_conv_silu(x: &Tensor, conv_weight: &Tensor) -> Result<Tensor> {
        let (b, s, c) = x.dims3()?;
        let k = conv_weight.dim(1)?;
        let dev = x.device();
        let mut steps = Vec::with_capacity(s);
        for t in 0..s {
            let mut acc = Tensor::zeros((b, c), DType::F32, dev)?;
            for j in 0..k {
                if t < j {
                    continue;
                }
                let x_t = x.narrow(1, t - j, 1)?.squeeze(1)?;
                let w = conv_weight.narrow(1, j, 1)?.squeeze(1)?;
                let prod = x_t.broadcast_mul(&w.unsqueeze(0)?)?;
                acc = (acc + prod)?;
            }
            steps.push(candle_nn::ops::silu(&acc)?);
        }
        Tensor::stack(&steps, 1)
    }

    pub fn forward(
        &self,
        x: &Tensor,
        pos: usize,
        inv_freq: Option<&Tensor>,
        state_cache: Option<&mut Tensor>,
    ) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        let device = x.device();
        let qkv = self
            .qkv_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        let mixed_qkv = Self::causal_depthwise_conv_silu(&qkv, &self.conv_weight)?;

        let key_dim = self.num_k_heads * self.head_k_dim;
        let value_dim = self.num_v_heads * self.head_v_dim;
        let expected_total = key_dim + key_dim + value_dim;
        let got_total = mixed_qkv.dim(candle_core::D::Minus1)?;
        if got_total != expected_total {
            return Err(candle_core::Error::Msg(format!(
                "qwen3_5 linear_attention qkv dim mismatch: expected {expected_total}, got {got_total}",
            )));
        }

        let query = mixed_qkv
            .narrow(candle_core::D::Minus1, 0, key_dim)?
            .reshape((b, seq_len, self.num_k_heads, self.head_k_dim))?;
        let key = mixed_qkv
            .narrow(candle_core::D::Minus1, key_dim, key_dim)?
            .reshape((b, seq_len, self.num_k_heads, self.head_k_dim))?;
        let value = mixed_qkv
            .narrow(candle_core::D::Minus1, key_dim + key_dim, value_dim)?
            .reshape((b, seq_len, self.num_v_heads, self.head_v_dim))?;

        let z = self
            .z_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?
            .reshape((b, seq_len, self.num_v_heads, self.head_v_dim))?;
        let beta = candle_nn::ops::sigmoid(
            &self
                .b_proj
                .forward(x)
                .map_err(|e| candle_core::Error::Msg(e.to_string()))?,
        )?
        .reshape((b, seq_len, self.num_v_heads))?;
        let a = self
            .a_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?
            .reshape((b, seq_len, self.num_v_heads))?;

        let a_log = self.a_log.to_dtype(DType::F32)?;
        let dt_bias = self.dt_bias.to_dtype(DType::F32)?;
        let g_pre = (a.broadcast_add(&dt_bias.reshape((1, 1, self.num_v_heads))?)?).to_dtype(DType::F32)?;
        let g_soft = (g_pre.exp()?.broadcast_add(&Tensor::new(1f32, device)?)?).log()?;
        let g = g_soft
            .broadcast_mul(&a_log.exp()?.reshape((1, 1, self.num_v_heads))?)?
            .neg()?;

        let mut query = Self::l2norm_last(&query, 1e-6)?;
        let mut key = Self::l2norm_last(&key, 1e-6)?;
        if self.num_v_heads > self.num_k_heads {
            let rep = self.num_v_heads / self.num_k_heads;
            query = Self::repeat_heads_bshd(&query, rep)?;
            key = Self::repeat_heads_bshd(&key, rep)?;
        }

        let mut state = if let Some(state_prev) = state_cache.as_ref() {
            (**state_prev).clone()
        } else {
            Tensor::zeros(
                (b, self.num_v_heads, self.head_k_dim, self.head_v_dim),
                DType::F32,
                device,
            )?
        };
        let mut outs = Vec::with_capacity(seq_len);
        for t in 0..seq_len {
            let q_t = query.narrow(1, t, 1)?.squeeze(1)?;
            let k_t = key.narrow(1, t, 1)?.squeeze(1)?;
            let v_t = value.narrow(1, t, 1)?.squeeze(1)?;
            let g_t = g.narrow(1, t, 1)?.squeeze(1)?;
            let beta_t = beta.narrow(1, t, 1)?.squeeze(1)?;

            let g_scale = g_t.exp()?.reshape((b, self.num_v_heads, 1, 1))?;
            state = state.broadcast_mul(&g_scale)?;

            let k_col = k_t.unsqueeze(candle_core::D::Minus1)?;
            let kv_mem = state
                .transpose(2, 3)?
                .contiguous()?
                .matmul(&k_col)?
                .squeeze(candle_core::D::Minus1)?;
            let delta = v_t
                .broadcast_sub(&kv_mem)?
                .broadcast_mul(&beta_t.unsqueeze(candle_core::D::Minus1)?)?;
            let delta_row = delta.unsqueeze(2)?;
            let upd = k_col.matmul(&delta_row)?;
            state = (state + upd)?;

            let out_t = state
                .transpose(2, 3)?
                .contiguous()?
                .matmul(&q_t.unsqueeze(candle_core::D::Minus1)?)?
                .squeeze(candle_core::D::Minus1)?;
            outs.push(out_t);
        }

        if let Some(state_prev) = state_cache {
            *state_prev = state.clone();
        }

        let mut y = Tensor::stack(&outs, 1)?;
        if let Some(inv_freq) = inv_freq {
            // Native path: apply rotary to q/k at load-time semantics; we preserve positional offset
            // threading here for future decode/cached calls.
            let _ = (inv_freq, pos);
        }
        let y_flat = y.reshape((b * seq_len * self.num_v_heads, self.head_v_dim))?;
        let z_flat = z.reshape((b * seq_len * self.num_v_heads, self.head_v_dim))?;
        let y_norm = self.norm.forward(&y_flat)?;
        let y_gate = y_norm.broadcast_mul(&candle_nn::ops::silu(&z_flat)?)?;
        y = y_gate.reshape((b, seq_len, value_dim))?;

        self.out_proj
            .forward(&y)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }
}

pub enum Qwen35AttentionBlock {
    Full(Qwen2Attention),
    Linear(Qwen35LinearAttention),
}

pub struct Qwen35Layer {
    pub input_layernorm: RmsNorm,
    pub attention: Qwen35AttentionBlock,
    pub post_attention_layernorm: RmsNorm,
    pub mlp: Qwen2MLP,
    pub inv_freq: Option<Tensor>,
}

impl Qwen35Layer {
    pub fn forward(
        &self,
        x: &Tensor,
        pos: usize,
        kv_cache: Option<&mut Qwen35LayerCache>,
    ) -> Result<Tensor> {
        let residual = x;
        let h = self.input_layernorm.forward(x)?;
        let h = match &self.attention {
            Qwen35AttentionBlock::Full(a) => {
                let cache = match kv_cache {
                    Some(Qwen35LayerCache::Full(kv)) => Some(kv),
                    Some(Qwen35LayerCache::Linear(_)) => {
                        return Err(candle_core::Error::Msg(
                            "qwen3_5 cache mismatch: full-attention layer received linear cache"
                                .to_string(),
                        ));
                    }
                    None => None,
                };
                a.forward(&h, pos, self.inv_freq.as_ref(), cache)?
            }
            Qwen35AttentionBlock::Linear(a) => {
                let cache = match kv_cache {
                    Some(Qwen35LayerCache::Linear(state)) => Some(state),
                    Some(Qwen35LayerCache::Full(_)) => {
                        return Err(candle_core::Error::Msg(
                            "qwen3_5 cache mismatch: linear-attention layer received KV cache"
                                .to_string(),
                        ));
                    }
                    None => None,
                };
                a.forward(&h, pos, self.inv_freq.as_ref(), cache)?
            }
        };
        let x = (residual + h)?;

        let residual = &x;
        let h = self.post_attention_layernorm.forward(&x)?;
        let h = self.mlp.forward(&h)?;
        residual + h
    }
}

pub struct Qwen35Model {
    pub embed_tokens: Tensor,
    pub layers: Vec<Qwen35Layer>,
    pub norm: RmsNorm,
    pub lm_head: QuantizedLinear,
}

impl Qwen35Model {
    pub fn forward(&self, input_ids: &Tensor) -> Result<Tensor> {
        let (b, seq_len) = input_ids.dims2()?;
        let d_model = self.embed_tokens.dim(1)?;
        let ids = input_ids.flatten_all()?;
        let mut x = self
            .embed_tokens
            .index_select(&ids, 0)?
            .reshape((b, seq_len, d_model))?;

        for layer in &self.layers {
            x = layer.forward(&x, 0, None)?;
        }
        let x = self.norm.forward(&x)?;
        self.lm_head
            .forward(&x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }

    pub fn forward_with_cache(
        &self,
        input_ids: &Tensor,
        pos: usize,
        cache: &mut Qwen35ForwardCache,
    ) -> Result<Tensor> {
        let (b, seq_len) = input_ids.dims2()?;
        let d_model = self.embed_tokens.dim(1)?;
        let ids = input_ids.flatten_all()?;
        let mut x = self
            .embed_tokens
            .index_select(&ids, 0)?
            .reshape((b, seq_len, d_model))?;

        for (i, layer) in self.layers.iter().enumerate() {
            let slot = cache.layers.get_mut(i).ok_or_else(|| {
                candle_core::Error::Msg(format!("qwen3_5 cache missing slot for layer {i}"))
            })?;
            if slot.is_none() {
                *slot = Some(match &layer.attention {
                    Qwen35AttentionBlock::Full(a) => Qwen35LayerCache::Full((
                        Tensor::zeros((b, a.n_kv_heads, 0, a.head_dim), DType::F32, x.device())?,
                        Tensor::zeros((b, a.n_kv_heads, 0, a.head_dim), DType::F32, x.device())?,
                    )),
                    Qwen35AttentionBlock::Linear(a) => Qwen35LayerCache::Linear(Tensor::zeros(
                        (b, a.num_v_heads, a.head_k_dim, a.head_v_dim),
                        DType::F32,
                        x.device(),
                    )?),
                });
            }
            x = layer.forward(&x, pos, slot.as_mut())?;
        }
        let x = self.norm.forward(&x)?;
        self.lm_head
            .forward(&x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }
}

pub enum Qwen35LayerCache {
    Full((Tensor, Tensor)),
    Linear(Tensor),
}

pub struct Qwen35ForwardCache {
    pub layers: Vec<Option<Qwen35LayerCache>>,
}

impl Qwen35ForwardCache {
    pub fn new(n_layers: usize) -> Self {
        let mut layers = Vec::with_capacity(n_layers);
        layers.resize_with(n_layers, || None);
        Self { layers }
    }
}

/// KV cache for efficient autoregressive generation.
pub struct ForwardCache {
    /// Vector of (Key, Value) status tensors per layer.
    pub kv: Vec<(Tensor, Tensor)>,
}

impl ForwardCache {
    /// Initialize an empty cache for N layers.
    pub fn new(n_layers: usize) -> Self {
        Self {
            kv: Vec::with_capacity(n_layers),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qlora_rs::qlora::QLoraConfig;

    /// Verify causal_mask has −∞ above diagonal and 0 on/below.
    #[test]
    fn causal_mask_shape_and_values() {
        let device = Device::Cpu;
        let mask = causal_mask(4, &device).expect("causal_mask");
        assert_eq!(mask.dims(), &[1, 1, 4, 4]);
        let flat: Vec<f32> = mask.flatten_all().unwrap().to_vec1().unwrap();
        // lower-triangle (including diagonal) → 0
        // upper-triangle → -inf
        for row in 0..4usize {
            for col in 0..4usize {
                let val = flat[row * 4 + col];
                if col <= row {
                    assert_eq!(val, 0.0, "row={row} col={col} expected 0");
                } else {
                    assert!(
                        val.is_infinite() && val < 0.0,
                        "row={row} col={col} expected -inf got {val}"
                    );
                }
            }
        }
    }

    /// Verify repeat_kv doubles the head dim when n_rep=2.
    #[test]
    fn repeat_kv_doubles_heads() {
        let device = Device::Cpu;
        let t = Tensor::zeros((1, 2, 4, 8), DType::F32, &device).unwrap();
        let r = repeat_kv(&t, 2).unwrap();
        assert_eq!(r.dims(), &[1, 4, 4, 8]);
    }

    /// Verify repeat_kv is a no-op when n_rep=1.
    #[test]
    fn repeat_kv_noop_when_one() {
        let device = Device::Cpu;
        let t = Tensor::zeros((2, 3, 5, 7), DType::F32, &device).unwrap();
        let r = repeat_kv(&t, 1).unwrap();
        assert_eq!(r.dims(), t.dims());
    }

    #[test]
    fn qwen35_linear_attention_forward_and_cache_progression() {
        let device = Device::Cpu;
        let qcfg = QLoraConfig::default();
        let d_model = 64usize;
        let key_heads = 2usize;
        let value_heads = 4usize;
        let head_k_dim = 8usize;
        let head_v_dim = 8usize;
        let key_dim = key_heads * head_k_dim;
        let value_dim = value_heads * head_v_dim;
        let qkv_rows = key_dim + key_dim + value_dim;

        let w_qkv = Tensor::randn(0f32, 0.01f32, (qkv_rows, d_model), &device).unwrap();
        let w_z = Tensor::randn(0f32, 0.01f32, (value_dim, d_model), &device).unwrap();
        let w_b = Tensor::randn(0f32, 0.01f32, (value_heads, d_model), &device).unwrap();
        let w_a = Tensor::randn(0f32, 0.01f32, (value_heads, d_model), &device).unwrap();
        let w_o = Tensor::randn(0f32, 0.01f32, (d_model, value_dim), &device).unwrap();
        let conv = Tensor::randn(0f32, 0.01f32, (qkv_rows, 4usize), &device).unwrap();
        let dt_bias = Tensor::zeros((value_heads,), DType::F32, &device).unwrap();
        let a_log = Tensor::zeros((value_heads,), DType::F32, &device).unwrap();
        let norm_w = Tensor::ones((head_v_dim,), DType::F32, &device).unwrap();

        let attn = Qwen35LinearAttention {
            qkv_proj: QuantizedLinear::from_weight(&w_qkv, None, &qcfg, &device).unwrap(),
            z_proj: QuantizedLinear::from_weight(&w_z, None, &qcfg, &device).unwrap(),
            b_proj: QuantizedLinear::from_weight(&w_b, None, &qcfg, &device).unwrap(),
            a_proj: QuantizedLinear::from_weight(&w_a, None, &qcfg, &device).unwrap(),
            out_proj: QuantizedLinear::from_weight(&w_o, None, &qcfg, &device).unwrap(),
            conv_weight: conv,
            dt_bias,
            a_log,
            norm: RmsNorm::new(norm_w, 1e-6),
            num_k_heads: key_heads,
            num_v_heads: value_heads,
            head_k_dim,
            head_v_dim,
        };

        let x_full = Tensor::randn(0f32, 1f32, (1usize, 3usize, d_model), &device).unwrap();
        let y_full = attn.forward(&x_full, 0, None, None).unwrap();
        assert_eq!(y_full.dims(), &[1, 3, d_model]);

        let mut state = Tensor::zeros(
            (1usize, value_heads, head_k_dim, head_v_dim),
            DType::F32,
            &device,
        )
        .unwrap();
        let x_tok1 = x_full.narrow(1, 0, 1).unwrap();
        let x_tok2 = x_full.narrow(1, 1, 1).unwrap();
        let _ = attn.forward(&x_tok1, 0, None, Some(&mut state)).unwrap();
        let _ = attn.forward(&x_tok2, 1, None, Some(&mut state)).unwrap();
        assert_eq!(state.dims(), &[1, value_heads, head_k_dim, head_v_dim]);
    }
}
