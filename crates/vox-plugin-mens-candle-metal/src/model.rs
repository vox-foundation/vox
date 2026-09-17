//! CandleModel: wrapper holding a loaded Qwen3.5 model for use by the MlBackend trait.
//!
//! The transformer block implementation (Qwen2Attention, Qwen35LinearAttention, etc.) is
//! copied verbatim from `vox-populi`'s `candle_model_qwen` module. In a future cleanup
//! pass (SP6+) this should be extracted into a shared `vox-candle-models` crate so there
//! is a single canonical copy.
//!
//! # SP3 extraction status
//!
//! The forward/backward training code in `vox-populi` is deeply tangled with vox-populi
//! types (`LoraTrainingConfig`, `QloraEmbedBundle`, `TrainingPair`, `CheckpointState`,
//! `vox_tensor`, `vox_secrets`, VoxDB async channel, etc.). Untangling into the plugin's
//! `training.rs` / `checkpoint.rs` is deferred to a follow-up commit; those modules
//! currently contain stubs that return `Err("not yet wired")`.
//!
//! `load_from_path` is also stubbed: the real implementation requires `QloraEmbedBundle`
//! (preflight logic that reads HF `config.json` and locates safetensors shards), which
//! lives in vox-populi. Batch 3 wires vox-populi to consume this plugin through the host;
//! at that point, the plugin can receive the already-loaded `Qwen35Model` via a serialized
//! handle rather than needing to replicate the full preflight.
//!
//! # TODO (batch 4): add `unload_model` verb to `MlBackend` trait to free the boxed
//! `CandleModel` and avoid the current memory leak on plugin unload.

use candle_core::quantized::QMatMul;
use candle_core::{DType, Device, Result, Tensor};
use candle_nn::{Module, RmsNorm};

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
    pub q_norm: Option<RmsNorm>,
    pub k_norm: Option<RmsNorm>,
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

        let q_dim = self.n_heads * self.head_dim;
        let (q, gate) = if q.dim(2)? > q_dim {
            let actual_q = q.narrow(2, 0, q_dim)?.contiguous()?;
            let gate = q.narrow(2, q_dim, q_dim)?.contiguous()?;
            (actual_q, Some(gate))
        } else {
            (q, None)
        };

        let q = q
            .contiguous()?
            .reshape((b, seq_len, self.n_heads, self.head_dim))?
            .transpose(1, 2)?
            .contiguous()?;
        let k = k
            .contiguous()?
            .reshape((b, seq_len, self.n_kv_heads, self.head_dim))?
            .transpose(1, 2)?
            .contiguous()?;
        let v = v
            .contiguous()?
            .reshape((b, seq_len, self.n_kv_heads, self.head_dim))?
            .transpose(1, 2)?
            .contiguous()?;

        let q = if let Some(q_norm) = &self.q_norm {
            q_norm.forward(&q)?
        } else {
            q
        };
        let k = if let Some(k_norm) = &self.k_norm {
            k_norm.forward(&k)?
        } else {
            k
        };

        let (q, k) = if let Some(inv_freq) = inv_freq {
            self.apply_rotary_emb(&q, &k, inv_freq, pos)?
        } else {
            (q, k)
        };

        let (k, v) = if let Some(cache) = kv_cache {
            if let Some((k_prev, v_prev)) = cache.as_mut() {
                let k_new = Tensor::cat(&[&*k_prev, &k], 2)?;
                let v_new = Tensor::cat(&[&*v_prev, &v], 2)?;
                *k_prev = k_new.clone();
                *v_prev = v_new.clone();
                (k_new, v_new)
            } else {
                *cache = Some((k.clone(), v.clone()));
                (k, v)
            }
        } else {
            (k, v)
        };

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
        state_cache: Option<&mut LinearStateCache>,
    ) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        let device = x.device();
        let qkv = self
            .qkv_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;

        let (conv_state, recurrent_state) = match state_cache {
            Some(sc) => (Some(&mut sc.conv_state), Some(&mut sc.recurrent_state)),
            None => (None, None),
        };

        let mixed_qkv = Self::causal_depthwise_conv_silu(&qkv, &self.conv_weight, conv_state)?;

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

        let mut state = if let Some(state_prev) = recurrent_state.as_deref() {
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

        if let Some(state_prev) = recurrent_state {
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
        let x = self.norm.forward(&x)?;
        let x = x.clamp(-64f64, 64f64)?;
        let x_out = if only_last_token && seq_len > 1 {
            x.narrow(1, seq_len - 1, 1)?
        } else {
            x
        };
        self.lm_head
            .forward(&x_out)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }
}

#[derive(Clone)]
pub struct LinearStateCache {
    pub recurrent_state: Tensor,
    pub conv_state: Option<Tensor>,
}

#[derive(Clone)]
pub enum Qwen35LayerCache {
    Full(Option<(Tensor, Tensor)>),
    Linear(LinearStateCache),
}

// ── CandleModel: the opaque handle stored across plugin calls ─────────────────

/// Opaque model handle stored by the plugin. In the current SP3 stub, `load_from_path`
/// returns an error — the actual construction requires `QloraEmbedBundle` from
/// vox-populi's preflight logic. Batch 3 wires vox-populi to construct the model and
/// pass it to the plugin via an alternative init path.
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
mod tests {
    use super::*;
    use qlora_rs::QLoraConfig;
    use qlora_rs::quantization::ComputeDType;

    fn tiny_attention(device: &Device) -> Qwen2Attention {
        let d = 8usize;
        let mut cfg = QLoraConfig::preset_all_bf16(4, 8);
        cfg.quantization.compute_dtype = ComputeDType::F32;
        cfg.cache_dequantized = false;
        let w = Tensor::randn(0f32, 0.02f32, (d, d), device).unwrap();
        let mk = || QuantizedLinear::from_weight(&w, None, &cfg, device).unwrap();
        Qwen2Attention {
            q_proj: mk(),
            k_proj: mk(),
            v_proj: mk(),
            o_proj: mk(),
            q_norm: None,
            k_norm: None,
            n_heads: 2,
            n_kv_heads: 2,
            head_dim: 4,
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
}
