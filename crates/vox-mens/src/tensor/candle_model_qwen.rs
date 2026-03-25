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
        let q = self.q_proj.forward(x).map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        let k = self.k_proj.forward(x).map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        let v = self.v_proj.forward(x).map_err(|e| candle_core::Error::Msg(e.to_string()))?;

        // Reshape to [batch, heads, seq, head_dim] for attention
        let q = q.reshape((b, seq_len, self.n_heads, self.head_dim))?.transpose(1, 2)?;
        let k = k.reshape((b, seq_len, self.n_kv_heads, self.head_dim))?.transpose(1, 2)?;
        let v = v.reshape((b, seq_len, self.n_kv_heads, self.head_dim))?.transpose(1, 2)?;

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
            let y = y.transpose(1, 2)?.contiguous()?.reshape((b, seq_len, self.n_heads * self.head_dim))?;
            self.o_proj.forward(&y).map_err(|e| candle_core::Error::Msg(e.to_string()))
        } else {
            // Inference (single token): no mask needed
            let att = candle_nn::ops::softmax(&att, candle_core::D::Minus1)?;
            let y = att.matmul(&v.contiguous()?)?;
            let y = y.transpose(1, 2)?.contiguous()?.reshape((b, seq_len, self.n_heads * self.head_dim))?;
            self.o_proj.forward(&y).map_err(|e| candle_core::Error::Msg(e.to_string()))
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
            &self.gate_proj.forward(x).map_err(|e| candle_core::Error::Msg(e.to_string()))?,
        )?;
        let rhs = self.up_proj.forward(x).map_err(|e| candle_core::Error::Msg(e.to_string()))?;
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
        let h = self.self_attn.forward(&h, pos, self.inv_freq.as_ref(), kv_cache)?;
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
        let mut x = self.embed_tokens.index_select(&ids, 0)?.reshape((b, seq_len, d_model))?;

        // Decoder layers (full-sequence training: pos=0)
        for layer in &self.layers {
            x = layer.forward(&x, 0, None)?;
        }

        // Final norm → LM head
        let x = self.norm.forward(&x)?;
        self.lm_head.forward(&x).map_err(|e| candle_core::Error::Msg(e.to_string()))
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
        let mut x = self.embed_tokens.index_select(&ids, 0)?.reshape((b, seq_len, d_model))?;

        // Decoder layers
        for (i, layer) in self.layers.iter().enumerate() {
            x = layer.forward(&x, pos, Some(&mut cache.kv[i]))?;
        }

        // Final norm → LM head
        let x = self.norm.forward(&x)?;
        self.lm_head.forward(&x).map_err(|e| candle_core::Error::Msg(e.to_string()))
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
        Self { kv: Vec::with_capacity(n_layers) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                    assert!(val.is_infinite() && val < 0.0, "row={row} col={col} expected -inf got {val}");
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
}
