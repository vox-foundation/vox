//! CandleModel: wrapper holding a loaded Qwen3.5 model for use by the MlBackend trait.
//!
//! The transformer block implementation (Qwen2Attention, Qwen35LinearAttention, etc.) is
//! copied verbatim from `vox-populi`'s `candle_model_qwen` module. In a future cleanup
//! pass (SP6+) this should be extracted into a shared `vox-candle-models` crate so there
//! is a single canonical copy.
//!
//! # Extraction status
//!
//! The full QLoRA training loop lives in `candle_qlora_train/`; `training.rs` wires
//! `run_full_training` through to it via the `TrainRequest` JSON envelope. The
//! per-step wrappers (`run_train_step`, `run_eval_step`) still require the
//! plugin-host streaming protocol and point callers at `run_full_training`.
//!
//! `load_from_path` builds the handle through `crate::inference::InferenceEngine`,
//! which runs its own preflight over HF `config.json` and the safetensors shards.

use candle_core::{DType, Device, Result, Tensor};
use candle_nn::{Module, RmsNorm};
use qlora_rs::qlora::QuantizedLinear;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn causal_mask(seq_len: usize, device: &Device) -> Result<Tensor> {
    let mut data = vec![0.0f32; seq_len * seq_len];
    for row in 0..seq_len {
        for col in (row + 1)..seq_len {
            data[row * seq_len + col] = f32::NEG_INFINITY;
        }
    }
    Tensor::from_vec(data, (1, 1, seq_len, seq_len), device)
}

fn repeat_kv(x: &Tensor, n_rep: usize) -> Result<Tensor> {
    if n_rep == 1 {
        return Ok(x.clone());
    }
    let (b, n_kv, seq, hd) = x.dims4()?;
    x.unsqueeze(2)?
        .expand((b, n_kv, n_rep, seq, hd))?
        .reshape((b, n_kv * n_rep, seq, hd))
}

fn rotate_half(x: &Tensor) -> Result<Tensor> {
    let last_dim = x.dim(candle_core::D::Minus1)?;
    let x1 = x.narrow(candle_core::D::Minus1, 0, last_dim / 2)?;
    let x2 = x.narrow(candle_core::D::Minus1, last_dim / 2, last_dim / 2)?;
    Tensor::cat(&[&x2.neg()?, &x1], candle_core::D::Minus1)
}

// ── Attention ─────────────────────────────────────────────────────────────────

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
    pub fn forward(
        &self,
        x: &Tensor,
        pos: usize,
        inv_freq: Option<&Tensor>,
        kv_cache: Option<&mut (Tensor, Tensor)>,
    ) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        let device = x.device();

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

        let q = q
            .reshape((b, seq_len, self.n_heads, self.head_dim))?
            .transpose(1, 2)?;
        let k = k
            .reshape((b, seq_len, self.n_kv_heads, self.head_dim))?
            .transpose(1, 2)?;
        let v = v
            .reshape((b, seq_len, self.n_kv_heads, self.head_dim))?
            .transpose(1, 2)?;

        let (q, k) = if let Some(inv_freq) = inv_freq {
            self.apply_rotary_emb(&q, &k, inv_freq, pos)?
        } else {
            (q, k)
        };

        let (k, v) = if let Some((k_prev, v_prev)) = kv_cache {
            let k = Tensor::cat(&[&*k_prev, &k], 2)?;
            let v = Tensor::cat(&[&*v_prev, &v], 2)?;
            *k_prev = k.clone();
            *v_prev = v.clone();
            (k, v)
        } else {
            (k, v)
        };

        let n_rep = self.n_heads / self.n_kv_heads;
        let k = repeat_kv(&k, n_rep)?;
        let v = repeat_kv(&v, n_rep)?;
        let v = v.clamp(-256f64, 256f64)?;

        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let mut att = (q.contiguous()?.matmul(&k.transpose(2, 3)?.contiguous()?)? * scale)?;
        att = att.clamp(-120f64, 120f64)?;

        if seq_len > 1 {
            let att_max = att.max_keepdim(candle_core::D::Minus1)?;
            att = att.broadcast_sub(&att_max)?;
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
        let rope_dim = inv_freq.elem_count().saturating_mul(2);
        if rope_dim == 0 || rope_dim > head_dim {
            return Err(candle_core::Error::Msg(format!(
                "RoPE inv_freq length inconsistent with head_dim: inv_freq_elems={} head_dim={head_dim}",
                inv_freq.elem_count()
            )));
        }
        let device = q.device();
        let t = Tensor::arange(pos as u32, (pos + seq_len) as u32, device)?
            .to_dtype(DType::F32)?
            .reshape((seq_len, 1))?;
        let freqs = t.matmul(&inv_freq.reshape((1, inv_freq.elem_count()))?)?;
        let freqs = Tensor::cat(&[&freqs, &freqs], 1)?;
        let cos = freqs.cos()?.reshape((1, 1, seq_len, rope_dim))?;
        let sin = freqs.sin()?.reshape((1, 1, seq_len, rope_dim))?;
        if rope_dim == head_dim {
            let q_embed = (q.broadcast_mul(&cos)? + rotate_half(q)?.broadcast_mul(&sin)?)?;
            let k_embed = (k.broadcast_mul(&cos)? + rotate_half(k)?.broadcast_mul(&sin)?)?;
            Ok((q_embed, k_embed))
        } else {
            let q_rot = q.narrow(candle_core::D::Minus1, 0, rope_dim)?;
            let q_pass = q.narrow(candle_core::D::Minus1, rope_dim, head_dim - rope_dim)?;
            let k_rot = k.narrow(candle_core::D::Minus1, 0, rope_dim)?;
            let k_pass = k.narrow(candle_core::D::Minus1, rope_dim, head_dim - rope_dim)?;
            let q_r = (q_rot.broadcast_mul(&cos)? + rotate_half(&q_rot)?.broadcast_mul(&sin)?)?;
            let k_r = (k_rot.broadcast_mul(&cos)? + rotate_half(&k_rot)?.broadcast_mul(&sin)?)?;
            let q_embed = Tensor::cat(&[&q_r, &q_pass], candle_core::D::Minus1)?;
            let k_embed = Tensor::cat(&[&k_r, &k_pass], candle_core::D::Minus1)?;
            Ok((q_embed, k_embed))
        }
    }
}

// ── MLP ───────────────────────────────────────────────────────────────────────

pub struct Qwen2MLP {
    pub gate_proj: QuantizedLinear,
    pub up_proj: QuantizedLinear,
    pub down_proj: QuantizedLinear,
}

impl Qwen2MLP {
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

// ── Qwen3.5 hybrid attention ──────────────────────────────────────────────────

pub struct Qwen35LinearAttention {
    pub qkv_proj: QuantizedLinear,
    pub z_proj: QuantizedLinear,
    pub b_proj: QuantizedLinear,
    pub a_proj: QuantizedLinear,
    pub out_proj: QuantizedLinear,
    pub conv_weight: Tensor,
    pub dt_bias: Tensor,
    pub a_log: Tensor,
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
        let g_pre =
            (a.broadcast_add(&dt_bias.reshape((1, 1, self.num_v_heads))?)?).to_dtype(DType::F32)?;
        let g_soft = (g_pre.exp()?.broadcast_add(&Tensor::new(1f32, device)?)?).log()?;
        let a_log_scale = a_log.exp()?.clamp(1e-6f64, 1e4f64)?;
        let g = g_soft
            .broadcast_mul(&a_log_scale.reshape((1, 1, self.num_v_heads))?)?
            .neg()?;
        let g = g.clamp(-80f64, 20f64)?;

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

#[allow(clippy::large_enum_variant)]
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
        let x = x.clamp(-64f64, 64f64)?;
        self.lm_head
            .forward(&x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }
}

pub enum Qwen35LayerCache {
    Full((Tensor, Tensor)),
    Linear(Tensor),
}

// ── CandleModel: the opaque handle stored across plugin calls ─────────────────

/// Opaque model handle stored by the plugin, built by `load_from_path` from a
/// loaded `InferenceEngine` and freed by the backend's `unload_model`.
pub struct CandleModel {
    pub _inner: Qwen35Model,
    /// Path to the model directory, stored so `run_inference` can reload the engine.
    pub model_path: String,
    /// Optional QLoRA trainer instance if this model is being used for training steps.
    pub trainer: Option<qlora_rs::training::QLoraTrainer>,
}

impl CandleModel {
    /// Load a Qwen3.5 QLoRA model from `model_path` for inference or resumption.
    pub fn load_from_path(model_path: &str) -> anyhow::Result<Self> {
        let path = std::path::Path::new(model_path);
        let engine =
            crate::inference::InferenceEngine::load(path, &crate::device::DeviceKind::Best)?;
        let crate::inference::InferenceModel::Qwen35(_inner) = engine.model;
        Ok(Self {
            _inner,
            model_path: model_path.to_string(),
            trainer: None, // Trainer is initialized dynamically during run_full_training
        })
    }
}

#[cfg(test)]
mod qwen2_attention_tests {
    //! Real (non-trivial) shape-correctness check for `Qwen2Attention::forward`.
    //!
    //! Builds a tiny single-batch, two-head attention block from actual
    //! `QuantizedLinear` weights (not mocks) and asserts the output tensor has
    //! the shape the reshape/transpose/rotary/attend/merge pipeline promises:
    //! `(batch, seq_len, n_heads * head_dim)`. This fails if the reshape or
    //! head-merge logic regresses — the class of bug a shape assertion alone
    //! catches without needing a golden numeric value.

    use super::*;
    use qlora_rs::QLoraConfig;

    fn tiny_attention(device: &Device) -> Qwen2Attention {
        let d = 8usize; // d_model == n_heads * head_dim
        let mut cfg = QLoraConfig::preset_all_bf16(4, 8);
        // CPU (this test's device) does not support BF16 matmul in this Candle build;
        // force F32 so the test exercises real forward math instead of skipping.
        cfg.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
        let w = Tensor::randn(0f32, 0.02f32, (d, d), device).unwrap();
        let mk = || QuantizedLinear::from_weight(&w, None, &cfg, device).unwrap();
        Qwen2Attention {
            q_proj: mk(),
            k_proj: mk(),
            v_proj: mk(),
            o_proj: mk(),
            n_heads: 2,
            n_kv_heads: 2,
            head_dim: 4,
        }
    }

    #[test]
    fn forward_preserves_batch_seq_and_merges_heads_back_to_d_model() {
        let device = Device::Cpu;
        let attn = tiny_attention(&device);
        let (batch, seq_len, d_model) = (1usize, 3usize, 8usize);
        let x = Tensor::randn(0f32, 0.02f32, (batch, seq_len, d_model), &device).unwrap();

        let out = attn
            .forward(&x, 0, None, None)
            .expect("forward should succeed");

        assert_eq!(
            out.dims(),
            &[batch, seq_len, d_model],
            "output must merge n_heads*head_dim back to d_model without dropping batch/seq"
        );
    }
}
