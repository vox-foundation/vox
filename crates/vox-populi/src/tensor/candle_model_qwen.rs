//! Candle Qwen2 transformer block implementation (training + inference).
//!
//! Pairs with `qlora-rs` for LoRA-trainable NF4 projections.

use candle_core::{DType, Device, Module, Result, Tensor};
use candle_nn::{LayerNorm, RMSNorm, VarBuilder};
use qlora_rs::QLoraConfig;
use qlora_rs::qlora::QuantizedLinear;

pub struct Qwen2Attention {
    pub q_proj: QuantizedLinear,
    pub k_proj: QuantizedLinear,
    pub v_proj: QuantizedLinear,
    pub o_proj: QuantizedLinear,
    pub n_heads: usize,
    pub n_kv_heads: usize,
    pub head_dim: usize,
}

impl Qwen2Attention {
    pub fn forward(&self, x: &Tensor, pos: usize, inv_freq: Option<&Tensor>) -> Result<Tensor> {
        let (b, seq_len, d_model) = x.dims3()?;
        let q = self.q_proj.forward(x)?;
        let k = self.k_proj.forward(x)?;
        let v = self.v_proj.forward(x)?;

        let q = q.reshape((b, seq_len, self.n_heads, self.head_dim))?.transpose(1, 2)?;
        let k = k.reshape((b, seq_len, self.n_kv_heads, self.head_dim))?.transpose(1, 2)?;
        let v = v.reshape((b, seq_len, self.n_kv_heads, self.head_dim))?.transpose(1, 2)?;

        let (q, k) = if let Some(inv_freq) = inv_freq {
            self.apply_rotary_emb(&q, &k, inv_freq, pos)?
        } else {
            (q, k)
        };

        // Standard attention (simplified for training, KV-cache handled in inference)
        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let att = (q.matmul(&k.transpose(2, 3)?)? * scale)?;
        let att = candle_nn::ops::softmax(&att, candle_core::D::Minus1)?;
        let y = att.matmul(&v)?;
        let y = y.transpose(1, 2)?.reshape((b, seq_len, d_model))?;
        self.o_proj.forward(&y)
    }

    fn apply_rotary_emb(
        &self,
        q: &Tensor,
        k: &Tensor,
        inv_freq: &Tensor,
        pos: usize,
    ) -> Result<(Tensor, Tensor)> {
        let (b, n_heads, seq_len, head_dim) = q.dims4()?;
        let device = q.device();
        let t = Tensor::arange(pos as u32, (pos + seq_len) as u32, device)?
            .to_dtype(DType::F32)?
            .reshape((seq_len, 1))?;
        let freqs = t.matmul(&inv_freq.reshape((1, inv_freq.elem_count()))?)?;
        let freqs = Tensor::cat(&[&freqs, &freqs], 1)?; // [seq, head_dim]
        // Cos/Sin
        let cos = freqs.cos()?.reshape((1, 1, seq_len, head_dim))?;
        let sin = freqs.sin()?.reshape((1, 1, seq_len, head_dim))?;
        
        let q_embed = (q.broadcast_mul(&cos)? + rotate_half(q)?.broadcast_mul(&sin)?)?;
        let k_embed = (k.broadcast_mul(&cos)? + rotate_half(k)?.broadcast_mul(&sin)?)?;
        Ok((q_embed, k_embed))
    }
}

fn rotate_half(x: &Tensor) -> Result<Tensor> {
    let last_dim = x.dim(candle_core::D::Minus1)?;
    let x1 = x.narrow(candle_core::D::Minus1, 0, last_dim / 2)?;
    let x2 = x.narrow(candle_core::D::Minus1, last_dim / 2, last_dim / 2)?;
    Tensor::cat(&[&x2.neg()?, &x1], candle_core::D::Minus1)
}

pub struct Qwen2MLP {
    pub gate_proj: QuantizedLinear,
    pub up_proj: QuantizedLinear,
    pub down_proj: QuantizedLinear,
}

impl Qwen2MLP {
    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let lhs = candle_nn::ops::silu(&self.gate_proj.forward(x)?)?;
        let rhs = self.up_proj.forward(x)?;
        self.down_proj.forward(&(lhs * rhs)?)
    }
}

pub struct Qwen2Layer {
    pub input_layernorm: RMSNorm,
    pub self_attn: Qwen2Attention,
    pub post_attention_layernorm: RMSNorm,
    pub mlp: Qwen2MLP,
    pub inv_freq: Option<Tensor>,
}

impl Qwen2Layer {
    pub fn forward(&self, x: &Tensor, pos: usize) -> Result<Tensor> {
        let residual = x;
        let x = self.input_layernorm.forward(x)?;
        let x = self.self_attn.forward(&x, pos, self.inv_freq.as_ref())?;
        let x = (residual + x)?;
        
        let residual = &x;
        let x = self.post_attention_layernorm.forward(&x)?;
        let x = self.mlp.forward(&x)?;
        (residual + x)
    }
}

pub struct Qwen2Model {
    pub embed_tokens: Tensor, // Frozen wte
    pub layers: Vec<Qwen2Layer>,
    pub norm: RMSNorm,
    pub lm_head: QuantizedLinear,
}

impl Qwen2Model {
    pub fn forward(&self, input_ids: &Tensor) -> Result<Tensor> {
        let (_b, seq_len) = input_ids.dims2()?;
        let mut x = self.embed_tokens.index_select(input_ids, 0)?;
        for i in 0..self.layers.len() {
            x = self.layers[i].forward(&x, 0)?; // training: pos 0 for full sequence
        }
        let x = self.norm.forward(&x)?;
        self.lm_head.forward(&x)
    }
}
