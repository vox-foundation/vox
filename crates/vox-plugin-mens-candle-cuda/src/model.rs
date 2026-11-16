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

use candle_core::{DType, Device, Result, Tensor};
use candle_nn::RmsNorm;
use qlora_rs::qlora::QuantizedLinear;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Projection wrapper that keeps low-fan-in Gated DeltaNet gate weights
/// (`in_proj_b` / `in_proj_a`) in unquantized FP32 while leaving all large
/// matrix multiplies quantized.
pub enum LinearProj {
    Quantized(QuantizedLinear),
    F32WeightT(Tensor),
}

impl LinearProj {
    /// Construct an unquantized FP32 projection from an `[out_features, in_features]` weight tensor.
    pub fn from_tensor(weight: Tensor) -> Result<Self> {
        let w_f32 = if weight.dtype() == DType::F32 {
            weight
        } else {
            weight.to_dtype(DType::F32)?
        };
        Ok(Self::F32WeightT(w_f32.t()?.contiguous()?))
    }

    /// Project `x` (`[..., in_features]`) into `[..., out_features]`.
    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        match self {
            Self::Quantized(ql) => ql
                .forward(x)
                .map_err(|e| candle_core::Error::Msg(e.to_string())),
            Self::F32WeightT(w_t) => {
                let x_f32 = if x.dtype() == DType::F32 {
                    x.clone()
                } else {
                    x.to_dtype(DType::F32)?
                };
                x_f32.broadcast_matmul(w_t)
            }
        }
    }
}

/// Per-token local positions within packed documents, resetting to `0` at each
/// document boundary.
pub fn packed_local_positions(doc_ids: &[u32]) -> Vec<u32> {
    let mut positions = Vec::with_capacity(doc_ids.len());
    let mut prev_doc: Option<u32> = None;
    let mut local = 0u32;
    for &doc_id in doc_ids {
        if prev_doc == Some(doc_id) {
            local = local.saturating_add(1);
        } else {
            local = 0;
            prev_doc = Some(doc_id);
        }
        positions.push(local);
    }
    positions
}

/// Block-diagonal causal mask (`(1, 1, seq_len, seq_len)`, `F32`) allowing attention
/// strictly when `doc_ids[row] == doc_ids[col]` and `col <= row`.
pub fn block_diagonal_causal_mask(doc_ids: &[u32], device: &Device) -> Result<Tensor> {
    let seq_len = doc_ids.len();
    let mut data = vec![f32::NEG_INFINITY; seq_len * seq_len];
    for row in 0..seq_len {
        let d_row = doc_ids[row];
        for col in 0..=row {
            if doc_ids[col] == d_row {
                data[row * seq_len + col] = 0.0f32;
            }
        }
    }
    Tensor::from_vec(data, (1, 1, seq_len, seq_len), device)
}

/// Precomputed packed-sequence RoPE positions and block-diagonal mask shared across
/// all layers in a single packed forward pass.
pub struct PackedAttnContext {
    pub doc_ids: Vec<u32>,
    /// Per-token local positions `(seq_len, 1)` in `F32`.
    pub local_pos_f32: Tensor,
    /// Block-diagonal causal mask `(1, 1, seq_len, seq_len)` (`F32`) or `None` when `seq_len <= 1`.
    pub mask: Option<Tensor>,
}

impl PackedAttnContext {
    /// Validate contiguous document runs and precompute RoPE positions and causal mask once.
    pub fn new(doc_ids: &[u32], device: &Device) -> Result<Self> {
        let mut seen = std::collections::HashSet::with_capacity(doc_ids.len());
        let mut prev: Option<u32> = None;
        for &d in doc_ids {
            if prev != Some(d) {
                if !seen.insert(d) {
                    return Err(candle_core::Error::Msg(format!(
                        "PackedAttnContext requires contiguous document runs; doc_id {d} reappeared"
                    )));
                }
                prev = Some(d);
            }
        }
        let seq_len = doc_ids.len();
        let pos_f32: Vec<f32> = packed_local_positions(doc_ids)
            .into_iter()
            .map(|p| p as f32)
            .collect();
        let local_pos_f32 = Tensor::from_vec(pos_f32, (seq_len, 1), device)?;
        let mask = if seq_len > 1 {
            Some(block_diagonal_causal_mask(doc_ids, device)?)
        } else {
            None
        };
        Ok(Self {
            doc_ids: doc_ids.to_vec(),
            local_pos_f32,
            mask,
        })
    }
}

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
    fn project_qkv_heads(
        &self,
        x: &Tensor,
        b: usize,
        seq_len: usize,
    ) -> Result<(Tensor, Tensor, Tensor, Option<Tensor>, DType)> {
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
        // The activation dtype follows the configured compute dtype (BF16 on CUDA,
        // F32 on CPU/tests). Biases are loaded F32; cast them to the activation dtype
        // at point-of-use so the broadcast_add never mixes dtypes (no-op on the F32 path).
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

        let (q, gate) = vox_plugin_mens_candle_core::attn_math::split_q_and_gate(
            q,
            b,
            seq_len,
            self.n_heads,
            self.head_dim,
        )?;
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
            .reshape((b, seq_len, self.n_kv_heads, self.head_dim))?
            .transpose(1, 2)?;

        Ok((q, k, v, gate, act_dtype))
    }

    #[allow(clippy::too_many_arguments)]
    fn attend_and_project(
        &self,
        q: &Tensor,
        k: &Tensor,
        v: &Tensor,
        gate: Option<&Tensor>,
        mask: Option<&Tensor>,
        b: usize,
        seq_len: usize,
        act_dtype: DType,
    ) -> Result<Tensor> {
        let n_rep = self.n_heads / self.n_kv_heads;
        let k = repeat_kv(k, n_rep)?;
        let v = repeat_kv(v, n_rep)?;

        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let att = (q.contiguous()?.matmul(&k.transpose(2, 3)?.contiguous()?)? * scale)?;

        // Softmax (and the surrounding max-subtraction / mask add) accumulate in F32
        // for numerical stability regardless of the activation dtype, then cast back to
        // the activation dtype for the att @ v matmul. On the all-F32 path these casts
        // are no-ops; on the BF16 path they keep the attention probabilities stable.
        let att = att.to_dtype(DType::F32)?;
        let att = match mask {
            Some(m) => att.broadcast_add(m)?,
            None => att,
        };
        let att = candle_nn::ops::softmax(&att, candle_core::D::Minus1)?.to_dtype(act_dtype)?;
        let y = att.matmul(&v.contiguous()?)?;
        let y =
            y.transpose(1, 2)?
                .contiguous()?
                .reshape((b, seq_len, self.n_heads * self.head_dim))?;
        let y = vox_plugin_mens_candle_core::attn_math::apply_attn_output_gate(y, gate)?;
        self.o_proj
            .forward(&y)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }

    pub fn forward(
        &self,
        x: &Tensor,
        pos: usize,
        inv_freq: Option<&Tensor>,
        kv_cache: Option<&mut Option<(Tensor, Tensor)>>,
    ) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        let device = x.device();
        let (q, k, v, gate, act_dtype) = self.project_qkv_heads(x, b, seq_len)?;

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

        if seq_len > 1 && k.dim(2)? != seq_len {
            return Err(candle_core::Error::Msg(format!(
                "multi-token forward over a populated KV cache is not supported: \
                 seq_len={seq_len}, cached+new keys={}. Prefill once, then feed one token per step.",
                k.dim(2)?
            )));
        }

        let mask = if seq_len > 1 {
            Some(causal_mask(seq_len, device)?)
        } else {
            None
        };
        self.attend_and_project(
            &q,
            &k,
            &v,
            gate.as_ref(),
            mask.as_ref(),
            b,
            seq_len,
            act_dtype,
        )
    }

    /// Forward pass for packed multi-document sequences (`b == 1`, `ctx.doc_ids.len() == seq_len`).
    pub fn forward_packed_ctx(
        &self,
        x: &Tensor,
        ctx: &PackedAttnContext,
        inv_freq: Option<&Tensor>,
    ) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        if b != 1 || ctx.doc_ids.len() != seq_len {
            return Err(candle_core::Error::Msg(format!(
                "forward_packed expects b=1 and doc_ids.len()==seq_len (got b={b}, seq_len={seq_len}, doc_ids={})",
                ctx.doc_ids.len()
            )));
        }
        let (q, k, v, gate, act_dtype) = self.project_qkv_heads(x, b, seq_len)?;
        let (q, k) = if let Some(inv_freq) = inv_freq {
            self.apply_rotary_emb_with_t(&q, &k, inv_freq, ctx.local_pos_f32.clone())?
        } else {
            (q, k)
        };
        self.attend_and_project(
            &q,
            &k,
            &v,
            gate.as_ref(),
            ctx.mask.as_ref(),
            b,
            seq_len,
            act_dtype,
        )
    }

    fn apply_rotary_emb(
        &self,
        q: &Tensor,
        k: &Tensor,
        inv_freq: &Tensor,
        pos: usize,
    ) -> Result<(Tensor, Tensor)> {
        let (_b, _n_heads, seq_len, _head_dim) = q.dims4()?;
        let t = Tensor::arange(pos as u32, (pos + seq_len) as u32, q.device())?
            .to_dtype(DType::F32)?
            .reshape((seq_len, 1))?;
        self.apply_rotary_emb_with_t(q, k, inv_freq, t)
    }

    fn apply_rotary_emb_with_t(
        &self,
        q: &Tensor,
        k: &Tensor,
        inv_freq: &Tensor,
        t: Tensor,
    ) -> Result<(Tensor, Tensor)> {
        let (_b, _n_heads, seq_len, head_dim) = q.dims4()?;
        let rope_dim = inv_freq.elem_count().saturating_mul(2);
        if rope_dim == 0 || rope_dim > head_dim {
            return Err(candle_core::Error::Msg(format!(
                "RoPE inv_freq length inconsistent with head_dim: inv_freq_elems={} head_dim={head_dim}",
                inv_freq.elem_count()
            )));
        }
        let freqs = t.matmul(&inv_freq.reshape((1, inv_freq.elem_count()))?)?;
        let freqs = Tensor::cat(&[&freqs, &freqs], 1)?;
        // cos/sin are computed in F32 for precision, then cast to the activation dtype
        // (q/k) so the broadcast_mul below never mixes dtypes (no-op on the F32 path).
        let act_dtype = q.dtype();
        let cos = freqs
            .cos()?
            .reshape((1, 1, seq_len, rope_dim))?
            .to_dtype(act_dtype)?;
        let sin = freqs
            .sin()?
            .reshape((1, 1, seq_len, rope_dim))?
            .to_dtype(act_dtype)?;
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
    pub b_proj: LinearProj,
    pub a_proj: LinearProj,
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
    const DELTANET_CHUNK_SIZE: usize = 64;

    fn repeat_heads_bshd(x: &Tensor, n_rep: usize) -> Result<Tensor> {
        if n_rep == 1 {
            return Ok(x.clone());
        }
        let (b, s, h, d) = x.dims4()?;
        x.unsqueeze(3)?
            .expand((b, s, h, n_rep, d))?
            .reshape((b, s, h * n_rep, d))
    }

    fn causal_depthwise_conv_silu_inner(
        x: &Tensor,
        conv_weight: &Tensor,
        conv_state: Option<&mut Option<Tensor>>,
        doc_ids: Option<&[u32]>,
    ) -> Result<Tensor> {
        let (b, s, c) = x.dims3()?;
        let k = conv_weight.dim(1)?;
        let dev = x.device();
        if s == 0 {
            return Tensor::zeros((b, 0, c), DType::F32, dev);
        }
        // The depthwise conv accumulates in F32 for stability and returns F32; the caller
        // (linear-attention) runs its recurrence in F32 anyway. Cast inputs to F32 so the
        // BF16-activation path doesn't mix dtypes (no-op on the all-F32 path).
        let x = x.to_dtype(DType::F32)?;
        let conv_weight = conv_weight.to_dtype(DType::F32)?;

        let (full_x, offset) = if let Some(cs) = conv_state.as_deref().and_then(|opt| opt.as_ref())
        {
            let cs = cs.to_dtype(DType::F32)?;
            let full = Tensor::cat(&[&cs, &x], 1)?;
            let p = cs.dim(1)?;
            (full, p)
        } else {
            (x.clone(), 0)
        };

        let total_s = full_x.dim(1)?;
        let mut acc: Option<Tensor> = None;
        for j in 0..k {
            let mut x_lag = if offset >= j {
                full_x.narrow(1, offset - j, s)?
            } else {
                let pad_len = (j - offset).min(s);
                let valid_len = s - pad_len;
                if valid_len == 0 {
                    continue;
                }
                let zeros = Tensor::zeros((b, pad_len, c), DType::F32, dev)?;
                let valid = full_x.narrow(1, 0, valid_len)?;
                Tensor::cat(&[&zeros, &valid], 1)?
            };

            if let Some(ids) = doc_ids
                && j > 0
            {
                let mask_vec: Vec<f32> = (0..s)
                    .map(|t| {
                        if t >= j && (1..=j).all(|r| ids[t - r] == ids[t]) {
                            1.0
                        } else {
                            0.0
                        }
                    })
                    .collect();
                if mask_vec.iter().all(|&m| m == 0.0) {
                    continue;
                }
                let mask = Tensor::from_vec(mask_vec, (1, s, 1), dev)?;
                x_lag = x_lag.broadcast_mul(&mask)?;
            }

            let w_j = conv_weight.narrow(1, k - 1 - j, 1)?.reshape((1, 1, c))?;
            let prod = x_lag.broadcast_mul(&w_j)?;
            acc = Some(match acc {
                None => prod,
                Some(prev) => (prev + prod)?,
            });
        }

        if let Some(cs_ref) = conv_state {
            let keep = k.saturating_sub(1).min(total_s);
            *cs_ref = if keep > 0 {
                Some(full_x.narrow(1, total_s - keep, keep)?)
            } else {
                None
            };
        }

        let acc = match acc {
            Some(t) => t,
            None => Tensor::zeros((b, s, c), DType::F32, dev)?,
        };
        candle_nn::ops::silu(&acc)
    }

    pub fn causal_depthwise_conv_silu(
        x: &Tensor,
        conv_weight: &Tensor,
        conv_state: Option<&mut Option<Tensor>>,
    ) -> Result<Tensor> {
        Self::causal_depthwise_conv_silu_inner(x, conv_weight, conv_state, None)
    }

    pub fn causal_depthwise_conv_silu_packed(
        x: &Tensor,
        conv_weight: &Tensor,
        doc_ids: &[u32],
    ) -> Result<Tensor> {
        Self::causal_depthwise_conv_silu_inner(x, conv_weight, None, Some(doc_ids))
    }

    fn compute_doc_spans(seq_len: usize, doc_ids: Option<&[u32]>) -> Vec<(usize, usize, bool)> {
        if seq_len == 0 {
            return Vec::new();
        }
        let Some(ids) = doc_ids else {
            return vec![(0, seq_len, false)];
        };
        let mut spans = Vec::new();
        let mut start = 0usize;
        for t in 1..seq_len {
            if ids[t] != ids[t - 1] {
                spans.push((start, t, start > 0));
                start = t;
            }
        }
        spans.push((start, seq_len, start > 0));
        spans
    }

    fn invert_unit_lower_tri_pow2(a_strict: &Tensor, p: usize) -> Result<Tensor> {
        let (b, h, rows, cols) = a_strict.dims4()?;
        if rows != p || cols != p || !p.is_power_of_two() || p < 2 {
            return Err(candle_core::Error::Msg(format!(
                "invert_unit_lower_tri_pow2 expected square pow2 >= 2, got ({b}, {h}, {rows}, {cols}) p={p}"
            )));
        }
        let dev = a_strict.device();
        let b2 = b * h;
        let mut eye_p_vec = vec![0.0f32; p * p];
        for i in 0..p {
            eye_p_vec[i * p + i] = 1.0;
        }
        let eye_p = Tensor::from_vec(eye_p_vec, (1, 1, p, p), dev)?;
        let m_unit = a_strict.broadcast_add(&eye_p)?.reshape((b2, p, p))?;

        let mut inv = Tensor::ones((b2 * p, 1, 1), DType::F32, dev)?;
        let mut m = 1usize;
        while m < p {
            let q = p / (2 * m);
            let inv_pair = inv.reshape((b2 * q, 2, m, m))?;
            let inv11 = inv_pair.narrow(1, 0, 1)?.squeeze(1)?.contiguous()?;
            let inv22 = inv_pair.narrow(1, 1, 1)?.squeeze(1)?.contiguous()?;
            let m21 = if q == 1 {
                m_unit.narrow(1, m, m)?.narrow(2, 0, m)?.contiguous()?
            } else {
                let mut eye_q_vec = vec![0.0f32; q * q];
                for i in 0..q {
                    eye_q_vec[i * q + i] = 1.0;
                }
                let eye_q = Tensor::from_vec(eye_q_vec, (1, q, q, 1, 1), dev)?;
                m_unit
                    .reshape((b2, q, 2 * m, q, 2 * m))?
                    .narrow(2, m, m)?
                    .narrow(4, 0, m)?
                    .transpose(2, 3)?
                    .broadcast_mul(&eye_q)?
                    .sum(2)?
                    .reshape((b2 * q, m, m))?
                    .contiguous()?
            };
            let inv21 = inv22.matmul(&m21)?.matmul(&inv11)?.neg()?;
            let zeros = Tensor::zeros((b2 * q, m, m), DType::F32, dev)?;
            let top = Tensor::cat(&[&inv11, &zeros], 2)?;
            let bot = Tensor::cat(&[&inv21, &inv22], 2)?;
            inv = Tensor::cat(&[&top, &bot], 1)?;
            m *= 2;
        }
        inv.reshape((b, h, p, p))
    }

    fn build_chunk_masks(l: usize, device: &Device) -> Result<(Tensor, Tensor, Tensor)> {
        let mut triu_vec = vec![0.0f32; l * l];
        let mut strict_vec = vec![0.0f32; l * l];
        let mut eye_vec = vec![0.0f32; l * l];
        for row in 0..l {
            eye_vec[row * l + row] = 1.0;
            for col in 0..l {
                if row <= col {
                    triu_vec[row * l + col] = 1.0;
                }
                if row > col {
                    strict_vec[row * l + col] = 1.0;
                }
            }
        }
        Ok((
            Tensor::from_vec(triu_vec, (l, l), device)?,
            Tensor::from_vec(strict_vec, (1, 1, l, l), device)?,
            Tensor::from_vec(eye_vec, (1, 1, l, l), device)?,
        ))
    }

    fn forward_inner(
        &self,
        x: &Tensor,
        mut state_cache: Option<&mut Option<LinearStateCache>>,
        doc_ids: Option<&[u32]>,
    ) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        let device = x.device();
        let act_dtype = x.dtype();
        let qkv = self
            .qkv_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;

        let mut conv_state = state_cache
            .as_ref()
            .and_then(|sc| sc.as_ref().and_then(|s| s.conv_state.clone()));

        let mixed_qkv = if let Some(ids) = doc_ids {
            Self::causal_depthwise_conv_silu_packed(&qkv, &self.conv_weight, ids)?
        } else {
            Self::causal_depthwise_conv_silu(
                &qkv,
                &self.conv_weight,
                if state_cache.is_some() {
                    Some(&mut conv_state)
                } else {
                    None
                },
            )?
        };

        let key_dim = self.num_k_heads * self.head_k_dim;
        let value_dim = self.num_v_heads * self.head_v_dim;
        let expected_total = key_dim + key_dim + value_dim;
        let got_total = mixed_qkv.dim(candle_core::D::Minus1)?;
        if got_total != expected_total {
            return Err(candle_core::Error::Msg(format!(
                "qwen3_5 linear_attention qkv dim mismatch: expected {expected_total}, got {got_total}",
            )));
        }

        // The gated-delta-net recurrence below carries an F32 state and many F32
        // constants (g, dt_bias, a_log). Run the whole linear-attention block in F32 for
        // numerical stability regardless of the activation dtype, then cast the gated
        // output back to the activation dtype before out_proj. On the all-F32 path these
        // casts are no-ops; on the BF16 path they avoid mixing BF16 activations with the
        // F32 recurrence state (and keep the state-decay numerically sound).
        let query = mixed_qkv
            .narrow(candle_core::D::Minus1, 0, key_dim)?
            .reshape((b, seq_len, self.num_k_heads, self.head_k_dim))?
            .to_dtype(DType::F32)?;
        let key = mixed_qkv
            .narrow(candle_core::D::Minus1, key_dim, key_dim)?
            .reshape((b, seq_len, self.num_k_heads, self.head_k_dim))?
            .to_dtype(DType::F32)?;
        let value = mixed_qkv
            .narrow(candle_core::D::Minus1, key_dim + key_dim, value_dim)?
            .reshape((b, seq_len, self.num_v_heads, self.head_v_dim))?
            .to_dtype(DType::F32)?;

        let z = self
            .z_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?
            .reshape((b, seq_len, self.num_v_heads, self.head_v_dim))?
            .to_dtype(DType::F32)?;
        let beta = candle_nn::ops::sigmoid(&self.b_proj.forward(x)?.to_dtype(DType::F32)?)?
            .reshape((b, seq_len, self.num_v_heads))?;
        let a = self
            .a_proj
            .forward(x)?
            .reshape((b, seq_len, self.num_v_heads))?
            .to_dtype(DType::F32)?;

        let a_log = self.a_log.to_dtype(DType::F32)?;
        let dt_bias = self.dt_bias.to_dtype(DType::F32)?;
        let g_pre =
            (a.broadcast_add(&dt_bias.reshape((1, 1, self.num_v_heads))?)?).to_dtype(DType::F32)?;
        let g_soft = vox_plugin_mens_candle_core::attn_math::stable_softplus(&g_pre)?;
        let a_log_scale = a_log.exp()?;
        let g = g_soft
            .broadcast_mul(&a_log_scale.reshape((1, 1, self.num_v_heads))?)?
            .neg()?;

        let mut query = vox_plugin_mens_candle_core::attn_math::l2norm_last_no_div(&query, 1e-6)?;
        let mut key = vox_plugin_mens_candle_core::attn_math::l2norm_last_no_div(&key, 1e-6)?;
        if self.num_v_heads > self.num_k_heads {
            let rep = self.num_v_heads / self.num_k_heads;
            query = Self::repeat_heads_bshd(&query, rep)?;
            key = Self::repeat_heads_bshd(&key, rep)?;
        }
        let query = (query * (1.0 / (self.head_k_dim as f64).sqrt()))?;

        let mut state = match state_cache
            .as_ref()
            .and_then(|s| s.as_ref().map(|sc| &sc.recurrent_state))
        {
            Some(state_prev) => state_prev.clone(),
            None => Tensor::zeros(
                (b, self.num_v_heads, self.head_k_dim, self.head_v_dim),
                DType::F32,
                device,
            )?,
        };

        let mut y = if seq_len == 0 {
            Tensor::zeros(
                (b, 0, self.num_v_heads, self.head_v_dim),
                DType::F32,
                device,
            )?
        } else {
            let q_all = query.transpose(1, 2)?.contiguous()?;
            let k_all = key.transpose(1, 2)?.contiguous()?;
            let v_all = value.transpose(1, 2)?.contiguous()?;
            let g_all = g.transpose(1, 2)?.contiguous()?;
            let beta_all = beta.transpose(1, 2)?.contiguous()?;

            let spans = Self::compute_doc_spans(seq_len, doc_ids);
            let mut chunk_outs = Vec::new();
            let mut mask_cache: std::collections::HashMap<usize, (Tensor, Tensor, Tensor)> =
                std::collections::HashMap::new();

            for (s0, s1, reset_state) in spans {
                if reset_state {
                    state = Tensor::zeros(
                        (b, self.num_v_heads, self.head_k_dim, self.head_v_dim),
                        DType::F32,
                        device,
                    )?;
                }
                for chunk_start in (s0..s1).step_by(Self::DELTANET_CHUNK_SIZE) {
                    let l = (s1 - chunk_start).min(Self::DELTANET_CHUNK_SIZE);
                    let q_c = q_all.narrow(2, chunk_start, l)?.contiguous()?;
                    let k_c = k_all.narrow(2, chunk_start, l)?.contiguous()?;
                    let v_c = v_all.narrow(2, chunk_start, l)?.contiguous()?;
                    let g_c = g_all.narrow(2, chunk_start, l)?.contiguous()?;
                    let beta_c = beta_all.narrow(2, chunk_start, l)?.contiguous()?;

                    if l == 1 {
                        let g_scale = g_c.exp()?.reshape((b, self.num_v_heads, 1, 1))?;
                        let state_decayed = state.broadcast_mul(&g_scale)?;
                        let kv_mem = k_c.matmul(&state_decayed)?;
                        let delta = v_c
                            .broadcast_sub(&kv_mem)?
                            .broadcast_mul(&beta_c.reshape((b, self.num_v_heads, 1, 1))?)?;
                        let upd = k_c.transpose(2, 3)?.contiguous()?.matmul(&delta)?;
                        state = (state_decayed + upd)?;
                        chunk_outs.push(q_c.matmul(&state)?);
                        continue;
                    }

                    let (triu_ones_2d, strict_mask, eye_l) = match mask_cache.entry(l) {
                        std::collections::hash_map::Entry::Occupied(e) => e.into_mut().clone(),
                        std::collections::hash_map::Entry::Vacant(e) => {
                            e.insert(Self::build_chunk_masks(l, device)?).clone()
                        }
                    };

                    let gamma = g_c
                        .reshape((b * self.num_v_heads, 1, l))?
                        .broadcast_matmul(&triu_ones_2d)?
                        .reshape((b, self.num_v_heads, l))?;
                    let gamma_exp = gamma.exp()?.unsqueeze(3)?;
                    let beta_col = beta_c.unsqueeze(3)?;
                    let b_rhs = v_c
                        .broadcast_sub(&k_c.matmul(&state)?.broadcast_mul(&gamma_exp)?)?
                        .broadcast_mul(&beta_col)?;

                    let diff = gamma.unsqueeze(3)?.broadcast_sub(&gamma.unsqueeze(2)?)?;
                    let gamma_strict = diff
                        .broadcast_mul(&strict_mask)?
                        .exp()?
                        .broadcast_mul(&strict_mask)?;
                    let k_c_t = k_c.transpose(2, 3)?.contiguous()?;
                    let a_mat = k_c
                        .matmul(&k_c_t)?
                        .broadcast_mul(&gamma_strict)?
                        .broadcast_mul(&beta_col)?;

                    let inv_i_plus_a = {
                        let p = l.next_power_of_two();
                        if p == l {
                            Self::invert_unit_lower_tri_pow2(&a_mat, p)?
                        } else {
                            let pad = p - l;
                            let right_zero =
                                Tensor::zeros((b, self.num_v_heads, l, pad), DType::F32, device)?;
                            let a_wide = Tensor::cat(&[&a_mat, &right_zero], 3)?;
                            let bot_zero =
                                Tensor::zeros((b, self.num_v_heads, pad, p), DType::F32, device)?;
                            let a_padded = Tensor::cat(&[&a_wide, &bot_zero], 2)?;
                            Self::invert_unit_lower_tri_pow2(&a_padded, p)?
                                .narrow(2, 0, l)?
                                .narrow(3, 0, l)?
                                .contiguous()?
                        }
                    };

                    let u_chunk = inv_i_plus_a.matmul(&b_rhs)?;
                    let gamma_causal = gamma_strict.broadcast_add(&eye_l)?;
                    let inter = q_c.matmul(&state)?.broadcast_mul(&gamma_exp)?;
                    let intra = q_c
                        .matmul(&k_c_t)?
                        .broadcast_mul(&gamma_causal)?
                        .matmul(&u_chunk)?;
                    chunk_outs.push((inter + intra)?);

                    let gamma_last = gamma.narrow(2, l - 1, 1)?;
                    let state_decay = gamma_last.exp()?.unsqueeze(3)?;
                    let tail_decay = gamma_last.broadcast_sub(&gamma)?.exp()?.unsqueeze(3)?;
                    let state_upd = k_c_t.matmul(&u_chunk.broadcast_mul(&tail_decay)?)?;
                    state = (state.broadcast_mul(&state_decay)? + state_upd)?;
                }
            }

            Tensor::cat(&chunk_outs, 2)?.transpose(1, 2)?.contiguous()?
        };

        if let Some(ref mut sc) = state_cache {
            **sc = Some(LinearStateCache {
                recurrent_state: state,
                conv_state,
            });
        }

        let y_flat = y.reshape((b * seq_len * self.num_v_heads, self.head_v_dim))?;
        let z_flat = z.reshape((b * seq_len * self.num_v_heads, self.head_v_dim))?;
        // forward_diff = differentiable (composed) RMSNorm; the Module `forward` uses a
        // fused apply_op2_no_bwd kernel with NO backward, which silently severs gradient
        // flow through the norm (only post-norm params would train). Same numerics.
        // y_flat is already F32 here; norm weights are F32, so this normalizes in F32.
        let y_norm = self.norm.forward_diff(&y_flat)?;
        let y_gate = y_norm.broadcast_mul(&candle_nn::ops::silu(&z_flat)?)?;
        y = y_gate
            .reshape((b, seq_len, value_dim))?
            .to_dtype(act_dtype)?;

        self.out_proj
            .forward(&y)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }

    pub fn forward(
        &self,
        x: &Tensor,
        pos: usize,
        inv_freq: Option<&Tensor>,
        state_cache: Option<&mut Option<LinearStateCache>>,
    ) -> Result<Tensor> {
        let _ = (pos, inv_freq);
        self.forward_inner(x, state_cache, None)
    }

    pub fn forward_packed_ctx(&self, x: &Tensor, ctx: &PackedAttnContext) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        if b != 1 || ctx.doc_ids.len() != seq_len {
            return Err(candle_core::Error::Msg(format!(
                "qwen3_5 linear_attention forward_packed expects b=1 and doc_ids.len()==seq_len (got b={b}, seq_len={seq_len}, doc_ids={})",
                ctx.doc_ids.len()
            )));
        }
        self.forward_inner(x, None, Some(&ctx.doc_ids))
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

    pub fn forward_packed_ctx(&self, x: &Tensor, ctx: &PackedAttnContext) -> Result<Tensor> {
        let residual = x;
        let h = rms_norm_f32(&self.input_layernorm, x)?;
        let h = match &self.attention {
            Qwen35AttentionBlock::Full(a) => {
                a.forward_packed_ctx(&h, ctx, self.inv_freq.as_ref())?
            }
            Qwen35AttentionBlock::Linear(a) => a.forward_packed_ctx(&h, ctx)?,
        };
        let x = (residual + h)?;
        let residual = &x;
        let h = rms_norm_f32(&self.post_attention_layernorm, &x)?;
        let h = self.mlp.forward(&h)?;
        residual + h
    }

    pub fn forward_packed(&self, x: &Tensor, doc_ids: &[u32]) -> Result<Tensor> {
        let ctx = PackedAttnContext::new(doc_ids, x.device())?;
        self.forward_packed_ctx(x, &ctx)
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
    /// Precomputed packed-attention context when built from `forward_checkpointed_packed`.
    pub packed_ctx: Option<std::sync::Arc<PackedAttnContext>>,
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
    /// Logits `[b, seq, vocab]`, F32. Tape spans only the last segment + head.
    pub logits: Tensor,
    /// Segments in forward order. The last entry's range ends at `num_layers`.
    pub segments: Vec<CheckpointSegment>,
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
        self.head_forward(&x)
    }

    pub fn uses_linear_attention(&self) -> bool {
        self.layers
            .iter()
            .any(|l| matches!(l.attention, Qwen35AttentionBlock::Linear(_)))
    }

    /// Packed multi-document forward pass (`b == 1`, `doc_ids.len() == seq_len`).
    ///
    /// Resets RoPE positions and linear-attention recurrent/conv state per document
    /// and prevents cross-document full-attention leakage via block-diagonal causal masking.
    pub fn forward_packed(&self, input_ids: &Tensor, doc_ids: &[u32]) -> Result<Tensor> {
        let (b, seq_len) = input_ids.dims2()?;
        if b != 1 || doc_ids.len() != seq_len {
            return Err(candle_core::Error::Msg(format!(
                "forward_packed expects batch=1 and doc_ids.len()==seq_len (got b={b}, seq_len={seq_len}, doc_ids={})",
                doc_ids.len()
            )));
        }
        let ctx = PackedAttnContext::new(doc_ids, self.embed_tokens.device())?;
        let d_model = self.embed_tokens.dim(1)?;
        let ids = input_ids.flatten_all()?;
        let mut x = self
            .embed_tokens
            .index_select(&ids, 0)?
            .reshape((b, seq_len, d_model))?;
        for layer in &self.layers {
            x = layer.forward_packed_ctx(&x, &ctx)?;
        }
        self.head_forward(&x)
    }

    pub fn empty_cache(&self) -> Vec<Qwen35LayerCache> {
        self.layers
            .iter()
            .map(|l| match &l.attention {
                Qwen35AttentionBlock::Full(_) => Qwen35LayerCache::Full(None),
                Qwen35AttentionBlock::Linear(_) => Qwen35LayerCache::Linear(None),
            })
            .collect()
    }

    pub fn forward_cached(
        &self,
        input_ids: &Tensor,
        pos: usize,
        cache: &mut [Qwen35LayerCache],
    ) -> Result<Tensor> {
        self.forward_cached_inner(input_ids, pos, cache, false)
    }

    /// Cached forward returning only the final sequence position `[b, 1, vocab]`
    /// when `seq_len > 1`, avoiding a full-prompt `lm_head` matmul during prefill.
    pub fn forward_cached_last_token(
        &self,
        input_ids: &Tensor,
        pos: usize,
        cache: &mut [Qwen35LayerCache],
    ) -> Result<Tensor> {
        self.forward_cached_inner(input_ids, pos, cache, true)
    }

    fn forward_cached_inner(
        &self,
        input_ids: &Tensor,
        pos: usize,
        cache: &mut [Qwen35LayerCache],
        only_last_token: bool,
    ) -> Result<Tensor> {
        if cache.len() != self.layers.len() {
            return Err(candle_core::Error::Msg(format!(
                "KV cache has {} slots but the model has {} layers",
                cache.len(),
                self.layers.len()
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
        let x_out = if only_last_token && seq_len > 1 {
            x.narrow(1, seq_len - 1, 1)?
        } else {
            x
        };
        self.head_forward(&x_out)
    }

    /// Final norm + lm_head + F32 cast, shared by the eager and
    /// checkpointed forward so logits are byte-identical for the same input.
    fn head_forward(&self, x: &Tensor) -> Result<Tensor> {
        let x = rms_norm_f32(&self.norm, x)?;
        let logits = self
            .lm_head
            .forward(&x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        // Cast logits to F32 before they leave the model so the downstream
        // cross-entropy (log_softmax + masked gather in `forward_masked_ce`) runs
        // entirely in F32 regardless of the activation dtype. No-op on the F32 path.
        logits.to_dtype(DType::F32)
    }

    /// Run the transformer layers in `[start, end)` over `x`. Identical op
    /// sequence to [`Self::forward`]'s layer loop, so recompute reproduces the
    /// original numerics exactly (training has no KV cache / `pos` is always 0).
    fn run_layers_with(
        &self,
        x: &Tensor,
        start: usize,
        end: usize,
        packed_ctx: Option<&PackedAttnContext>,
    ) -> Result<Tensor> {
        let mut h = x.clone();
        for layer in &self.layers[start..end] {
            h = match packed_ctx {
                Some(ctx) => layer.forward_packed_ctx(&h, ctx)?,
                None => layer.forward(&h, 0, None)?,
            };
        }
        Ok(h)
    }

    fn forward_checkpointed_inner(
        &self,
        input_ids: &Tensor,
        n_segments: usize,
        packed_ctx: Option<std::sync::Arc<PackedAttnContext>>,
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
            let seg_out = self.run_layers_with(&seg_input, start, end, packed_ctx.as_deref())?;
            segments.push(CheckpointSegment {
                input: seg_input.clone(),
                start_layer: start,
                end_layer: end,
                packed_ctx: packed_ctx.clone(),
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

    /// Checkpointed forward: split the `num_layers` transformer layers into
    /// `n_segments` roughly-equal contiguous segments, detaching the activation
    /// at each boundary so the autograd tape never spans more than one segment +
    /// the head. This bounds the single-backward VRAM peak that OOMs 3B on 16GB.
    ///
    /// `n_segments` is clamped to `[1, num_layers]`.
    pub fn forward_checkpointed(
        &self,
        input_ids: &Tensor,
        n_segments: usize,
    ) -> Result<CheckpointedForward> {
        self.forward_checkpointed_inner(input_ids, n_segments, None)
    }

    /// Checkpointed packed multi-document forward pass (`b == 1`, `doc_ids.len() == seq_len`).
    pub fn forward_checkpointed_packed(
        &self,
        input_ids: &Tensor,
        doc_ids: &[u32],
        n_segments: usize,
    ) -> Result<CheckpointedForward> {
        let (b, seq_len) = input_ids.dims2()?;
        if b != 1 || doc_ids.len() != seq_len {
            return Err(candle_core::Error::Msg(format!(
                "forward_checkpointed_packed expects batch=1 and doc_ids.len()==seq_len (got b={b}, seq_len={seq_len}, doc_ids={})",
                doc_ids.len()
            )));
        }
        let packed_ctx =
            std::sync::Arc::new(PackedAttnContext::new(doc_ids, self.embed_tokens.device())?);
        self.forward_checkpointed_inner(input_ids, n_segments, Some(packed_ctx))
    }

    /// Recompute one checkpoint segment's forward from its detached boundary
    /// `input` with the tape live, returning `(input_var, seg_output)`. After
    /// backprop, `input_var`'s gradient is `dL/d(input)` — the cotangent to
    /// thread to the previous segment.
    pub fn recompute_segment(&self, seg: &CheckpointSegment) -> Result<(candle_core::Var, Tensor)> {
        let input_var = candle_core::Var::from_tensor(&seg.input)?;
        let out = self.run_layers_with(
            input_var.as_tensor(),
            seg.start_layer,
            seg.end_layer,
            seg.packed_ctx.as_deref(),
        )?;
        Ok((input_var, out))
    }

    /// Explicitly recompute one checkpoint segment under a caller-provided [`PackedAttnContext`].
    pub fn recompute_segment_packed(
        &self,
        seg: &CheckpointSegment,
        ctx: &PackedAttnContext,
    ) -> Result<(candle_core::Var, Tensor)> {
        let input_var = candle_core::Var::from_tensor(&seg.input)?;
        let out = self.run_layers_with(
            input_var.as_tensor(),
            seg.start_layer,
            seg.end_layer,
            Some(ctx),
        )?;
        Ok((input_var, out))
    }
}

#[derive(Debug, Clone)]
pub struct LinearStateCache {
    pub recurrent_state: Tensor,
    pub conv_state: Option<Tensor>,
}

#[derive(Debug, Clone)]
pub enum Qwen35LayerCache {
    Full(Option<(Tensor, Tensor)>),
    Linear(Option<LinearStateCache>),
}

// ── CandleModel: the opaque handle stored across plugin calls ─────────────────

/// Opaque model handle stored by the plugin. In the current SP3 stub, `load_from_path`
/// returns an error — the actual construction requires `QloraEmbedBundle` from
/// vox-populi's preflight logic. Batch 3 wires vox-populi to construct the model and
/// pass it to the plugin via an alternative init path.
pub struct CandleModel {
    pub engine: std::sync::Mutex<crate::inference::InferenceEngine>,
    #[allow(dead_code)]
    pub model_path: String,
    #[allow(dead_code)]
    pub trainer: Option<qlora_rs::training::QLoraTrainer>,
}

impl CandleModel {
    pub fn load_from_path(model_path: &str) -> anyhow::Result<Self> {
        let path = std::path::Path::new(model_path);
        let engine =
            crate::inference::InferenceEngine::load(path, &crate::device::DeviceKind::Best)?;
        Ok(Self {
            engine: std::sync::Mutex::new(engine),
            model_path: model_path.to_string(),
            trainer: None,
        })
    }
}

#[cfg(test)]
mod gradient_checkpoint_tests {
    //! End-to-end correctness for activation/gradient checkpointing on the real
    //! `Qwen35Model`: a checkpointed segmented backward must produce the **same**
    //! LoRA gradients as an eager full-graph backward. CPU/F32 so it runs in
    //! normal `cargo test` (no CUDA). A silently-wrong backward is the worst
    //! possible outcome, so this is the load-bearing safety gate.

    use super::*;
    use candle_core::Var;
    use candle_nn::{VarBuilder, VarMap};
    use qlora_rs::QLoraConfig;

    pub(super) fn qcfg() -> QLoraConfig {
        let mut c = QLoraConfig::preset_all_bf16(4, 8);
        c.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
        c.cache_dequantized = true;
        c
    }

    pub(super) fn qlin(vb: VarBuilder, d_out: usize, d_in: usize, dev: &Device) -> QuantizedLinear {
        let w = Tensor::randn(0f32, 0.02f32, (d_out, d_in), dev).unwrap();
        QuantizedLinear::from_weight_with_varbuilder(&w, None, &qcfg(), vb).unwrap()
    }

    pub(super) fn build_layer(
        vb: VarBuilder,
        d: usize,
        n_heads: usize,
        dev: &Device,
    ) -> Qwen35Layer {
        let head_dim = d / n_heads;
        let attn = Qwen2Attention {
            q_proj: qlin(vb.pp("q"), d, d, dev),
            k_proj: qlin(vb.pp("k"), d, d, dev),
            v_proj: qlin(vb.pp("v"), d, d, dev),
            o_proj: qlin(vb.pp("o"), d, d, dev),
            q_bias: None,
            k_bias: None,
            v_bias: None,
            q_norm: None,
            k_norm: None,
            n_heads,
            n_kv_heads: n_heads,
            head_dim,
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
}

#[cfg(test)]
mod bf16_activation_tests {
    //! Tests for the "BF16 bundle": activations follow the configured compute dtype so
    //! the forward stack runs in BF16 on CUDA (halving activation VRAM) while staying F32
    //! on CPU (where BF16 matmul is unsupported in this Candle build). The key property is
    //! that the model is now *dtype-following*: a forward pass on a BF16 activation stream
    //! produces a BF16 output (no silent cast back to F32 by the qlora base matmul) AND
    //! stays numerically finite, while the F32 path is byte-for-byte preserved.

    use super::gradient_checkpoint_tests::{build_layer, qcfg, qlin};
    use super::*;
    use candle_nn::{VarBuilder, VarMap};
    use qlora_rs::QLoraConfig;
    use qlora_rs::qlora::QuantizedLinear;
    use qlora_rs::quantization::ComputeDType;

    /// Build a tiny single-head `Qwen2Attention` whose base weights are quantized at the
    /// given compute dtype.
    fn tiny_attention(device: &Device, compute: ComputeDType) -> Qwen2Attention {
        let d = 8usize; // d_model == n_heads * head_dim
        let mut cfg = QLoraConfig::preset_all_bf16(4, 8);
        cfg.quantization.compute_dtype = compute;
        cfg.cache_dequantized = false;
        let w = Tensor::randn(0f32, 0.02f32, (d, d), device).unwrap();
        let mk = || QuantizedLinear::from_weight(&w, None, &cfg, device).unwrap();
        Qwen2Attention {
            q_proj: mk(),
            k_proj: mk(),
            v_proj: mk(),
            o_proj: mk(),
            // F32 biases on purpose: the forward must cast them to the activation dtype.
            q_bias: Some(Tensor::zeros(d, DType::F32, device).unwrap()),
            k_bias: Some(Tensor::zeros(d, DType::F32, device).unwrap()),
            v_bias: Some(Tensor::zeros(d, DType::F32, device).unwrap()),
            q_norm: None,
            k_norm: None,
            n_heads: 2,
            n_kv_heads: 2,
            head_dim: 4,
        }
    }

    /// Dense Qwen3's per-head q_norm/k_norm must actually change the forward
    /// output when present — confirmed load-bearing on a real Qwen/Qwen3-0.6B
    /// checkpoint this session: omitting it produced fluent-looking garbage
    /// text, not a crash. Builds two attention blocks from the SAME
    /// deterministic weight tensors (quantization is deterministic given the
    /// same input) so the only difference is q_norm/k_norm's presence.
    #[test]
    fn qk_norm_changes_output_when_present() {
        let device = Device::Cpu;
        let d = 8usize;
        let mut cfg = qlora_rs::QLoraConfig::preset_all_bf16(4, 8);
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

    /// q_norm/k_norm are applied exactly once, per head, before RoPE (HF Qwen3:
    /// `q_norm(q_proj(x).view(.., head_dim))`). RMSNorm with a non-uniform weight is not
    /// idempotent, so applying it twice changes the output; compare against a reference.
    #[test]
    fn qk_norm_is_applied_exactly_once() {
        use candle_nn::Module as _;
        let device = Device::Cpu;
        let (d, heads, hd, seq) = (8usize, 2usize, 4usize, 3usize);
        let mut cfg = QLoraConfig::preset_all_bf16(4, 8);
        cfg.quantization.compute_dtype = ComputeDType::F32;
        let w = Tensor::arange(0u32, (d * d) as u32, &device)
            .unwrap()
            .to_dtype(DType::F32)
            .unwrap()
            .reshape((d, d))
            .unwrap()
            .affine(0.01, 0.0)
            .unwrap();
        let lin = || QuantizedLinear::from_weight(&w, None, &cfg, &device).unwrap();
        let norm_w = Tensor::new(&[2.0f32, 0.5, 3.0, 1.5], &device).unwrap();
        let norm = || RmsNorm::new(norm_w.clone(), 1e-6);
        let attn = Qwen2Attention {
            q_proj: lin(),
            k_proj: lin(),
            v_proj: lin(),
            o_proj: lin(),
            q_bias: None,
            k_bias: None,
            v_bias: None,
            n_heads: heads,
            n_kv_heads: heads,
            head_dim: hd,
            q_norm: Some(norm()),
            k_norm: Some(norm()),
        };
        let x = Tensor::randn(0f32, 1f32, (1, seq, d), &device).unwrap();
        let got = attn.forward(&x, 0, None, None).unwrap();

        // Reference: one per-head RMSNorm on q and k, causal softmax attention, o_proj.
        let per_head = |t: Tensor, n: Option<RmsNorm>| {
            let t = t.reshape((1, seq, heads, hd)).unwrap();
            let t = n.map_or(t.clone(), |n| n.forward(&t).unwrap());
            t.transpose(1, 2).unwrap().contiguous().unwrap()
        };
        let q = per_head(attn.q_proj.forward(&x).unwrap(), Some(norm()));
        let k = per_head(attn.k_proj.forward(&x).unwrap(), Some(norm()));
        let v = per_head(attn.v_proj.forward(&x).unwrap(), None);
        let kt = k.transpose(2, 3).unwrap().contiguous().unwrap();
        let att = (q.matmul(&kt).unwrap() * (1.0 / (hd as f64).sqrt())).unwrap();
        let att = att
            .broadcast_add(&causal_mask(seq, &device).unwrap())
            .unwrap();
        let att = candle_nn::ops::softmax(&att, candle_core::D::Minus1).unwrap();
        let y = att
            .matmul(&v)
            .unwrap()
            .transpose(1, 2)
            .unwrap()
            .contiguous()
            .unwrap()
            .reshape((1, seq, d))
            .unwrap();
        let want = attn.o_proj.forward(&y).unwrap();
        let diff = (got - want)
            .unwrap()
            .abs()
            .unwrap()
            .flatten_all()
            .unwrap()
            .max(0)
            .unwrap()
            .to_scalar::<f32>()
            .unwrap();
        assert!(
            diff < 1e-4,
            "max |forward - single-norm reference| = {diff}"
        );
    }

    /// CPU path: F32 activations in → F32 out, finite. This must NOT regress (BF16 matmul
    /// is unsupported on CPU, so the F32 path is what every CPU unit test exercises).
    #[test]
    fn f32_activations_preserved_on_cpu() {
        let device = Device::Cpu;
        let attn = tiny_attention(&device, ComputeDType::F32);
        // [batch=1, seq=3, d_model=8]
        let x = Tensor::randn(0f32, 1f32, (1, 3, 8), &device).unwrap();
        let out = attn.forward(&x, 0, None, None).unwrap();
        assert_eq!(out.dtype(), DType::F32, "F32 activation path must stay F32");
        let v = out.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        assert!(
            v.iter().all(|f| f.is_finite()),
            "F32 forward must be finite"
        );
    }

    /// `rms_norm_f32` is the numerically-stable RMSNorm wrapper: it preserves the input
    /// dtype but reduces in F32. On the F32 path it is exactly `forward_diff`.
    #[test]
    fn rms_norm_f32_preserves_dtype_and_matches_forward_diff_on_f32() {
        let device = Device::Cpu;
        let w = Tensor::ones(8, DType::F32, &device).unwrap();
        let norm = RmsNorm::new(w, 1e-6);
        let x = Tensor::randn(0f32, 1f32, (1, 3, 8), &device).unwrap();
        let a = rms_norm_f32(&norm, &x).unwrap();
        let b = norm.forward_diff(&x).unwrap();
        assert_eq!(a.dtype(), DType::F32);
        let (av, bv) = (
            a.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            b.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
        );
        for (x, y) in av.iter().zip(bv.iter()) {
            assert!(
                (x - y).abs() < 1e-6,
                "rms_norm_f32 must match forward_diff on F32"
            );
        }
    }

    /// CUDA path: BF16 activations propagate through the attention block (the output is
    /// BF16, proving the qlora base matmul no longer casts back to F32) and stay finite.
    /// Skips gracefully when no CUDA device is available.
    #[cfg(feature = "cuda")]
    #[test]
    fn bf16_activations_propagate_and_finite_on_cuda() {
        let device = match Device::new_cuda(0) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("skipping: no CUDA device");
                return;
            }
        };
        let attn = tiny_attention(&device, ComputeDType::BF16);
        let x = Tensor::randn(0f32, 1f32, (1, 3, 8), &device)
            .unwrap()
            .to_dtype(DType::BF16)
            .unwrap();
        let out = attn.forward(&x, 0, None, None).unwrap();
        assert_eq!(
            out.dtype(),
            DType::BF16,
            "BF16 activations must propagate (no cast back to F32)"
        );
        let v = out
            .to_dtype(DType::F32)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        assert!(
            v.iter().all(|f| f.is_finite()),
            "BF16 forward must be finite"
        );
    }

    #[test]
    fn test_lm_head_quantization_with_bf16_dtype() {
        let device = Device::Cpu;
        let mut config = QLoraConfig::default();
        config.quantization.compute_dtype = ComputeDType::BF16;
        let varmap = candle_nn::VarMap::new();
        let vb = candle_nn::VarBuilder::from_varmap(&varmap, DType::F32, &device);

        let wte = Tensor::zeros(&[8, 8], DType::BF16, &device).unwrap();
        // Mimic the fix: cast to F32 first
        let w_lm = wte.to_dtype(DType::F32).unwrap();
        let lm_head =
            QuantizedLinear::from_weight_with_varbuilder(&w_lm, None, &config, vb.pp("lm_head"));
        assert!(
            lm_head.is_ok(),
            "lm_head building with F32 cast should succeed but failed: {:?}",
            lm_head.err()
        );
    }

    /// Q/K RMSNorm must be applied exactly once, before RoPE. A second
    /// application is invisible with a uniform norm weight (RMSNorm of an
    /// already-normalised vector times a constant is idempotent), so this uses
    /// a non-uniform weight like real Qwen3 checkpoints have, and compares the
    /// forward pass against a reference built from the attention's own
    /// projections with a single `rms_norm_f32` per side.
    #[test]
    fn qk_norm_matches_single_rms_norm_f32_reference() {
        let device = Device::Cpu;
        let (d, n_heads, head_dim) = (8usize, 2usize, 4usize);
        let mut cfg = QLoraConfig::preset_all_bf16(4, 8);
        cfg.quantization.compute_dtype = ComputeDType::F32;
        let w = Tensor::arange(0u32, (d * d) as u32, &device)
            .unwrap()
            .to_dtype(DType::F32)
            .unwrap()
            .reshape((d, d))
            .unwrap()
            .affine(0.01, 0.0)
            .unwrap();
        let mk = || QuantizedLinear::from_weight(&w, None, &cfg, &device).unwrap();
        let norm_w = Tensor::new(&[2.0f32, 0.5, 3.0, 1.5], &device).unwrap();
        let attn = Qwen2Attention {
            q_proj: mk(),
            k_proj: mk(),
            v_proj: mk(),
            o_proj: mk(),
            q_bias: None,
            k_bias: None,
            v_bias: None,
            n_heads,
            n_kv_heads: n_heads,
            head_dim,
            q_norm: Some(RmsNorm::new(norm_w.clone(), 1e-6)),
            k_norm: Some(RmsNorm::new(norm_w, 1e-6)),
        };

        let x = Tensor::randn(0f32, 1f32, (1, 3, d), &device).unwrap();
        let actual = attn.forward(&x, 0, None, None).unwrap();

        // Reference: same projections, one norm per side, no RoPE (inv_freq = None),
        // n_heads == n_kv_heads so repeat_kv is the identity.
        let (b, s, _) = x.dims3().unwrap();
        let q = attn
            .q_proj
            .forward(&x)
            .unwrap()
            .reshape((b, s, n_heads, head_dim))
            .unwrap();
        let q = rms_norm_f32(attn.q_norm.as_ref().unwrap(), &q)
            .unwrap()
            .transpose(1, 2)
            .unwrap();
        let k = attn
            .k_proj
            .forward(&x)
            .unwrap()
            .reshape((b, s, n_heads, head_dim))
            .unwrap();
        let k = rms_norm_f32(attn.k_norm.as_ref().unwrap(), &k)
            .unwrap()
            .transpose(1, 2)
            .unwrap();
        let v = attn
            .v_proj
            .forward(&x)
            .unwrap()
            .reshape((b, s, n_heads, head_dim))
            .unwrap()
            .transpose(1, 2)
            .unwrap();
        let scale = 1.0 / (head_dim as f64).sqrt();
        let att = (q
            .contiguous()
            .unwrap()
            .matmul(&k.transpose(2, 3).unwrap().contiguous().unwrap())
            .unwrap()
            * scale)
            .unwrap();
        let att = att
            .broadcast_add(&causal_mask(s, &device).unwrap())
            .unwrap();
        let att = candle_nn::ops::softmax(&att, candle_core::D::Minus1).unwrap();
        let y = att
            .matmul(&v.contiguous().unwrap())
            .unwrap()
            .transpose(1, 2)
            .unwrap()
            .contiguous()
            .unwrap()
            .reshape((b, s, n_heads * head_dim))
            .unwrap();
        let expected = attn.o_proj.forward(&y).unwrap();

        let (a, e) = (
            actual.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            expected.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
        );
        assert_eq!(a.len(), e.len());
        for (i, (av, ev)) in a.iter().zip(e.iter()).enumerate() {
            assert!(
                (av - ev).abs() < 1e-4,
                "element {i}: forward={av} reference={ev} — Q/K norm is not applied exactly once"
            );
        }
    }

    fn build_hybrid_layer(vb: VarBuilder, d: usize, n_heads: usize, dev: &Device) -> Qwen35Layer {
        let head_dim = d / n_heads;
        let qkv_dim = d * 3;
        let conv_k = 3;
        let linear = Qwen35LinearAttention {
            qkv_proj: qlin(vb.pp("qkv"), qkv_dim, d, dev),
            z_proj: qlin(vb.pp("z"), d, d, dev),
            b_proj: LinearProj::from_tensor(
                Tensor::randn(0f32, 0.2f32, (n_heads, d), dev).unwrap(),
            )
            .unwrap(),
            a_proj: LinearProj::from_tensor(
                Tensor::randn(0f32, 0.2f32, (n_heads, d), dev).unwrap(),
            )
            .unwrap(),
            out_proj: qlin(vb.pp("out"), d, d, dev),
            conv_weight: Tensor::randn(0f32, 0.2f32, (qkv_dim, conv_k), dev).unwrap(),
            a_log: Tensor::zeros(n_heads, DType::F32, dev).unwrap(),
            dt_bias: Tensor::zeros(n_heads, DType::F32, dev).unwrap(),
            norm: RmsNorm::new(Tensor::ones(head_dim, DType::F32, dev).unwrap(), 1e-6),
            num_k_heads: n_heads,
            num_v_heads: n_heads,
            head_k_dim: head_dim,
            head_v_dim: head_dim,
        };
        let mlp = Qwen2MLP {
            gate_proj: qlin(vb.pp("g"), d * 2, d, dev),
            up_proj: qlin(vb.pp("u"), d * 2, d, dev),
            down_proj: qlin(vb.pp("dn"), d, d * 2, dev),
        };
        Qwen35Layer {
            input_layernorm: RmsNorm::new(Tensor::ones(d, DType::F32, dev).unwrap(), 1e-6),
            attention: Qwen35AttentionBlock::Linear(linear),
            post_attention_layernorm: RmsNorm::new(Tensor::ones(d, DType::F32, dev).unwrap(), 1e-6),
            mlp,
            inv_freq: None,
        }
    }

    fn build_hybrid_model(vb: VarBuilder, d: usize, vocab: usize, dev: &Device) -> Qwen35Model {
        let embed = Tensor::randn(0f32, 0.2f32, (vocab, d), dev).unwrap();
        let mut full_layer = build_layer(vb.pp("layer0"), d, 2, dev);
        let head_dim = d / 2;
        full_layer.inv_freq = Some(
            vox_plugin_mens_candle_core::rope::synthesize_rope_inv_freq(head_dim, None, dev)
                .unwrap(),
        );
        let lin_layer = build_hybrid_layer(vb.pp("layer1"), d, 2, dev);
        let lm_head = QuantizedLinear::from_weight_with_varbuilder(
            &embed.clone(),
            None,
            &qcfg(),
            vb.pp("lm_head"),
        )
        .unwrap();
        Qwen35Model {
            embed_tokens: embed,
            layers: vec![lin_layer, full_layer],
            norm: RmsNorm::new(Tensor::ones(d, DType::F32, dev).unwrap(), 1e-6),
            lm_head,
        }
    }

    #[test]
    fn linear_proj_f32_weight_t_matches_matmul_and_handles_bf16() {
        let dev = Device::Cpu;
        let w = Tensor::new(&[[1.0f32, 2.0, -1.0], [0.5, -1.5, 3.0]], &dev).unwrap();
        let proj = LinearProj::from_tensor(w.clone()).unwrap();
        let x = Tensor::new(&[2.0f32, -1.0, 4.0, 0.5, 1.0, -2.0], &dev)
            .unwrap()
            .reshape((1, 2, 3))
            .unwrap();
        let y_f32 = proj.forward(&x).unwrap();
        let y_bf16 = proj.forward(&x.to_dtype(DType::BF16).unwrap()).unwrap();
        let expected = x.broadcast_matmul(&w.t().unwrap()).unwrap();
        let got_f32 = y_f32.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        let got_bf16 = y_bf16.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        let exp_v = expected.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        for (i, (a, b)) in got_f32.iter().zip(exp_v.iter()).enumerate() {
            assert!((a - b).abs() < 1e-5, "f32 idx {i}: {a} vs {b}");
        }
        for (i, (a, b)) in got_bf16.iter().zip(exp_v.iter()).enumerate() {
            assert!((a - b).abs() < 2e-2, "bf16 idx {i}: {a} vs {b}");
        }
    }

    #[test]
    fn incremental_decode_on_hybrid_model_matches_full_forward() {
        let dev = Device::Cpu;
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &dev);
        let model = build_hybrid_model(vb, 8, 16, &dev);

        let full_tokens: Vec<u32> = vec![1, 4, 2, 7, 5, 9];
        let full_ids = Tensor::new(&full_tokens[..], &dev)
            .unwrap()
            .unsqueeze(0)
            .unwrap();
        let full_logits = model.forward(&full_ids).unwrap();
        let full_last = full_logits
            .narrow(1, full_tokens.len() - 1, 1)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();

        let mut cache = model.empty_cache();
        let prefill_len = 3;
        let prefill = Tensor::new(&full_tokens[..prefill_len], &dev)
            .unwrap()
            .unsqueeze(0)
            .unwrap();
        let prefill_logits = model
            .forward_cached_last_token(&prefill, 0, &mut cache)
            .unwrap();
        assert_eq!(prefill_logits.dims(), &[1, 1, 16]);
        assert!(matches!(cache[0], Qwen35LayerCache::Linear(Some(_))));

        let mut last_logits = Vec::new();
        for (step, &tok) in full_tokens[prefill_len..].iter().enumerate() {
            let t_in = Tensor::new(&[tok], &dev).unwrap().unsqueeze(0).unwrap();
            let step_logits = model
                .forward_cached_last_token(&t_in, prefill_len + step, &mut cache)
                .unwrap();
            last_logits = step_logits.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        }

        assert_eq!(last_logits.len(), full_last.len());
        for (i, (a, b)) in last_logits.iter().zip(full_last.iter()).enumerate() {
            assert!(
                (a - b).abs() < 1e-4,
                "vocab idx {i}: cached={a} full={b} mismatch"
            );
        }
    }

    #[test]
    fn packed_forward_on_hybrid_model_isolates_documents_and_matches_single_doc() {
        let dev = Device::Cpu;
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &dev);
        let model = build_hybrid_model(vb, 8, 16, &dev);

        let doc_a = [1u32, 3, 5, 2];
        let doc_b = [6u32, 4, 8];
        let mut packed = Vec::new();
        packed.extend_from_slice(&doc_a);
        packed.extend_from_slice(&doc_b);
        let doc_ids = vec![0u32, 0, 0, 0, 1, 1, 1];

        let t_a = Tensor::new(&doc_a[..], &dev).unwrap().unsqueeze(0).unwrap();
        let t_b = Tensor::new(&doc_b[..], &dev).unwrap().unsqueeze(0).unwrap();
        let t_p = Tensor::new(&packed[..], &dev)
            .unwrap()
            .unsqueeze(0)
            .unwrap();

        let ref_a = model
            .forward(&t_a)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let ref_b = model
            .forward(&t_b)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let packed_logits = model.forward_packed(&t_p, &doc_ids).unwrap();
        let ckpt_packed = model
            .forward_checkpointed_packed(&t_p, &doc_ids, 2)
            .unwrap();

        let out_a = packed_logits
            .narrow(1, 0, doc_a.len())
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let out_b = packed_logits
            .narrow(1, doc_a.len(), doc_b.len())
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let ckpt_v = ckpt_packed
            .logits
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let packed_v = packed_logits
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();

        for (i, (a, b)) in out_a.iter().zip(ref_a.iter()).enumerate() {
            assert!((a - b).abs() < 1e-4, "docA[{i}]: packed={a} ref={b}");
        }
        for (i, (a, b)) in out_b.iter().zip(ref_b.iter()).enumerate() {
            assert!((a - b).abs() < 1e-4, "docB[{i}]: packed={a} ref={b}");
        }
        for (i, (a, b)) in ckpt_v.iter().zip(packed_v.iter()).enumerate() {
            assert!((a - b).abs() < 1e-4, "ckpt_packed[{i}]: ckpt={a} ref={b}");
        }
    }
}
