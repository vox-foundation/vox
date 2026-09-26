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

use candle_core::quantized::QMatMul;
use candle_core::{DType, Device, Result, Tensor};
use candle_nn::RmsNorm;

#[allow(clippy::large_enum_variant)]
pub enum QuantizedLinear {
    QLora(qlora_rs::qlora::QuantizedLinear),
    QMatMul(QMatMul),
    Unquantized(Tensor),
}

impl QuantizedLinear {
    pub fn from_weight(
        weight: &Tensor,
        bias: Option<Tensor>,
        config: &qlora_rs::QLoraConfig,
        device: &Device,
    ) -> Result<Self> {
        let l = qlora_rs::qlora::QuantizedLinear::from_weight(weight, bias, config, device)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        Ok(Self::QLora(l))
    }

    pub fn from_weight_with_varbuilder(
        weight: &Tensor,
        bias: Option<Tensor>,
        config: &qlora_rs::QLoraConfig,
        vb: candle_nn::VarBuilder,
    ) -> Result<Self> {
        let l =
            qlora_rs::qlora::QuantizedLinear::from_weight_with_varbuilder(weight, bias, config, vb)
                .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        Ok(Self::QLora(l))
    }

    pub fn from_qmatmul(qmm: QMatMul) -> Self {
        Self::QMatMul(qmm)
    }

    pub fn from_tensor(t: Tensor) -> Result<Self> {
        let t_f32 = if t.dtype() == DType::F32 {
            t
        } else {
            t.to_dtype(DType::F32)?
        };
        // Pre-transpose and make contiguous to avoid repeated .t() operations in forward
        let wt = t_f32.t()?.contiguous()?;
        Ok(Self::Unquantized(wt))
    }

    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        match self {
            Self::QLora(ql) => ql
                .forward(x)
                .map_err(|e| candle_core::Error::Msg(e.to_string())),
            Self::QMatMul(qmm) => candle_nn::Module::forward(qmm, x),
            Self::Unquantized(wt) => {
                if x.dtype() != wt.dtype() {
                    let x_cast = x.to_dtype(wt.dtype())?;
                    x_cast.broadcast_matmul(wt)
                } else {
                    x.broadcast_matmul(wt)
                }
            }
        }
    }
}

impl From<qlora_rs::qlora::QuantizedLinear> for QuantizedLinear {
    fn from(l: qlora_rs::qlora::QuantizedLinear) -> Self {
        Self::QLora(l)
    }
}

impl From<QMatMul> for QuantizedLinear {
    fn from(qmm: QMatMul) -> Self {
        Self::QMatMul(qmm)
    }
}

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

/// Differentiable RMSNorm with an F32-stable reduction that preserves the activation
/// dtype. candle's `RmsNorm::forward_diff` upcasts BF16/F16 to F32 for the variance, but
/// casts the normalized result back to the *input* dtype before multiplying by the (F32)
/// norm weight — which would mix dtypes on the BF16-activation path. We sidestep that by
/// running `forward_diff` in F32 (norm weights are F32) and casting the result back to the
/// input's activation dtype. On the all-F32 path both casts are no-ops.
fn rms_norm_f32(norm: &RmsNorm, x: &Tensor) -> Result<Tensor> {
    let in_dtype = x.dtype();
    if in_dtype == DType::F32 {
        return norm.forward_diff(x);
    }
    norm.forward_diff(&x.to_dtype(DType::F32)?)?
        .to_dtype(in_dtype)
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
    let x1 = x
        .narrow(candle_core::D::Minus1, 0, last_dim / 2)?
        .contiguous()?;
    let x2 = x
        .narrow(candle_core::D::Minus1, last_dim / 2, last_dim / 2)?
        .contiguous()?;
    Tensor::cat(&[&x2.neg()?, &x1], candle_core::D::Minus1)
}

// ── Attention ─────────────────────────────────────────────────────────────────

pub struct Qwen2Attention {
    pub q_proj: QuantizedLinear,
    pub k_proj: QuantizedLinear,
    pub v_proj: QuantizedLinear,
    pub o_proj: QuantizedLinear,
    /// Qwen2/Qwen2.5 use additive biases on the q/k/v projections. Omitting them
    /// makes the forward subtly wrong (the model — and any adapter trained against
    /// it — only matches a bias-less engine, not standard Qwen2). `None` for
    /// architectures without qkv bias.
    pub q_bias: Option<Tensor>,
    pub k_bias: Option<Tensor>,
    pub v_bias: Option<Tensor>,
    pub n_heads: usize,
    pub n_kv_heads: usize,
    pub head_dim: usize,
    /// Dense Qwen3's per-head RMSNorm on Q/K, applied right after projection
    /// and before RoPE. `None` for Qwen2/Qwen2.5-style checkpoints, which
    /// don't have it. Confirmed load-bearing: a real Qwen/Qwen3-0.6B
    /// checkpoint produced fluent-looking garbage output without it (real
    /// weights, real tokenizer, no crash — just wrong numbers).
    pub q_norm: Option<RmsNorm>,
    pub k_norm: Option<RmsNorm>,
}

impl Qwen2Attention {
    /// `kv_cache` is a *slot*, not a pre-shaped buffer: `Some(&mut None)` means
    /// "cache this layer, starting empty" (the prefill) and `Some(&mut Some(..))`
    /// appends to what is already there (each decode step). Passing `None`
    /// disables caching entirely, which is what training does.
    pub fn forward(
        &self,
        x: &Tensor,
        pos: usize,
        inv_freq: Option<&Tensor>,
        kv_cache: Option<&mut Option<(Tensor, Tensor)>>,
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

        // Qwen2/Qwen2.5 additive qkv biases (broadcast over [b, seq, out_features]).
        // The activation dtype follows the configured compute dtype. Biases are
        // loaded F32; cast them to the activation dtype at point-of-use so the
        // broadcast_add never mixes dtypes (no-op on the F32 path).
        let act_dtype = q.dtype();
        let q = match &self.q_bias {
            Some(bias) => q.broadcast_add(&bias.to_dtype(act_dtype)?)?,
            None => q,
        };
        let k = match &self.k_bias {
            Some(bias) => k.broadcast_add(&bias.to_dtype(act_dtype)?)?,
            None => k,
        };
        let v = match &self.v_bias {
            Some(bias) => v.broadcast_add(&bias.to_dtype(act_dtype)?)?,
            None => v,
        };

        let q_dim = self.n_heads * self.head_dim;
        let (q, gate) = if q.dim(2)? > q_dim {
            let actual_q = q.narrow(2, 0, q_dim)?.contiguous()?;
            let gate = q.narrow(2, q_dim, q_dim)?.contiguous()?;
            (actual_q, Some(gate))
        } else {
            (q, None)
        };

        let q = q.reshape((b, seq_len, self.n_heads, self.head_dim))?;
        let q = match &self.q_norm {
            Some(norm) => rms_norm_f32(norm, &q)?,
            None => q,
        };
        let q = q.transpose(1, 2)?;

        let k = k.reshape((b, seq_len, self.n_kv_heads, self.head_dim))?;
        let k = match &self.k_norm {
            Some(norm) => rms_norm_f32(norm, &k)?,
            None => k,
        };
        let k = k.transpose(1, 2)?;

        let v = v
            .contiguous()?
            .reshape((b, seq_len, self.n_kv_heads, self.head_dim))?
            .transpose(1, 2)?
            .contiguous()?;

        let (q, k) = if let Some(inv_freq) = inv_freq {
            self.apply_rotary_emb(&q, &k, inv_freq, pos)?
        } else {
            (q, k)
        };

        let (k, v) = match kv_cache {
            Some(slot) => {
                let (k, v) = match slot.as_ref() {
                    Some((k_prev, v_prev)) => (
                        Tensor::cat(&[k_prev, &k], 2)?,
                        Tensor::cat(&[v_prev, &v], 2)?,
                    ),
                    None => (k, v),
                };
                *slot = Some((k.clone(), v.clone()));
                (k, v)
            }
            None => (k, v),
        };

        // `causal_mask` below is square in `seq_len`, so a multi-token forward is
        // only correct when nothing was cached before it (the prefill). Feeding a
        // multi-token chunk on top of a populated cache would silently mask the
        // wrong positions rather than fail, so it fails here instead.
        if seq_len > 1 && k.dim(2)? != seq_len {
            return Err(candle_core::Error::Msg(format!(
                "multi-token forward over a populated KV cache is not supported: \
                 seq_len={seq_len}, cached+new keys={}. Prefill once, then feed one token per step.",
                k.dim(2)?
            )));
        }

        let n_rep = self.n_heads / self.n_kv_heads;
        let k = repeat_kv(&k, n_rep)?;
        let v = repeat_kv(&v, n_rep)?;

        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let att = (q.contiguous()?.matmul(&k.transpose(2, 3)?.contiguous()?)? * scale)?;

        let att = if seq_len > 1 {
            let mask = causal_mask(seq_len, device)?;
            let masked_att = att.broadcast_add(&mask)?;
            candle_nn::ops::softmax(&masked_att, candle_core::D::Minus1)?
        } else {
            candle_nn::ops::softmax(&att, candle_core::D::Minus1)?
        };
        let y = att.matmul(&v.contiguous()?)?;
        let mut y =
            y.transpose(1, 2)?
                .contiguous()?
                .reshape((b, seq_len, self.n_heads * self.head_dim))?;
        if let Some(g) = gate {
            let g = candle_nn::ops::silu(&g)?;
            y = (y * g)?;
        }
        self.o_proj
            .forward(&y)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
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

    pub fn causal_depthwise_conv_silu(
        x: &Tensor,
        conv_weight: &Tensor,
        conv_state: Option<&mut Option<Tensor>>,
    ) -> Result<Tensor> {
        let (b, s, c) = x.dims3()?;
        let k = conv_weight.dim(1)?;
        let dev = x.device();

        let (full_x, offset) = if let Some(cs) = conv_state.as_deref().and_then(|opt| opt.as_ref())
        {
            let full = Tensor::cat(&[cs, x], 1)?;
            let p = cs.dim(1)?;
            (full, p)
        } else {
            (x.clone(), 0)
        };

        let total_s = full_x.dim(1)?;
        let mut steps = Vec::with_capacity(s);
        for t in offset..total_s {
            let mut acc = Tensor::zeros((b, c), DType::F32, dev)?;
            for j in 0..k {
                if t < j {
                    continue;
                }
                let x_t = full_x.narrow(1, t - j, 1)?.squeeze(1)?;
                let w = conv_weight.narrow(1, k - 1 - j, 1)?.squeeze(1)?;
                let prod = x_t.broadcast_mul(&w.unsqueeze(0)?)?;
                acc = (acc + prod)?;
            }
            steps.push(candle_nn::ops::silu(&acc)?);
        }

        if let Some(cs_ref) = conv_state {
            let keep = (k - 1).min(total_s);
            let new_cs = full_x.narrow(1, total_s - keep, keep)?;
            *cs_ref = Some(new_cs);
        }

        Tensor::stack(&steps, 1)
    }

    pub fn forward(
        &self,
        x: &Tensor,
        pos: usize,
        inv_freq: Option<&Tensor>,
        state_cache: Option<&mut Option<LinearStateCache>>,
    ) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        let device = x.device();
        let qkv = self
            .qkv_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;

        let mut conv_state = state_cache
            .as_ref()
            .and_then(|sc| sc.as_ref().and_then(|s| s.conv_state.clone()));

        let mixed_qkv = Self::causal_depthwise_conv_silu(
            &qkv,
            &self.conv_weight,
            if state_cache.is_some() {
                Some(&mut conv_state)
            } else {
                None
            },
        )?;

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

        let mut state = if let Some(state_prev) = state_cache
            .as_ref()
            .and_then(|s| s.as_ref().map(|sc| &sc.recurrent_state))
        {
            state_prev.clone()
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

        if let Some(sc) = state_cache {
            *sc = Some(LinearStateCache {
                recurrent_state: state,
                conv_state,
            });
        }

        let mut y = Tensor::stack(&outs, 1)?;
        if let Some(inv_freq) = inv_freq {
            let _ = (inv_freq, pos);
        }
        let y_flat = y.reshape((b * seq_len * self.num_v_heads, self.head_v_dim))?;
        let z_flat = z.reshape((b * seq_len * self.num_v_heads, self.head_v_dim))?;
        let y_norm = rms_norm_f32(&self.norm, &y_flat)?;
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
        let h = rms_norm_f32(&self.input_layernorm, x)?;
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
        let h = rms_norm_f32(&self.post_attention_layernorm, &x)?;
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

/// One checkpoint segment: a contiguous range of transformer layers
/// `[start_layer, end_layer)` whose **input** activation was severed from the
/// autograd tape at the segment boundary so only one segment's activations live
/// on the tape at a time during the eventual backward.
///
/// The boundary is materialized as a fresh, storage-independent leaf
/// ([`Tensor::copy`]'s deep copy, then detached) — **not** a bare
/// [`Tensor::detach`], which shares storage with the still-live grad-tracked
/// producer and (candle 0.9 quirk, see `forward_checkpointed`) yields a *wrong*
/// input gradient at the boundary. Storage independence is load-bearing for
/// gradient correctness.
pub struct CheckpointSegment {
    /// Input activation `[b, seq, d_model]` at this segment's boundary, as an
    /// independent detached leaf. During the live forward this is the tensor the
    /// next segment consumed; during recompute it is re-wrapped in a `Var` so its
    /// gradient (the cotangent to thread back) can be read.
    pub input: Tensor,
    /// First layer index (inclusive) in this segment.
    pub start_layer: usize,
    /// One-past-last layer index (exclusive) in this segment.
    pub end_layer: usize,
}

/// Materialize `x`'s values into a fresh, storage-independent detached leaf.
///
/// `Tensor::detach` shares storage with its source; when the source is a live
/// grad-tracked tensor, feeding that shared-storage detach as a segment boundary
/// makes candle 0.9 compute the **wrong** input gradient at the boundary (the
/// gradient is silently attenuated — verified against the eager full-graph
/// backward). Deep-copying first (`make_var` allocates new storage) and then
/// detaching gives a clean, independent leaf whose boundary gradient matches the
/// eager reference exactly.
fn independent_boundary(x: &Tensor) -> Result<Tensor> {
    // Var::from_tensor deep-copies a non-variable tensor into fresh storage (a new
    // leaf whose gradient candle records correctly). We first detach to get a
    // plain (non-variable) source, then wrap as a Var — this both severs the tape
    // AND breaks storage aliasing with the live grad-tracked producer. A bare
    // detach (shared storage) silently produces a WRONG boundary gradient in
    // candle 0.9. The boundary being a Var is harmless: it is not in the trainer's
    // VarMap, so the optimizer never touches it; backward still records its grad.
    let detached = x.detach();
    let var = candle_core::Var::from_tensor(&detached)?;
    Ok(var.as_tensor().clone())
}

/// Result of a checkpointed forward pass. `logits`'s autograd tape spans **only
/// the final segment + head**; `segments` carries the detached boundary
/// activations needed to recompute-and-backward earlier segments in reverse.
pub struct CheckpointedForward {
    /// Logits `[b, seq, vocab]`. Tape spans only the last segment + head.
    pub logits: Tensor,
    /// Segments in forward order. The last entry's range ends at `num_layers`.
    pub segments: Vec<CheckpointSegment>,
}

impl Qwen35Model {
    pub fn forward(&self, input_ids: &Tensor) -> Result<Tensor> {
        self.forward_with_cache(input_ids, 0, None)
    }

    pub fn forward_with_cache(
        &self,
        input_ids: &Tensor,
        pos: usize,
        caches: Option<&mut [Qwen35LayerCache]>,
    ) -> Result<Tensor> {
        self.forward_with_cache_opt(input_ids, pos, caches, false)
    }

    pub fn forward_with_cache_opt(
        &self,
        input_ids: &Tensor,
        pos: usize,
        mut caches: Option<&mut [Qwen35LayerCache]>,
        only_last_token: bool,
    ) -> Result<Tensor> {
        let (b, seq_len) = input_ids.dims2()?;
        let d_model = self.embed_tokens.dim(1)?;
        let ids = input_ids.flatten_all()?;
        let mut x = self
            .embed_tokens
            .index_select(&ids, 0)?
            .reshape((b, seq_len, d_model))?;

        for (i, layer) in self.layers.iter().enumerate() {
            let layer_cache = caches.as_deref_mut().and_then(|c| c.get_mut(i));
            x = layer.forward(&x, pos, layer_cache)?;
        }
        let x_out = if only_last_token && seq_len > 1 {
            x.narrow(1, seq_len - 1, 1)?
        } else {
            x
        };
        self.head_forward(&x_out)
    }

    /// An empty decode cache, one slot per layer, shaped to each layer's
    /// attention kind. Every slot starts empty; the prefill fills it.
    pub fn empty_cache(&self) -> Vec<Qwen35LayerCache> {
        self.layers
            .iter()
            .map(|l| match &l.attention {
                Qwen35AttentionBlock::Full(_) => Qwen35LayerCache::Full(None),
                Qwen35AttentionBlock::Linear(_) => Qwen35LayerCache::Linear(None),
            })
            .collect()
    }

    /// Forward over `input_ids` starting at absolute position `pos`, carrying
    /// per-layer decode state in `cache`.
    ///
    /// The generation loop calls this **once** with the whole prompt at `pos = 0`
    /// (the prefill), then once per step with a single new token at
    /// `pos = number of tokens already cached`. `pos` drives RoPE, so feeding a
    /// token at the wrong position silently produces wrong logits; the caller is
    /// responsible for advancing it by the number of tokens it just submitted.
    pub fn forward_cached(
        &self,
        input_ids: &Tensor,
        pos: usize,
        cache: &mut [Qwen35LayerCache],
    ) -> Result<Tensor> {
        if cache.len() != self.layers.len() {
            return Err(candle_core::Error::Msg(format!(
                "KV cache has {} slots but the model has {} layers",
                cache.len(),
                self.layers.len()
            )));
        }
        // A Gated-DeltaNet layer's recurrent state IS cached correctly, but the
        // k-tap causal short conv in front of it
        // (`Qwen35LinearAttention::causal_depthwise_conv_silu`) keeps no
        // cross-call state: it reads taps `t - j` from the CURRENT chunk and
        // treats anything earlier as zero. A whole-context forward therefore
        // always saw the real history; a one-token decode step would silently
        // drop every tap but `j = 0` and return plausible, wrong logits. Refuse
        // the second call onto a populated Linear slot rather than serve that.
        // Fixing it properly means caching the conv's last `k - 1` inputs, which
        // is a larger change than this path is scoped for.
        if let Some(i) = cache
            .iter()
            .position(|c| matches!(c, Qwen35LayerCache::Linear(Some(_))))
        {
            return Err(candle_core::Error::Msg(format!(
                "incremental decode over a linear-attention layer is unsupported \
                 (layer {i}): short-conv state caching is unimplemented, so the \
                 depthwise conv would see zeros where the previous tokens should be. \
                 Re-forward the full context for hybrid models instead."
            )));
        }
        let (b, seq_len) = input_ids.dims2()?;
        let d_model = self.embed_tokens.dim(1)?;
        let ids = input_ids.flatten_all()?;
        let mut x = self
            .embed_tokens
            .index_select(&ids, 0)?
            .reshape((b, seq_len, d_model))?;

        for (layer, slot) in self.layers.iter().zip(cache.iter_mut()) {
            x = layer.forward(&x, pos, Some(slot))?;
        }
        self.head_forward(&x)
    }

    /// Final norm + clamp + lm_head, shared by the eager and checkpointed
    /// forward so logits are identical for the same input.
    fn head_forward(&self, x: &Tensor) -> Result<Tensor> {
        let x = rms_norm_f32(&self.norm, x)?;
        let x = x.clamp(-64f64, 64f64)?;
        self.lm_head
            .forward(&x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }

    /// Run the transformer layers in `[start, end)` over `x`. Identical op
    /// sequence to [`Self::forward`]'s layer loop, so recompute reproduces the
    /// original numerics exactly (training has no KV cache / `pos` is always 0).
    fn run_layers(&self, x: &Tensor, start: usize, end: usize) -> Result<Tensor> {
        let mut h = x.clone();
        for layer in &self.layers[start..end] {
            h = layer.forward(&h, 0, None)?;
        }
        Ok(h)
    }

    /// Checkpointed forward: split the `num_layers` transformer layers into
    /// `n_segments` roughly-equal contiguous segments, detaching the activation
    /// at each boundary so the autograd tape never spans more than one segment +
    /// the head. This bounds the single-backward VRAM peak.
    ///
    /// `n_segments` is clamped to `[1, num_layers]`.
    pub fn forward_checkpointed(
        &self,
        input_ids: &Tensor,
        n_segments: usize,
    ) -> Result<CheckpointedForward> {
        let num_layers = self.layers.len();
        let n_segments = n_segments.clamp(1, num_layers.max(1));

        let (b, seq_len) = input_ids.dims2()?;
        let d_model = self.embed_tokens.dim(1)?;
        let ids = input_ids.flatten_all()?;
        let embed = self
            .embed_tokens
            .index_select(&ids, 0)?
            .reshape((b, seq_len, d_model))?;

        // Even split of layers across segments (remainder to the front segments).
        let base = num_layers / n_segments;
        let rem = num_layers % n_segments;
        let mut bounds = Vec::with_capacity(n_segments + 1);
        bounds.push(0usize);
        let mut acc = 0usize;
        for s in 0..n_segments {
            acc += base + usize::from(s < rem);
            bounds.push(acc);
        }

        let mut segments = Vec::with_capacity(n_segments);
        // First segment input is the embedding as an independent detached leaf
        // (embeddings are frozen — no grad needed through them).
        let mut seg_input = independent_boundary(&embed)?;
        let mut logits = None;
        for s in 0..n_segments {
            let start = bounds[s];
            let end = bounds[s + 1];
            let seg_out = self.run_layers(&seg_input, start, end)?;
            segments.push(CheckpointSegment {
                input: seg_input.clone(),
                start_layer: start,
                end_layer: end,
            });
            if s + 1 == n_segments {
                // Last segment flows into the head WITHOUT severing.
                logits = Some(self.head_forward(&seg_out)?);
            } else {
                // Sever the boundary into a fresh, storage-independent leaf so the
                // next segment's backward computes a correct boundary gradient.
                seg_input = independent_boundary(&seg_out)?;
            }
        }

        Ok(CheckpointedForward {
            logits: logits.expect("at least one segment"),
            segments,
        })
    }

    /// Recompute one checkpoint segment's forward from its detached boundary
    /// `input` with the tape live, returning `(input_var, seg_output)`. After
    /// backprop, `input_var`'s gradient is `dL/d(input)` — the cotangent to
    /// thread to the previous segment.
    pub fn recompute_segment(&self, seg: &CheckpointSegment) -> Result<(candle_core::Var, Tensor)> {
        let input_var = candle_core::Var::from_tensor(&seg.input)?;
        let out = self.run_layers(input_var.as_tensor(), seg.start_layer, seg.end_layer)?;
        Ok((input_var, out))
    }
}

#[derive(Clone)]
pub struct LinearStateCache {
    pub recurrent_state: Tensor,
    pub conv_state: Option<Tensor>,
}

/// Per-layer decode state. Each variant holds `None` until the prefill fills it,
/// so a fresh cache needs no shape or device knowledge to construct.
#[derive(Clone)]
pub enum Qwen35LayerCache {
    /// Full attention: the concatenated `(keys, values)` for every position so far.
    Full(Option<(Tensor, Tensor)>),
    /// Gated-DeltaNet linear attention: the recurrent state and conv state cache.
    Linear(Option<LinearStateCache>),
}

// ── CandleModel: the opaque handle stored across plugin calls ─────────────────

/// Opaque model handle stored by the plugin, built by `load_from_path` from a
/// loaded `InferenceEngine` and freed by the backend's `unload_model`.
pub struct CandleModel {
    pub engine: std::sync::Mutex<crate::inference::InferenceEngine>,
    /// Path to the model directory.
    #[allow(dead_code)]
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
        Ok(Self {
            engine: std::sync::Mutex::new(engine),
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
        cfg.cache_dequantized = false;
        let w = Tensor::randn(0f32, 0.02f32, (d, d), device).unwrap();
        let mk = || QuantizedLinear::from_weight(&w, None, &cfg, device).unwrap();
        Qwen2Attention {
            q_proj: mk(),
            k_proj: mk(),
            v_proj: mk(),
            o_proj: mk(),
            q_bias: None,
            k_bias: None,
            v_bias: None,
            n_heads: 2,
            n_kv_heads: 2,
            head_dim: 4,
            q_norm: None,
            k_norm: None,
        }
    }

    #[test]
    fn test_qwen2_attention_forward_and_cache() {
        let device = Device::Cpu;
        let attn = tiny_attention(&device);
        let x1 = Tensor::randn(0f32, 1f32, (1, 2, 8), &device).unwrap();
        let mut cache = None;
        let out1 = attn.forward(&x1, 0, None, Some(&mut cache)).unwrap();
        assert_eq!(out1.dims(), &[1, 2, 8]);
        assert!(
            cache.is_some(),
            "KV cache should be populated after prefill"
        );

        let (k_cached, v_cached) = cache.as_ref().unwrap();
        assert_eq!(k_cached.dims(), &[1, 2, 2, 4]);
        assert_eq!(v_cached.dims(), &[1, 2, 2, 4]);

        let x2 = Tensor::randn(0f32, 1f32, (1, 1, 8), &device).unwrap();
        let out2 = attn.forward(&x2, 2, None, Some(&mut cache)).unwrap();
        assert_eq!(out2.dims(), &[1, 1, 8]);

        let (k_cached2, v_cached2) = cache.as_ref().unwrap();
        assert_eq!(k_cached2.dims(), &[1, 2, 3, 4]);
        assert_eq!(v_cached2.dims(), &[1, 2, 3, 4]);

        let v = out2.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        assert!(
            v.iter().all(|f| f.is_finite()),
            "Attention output must be finite"
        );
    }

    #[test]
    fn test_qwen35_linear_attention_conv_cache() {
        let device = Device::Cpu;
        let c = 8usize;
        let k = 4usize;
        let s = 5usize;

        // Weights: shape (c, k)
        let conv_weight = Tensor::randn(0f32, 1f32, (c, k), &device).unwrap();
        // Full sequence: shape (1, s, c)
        let x_full = Tensor::randn(0f32, 1f32, (1, s, c), &device).unwrap();

        // 1. Compute full-sequence convolution without prior cache
        let out_full =
            Qwen35LinearAttention::causal_depthwise_conv_silu(&x_full, &conv_weight, None).unwrap();
        assert_eq!(out_full.dims(), &[1, s, c]);

        // 2. Compute step-by-step with conv_state cache
        let mut conv_state = None;
        let mut step_outputs = Vec::with_capacity(s);
        for t in 0..s {
            let x_step = x_full.narrow(1, t, 1).unwrap();
            let out_step = Qwen35LinearAttention::causal_depthwise_conv_silu(
                &x_step,
                &conv_weight,
                Some(&mut conv_state),
            )
            .unwrap();
            assert_eq!(out_step.dims(), &[1, 1, c]);
            step_outputs.push(out_step);
        }
        let out_stepped = Tensor::cat(&step_outputs.iter().collect::<Vec<_>>(), 1).unwrap();
        assert_eq!(out_stepped.dims(), &[1, s, c]);

        // 3. Verify step-by-step outputs match full-sequence outputs exactly
        let diff = (out_full - out_stepped).unwrap().abs().unwrap();
        let max_diff: f32 = diff
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap()
            .into_iter()
            .fold(0.0, f32::max);
        assert!(
            max_diff < 1e-5,
            "Cached conv output must match full-sequence conv: max_diff = {max_diff}"
        );
    }

    #[test]
    fn test_quantized_linear_unquantized_f32_and_bf16_forward() {
        let device = Device::Cpu;
        // Weight: (out_features=4, in_features=8)
        let w_f32 = Tensor::randn(0f32, 1f32, (4, 8), &device).unwrap();
        let ql_f32 = QuantizedLinear::from_tensor(w_f32).unwrap();
        let x = Tensor::randn(0f32, 1f32, (1, 3, 8), &device).unwrap();
        let out_f32 = ql_f32.forward(&x).unwrap();
        assert_eq!(out_f32.dims(), &[1, 3, 4]);

        // Weight in BF16:
        let w_bf16 = Tensor::randn(0f32, 1f32, (4, 8), &device)
            .unwrap()
            .to_dtype(DType::BF16)
            .unwrap();
        let ql_bf16 = QuantizedLinear::from_tensor(w_bf16).unwrap();
        let out_bf16 = ql_bf16.forward(&x).unwrap();
        assert_eq!(out_bf16.dims(), &[1, 3, 4]);
        assert_eq!(out_bf16.dtype(), DType::F32);
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

    /// Dense Qwen3's per-head q_norm/k_norm must actually change the forward
    /// output when present — confirmed load-bearing on a real Qwen/Qwen3-0.6B
    /// checkpoint: omitting it produced fluent-looking garbage text, not a
    /// crash. Builds two attention blocks from the SAME deterministic weight
    /// tensors (quantization is deterministic given the same input) so the
    /// only difference is q_norm/k_norm's presence.
    #[test]
    fn qk_norm_changes_output_when_present() {
        let device = Device::Cpu;
        let d = 8usize;
        let mut cfg = QLoraConfig::preset_all_bf16(4, 8);
        cfg.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
        let w = Tensor::arange(0u32, (d * d) as u32, &device)
            .unwrap()
            .to_dtype(DType::F32)
            .unwrap()
            .reshape((d, d))
            .unwrap()
            .affine(0.01, 0.0)
            .unwrap();
        let build = |q_norm: Option<RmsNorm>, k_norm: Option<RmsNorm>| Qwen2Attention {
            q_proj: QuantizedLinear::from_weight(&w, None, &cfg, &device).unwrap(),
            k_proj: QuantizedLinear::from_weight(&w, None, &cfg, &device).unwrap(),
            v_proj: QuantizedLinear::from_weight(&w, None, &cfg, &device).unwrap(),
            o_proj: QuantizedLinear::from_weight(&w, None, &cfg, &device).unwrap(),
            q_bias: None,
            k_bias: None,
            v_bias: None,
            n_heads: 2,
            n_kv_heads: 2,
            head_dim: 4,
            q_norm,
            k_norm,
        };

        let x = Tensor::randn(0f32, 1f32, (1, 3, d), &device).unwrap();
        let without_norm = build(None, None)
            .forward(&x, 0, None, None)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();

        let norm_weight = Tensor::new(&[2.0f32, 0.5, 3.0, 1.5], &device).unwrap();
        let with_norm = build(
            Some(RmsNorm::new(norm_weight.clone(), 1e-6)),
            Some(RmsNorm::new(norm_weight, 1e-6)),
        )
        .forward(&x, 0, None, None)
        .unwrap()
        .flatten_all()
        .unwrap()
        .to_vec1::<f32>()
        .unwrap();

        assert_ne!(
            without_norm, with_norm,
            "q_norm/k_norm must change the forward output — if this fails, \
             Qwen2Attention is silently ignoring them"
        );
    }
}

#[cfg(test)]
mod kv_cache_tests {
    //! The load-bearing correctness gate for KV-cached decoding: a prefill
    //! followed by one-token steps must produce the **same** logits as
    //! re-forwarding the whole context, or generation gets fast and wrong —
    //! strictly worse than slow and right. RoPE is switched on (`inv_freq` is
    //! `Some`) precisely so an off-by-one in `pos` shows up here; with no rotary
    //! table the positions are indistinguishable and the test proves nothing.

    use super::*;
    use qlora_rs::QLoraConfig;

    fn qcfg() -> QLoraConfig {
        let mut c = QLoraConfig::preset_all_bf16(4, 8);
        c.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
        c
    }

    fn model_with_rope(d: usize, n_heads: usize, n_layers: usize, vocab: usize) -> Qwen35Model {
        let dev = Device::Cpu;
        let head_dim = d / n_heads;
        let qlin = |d_out: usize, d_in: usize| {
            // Deterministic, non-symmetric weights: a constant weight makes every
            // position identical, which would hide a position bug.
            let w = Tensor::arange(0u32, (d_out * d_in) as u32, &dev)
                .unwrap()
                .to_dtype(DType::F32)
                .unwrap()
                .reshape((d_out, d_in))
                .unwrap()
                .affine(0.003, -0.1)
                .unwrap();
            QuantizedLinear::from_weight(&w, None, &qcfg(), &dev).unwrap()
        };
        let inv_freq = Tensor::from_vec(
            (0..head_dim / 2)
                .map(|i| 1.0f32 / 10_000f32.powf(2.0 * i as f32 / head_dim as f32))
                .collect::<Vec<f32>>(),
            (head_dim / 2,),
            &dev,
        )
        .unwrap();
        let layers = (0..n_layers)
            .map(|_| Qwen35Layer {
                input_layernorm: RmsNorm::new(Tensor::ones(d, DType::F32, &dev).unwrap(), 1e-6),
                attention: Qwen35AttentionBlock::Full(Qwen2Attention {
                    q_proj: qlin(d, d),
                    k_proj: qlin(d, d),
                    v_proj: qlin(d, d),
                    o_proj: qlin(d, d),
                    q_bias: None,
                    k_bias: None,
                    v_bias: None,
                    n_heads,
                    n_kv_heads: n_heads,
                    head_dim,
                    q_norm: None,
                    k_norm: None,
                }),
                post_attention_layernorm: RmsNorm::new(
                    Tensor::ones(d, DType::F32, &dev).unwrap(),
                    1e-6,
                ),
                mlp: Qwen2MLP {
                    gate_proj: qlin(d * 2, d),
                    up_proj: qlin(d * 2, d),
                    down_proj: qlin(d, d * 2),
                },
                inv_freq: Some(inv_freq.clone()),
            })
            .collect();
        Qwen35Model {
            embed_tokens: Tensor::randn(0f32, 0.02f32, (vocab, d), &dev).unwrap(),
            layers,
            norm: RmsNorm::new(Tensor::ones(d, DType::F32, &dev).unwrap(), 1e-6),
            lm_head: QuantizedLinear::from_weight(
                &Tensor::randn(0f32, 0.02f32, (vocab, d), &dev).unwrap(),
                None,
                &qcfg(),
                &dev,
            )
            .unwrap(),
        }
    }

    fn last_row(logits: &Tensor) -> Vec<f32> {
        let l = logits.squeeze(0).unwrap();
        let n = l.dim(0).unwrap();
        l.narrow(0, n - 1, 1)
            .unwrap()
            .squeeze(0)
            .unwrap()
            .to_vec1::<f32>()
            .unwrap()
    }

    fn ids(v: &[u32]) -> Tensor {
        Tensor::from_vec(v.to_vec(), (1, v.len()), &Device::Cpu).unwrap()
    }

    #[test]
    fn prefill_then_single_token_steps_match_the_uncached_forward() {
        let model = model_with_rope(8, 2, 2, 16);
        let seq: [u32; 5] = [1, 7, 3, 12, 5];

        // Reference: what the old (quadratic) loop computed at the last position.
        let reference = last_row(&model.forward(&ids(&seq)).unwrap());

        // Cached: prefill the first three, then feed one token per step.
        let mut cache = model.empty_cache();
        assert_eq!(cache.len(), 2);
        model
            .forward_cached(&ids(&seq[..3]), 0, &mut cache)
            .unwrap();
        model
            .forward_cached(&ids(&seq[3..4]), 3, &mut cache)
            .unwrap();
        let cached = last_row(
            &model
                .forward_cached(&ids(&seq[4..5]), 4, &mut cache)
                .unwrap(),
        );

        for (i, (a, b)) in reference.iter().zip(cached.iter()).enumerate() {
            assert!(
                (a - b).abs() < 1e-3,
                "cached decode diverged from the full forward at logit {i}: {a} vs {b}"
            );
        }
    }

    /// The position argument must actually be load-bearing: feeding the last
    /// token at the wrong `pos` has to change the logits. If this passes with
    /// identical values, RoPE is not being applied and the test above would
    /// accept an off-by-one.
    #[test]
    fn feeding_a_token_at_the_wrong_position_changes_the_logits() {
        let model = model_with_rope(8, 2, 2, 16);
        let seq: [u32; 5] = [1, 7, 3, 12, 5];

        let run = |last_pos: usize| {
            let mut cache = model.empty_cache();
            model
                .forward_cached(&ids(&seq[..4]), 0, &mut cache)
                .unwrap();
            last_row(
                &model
                    .forward_cached(&ids(&seq[4..5]), last_pos, &mut cache)
                    .unwrap(),
            )
        };
        assert_ne!(run(4), run(9), "RoPE position is not reaching the forward");
    }

    /// A multi-token chunk on top of a populated cache would be masked wrong
    /// rather than rejected, so the attention guards it. Silent wrongness here
    /// is the failure mode this whole task exists to avoid.
    #[test]
    fn a_multi_token_forward_over_a_populated_cache_is_refused() {
        let model = model_with_rope(8, 2, 2, 16);
        let seq: [u32; 5] = [1, 7, 3, 12, 5];
        let mut cache = model.empty_cache();
        model
            .forward_cached(&ids(&seq[..3]), 0, &mut cache)
            .unwrap();
        let err = model
            .forward_cached(&ids(&seq[3..5]), 3, &mut cache)
            .expect_err("a 2-token chunk over a 3-token cache must not be silently mis-masked");
        assert!(
            err.to_string().contains("populated KV cache"),
            "unexpected error: {err}"
        );
    }

    /// A two-layer hybrid stack: one full-attention layer and one Gated-DeltaNet
    /// `Linear` layer, matching what `inference.rs` builds for a `layer_types`
    /// config carrying `"linear_attention"`. `d = 64` with 8 heads keeps every
    /// projection a multiple of the NF4 quantizer's 64-element block.
    fn hybrid_model() -> Qwen35Model {
        let dev = Device::Cpu;
        let d = 64usize;
        let (num_k_heads, num_v_heads) = (8usize, 8usize);
        let (head_k_dim, head_v_dim) = (d / num_k_heads, d / num_v_heads);
        let key_dim = num_k_heads * head_k_dim;
        let value_dim = num_v_heads * head_v_dim;
        let qkv_dim = key_dim * 2 + value_dim;
        let kernel = 4usize;
        let qlin = |d_out: usize, d_in: usize| {
            let w = Tensor::randn(0f32, 0.02f32, (d_out, d_in), &dev).unwrap();
            QuantizedLinear::from_weight(&w, None, &qcfg(), &dev).unwrap()
        };
        let norm = |n: usize| RmsNorm::new(Tensor::ones(n, DType::F32, &dev).unwrap(), 1e-6);
        let mlp = || Qwen2MLP {
            gate_proj: qlin(d * 2, d),
            up_proj: qlin(d * 2, d),
            down_proj: qlin(d, d * 2),
        };
        let linear = Qwen35Layer {
            input_layernorm: norm(d),
            attention: Qwen35AttentionBlock::Linear(Qwen35LinearAttention {
                qkv_proj: qlin(qkv_dim, d),
                z_proj: qlin(value_dim, d),
                b_proj: qlin(num_v_heads, d),
                a_proj: qlin(num_v_heads, d),
                out_proj: qlin(d, value_dim),
                conv_weight: Tensor::randn(0f32, 0.02f32, (qkv_dim, kernel), &dev).unwrap(),
                dt_bias: Tensor::zeros(num_v_heads, DType::F32, &dev).unwrap(),
                a_log: Tensor::zeros(num_v_heads, DType::F32, &dev).unwrap(),
                norm: norm(head_v_dim),
                num_k_heads,
                num_v_heads,
                head_k_dim,
                head_v_dim,
            }),
            post_attention_layernorm: norm(d),
            mlp: mlp(),
            inv_freq: None,
        };
        let full = Qwen35Layer {
            input_layernorm: norm(d),
            attention: Qwen35AttentionBlock::Full(Qwen2Attention {
                q_proj: qlin(d, d),
                k_proj: qlin(d, d),
                v_proj: qlin(d, d),
                o_proj: qlin(d, d),
                q_bias: None,
                k_bias: None,
                v_bias: None,
                n_heads: 8,
                n_kv_heads: 8,
                head_dim: d / 8,
                q_norm: None,
                k_norm: None,
            }),
            post_attention_layernorm: norm(d),
            mlp: mlp(),
            inv_freq: None,
        };
        Qwen35Model {
            embed_tokens: Tensor::randn(0f32, 0.02f32, (16, d), &dev).unwrap(),
            layers: vec![full, linear],
            norm: norm(d),
            lm_head: QuantizedLinear::from_weight(
                &Tensor::randn(0f32, 0.02f32, (16, d), &dev).unwrap(),
                None,
                &qcfg(),
                &dev,
            )
            .unwrap(),
        }
    }

    /// `Qwen35LinearAttention`'s k-tap causal short conv keeps no cross-call
    /// state — it reads taps from the current chunk and treats anything earlier
    /// as zero. So a one-token decode step on a hybrid model would drop every
    /// tap but `j = 0` and return plausible, wrong logits. Fast and wrong is
    /// strictly worse than slow and right, so the step is refused.
    ///
    /// This is not a hypothetical layer: `inference.rs` builds
    /// `Qwen35AttentionBlock::Linear` whenever `config.json`'s `layer_types`
    /// names `linear_attention`.
    #[test]
    fn an_incremental_step_on_a_linear_attention_layer_is_refused() {
        let model = hybrid_model();
        let seq: [u32; 5] = [1, 7, 3, 12, 5];
        let mut cache = model.empty_cache();
        assert!(
            matches!(cache[1], Qwen35LayerCache::Linear(None)),
            "fixture must actually contain a linear-attention layer"
        );

        // The prefill is a single forward over an EMPTY cache — the conv sees the
        // whole context, exactly as the uncached path did. It must NOT trip.
        model
            .forward_cached(&ids(&seq[..4]), 0, &mut cache)
            .expect("a prefill into an empty cache is the shape the short conv can serve");
        assert!(
            matches!(cache[1], Qwen35LayerCache::Linear(Some(_))),
            "the prefill must have filled the linear layer's recurrent state"
        );

        let err = model
            .forward_cached(&ids(&seq[4..5]), 4, &mut cache)
            .expect_err("a decode step over a linear layer must be refused, not mis-convolved");
        assert!(
            err.to_string()
                .contains("short-conv state caching is unimplemented"),
            "unexpected error: {err}"
        );
    }

    /// The guard must be specific to linear attention: a pure full-attention
    /// stack decodes incrementally and must not be caught by it.
    #[test]
    fn the_linear_attention_guard_does_not_fire_on_a_full_attention_stack() {
        let model = model_with_rope(8, 2, 2, 16);
        let mut cache = model.empty_cache();
        model
            .forward_cached(&ids(&[1, 7, 3]), 0, &mut cache)
            .unwrap();
        model
            .forward_cached(&ids(&[12]), 3, &mut cache)
            .expect("full attention caches keys and values correctly and must still decode");
    }

    #[test]
    fn an_empty_cache_has_one_slot_per_layer_and_starts_unfilled() {
        let model = model_with_rope(8, 2, 3, 16);
        let cache = model.empty_cache();
        assert_eq!(cache.len(), 3);
        assert!(
            cache
                .iter()
                .all(|c| matches!(c, Qwen35LayerCache::Full(None))),
            "every slot must start empty so the prefill fills it"
        );
        assert!(
            model
                .forward_cached(&ids(&[1, 2]), 0, &mut model.empty_cache()[..2])
                .is_err(),
            "a cache with the wrong slot count must be rejected, not silently truncated"
        );
    }
}

#[cfg(test)]
mod gradient_checkpoint_tests {
    //! End-to-end correctness for activation/gradient checkpointing on the real
    //! `Qwen35Model`: a checkpointed segmented backward must produce the **same**
    //! LoRA gradients as an eager full-graph backward. CPU/F32 so it runs in
    //! normal `cargo test` (no Metal device required). A silently-wrong backward
    //! is the worst possible outcome, so this is the load-bearing safety gate.

    use super::*;
    use candle_core::Var;
    use candle_nn::{VarBuilder, VarMap};
    use qlora_rs::QLoraConfig;

    fn qcfg() -> QLoraConfig {
        let mut c = QLoraConfig::preset_all_bf16(4, 8);
        c.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
        c.cache_dequantized = true;
        c
    }

    fn qlin(vb: VarBuilder, d_out: usize, d_in: usize, dev: &Device) -> QuantizedLinear {
        let w = Tensor::randn(0f32, 0.02f32, (d_out, d_in), dev).unwrap();
        QuantizedLinear::from_weight_with_varbuilder(&w, None, &qcfg(), vb).unwrap()
    }

    fn build_layer(vb: VarBuilder, d: usize, n_heads: usize, dev: &Device) -> Qwen35Layer {
        let head_dim = d / n_heads;
        let attn = Qwen2Attention {
            q_proj: qlin(vb.pp("q"), d, d, dev),
            k_proj: qlin(vb.pp("k"), d, d, dev),
            v_proj: qlin(vb.pp("v"), d, d, dev),
            o_proj: qlin(vb.pp("o"), d, d, dev),
            q_bias: None,
            k_bias: None,
            v_bias: None,
            n_heads,
            n_kv_heads: n_heads,
            head_dim,
            q_norm: None,
            k_norm: None,
        };
        let mlp = Qwen2MLP {
            gate_proj: qlin(vb.pp("g"), d * 2, d, dev),
            up_proj: qlin(vb.pp("u"), d * 2, d, dev),
            down_proj: qlin(vb.pp("dn"), d, d * 2, dev),
        };
        Qwen35Layer {
            input_layernorm: RmsNorm::new(Tensor::ones(d, DType::F32, dev).unwrap(), 1e-6),
            attention: Qwen35AttentionBlock::Full(attn),
            post_attention_layernorm: RmsNorm::new(Tensor::ones(d, DType::F32, dev).unwrap(), 1e-6),
            mlp,
            inv_freq: None,
        }
    }

    /// Builds a layer whose attention is the Gated-DeltaNet `Linear` variant
    /// (not `Full`), so gradient tests can exercise `Qwen35LinearAttention::forward`'s
    /// `self.norm` call — the site `build_layer` structurally cannot reach.
    fn build_layer_linear(vb: VarBuilder, d: usize, dev: &Device) -> Qwen35Layer {
        // num_v_heads * d must be a multiple of the quantizer's block size (64) for
        // b_proj/a_proj to quantize; d=64 with 8 heads keeps every projection here
        // a clean multiple of 64.
        let num_k_heads = 8usize;
        let num_v_heads = 8usize;
        let head_k_dim = d / num_k_heads;
        let head_v_dim = d / num_v_heads;
        let key_dim = num_k_heads * head_k_dim;
        let value_dim = num_v_heads * head_v_dim;
        let qkv_dim = key_dim + key_dim + value_dim;
        let kernel = 4usize;

        let attn = Qwen35LinearAttention {
            qkv_proj: qlin(vb.pp("qkv"), qkv_dim, d, dev),
            z_proj: qlin(vb.pp("z"), value_dim, d, dev),
            b_proj: qlin(vb.pp("b"), num_v_heads, d, dev),
            a_proj: qlin(vb.pp("a"), num_v_heads, d, dev),
            out_proj: qlin(vb.pp("o"), d, value_dim, dev),
            conv_weight: Tensor::randn(0f32, 0.02f32, (qkv_dim, kernel), dev).unwrap(),
            dt_bias: Tensor::zeros(num_v_heads, DType::F32, dev).unwrap(),
            a_log: Tensor::zeros(num_v_heads, DType::F32, dev).unwrap(),
            norm: RmsNorm::new(Tensor::ones(head_v_dim, DType::F32, dev).unwrap(), 1e-6),
            num_k_heads,
            num_v_heads,
            head_k_dim,
            head_v_dim,
        };
        Qwen35Layer {
            input_layernorm: RmsNorm::new(Tensor::ones(d, DType::F32, dev).unwrap(), 1e-6),
            attention: Qwen35AttentionBlock::Linear(attn),
            post_attention_layernorm: RmsNorm::new(Tensor::ones(d, DType::F32, dev).unwrap(), 1e-6),
            mlp: Qwen2MLP {
                gate_proj: qlin(vb.pp("g"), d * 2, d, dev),
                up_proj: qlin(vb.pp("u"), d * 2, d, dev),
                down_proj: qlin(vb.pp("dn"), d, d * 2, dev),
            },
            inv_freq: None,
        }
    }

    fn build_model(
        vb: VarBuilder,
        d: usize,
        n_layers: usize,
        vocab: usize,
        dev: &Device,
    ) -> Qwen35Model {
        let embed = Tensor::randn(0f32, 0.02f32, (vocab, d), dev).unwrap();
        let layers: Vec<Qwen35Layer> = (0..n_layers)
            .map(|i| build_layer(vb.pp(format!("layer{i}")), d, 2, dev))
            .collect();
        let lm_head = QuantizedLinear::from_weight_with_varbuilder(
            &embed.clone(),
            None,
            &qcfg(),
            vb.pp("lm_head"),
        )
        .unwrap();
        Qwen35Model {
            embed_tokens: embed,
            layers,
            norm: RmsNorm::new(Tensor::ones(d, DType::F32, dev).unwrap(), 1e-6),
            lm_head,
        }
    }

    fn ce_loss(logits: &Tensor, targets: &Tensor) -> Tensor {
        let logits = logits.flatten_to(1).unwrap();
        let log_sm = candle_nn::ops::log_softmax(&logits, 1).unwrap();
        let lp = log_sm
            .gather(&targets.flatten_all().unwrap().unsqueeze(1).unwrap(), 1)
            .unwrap();
        lp.neg().unwrap().mean_all().unwrap()
    }

    fn varmap_name(varmap: &VarMap, v: &Var) -> String {
        let data = varmap.data().lock().unwrap();
        for (k, vv) in data.iter() {
            if vv.as_tensor().id() == v.as_tensor().id() {
                return k.clone();
            }
        }
        "<unknown>".to_string()
    }

    #[test]
    fn checkpointed_model_backward_matches_eager_grads() {
        let dev = Device::Cpu;
        let (d, n_layers, vocab, seq) = (8usize, 4usize, 16usize, 5usize);
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &dev);
        let model = build_model(vb, d, n_layers, vocab, &dev);

        let input_ids = Tensor::from_vec(vec![1u32, 2, 3, 4, 5], (1, seq), &dev).unwrap();
        let targets = Tensor::from_vec(vec![2u32, 3, 4, 5, 6], (1, seq), &dev).unwrap();

        let lora_vars: Vec<Var> = varmap.all_vars();
        assert!(!lora_vars.is_empty(), "no LoRA vars registered");

        // ── Eager: full-graph backward ──────────────────────────────────────
        let logits_eager = model.forward(&input_ids).unwrap();
        let loss_eager = ce_loss(&logits_eager, &targets);
        let g_eager = loss_eager.backward().unwrap();

        // ── Checkpointed: 2 segments, segmented recompute backward ──────────
        let ckpt = model.forward_checkpointed(&input_ids, 2).unwrap();
        assert_eq!(ckpt.segments.len(), 2);

        // logits must match eager forward exactly (same ops, just severed tape).
        let le: Vec<f32> = logits_eager.flatten_all().unwrap().to_vec1().unwrap();
        let lc: Vec<f32> = ckpt.logits.flatten_all().unwrap().to_vec1().unwrap();
        for (a, b) in le.iter().zip(lc.iter()) {
            assert!(
                (a - b).abs() < 1e-4,
                "checkpointed logits diverged: {a} vs {b}"
            );
        }

        let loss_ck = ce_loss(&ckpt.logits, &targets);
        let mut grads = loss_ck.backward().unwrap();
        let last = ckpt.segments.last().unwrap();
        let mut upstream = grads.get(&last.input).cloned().unwrap();
        for seg in ckpt.segments.iter().rev().skip(1) {
            let (input_var, seg_out) = model.recompute_segment(seg).unwrap();
            let seg_grads = qlora_rs::backward_from_cotangent(&seg_out, &upstream).unwrap();
            qlora_rs::accumulate_grads_for_vars(&mut grads, &seg_grads, &lora_vars).unwrap();
            upstream = seg_grads.get(input_var.as_tensor()).cloned().unwrap();
        }

        // ── Compare per-Var grads ───────────────────────────────────────────
        let mut compared = 0usize;
        for v in &lora_vars {
            match (g_eager.get(v.as_tensor()), grads.get(v.as_tensor())) {
                (Some(a), Some(b)) => {
                    let name = varmap_name(&varmap, v);
                    let av: Vec<f32> = a.flatten_all().unwrap().to_vec1().unwrap();
                    let bv: Vec<f32> = b.flatten_all().unwrap().to_vec1().unwrap();
                    for (x, y) in av.iter().zip(bv.iter()) {
                        let denom = x.abs().max(1e-3);
                        assert!(
                            (x - y).abs() / denom < 1e-2,
                            "grad mismatch for {name}: eager={x} ckpt={y}"
                        );
                    }
                    compared += 1;
                }
                (None, None) => {}
                (a, b) => panic!(
                    "grad presence mismatch: eager={} ckpt={}",
                    a.is_some(),
                    b.is_some()
                ),
            }
        }
        assert!(
            compared >= n_layers,
            "expected grads for >= {n_layers} vars, got {compared}"
        );
    }

    /// Same eager-vs-checkpointed grad comparison as
    /// `checkpointed_model_backward_matches_eager_grads`, but with a Gated-DeltaNet
    /// `Linear`-attention layer in the mix. `build_layer` only ever constructs
    /// `Qwen35AttentionBlock::Full`, so that test cannot reach
    /// `Qwen35LinearAttention::forward`'s `self.norm` call — this one does, and is
    /// the regression guard for that site specifically.
    #[test]
    fn checkpointed_model_backward_matches_eager_grads_linear_attention() {
        let dev = Device::Cpu;
        // d=64 (rather than the 8 other tests in this module use) because
        // Qwen35LinearAttention's b_proj/a_proj project to num_v_heads outputs,
        // and the quantizer requires each weight's element count to be a
        // multiple of its 64-element block size.
        let (d, n_layers, vocab, seq) = (64usize, 4usize, 16usize, 5usize);
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &dev);

        let embed = Tensor::randn(0f32, 0.02f32, (vocab, d), &dev).unwrap();
        let layers: Vec<Qwen35Layer> = (0..n_layers)
            .map(|i| {
                let lvb = vb.pp(format!("layer{i}"));
                if i % 2 == 0 {
                    build_layer_linear(lvb, d, &dev)
                } else {
                    build_layer(lvb, d, 2, &dev)
                }
            })
            .collect();
        let lm_head = QuantizedLinear::from_weight_with_varbuilder(
            &embed.clone(),
            None,
            &qcfg(),
            vb.pp("lm_head"),
        )
        .unwrap();
        let model = Qwen35Model {
            embed_tokens: embed,
            layers,
            norm: RmsNorm::new(Tensor::ones(d, DType::F32, &dev).unwrap(), 1e-6),
            lm_head,
        };

        let input_ids = Tensor::from_vec(vec![1u32, 2, 3, 4, 5], (1, seq), &dev).unwrap();
        let targets = Tensor::from_vec(vec![2u32, 3, 4, 5, 6], (1, seq), &dev).unwrap();

        let lora_vars: Vec<Var> = varmap.all_vars();
        assert!(!lora_vars.is_empty(), "no LoRA vars registered");

        let logits_eager = model.forward(&input_ids).unwrap();
        let loss_eager = ce_loss(&logits_eager, &targets);
        let g_eager = loss_eager.backward().unwrap();

        // Direct regression guard for the bug this test exists to catch: candle-nn's
        // fused `RmsNorm::forward` records `BackpropOp::none()` — it doesn't produce
        // wrong gradients, it silently SEVERS the graph, so nothing upstream of it
        // gets a gradient at all. In `Qwen35LinearAttention::forward`, `self.norm`
        // sits between the qkv/z/b/a projections and `out_proj`, so if `.norm.forward()`
        // (rather than `rms_norm_f32`) is used, every LoRA weight feeding those
        // projections silently gets `None` in both the eager AND the checkpointed
        // backward alike — which is exactly why the eager-vs-checkpointed comparison
        // below cannot detect this class of bug: both sides agree on the same missing
        // gradient. Assert directly that gradient reaches a pre-norm var instead.
        let pre_norm_var = lora_vars
            .iter()
            .find(|v| varmap_name(&varmap, v) == "layer0.qkv.lora_a.weight")
            .expect("layer0 must be the Linear-attention variant with a qkv LoRA var");
        let pre_norm_grad = g_eager
            .get(pre_norm_var.as_tensor())
            .expect(
                "no eager gradient reached layer0.qkv.lora_a.weight — self.norm in \
                 Qwen35LinearAttention::forward is almost certainly using the fused \
                 `.forward()` (BackpropOp::none, severs the graph) instead of \
                 `rms_norm_f32`",
            )
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        assert!(
            pre_norm_grad.iter().any(|g| g.abs() > 1e-8),
            "gradient for layer0.qkv.lora_a.weight is all-zero — self.norm in \
             Qwen35LinearAttention::forward is severing the graph"
        );

        let ckpt = model.forward_checkpointed(&input_ids, 2).unwrap();
        assert_eq!(ckpt.segments.len(), 2);

        let loss_ck = ce_loss(&ckpt.logits, &targets);
        let mut grads = loss_ck.backward().unwrap();
        let last = ckpt.segments.last().unwrap();
        let mut upstream = grads.get(&last.input).cloned().unwrap();
        for seg in ckpt.segments.iter().rev().skip(1) {
            let (input_var, seg_out) = model.recompute_segment(seg).unwrap();
            let seg_grads = qlora_rs::backward_from_cotangent(&seg_out, &upstream).unwrap();
            qlora_rs::accumulate_grads_for_vars(&mut grads, &seg_grads, &lora_vars).unwrap();
            upstream = seg_grads.get(input_var.as_tensor()).cloned().unwrap();
        }

        let mut compared = 0usize;
        for v in &lora_vars {
            match (g_eager.get(v.as_tensor()), grads.get(v.as_tensor())) {
                (Some(a), Some(b)) => {
                    let name = varmap_name(&varmap, v);
                    let av: Vec<f32> = a.flatten_all().unwrap().to_vec1().unwrap();
                    let bv: Vec<f32> = b.flatten_all().unwrap().to_vec1().unwrap();
                    for (x, y) in av.iter().zip(bv.iter()) {
                        let denom = x.abs().max(1e-3);
                        assert!(
                            (x - y).abs() / denom < 1e-2,
                            "grad mismatch for {name}: eager={x} ckpt={y}"
                        );
                    }
                    compared += 1;
                }
                (None, None) => {}
                (a, b) => panic!(
                    "grad presence mismatch: eager={} ckpt={}",
                    a.is_some(),
                    b.is_some()
                ),
            }
        }
        assert!(
            compared >= n_layers,
            "expected grads for >= {n_layers} vars, got {compared}"
        );
    }
}

#[cfg(all(test, target_os = "macos"))]
mod metal_backward_tests {
    //! GQA training on Metal: `repeat_kv` goes through a rank-5 `expand`, whose
    //! backward is a `sum_keepdim` over a middle axis of a rank-5 tensor.
    //! candle-metal-kernels 0.10.2 mis-indexed every rank>4 strided reduce, so
    //! the K/V gradients were garbage, grew ~1.5x per layer, and overflowed to
    //! inf/NaN within a few steps; AdamW then wrote NaN into every LoRA var
    //! (even at lr=0, since 0 * NaN = NaN). See patches/candle-metal-kernels-0.10.2.

    use super::*;
    use candle_core::Var;

    fn grad_of_repeat_kv(dev: &Device, x0: &Tensor, w0: &Tensor) -> Vec<f32> {
        let x = Var::from_tensor(&x0.to_device(dev).unwrap()).unwrap();
        let w = w0.to_device(dev).unwrap();
        let y = repeat_kv(x.as_tensor(), 2).unwrap();
        let grads = (y * w).unwrap().sum_all().unwrap().backward().unwrap();
        grads
            .get(x.as_tensor())
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_device(&Device::Cpu)
            .unwrap()
            .to_vec1()
            .unwrap()
    }

    #[test]
    fn repeat_kv_backward_on_metal_matches_cpu() {
        let Ok(metal) = Device::new_metal(0) else {
            return; // no Metal device (e.g. a macOS CI VM): nothing to compare
        };
        let cpu = Device::Cpu;
        // (b, n_kv, seq, head_dim) — Qwen3-0.6B shape class, shortened.
        let x0 = Tensor::randn(0f32, 1.0, (1, 2, 64, 16), &cpu).unwrap();
        let w0 = Tensor::randn(0f32, 1.0, (1, 4, 64, 16), &cpu).unwrap();
        let want = grad_of_repeat_kv(&cpu, &x0, &w0);
        let got = grad_of_repeat_kv(&metal, &x0, &w0);
        let max_diff = want
            .iter()
            .zip(&got)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        assert!(
            max_diff < 1e-4,
            "repeat_kv K/V gradient differs between Metal and CPU (max |diff| = {max_diff})"
        );
    }
}
