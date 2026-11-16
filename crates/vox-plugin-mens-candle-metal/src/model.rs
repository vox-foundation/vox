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

// A model holds a handful of these per layer (tens in total) and QLora is the common
// variant, so the size gap costs nothing measurable; boxing would add a pointer hop to
// every forward.
#[allow(clippy::large_enum_variant)]
pub enum QuantizedLinear {
    QLora(Box<qlora_rs::qlora::QuantizedLinear>),
    QLoraPadded {
        inner: Box<qlora_rs::qlora::QuantizedLinear>,
        gemm_rows: usize,
    },
    QLoraF32Head(Box<qlora_rs::qlora::QuantizedLinear>),
    Dense {
        weight_t: Tensor,
        bias: Option<Tensor>,
    },
    QMatMul(QMatMul),
    Unquantized(Tensor),
}

/// Target row count when padding decode rows up to `gemm_rows` for width-invariant GEMM.
#[cfg(test)]
#[must_use]
pub fn padded_rows(rows: usize, gemm_rows: usize) -> usize {
    qlora_rs::qlora::QuantizedLinear::padded_rows(rows, gemm_rows)
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
        Ok(Self::QLora(Box::new(l)))
    }

    pub fn from_weight_padded(
        weight: &Tensor,
        bias: Option<Tensor>,
        config: &qlora_rs::QLoraConfig,
        device: &Device,
        gemm_rows: usize,
    ) -> Result<Self> {
        let l = qlora_rs::qlora::QuantizedLinear::from_weight(weight, bias, config, device)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        Ok(Self::QLoraPadded {
            inner: Box::new(l),
            gemm_rows,
        })
    }

    pub fn from_weight_f32_head(
        weight: &Tensor,
        bias: Option<Tensor>,
        config: &qlora_rs::QLoraConfig,
        device: &Device,
    ) -> Result<Self> {
        let l = qlora_rs::qlora::QuantizedLinear::from_weight(weight, bias, config, device)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        Ok(Self::QLoraF32Head(Box::new(l)))
    }

    pub fn from_weight_frozen(
        weight: &Tensor,
        bias: Option<Tensor>,
        config: &qlora_rs::QLoraConfig,
        device: &Device,
    ) -> Result<Self> {
        let l = qlora_rs::qlora::QuantizedLinear::from_weight_frozen(weight, bias, config, device)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
        Ok(Self::QLora(Box::new(l)))
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
        Ok(Self::QLora(Box::new(l)))
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
            Self::QLoraPadded { inner, gemm_rows } => inner
                .forward_padded_gemm(x, *gemm_rows)
                .map_err(|e| candle_core::Error::Msg(e.to_string())),
            Self::QLoraF32Head(ql) => ql
                .forward_f32_head(x)
                .map_err(|e| candle_core::Error::Msg(e.to_string())),
            Self::Dense { weight_t, bias } => {
                let in_dtype = x.dtype();
                let w_dtype = weight_t.dtype();
                let base_out = if x.dims().len() == 3 {
                    let (b, s, k) = x.dims3()?;
                    let lhs = if w_dtype != in_dtype {
                        x.reshape((b * s, k))?.to_dtype(w_dtype)?
                    } else {
                        x.reshape((b * s, k))?
                    };
                    let out_f = weight_t.dim(1)?;
                    let mut out = lhs.matmul(weight_t)?.reshape((b, s, out_f))?;
                    if w_dtype != in_dtype {
                        out = out.to_dtype(in_dtype)?;
                    }
                    out
                } else {
                    let lhs = if w_dtype != in_dtype {
                        x.to_dtype(w_dtype)?
                    } else {
                        x.clone()
                    };
                    let mut out = lhs.matmul(weight_t)?;
                    if w_dtype != in_dtype {
                        out = out.to_dtype(in_dtype)?;
                    }
                    out
                };
                match bias {
                    Some(b) => {
                        if b.dtype() != base_out.dtype() {
                            base_out.broadcast_add(&b.to_dtype(base_out.dtype())?)
                        } else {
                            base_out.broadcast_add(b)
                        }
                    }
                    None => Ok(base_out),
                }
            }
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

    fn build_dense_from_qlora(ql: &qlora_rs::qlora::QuantizedLinear) -> Result<Self> {
        if !ql.is_lora_zero() {
            let (_a, b) = ql.lora_weights();
            let b_sq = b
                .to_dtype(DType::F32)?
                .sqr()?
                .sum_all()?
                .to_scalar::<f32>()?;
            if b_sq > 0.0 {
                return Err(candle_core::Error::Msg(
                    "cannot collapse QLora layer with non-zero LoRA B weights into Dense"
                        .to_string(),
                ));
            }
        }
        let Some(w) = ql.cached_weight() else {
            return Err(candle_core::Error::Msg(
                "into_inference requires cached dequantized weight".to_string(),
            ));
        };
        let weight_t = w.t()?.contiguous()?;
        Ok(Self::Dense {
            weight_t,
            bias: ql.bias().cloned(),
        })
    }

    /// Collapse a QLoRA linear layer into a contiguous pre-transposed dense weight for inference.
    pub fn into_inference(self) -> Result<Self> {
        match &self {
            Self::QLora(ql) | Self::QLoraPadded { inner: ql, .. } | Self::QLoraF32Head(ql) => {
                Self::build_dense_from_qlora(ql)
            }
            Self::Dense { .. } | Self::QMatMul(_) | Self::Unquantized(_) => Ok(self),
        }
    }
}

fn convert_slot_to_inference(slot: &mut QuantizedLinear) -> Result<usize> {
    let dense = match slot {
        QuantizedLinear::QLora(ql)
        | QuantizedLinear::QLoraPadded { inner: ql, .. }
        | QuantizedLinear::QLoraF32Head(ql) => QuantizedLinear::build_dense_from_qlora(ql)?,
        QuantizedLinear::Dense { .. }
        | QuantizedLinear::QMatMul(_)
        | QuantizedLinear::Unquantized(_) => return Ok(0),
    };
    *slot = dense;
    Ok(1)
}

impl From<qlora_rs::qlora::QuantizedLinear> for QuantizedLinear {
    fn from(l: qlora_rs::qlora::QuantizedLinear) -> Self {
        Self::QLora(Box::new(l))
    }
}

impl From<QMatMul> for QuantizedLinear {
    fn from(qmm: QMatMul) -> Self {
        Self::QMatMul(qmm)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

thread_local! {
    static INFERENCE_FAST_PATH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

struct InferenceFastPathGuard {
    prev: bool,
}

impl Drop for InferenceFastPathGuard {
    fn drop(&mut self) {
        INFERENCE_FAST_PATH.with(|c| c.set(self.prev));
    }
}

/// Run `f` under `candle_core::with_no_grad` with fused Metal inference kernels
/// (`rmsnorm`, `rope`, GQA `sdpa`) enabled. Kept separate from training's
/// `forward_checkpointed` `with_no_grad` segments so checkpointed forward and
/// backward recompute remain op-identical.
pub fn with_inference_fast_path<F: FnOnce() -> R, R>(f: F) -> R {
    let prev = INFERENCE_FAST_PATH.with(|c| {
        let p = c.get();
        c.set(true);
        p
    });
    let _guard = InferenceFastPathGuard { prev };
    candle_core::with_no_grad(f)
}

#[inline]
fn is_inference_fast_path() -> bool {
    INFERENCE_FAST_PATH.with(|c| c.get())
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

/// Per-token local positions (`0..len_i`) within a packed sequence of contiguous
/// document IDs, resetting to `0` at every document boundary.
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

/// Precomputed packed-sequence RoPE position and causal-mask context shared
/// across every layer and checkpoint recomputation in a single packed forward.
pub struct PackedAttnContext {
    pub doc_ids: Vec<u32>,
    /// Per-token local positions `(seq_len, 1)` in `F32`.
    pub local_pos_f32: Tensor,
    /// Block-diagonal causal mask `(1, 1, seq_len, seq_len)` (`F32`) or `None` when `seq_len <= 1`.
    pub mask: Option<Tensor>,
}

impl PackedAttnContext {
    /// Validate that `doc_ids` form contiguous document runs (rejecting non-contiguous
    /// duplicate IDs) and precompute `local_pos_f32` + `mask` once on `device`.
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

/// Differentiable RMSNorm with an F32-stable reduction that preserves the activation
/// dtype. candle's `RmsNorm::forward_diff` upcasts BF16/F16 to F32 for the variance, but
/// casts the normalized result back to the *input* dtype before multiplying by the (F32)
/// norm weight — which would mix dtypes on the BF16-activation path. We sidestep that by
/// running `forward_diff` in F32 (norm weights are F32) and casting the result back to the
/// input's activation dtype. On the all-F32 path both casts are no-ops.
fn rms_norm_f32(norm: &RmsNorm, x: &Tensor) -> Result<Tensor> {
    let in_dtype = x.dtype();
    if is_inference_fast_path() && x.device().is_metal() {
        let x_f32 = if in_dtype == DType::F32 {
            x.contiguous()?
        } else {
            x.to_dtype(DType::F32)?.contiguous()?
        };
        let out = candle_core::Module::forward(norm, &x_f32)?;
        return if in_dtype == DType::F32 {
            Ok(out)
        } else {
            out.to_dtype(in_dtype)
        };
    }
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

enum KvSlot<'a> {
    Tuple(&'a mut Option<(Tensor, Tensor)>),
    Buffered(&'a mut Option<FullKvCache>),
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
        self.forward_inner(x, pos, inv_freq, kv_cache.map(KvSlot::Tuple))
    }

    /// Cached forward backed by pre-allocated [`FullKvCache`] buffers (`slice_set`
    /// instead of per-token `Tensor::cat` reallocation).
    pub fn forward_kv_buf(
        &self,
        x: &Tensor,
        pos: usize,
        inv_freq: Option<&Tensor>,
        kv_cache: Option<&mut Option<FullKvCache>>,
    ) -> Result<Tensor> {
        self.forward_inner(x, pos, inv_freq, kv_cache.map(KvSlot::Buffered))
    }

    pub fn forward_rows(
        &self,
        x: &Tensor,
        pos_f32: &Tensor,
        inv_freq: Option<&Tensor>,
        caches: &mut [&mut Option<FullKvCache>],
    ) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        let (pos_b, pos_cols) = pos_f32.dims2()?;
        if seq_len != 1 || pos_b != b || pos_cols != 1 || b != caches.len() {
            return Err(candle_core::Error::Msg(format!(
                "forward_rows expects seq_len=1 and matching batch={b}, pos_f32=[{pos_b}, {pos_cols}], caches={}",
                caches.len()
            )));
        }
        let (q, k, v) = self.project_qkv(x)?;
        let device = q.device().clone();
        let act_dtype = q.dtype();
        let (q, gate) = vox_plugin_mens_candle_core::attn_math::split_q_and_gate(
            q,
            b,
            1,
            self.n_heads,
            self.head_dim,
        )?;

        let q = q.reshape((b, 1, self.n_heads, self.head_dim))?;
        let q = match &self.q_norm {
            Some(norm) => rms_norm_f32(norm, &q)?,
            None => q,
        };
        let q = q
            .contiguous()?
            .reshape((b, self.n_heads, 1, self.head_dim))?;

        let k = k.reshape((b, 1, self.n_kv_heads, self.head_dim))?;
        let k = match &self.k_norm {
            Some(norm) => rms_norm_f32(norm, &k)?,
            None => k,
        };
        let k = k
            .contiguous()?
            .reshape((b, self.n_kv_heads, 1, self.head_dim))?;

        let v = v
            .contiguous()?
            .reshape((b, self.n_kv_heads, 1, self.head_dim))?;

        let (q, k) = if let Some(inv_freq) = inv_freq {
            self.apply_rotary_emb_rows(&q, &k, inv_freq, pos_f32)?
        } else {
            (q, k)
        };

        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let use_metal_sdpa = is_inference_fast_path()
            && device.is_metal()
            && matches!(self.head_dim, 32 | 64 | 72 | 80 | 96 | 128 | 256);

        let mut ys = Vec::with_capacity(b);
        for (row, slot) in caches.iter_mut().enumerate().take(b) {
            let q_r = q.narrow(0, row, 1)?;
            let k_r = k.narrow(0, row, 1)?;
            let v_r = v.narrow(0, row, 1)?;

            let (k_seq, v_seq) = match **slot {
                None => {
                    let k_c = k_r.contiguous()?;
                    let v_c = v_r.contiguous()?;
                    let cap = 2049usize.next_multiple_of(256);
                    let k_buf = Tensor::zeros(
                        (1, self.n_kv_heads, cap, self.head_dim),
                        k_c.dtype(),
                        &device,
                    )?;
                    let v_buf = Tensor::zeros(
                        (1, self.n_kv_heads, cap, self.head_dim),
                        v_c.dtype(),
                        &device,
                    )?;
                    k_buf.slice_set(&k_c, 2, 0)?;
                    v_buf.slice_set(&v_c, 2, 0)?;
                    **slot = Some(FullKvCache {
                        k: k_buf,
                        v: v_buf,
                        len: 1,
                    });
                    (k_c, v_c)
                }
                Some(ref mut cache) => {
                    let new_len = cache.len + 1;
                    let cur_cap = cache.k.dim(2)?;
                    if new_len > cur_cap {
                        let new_cap = (new_len + 2048).max(cur_cap * 2);
                        let new_k = Tensor::zeros(
                            (1, self.n_kv_heads, new_cap, self.head_dim),
                            cache.k.dtype(),
                            &device,
                        )?;
                        let new_v = Tensor::zeros(
                            (1, self.n_kv_heads, new_cap, self.head_dim),
                            cache.v.dtype(),
                            &device,
                        )?;
                        new_k.slice_set(&cache.k.narrow(2, 0, cache.len)?.contiguous()?, 2, 0)?;
                        new_v.slice_set(&cache.v.narrow(2, 0, cache.len)?.contiguous()?, 2, 0)?;
                        cache.k = new_k;
                        cache.v = new_v;
                    }
                    cache.k.slice_set(&k_r, 2, cache.len)?;
                    cache.v.slice_set(&v_r, 2, cache.len)?;
                    cache.len = new_len;
                    (
                        cache.k.narrow(2, 0, new_len)?,
                        cache.v.narrow(2, 0, new_len)?,
                    )
                }
            };

            let y_r = if use_metal_sdpa {
                candle_nn::ops::sdpa(&q_r, &k_seq, &v_seq, None, false, scale as f32, 1.0)?
            } else {
                let n_rep = self.n_heads / self.n_kv_heads;
                let k_rep = repeat_kv(&k_seq, n_rep)?;
                let v_rep = repeat_kv(&v_seq, n_rep)?;
                let q_scaled = (q_r.contiguous()? * scale)?;
                let att = q_scaled
                    .matmul(&k_rep.transpose(2, 3)?.contiguous()?)?
                    .to_dtype(DType::F32)?
                    .softmax_last_dim()?
                    .to_dtype(act_dtype)?;
                att.matmul(&v_rep.contiguous()?)?
            };
            ys.push(
                y_r.contiguous()?
                    .reshape((1, 1, self.n_heads * self.head_dim))?,
            );
        }

        let y = Tensor::cat(&ys, 0)?;
        let y = vox_plugin_mens_candle_core::attn_math::apply_attn_output_gate(y, gate.as_ref())?;
        self.o_proj
            .forward(&y)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }

    fn project_qkv(&self, x: &Tensor) -> Result<(Tensor, Tensor, Tensor)> {
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
        Ok((q, k, v))
    }

    #[allow(clippy::too_many_arguments)]
    fn prep_qkv_heads(
        &self,
        q: Tensor,
        k: Tensor,
        v: Tensor,
        b: usize,
        seq_len: usize,
    ) -> Result<(Tensor, Tensor, Tensor, Option<Tensor>, Device, DType)> {
        let device = q.device().clone();
        let act_dtype = q.dtype();
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
            .contiguous()?
            .reshape((b, seq_len, self.n_kv_heads, self.head_dim))?
            .transpose(1, 2)?
            .contiguous()?;

        Ok((q, k, v, gate, device, act_dtype))
    }

    fn attend_eager(
        &self,
        q: &Tensor,
        k: &Tensor,
        v: &Tensor,
        mask: Option<&Tensor>,
        act_dtype: DType,
    ) -> Result<Tensor> {
        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let n_rep = self.n_heads / self.n_kv_heads;
        let k = repeat_kv(k, n_rep)?;
        let v = repeat_kv(v, n_rep)?;

        let q_scaled = (q.contiguous()? * scale)?;
        let att = q_scaled
            .matmul(&k.transpose(2, 3)?.contiguous()?)?
            .to_dtype(DType::F32)?;

        let att = match mask {
            Some(m) => att.broadcast_add(m)?.softmax_last_dim()?,
            None => att.softmax_last_dim()?,
        };
        let att = att.to_dtype(act_dtype)?;
        att.matmul(&v.contiguous()?)
    }

    fn attend_projected(
        &self,
        q: Tensor,
        k: Tensor,
        v: Tensor,
        b: usize,
        seq_len: usize,
        pos: usize,
        inv_freq: Option<&Tensor>,
        kv_cache: Option<KvSlot<'_>>,
    ) -> Result<Tensor> {
        let (q, k, v, gate, device, act_dtype) = self.prep_qkv_heads(q, k, v, b, seq_len)?;

        let (q, k) = if let Some(inv_freq) = inv_freq {
            self.apply_rotary_emb(&q, &k, inv_freq, pos)?
        } else {
            (q, k)
        };

        let (k, v) = match kv_cache {
            Some(KvSlot::Tuple(slot)) => {
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
            Some(KvSlot::Buffered(slot)) => match slot {
                None => {
                    let k_c = k.contiguous()?;
                    let v_c = v.contiguous()?;
                    let cap = (seq_len + 2048).next_multiple_of(256);
                    let k_buf = Tensor::zeros(
                        (b, self.n_kv_heads, cap, self.head_dim),
                        k_c.dtype(),
                        &device,
                    )?;
                    let v_buf = Tensor::zeros(
                        (b, self.n_kv_heads, cap, self.head_dim),
                        v_c.dtype(),
                        &device,
                    )?;
                    k_buf.slice_set(&k_c, 2, 0)?;
                    v_buf.slice_set(&v_c, 2, 0)?;
                    *slot = Some(FullKvCache {
                        k: k_buf,
                        v: v_buf,
                        len: seq_len,
                    });
                    (k_c, v_c)
                }
                Some(cache) => {
                    if seq_len > 1 && cache.len > 0 {
                        return Err(candle_core::Error::Msg(format!(
                            "multi-token forward over a populated KV cache is not supported: \
                             seq_len={seq_len}, cached+new keys={}. Prefill once, then feed one token per step.",
                            cache.len + seq_len
                        )));
                    }
                    let new_len = cache.len + seq_len;
                    let cur_cap = cache.k.dim(2)?;
                    if new_len > cur_cap {
                        let new_cap = (new_len + 2048).max(cur_cap * 2);
                        let new_k = Tensor::zeros(
                            (b, self.n_kv_heads, new_cap, self.head_dim),
                            cache.k.dtype(),
                            &device,
                        )?;
                        let new_v = Tensor::zeros(
                            (b, self.n_kv_heads, new_cap, self.head_dim),
                            cache.v.dtype(),
                            &device,
                        )?;
                        new_k.slice_set(&cache.k.narrow(2, 0, cache.len)?.contiguous()?, 2, 0)?;
                        new_v.slice_set(&cache.v.narrow(2, 0, cache.len)?.contiguous()?, 2, 0)?;
                        cache.k = new_k;
                        cache.v = new_v;
                    }
                    let k_c = k.contiguous()?;
                    let v_c = v.contiguous()?;
                    cache.k.slice_set(&k_c, 2, cache.len)?;
                    cache.v.slice_set(&v_c, 2, cache.len)?;
                    cache.len = new_len;
                    (
                        cache.k.narrow(2, 0, new_len)?,
                        cache.v.narrow(2, 0, new_len)?,
                    )
                }
            },
            None => (k, v),
        };

        // `causal_mask` below is square in `seq_len`, so a multi-token forward is
        // only correct when nothing was cached before it (the prefill). Feeding a
        // multi-token chunk on top of a populated cache would silently mask the
        // wrong positions rather than fail, so it fails here instead.
        let k_seq = k.dim(2)?;
        if seq_len > 1 && k_seq != seq_len {
            return Err(candle_core::Error::Msg(format!(
                "multi-token forward over a populated KV cache is not supported: \
                 seq_len={seq_len}, cached+new keys={}. Prefill once, then feed one token per step.",
                k_seq
            )));
        }

        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let use_metal_sdpa = is_inference_fast_path()
            && device.is_metal()
            && b == 1
            && (seq_len == 1
                || (seq_len > 8
                    && k_seq == seq_len
                    && !(self.head_dim == 256 && act_dtype == DType::F32)))
            && matches!(self.head_dim, 32 | 64 | 72 | 80 | 96 | 128 | 256);

        let y = if use_metal_sdpa {
            let q_c = q.contiguous()?;
            candle_nn::ops::sdpa(&q_c, &k, &v, None, seq_len > 1, scale as f32, 1.0)?
        } else {
            let mask = if seq_len > 1 {
                Some(causal_mask(seq_len, &device)?)
            } else {
                None
            };
            self.attend_eager(&q, &k, &v, mask.as_ref(), act_dtype)?
        };
        let mut y =
            y.transpose(1, 2)?
                .contiguous()?
                .reshape((b, seq_len, self.n_heads * self.head_dim))?;
        y = vox_plugin_mens_candle_core::attn_math::apply_attn_output_gate(y, gate.as_ref())?;
        Ok(y)
    }

    /// Forward pass for packed multi-document sequences (`b == 1`, `ctx.doc_ids.len() == seq_len`)
    /// using a precomputed [`PackedAttnContext`].
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
        let (q, k, v) = self.project_qkv(x)?;
        let (q, k, v, gate, _device, act_dtype) = self.prep_qkv_heads(q, k, v, b, seq_len)?;
        let (q, k) = if let Some(inv_freq) = inv_freq {
            self.apply_rotary_emb_with_t(&q, &k, inv_freq, ctx.local_pos_f32.clone(), false)?
        } else {
            (q, k)
        };
        let y = self.attend_eager(&q, &k, &v, ctx.mask.as_ref(), act_dtype)?;
        let y =
            y.transpose(1, 2)?
                .contiguous()?
                .reshape((b, seq_len, self.n_heads * self.head_dim))?;
        let y = vox_plugin_mens_candle_core::attn_math::apply_attn_output_gate(y, gate.as_ref())?;
        self.o_proj
            .forward(&y)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }

    /// Forward pass for packed multi-document sequences (`b == 1`, `doc_ids.len() == seq_len`).
    ///
    /// Resets RoPE positions per document via [`Self::apply_rotary_emb_positions`] and
    /// masks attention strictly to causal within-document positions via
    /// [`block_diagonal_causal_mask`].
    pub fn forward_packed(
        &self,
        x: &Tensor,
        doc_ids: &[u32],
        inv_freq: Option<&Tensor>,
    ) -> Result<Tensor> {
        let ctx = PackedAttnContext::new(doc_ids, x.device())?;
        self.forward_packed_ctx(x, &ctx, inv_freq)
    }

    fn forward_inner(
        &self,
        x: &Tensor,
        pos: usize,
        inv_freq: Option<&Tensor>,
        kv_cache: Option<KvSlot<'_>>,
    ) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        let (q, k, v) = self.project_qkv(x)?;
        let y = self.attend_projected(q, k, v, b, seq_len, pos, inv_freq, kv_cache)?;
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
        let (_b, _n_heads, seq_len, _head_dim) = q.dims4()?;
        let t = Tensor::arange(pos as u32, (pos + seq_len) as u32, q.device())?
            .to_dtype(DType::F32)?
            .reshape((seq_len, 1))?;
        self.apply_rotary_emb_with_t(q, k, inv_freq, t, true)
    }

    pub fn apply_rotary_emb_positions(
        &self,
        q: &Tensor,
        k: &Tensor,
        inv_freq: &Tensor,
        doc_ids: &[u32],
    ) -> Result<(Tensor, Tensor)> {
        let (_b, _n_heads, seq_len, _head_dim) = q.dims4()?;
        if doc_ids.len() != seq_len {
            return Err(candle_core::Error::Msg(format!(
                "apply_rotary_emb_positions: doc_ids.len()={} != seq_len={seq_len}",
                doc_ids.len()
            )));
        }
        let pos_f32: Vec<f32> = packed_local_positions(doc_ids)
            .into_iter()
            .map(|p| p as f32)
            .collect();
        let t = Tensor::from_vec(pos_f32, (seq_len, 1), q.device())?;
        self.apply_rotary_emb_with_t(q, k, inv_freq, t, false)
    }

    fn apply_rotary_emb_with_t(
        &self,
        q: &Tensor,
        k: &Tensor,
        inv_freq: &Tensor,
        t: Tensor,
        allow_metal_fast_rope: bool,
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
        let act_dtype = q.dtype();
        let freqs_half = t.matmul(&inv_freq.reshape((1, inv_freq.elem_count()))?)?;
        if allow_metal_fast_rope
            && is_inference_fast_path()
            && device.is_metal()
            && rope_dim == head_dim
        {
            let cos = freqs_half.cos()?.to_dtype(act_dtype)?.contiguous()?;
            let sin = freqs_half.sin()?.to_dtype(act_dtype)?.contiguous()?;
            let q_embed = candle_nn::rotary_emb::rope(&q.contiguous()?, &cos, &sin)?;
            let k_embed = candle_nn::rotary_emb::rope(&k.contiguous()?, &cos, &sin)?;
            return Ok((q_embed, k_embed));
        }
        let freqs = Tensor::cat(&[&freqs_half, &freqs_half], 1)?;
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

    fn apply_rotary_emb_rows(
        &self,
        q: &Tensor,
        k: &Tensor,
        inv_freq: &Tensor,
        pos_f32: &Tensor,
    ) -> Result<(Tensor, Tensor)> {
        let (b, _n_heads, seq_len, head_dim) = q.dims4()?;
        let rope_dim = inv_freq.elem_count().saturating_mul(2);
        if rope_dim == 0 || rope_dim > head_dim {
            return Err(candle_core::Error::Msg(format!(
                "RoPE inv_freq length inconsistent with head_dim: inv_freq_elems={} head_dim={head_dim}",
                inv_freq.elem_count()
            )));
        }
        let device = q.device();
        let act_dtype = q.dtype();
        let freqs_half = pos_f32.broadcast_mul(&inv_freq.reshape((1, inv_freq.elem_count()))?)?;
        if is_inference_fast_path() && device.is_metal() && rope_dim == head_dim {
            let freqs_3d = freqs_half.unsqueeze(1)?;
            let cos = freqs_3d.cos()?.to_dtype(act_dtype)?.contiguous()?;
            let sin = freqs_3d.sin()?.to_dtype(act_dtype)?.contiguous()?;
            let q_embed = candle_nn::rotary_emb::rope(&q.contiguous()?, &cos, &sin)?;
            let k_embed = candle_nn::rotary_emb::rope(&k.contiguous()?, &cos, &sin)?;
            return Ok((q_embed, k_embed));
        }
        let freqs = Tensor::cat(&[&freqs_half, &freqs_half], 1)?;
        let cos = freqs
            .cos()?
            .reshape((b, 1, seq_len, rope_dim))?
            .to_dtype(act_dtype)?;
        let sin = freqs
            .sin()?
            .reshape((b, 1, seq_len, rope_dim))?
            .to_dtype(act_dtype)?;
        if rope_dim == head_dim {
            let q_embed = (q.broadcast_mul(&cos)? + rotate_half(q)?.broadcast_mul(&sin)?)?;
            let k_embed = (k.broadcast_mul(&cos)? + rotate_half(k)?.broadcast_mul(&sin)?)?;
            Ok((q_embed.contiguous()?, k_embed.contiguous()?))
        } else {
            let q_rot = q.narrow(candle_core::D::Minus1, 0, rope_dim)?;
            let q_pass = q.narrow(candle_core::D::Minus1, rope_dim, head_dim - rope_dim)?;
            let k_rot = k.narrow(candle_core::D::Minus1, 0, rope_dim)?;
            let k_pass = k.narrow(candle_core::D::Minus1, rope_dim, head_dim - rope_dim)?;
            let q_r = (q_rot.broadcast_mul(&cos)? + rotate_half(&q_rot)?.broadcast_mul(&sin)?)?;
            let k_r = (k_rot.broadcast_mul(&cos)? + rotate_half(&k_rot)?.broadcast_mul(&sin)?)?;
            let q_embed = Tensor::cat(&[&q_r, &q_pass], candle_core::D::Minus1)?;
            let k_embed = Tensor::cat(&[&k_r, &k_pass], candle_core::D::Minus1)?;
            Ok((q_embed.contiguous()?, k_embed.contiguous()?))
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn into_inference(&mut self) -> Result<usize> {
        let mut count = 0;
        count += convert_slot_to_inference(&mut self.q_proj)?;
        count += convert_slot_to_inference(&mut self.k_proj)?;
        count += convert_slot_to_inference(&mut self.v_proj)?;
        count += convert_slot_to_inference(&mut self.o_proj)?;
        Ok(count)
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

    #[allow(clippy::wrong_self_convention)]
    pub fn into_inference(&mut self) -> Result<usize> {
        let mut count = 0;
        count += convert_slot_to_inference(&mut self.gate_proj)?;
        count += convert_slot_to_inference(&mut self.up_proj)?;
        count += convert_slot_to_inference(&mut self.down_proj)?;
        Ok(count)
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

    /// Invert `I_p + a_strict` for power-of-2 `p >= 2` using exact $2\times 2$ block elimination.
    /// Avoids Neumann squaring (which overflows `f32` precision at `p=64`) and avoids `index_select`.
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
        let beta = candle_nn::ops::sigmoid(
            &self
                .b_proj
                .forward(x)
                .map_err(|e| candle_core::Error::Msg(e.to_string()))?
                .to_dtype(DType::F32)?,
        )?
        .reshape((b, seq_len, self.num_v_heads))?;
        let a = self
            .a_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?
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
                    // Mask both before and after `.exp()` so positive upper-triangle differences
                    // cannot overflow f32 in forward or backward.
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
        let y_norm = rms_norm_f32(&self.norm, &y_flat)?;
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

    #[allow(clippy::wrong_self_convention)]
    pub fn into_inference(&mut self) -> Result<usize> {
        let mut count = 0;
        count += convert_slot_to_inference(&mut self.qkv_proj)?;
        count += convert_slot_to_inference(&mut self.z_proj)?;
        count += convert_slot_to_inference(&mut self.b_proj)?;
        count += convert_slot_to_inference(&mut self.a_proj)?;
        count += convert_slot_to_inference(&mut self.out_proj)?;
        Ok(count)
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
                a.forward_kv_buf(&h, pos, self.inv_freq.as_ref(), cache)?
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

    pub fn forward_rows(
        &self,
        x: &Tensor,
        pos_f32: &Tensor,
        caches: &mut [&mut Qwen35LayerCache],
    ) -> Result<Tensor> {
        let residual = x;
        let h = rms_norm_f32(&self.input_layernorm, x)?;
        let h = match &self.attention {
            Qwen35AttentionBlock::Full(a) => {
                let mut full_caches: Vec<&mut Option<FullKvCache>> =
                    Vec::with_capacity(caches.len());
                for c in caches.iter_mut() {
                    match c {
                        Qwen35LayerCache::Full(kv) => full_caches.push(kv),
                        Qwen35LayerCache::Linear(_) => {
                            return Err(candle_core::Error::Msg(
                                "qwen3_5 cache mismatch: full-attention layer received linear cache"
                                    .to_string(),
                            ));
                        }
                    }
                }
                a.forward_rows(&h, pos_f32, self.inv_freq.as_ref(), &mut full_caches)?
            }
            Qwen35AttentionBlock::Linear(_) => {
                return Err(candle_core::Error::Msg(
                    "batched incremental decode over linear-attention layers is unsupported"
                        .to_string(),
                ));
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

    #[allow(clippy::wrong_self_convention)]
    pub fn into_inference(&mut self) -> Result<usize> {
        let mut count = match &mut self.attention {
            Qwen35AttentionBlock::Full(a) => a.into_inference()?,
            Qwen35AttentionBlock::Linear(a) => a.into_inference()?,
        };
        count += self.mlp.into_inference()?;
        Ok(count)
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
    /// Optional packed-sequence RoPE + causal-mask context preserved for recomputation.
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

    /// Whether any layer in this model uses `Qwen35AttentionBlock::Linear`.
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

    /// Bytes of KV cache consumed per sequence token across all full-attention layers.
    pub fn kv_bytes_per_token(&self) -> usize {
        let elem_bytes = self.embed_tokens.dtype().size_in_bytes();
        self.layers
            .iter()
            .map(|l| match &l.attention {
                Qwen35AttentionBlock::Full(a) => 2 * a.n_kv_heads * a.head_dim * elem_bytes,
                Qwen35AttentionBlock::Linear(_) => 0,
            })
            .sum()
    }

    /// Batched single-token decode across `B` active sequences (`input_ids` `[B, 1]`),
    /// sharing one weight read across all `B` sequences per linear projection.
    pub fn forward_decode_rows(
        &self,
        input_ids: &Tensor,
        positions: &[usize],
        caches: &mut [Vec<Qwen35LayerCache>],
    ) -> Result<Tensor> {
        let (b, seq_len) = input_ids.dims2()?;
        if seq_len != 1 || b != positions.len() || b != caches.len() {
            return Err(candle_core::Error::Msg(format!(
                "forward_decode_rows expects [B, 1] input and matching slices, got [{b}, {seq_len}], positions={}, caches={}",
                positions.len(),
                caches.len()
            )));
        }
        for c in caches.iter() {
            if c.len() != self.layers.len() {
                return Err(candle_core::Error::Msg(format!(
                    "KV cache has {} slots but the model has {} layers",
                    c.len(),
                    self.layers.len()
                )));
            }
        }
        let d_model = self.embed_tokens.dim(1)?;
        let ids = input_ids.flatten_all()?;
        let mut x = self
            .embed_tokens
            .index_select(&ids, 0)?
            .reshape((b, 1, d_model))?;
        let pos_vec: Vec<f32> = positions.iter().map(|&p| p as f32).collect();
        let pos_f32 = Tensor::from_vec(pos_vec, (b, 1), x.device())?;

        for (layer_idx, layer) in self.layers.iter().enumerate() {
            let mut layer_caches: Vec<&mut Qwen35LayerCache> =
                caches.iter_mut().map(|c| &mut c[layer_idx]).collect();
            x = layer.forward_rows(&x, &pos_f32, &mut layer_caches)?;
        }
        self.head_forward(&x)
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn into_inference(&mut self) -> Result<usize> {
        let mut count = 0;
        for layer in &mut self.layers {
            count += layer.into_inference()?;
        }
        count += convert_slot_to_inference(&mut self.lm_head)?;
        Ok(count)
    }

    /// Final norm + lm_head + F32 cast, shared by the eager and
    /// checkpointed forward so logits are identical for the same input.
    fn head_forward(&self, x: &Tensor) -> Result<Tensor> {
        let x = rms_norm_f32(&self.norm, x)?;
        let logits = self
            .lm_head
            .forward(&x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;
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
        let mut seg_input = embed.detach();
        let mut logits = None;
        for s in 0..n_segments {
            let start = bounds[s];
            let end = bounds[s + 1];
            if s + 1 == n_segments {
                // Final segment runs WITH autograd enabled from an independent Var leaf
                // so `loss.backward()` computes both final-segment LoRA grads and dL/d(boundary).
                let boundary = independent_boundary(&seg_input)?;
                let seg_out = self.run_layers_with(&boundary, start, end, packed_ctx.as_deref())?;
                segments.push(CheckpointSegment {
                    input: boundary,
                    start_layer: start,
                    end_layer: end,
                    packed_ctx: packed_ctx.clone(),
                });
                logits = Some(self.head_forward(&seg_out)?);
            } else {
                // Earlier segments run under `with_no_grad`: zero autograd graph nodes or
                // intermediate activations are retained; `recompute_segment` rebuilds each
                // segment's tape one-at-a-time during backward.
                segments.push(CheckpointSegment {
                    input: seg_input.clone(),
                    start_layer: start,
                    end_layer: end,
                    packed_ctx: packed_ctx.clone(),
                });
                let seg_out = candle_core::with_no_grad(|| {
                    self.run_layers_with(&seg_input, start, end, packed_ctx.as_deref())
                })?;
                seg_input = seg_out.detach();
            }
        }

        Ok(CheckpointedForward {
            logits: logits.expect("at least one segment"),
            segments,
        })
    }

    /// Checkpointed forward: split the `num_layers` transformer layers into
    /// `n_segments` roughly-equal contiguous segments, running earlier segments
    /// under `with_no_grad` so zero autograd tape or intermediate activations
    /// are retained until the final segment + head.
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
    ///
    /// Shares a single precomputed [`PackedAttnContext`] across all segments and checkpoint
    /// recomputations. Rejects non-contiguous `doc_ids`.
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
}

#[derive(Clone)]
pub struct LinearStateCache {
    pub recurrent_state: Tensor,
    pub conv_state: Option<Tensor>,
}

#[derive(Clone)]
pub struct FullKvCache {
    pub k: Tensor,
    pub v: Tensor,
    pub len: usize,
}

/// Per-layer decode state. Each variant holds `None` until the prefill fills it,
/// so a fresh cache needs no shape or device knowledge to construct.
#[derive(Clone)]
pub enum Qwen35LayerCache {
    /// Full attention: pre-allocated `(keys, values)` buffers grown amortized via `slice_set`.
    Full(Option<FullKvCache>),
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

    pub(super) fn model_with_rope(
        d: usize,
        n_heads: usize,
        n_layers: usize,
        vocab: usize,
    ) -> Qwen35Model {
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

    pub(super) fn ids(v: &[u32]) -> Tensor {
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

    /// `Qwen35LinearAttention`'s k-tap causal short conv caches the last `k - 1`
    /// inputs alongside the recurrent state, so incremental single-token decode
    /// after prefill reproduces the full uncached forward pass.
    #[test]
    fn incremental_decode_on_hybrid_model_matches_full_forward() {
        let model = hybrid_model();
        let seq: [u32; 5] = [1, 7, 3, 12, 5];
        let full = model.forward(&ids(&seq)).unwrap();
        let mut cache = model.empty_cache();
        assert!(
            matches!(cache[1], Qwen35LayerCache::Linear(None)),
            "fixture must actually contain a linear-attention layer"
        );

        let prefill = model
            .forward_cached(&ids(&seq[..3]), 0, &mut cache)
            .expect("prefill into empty hybrid cache succeeds");
        assert!(
            matches!(
                &cache[1],
                Qwen35LayerCache::Linear(Some(sc)) if sc.conv_state.is_some()
            ),
            "prefill must populate both recurrent_state and conv_state"
        );
        let step1 = model
            .forward_cached(&ids(&seq[3..4]), 3, &mut cache)
            .unwrap();
        let step2 = model
            .forward_cached(&ids(&seq[4..5]), 4, &mut cache)
            .unwrap();

        let prefill_v = prefill
            .to_dtype(DType::F32)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let step1_v = step1
            .to_dtype(DType::F32)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let step2_v = step2
            .to_dtype(DType::F32)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let full_v = full
            .to_dtype(DType::F32)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let vocab = step1_v.len();
        assert_eq!(prefill_v.len(), 3 * vocab);
        for (idx, (&got, &want)) in prefill_v
            .iter()
            .chain(step1_v.iter())
            .chain(step2_v.iter())
            .zip(full_v.iter())
            .enumerate()
        {
            assert!(
                (got - want).abs() < 1e-4,
                "hybrid cached mismatch at {idx}: got={got}, want={want}"
            );
        }
    }

    #[test]
    fn packed_forward_on_hybrid_model_isolates_documents_and_matches_single_doc() {
        let model = hybrid_model();
        let doc1: [u32; 3] = [1, 7, 3];
        let doc2: [u32; 5] = [12, 5, 9, 2, 4];
        let joined: [u32; 8] = [1, 7, 3, 12, 5, 9, 2, 4];
        let doc_ids: [u32; 8] = [0, 0, 0, 1, 1, 1, 1, 1];

        let out1 = model
            .forward(&ids(&doc1))
            .unwrap()
            .to_dtype(DType::F32)
            .unwrap();
        let out2 = model
            .forward(&ids(&doc2))
            .unwrap()
            .to_dtype(DType::F32)
            .unwrap();
        let packed = model
            .forward_packed(&ids(&joined), &doc_ids)
            .unwrap()
            .to_dtype(DType::F32)
            .unwrap();
        let ckpt = model
            .forward_checkpointed_packed(&ids(&joined), &doc_ids, 2)
            .unwrap()
            .logits
            .to_dtype(DType::F32)
            .unwrap();

        let p1 = packed
            .narrow(1, 0, doc1.len())
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let w1 = out1.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        for (i, (&g, &w)) in p1.iter().zip(w1.iter()).enumerate() {
            assert!(
                (g - w).abs() < 1e-4,
                "doc1 mismatch at {i}: got={g}, want={w}"
            );
        }
        let p2 = packed
            .narrow(1, doc1.len(), doc2.len())
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let w2 = out2.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        for (i, (&g, &w)) in p2.iter().zip(w2.iter()).enumerate() {
            assert!(
                (g - w).abs() < 1e-4,
                "doc2 mismatch at {i}: got={g}, want={w}"
            );
        }
        let pv = packed.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        let cv = ckpt.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        for (i, (&g, &w)) in pv.iter().zip(cv.iter()).enumerate() {
            assert!(
                (g - w).abs() < 1e-5,
                "packed vs checkpointed mismatch at {i}: got={g}, want={w}"
            );
        }
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
    fn uses_linear_attention_detects_hybrid_layers() {
        assert!(hybrid_model().uses_linear_attention());
        assert!(!model_with_rope(8, 2, 2, 16).uses_linear_attention());
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

    #[test]
    fn untracked_frozen_weight_skips_weight_grad_and_preserves_input_grad() {
        let dev = Device::new_metal(0).unwrap_or(Device::Cpu);
        let x = Var::from_tensor(&Tensor::randn(0f32, 1.0, (4, 16), &dev).unwrap()).unwrap();
        let w_frozen = Tensor::randn(0f32, 1.0, (32, 16), &dev).unwrap();
        let w_tracked = Var::from_tensor(&w_frozen).unwrap();

        let y_frozen = x.as_tensor().matmul(&w_frozen.t().unwrap()).unwrap();
        let g_frozen = y_frozen.sum_all().unwrap().backward().unwrap();
        assert!(
            g_frozen.get(&w_frozen).is_none(),
            "frozen base weight must not allocate/store a gradient in GradStore"
        );

        let y_ref = x
            .as_tensor()
            .matmul(&w_tracked.as_tensor().t().unwrap())
            .unwrap();
        let g_ref = y_ref.sum_all().unwrap().backward().unwrap();

        let dx_frozen: Vec<f32> = g_frozen
            .get(x.as_tensor())
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        let dx_ref: Vec<f32> = g_ref
            .get(x.as_tensor())
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        assert_eq!(dx_frozen, dx_ref, "dX must be bitwise identical");

        let no_grad_out =
            candle_core::with_no_grad(|| x.as_tensor().matmul(&w_frozen.t().unwrap()).unwrap());
        let g_no_grad = no_grad_out.sum_all().unwrap().backward().unwrap();
        assert!(
            g_no_grad.get(x.as_tensor()).is_none(),
            "with_no_grad must suppress autograd tape construction"
        );
    }

    #[test]
    fn bf16_checkpointed_backward_matches_eager_on_metal() {
        let Ok(dev) = Device::new_metal(0) else {
            return;
        };
        let (d, n_heads, n_kv_heads, n_layers, vocab, seq) =
            (64usize, 4usize, 2usize, 4usize, 32usize, 8usize);
        let head_dim = d / n_heads;
        let mut qcfg = qlora_rs::QLoraConfig::preset_all_bf16(4, 8);
        qcfg.cache_dequantized = true;

        let varmap = candle_nn::VarMap::new();
        let vb = candle_nn::VarBuilder::from_varmap(&varmap, DType::F32, &dev);
        let qlin = |vb: candle_nn::VarBuilder, d_out: usize, d_in: usize| {
            let w = Tensor::randn(0f32, 0.02f32, (d_out, d_in), &Device::Cpu).unwrap();
            QuantizedLinear::from_weight_with_varbuilder(&w, None, &qcfg, vb).unwrap()
        };

        let inv_freq = Tensor::ones(head_dim / 2, DType::F32, &dev).unwrap();
        let layers: Vec<Qwen35Layer> = (0..n_layers)
            .map(|i| {
                let p = vb.pp(format!("l{i}"));
                let attn = Qwen2Attention {
                    q_proj: qlin(p.pp("q"), d, d),
                    k_proj: qlin(p.pp("k"), n_kv_heads * head_dim, d),
                    v_proj: qlin(p.pp("v"), n_kv_heads * head_dim, d),
                    o_proj: qlin(p.pp("o"), d, d),
                    q_bias: None,
                    k_bias: None,
                    v_bias: None,
                    n_heads,
                    n_kv_heads,
                    head_dim,
                    q_norm: Some(RmsNorm::new(
                        Tensor::ones(head_dim, DType::F32, &dev).unwrap(),
                        1e-6,
                    )),
                    k_norm: Some(RmsNorm::new(
                        Tensor::ones(head_dim, DType::F32, &dev).unwrap(),
                        1e-6,
                    )),
                };
                let mlp = Qwen2MLP {
                    gate_proj: qlin(p.pp("g"), d * 2, d),
                    up_proj: qlin(p.pp("u"), d * 2, d),
                    down_proj: qlin(p.pp("dn"), d, d * 2),
                };
                Qwen35Layer {
                    input_layernorm: RmsNorm::new(Tensor::ones(d, DType::F32, &dev).unwrap(), 1e-6),
                    attention: Qwen35AttentionBlock::Full(attn),
                    post_attention_layernorm: RmsNorm::new(
                        Tensor::ones(d, DType::F32, &dev).unwrap(),
                        1e-6,
                    ),
                    mlp,
                    inv_freq: Some(inv_freq.clone()),
                }
            })
            .collect();

        let embed_f32 = Tensor::randn(0f32, 0.02f32, (vocab, d), &Device::Cpu).unwrap();
        let lm_head =
            QuantizedLinear::from_weight_with_varbuilder(&embed_f32, None, &qcfg, vb.pp("lm_head"))
                .unwrap();
        let embed_bf16 = embed_f32
            .to_device(&dev)
            .unwrap()
            .to_dtype(DType::BF16)
            .unwrap();
        let model = Qwen35Model {
            embed_tokens: embed_bf16,
            layers,
            norm: RmsNorm::new(Tensor::ones(d, DType::F32, &dev).unwrap(), 1e-6),
            lm_head,
        };

        let input_ids = Tensor::from_vec(vec![1u32, 2, 3, 4, 5, 6, 7, 8], (1, seq), &dev).unwrap();
        let targets = Tensor::from_vec(vec![2u32, 3, 4, 5, 6, 7, 8, 9], (1, seq), &dev).unwrap();
        let lora_vars: Vec<Var> = varmap.all_vars();

        let logits_eager = model.forward(&input_ids).unwrap();
        assert_eq!(logits_eager.dtype(), DType::F32);
        let loss_eager = candle_nn::ops::log_softmax(&logits_eager.flatten_to(1).unwrap(), 1)
            .unwrap()
            .gather(&targets.flatten_all().unwrap().unsqueeze(1).unwrap(), 1)
            .unwrap()
            .neg()
            .unwrap()
            .mean_all()
            .unwrap();
        let g_eager = loss_eager.backward().unwrap();

        let ckpt = model.forward_checkpointed(&input_ids, 2).unwrap();
        let loss_ck = candle_nn::ops::log_softmax(&ckpt.logits.flatten_to(1).unwrap(), 1)
            .unwrap()
            .gather(&targets.flatten_all().unwrap().unsqueeze(1).unwrap(), 1)
            .unwrap()
            .neg()
            .unwrap()
            .mean_all()
            .unwrap();
        let mut grads = loss_ck.backward().unwrap();
        let last = ckpt.segments.last().unwrap();
        let mut upstream = grads.remove(&last.input).unwrap();
        for seg in ckpt.segments.iter().rev().skip(1) {
            let (input_var, seg_out) = model.recompute_segment(seg).unwrap();
            let seg_grads = qlora_rs::backward_from_cotangent(&seg_out, &upstream).unwrap();
            qlora_rs::accumulate_grads_for_vars(&mut grads, &seg_grads, &lora_vars).unwrap();
            upstream = seg_grads.get(input_var.as_tensor()).cloned().unwrap();
        }

        for v in &lora_vars {
            let ge: Vec<f32> = g_eager
                .get(v.as_tensor())
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1()
                .unwrap();
            let gc: Vec<f32> = grads
                .get(v.as_tensor())
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1()
                .unwrap();
            for (x, y) in ge.iter().zip(gc.iter()) {
                assert!(x.is_finite() && y.is_finite());
                let denom = x.abs().max(1e-3);
                assert!(
                    (x - y).abs() / denom < 2e-2,
                    "BF16 Metal grad mismatch: eager={x} ckpt={y}"
                );
            }
        }
    }

    #[test]
    fn fused_softmax_last_dim_matches_reference_forward_and_backward() {
        let Ok(dev) = Device::new_metal(0) else {
            return;
        };
        let x1 =
            Var::from_tensor(&Tensor::randn(0f32, 1f32, (2, 4, 32, 32), &dev).unwrap()).unwrap();
        let x2 = Var::from_tensor(x1.as_tensor()).unwrap();
        let w = Tensor::randn(0f32, 1f32, (2, 4, 32, 32), &dev).unwrap();

        let y_ref = candle_nn::ops::softmax(x1.as_tensor(), candle_core::D::Minus1).unwrap();
        let l_ref = (&y_ref * &w).unwrap().sum_all().unwrap();
        let g_ref = l_ref.backward().unwrap();

        let y_fused = x2.as_tensor().softmax_last_dim().unwrap();
        let l_fused = (&y_fused * &w).unwrap().sum_all().unwrap();
        let g_fused = l_fused.backward().unwrap();

        let yr: Vec<f32> = y_ref.flatten_all().unwrap().to_vec1().unwrap();
        let yf: Vec<f32> = y_fused.flatten_all().unwrap().to_vec1().unwrap();
        for (a, b) in yr.iter().zip(yf.iter()) {
            assert!((a - b).abs() < 1e-5, "fwd mismatch: ref={a} fused={b}");
        }

        let gr: Vec<f32> = g_ref
            .get(x1.as_tensor())
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        let gf: Vec<f32> = g_fused
            .get(x2.as_tensor())
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        for (a, b) in gr.iter().zip(gf.iter()) {
            assert!((a - b).abs() < 1e-5, "bwd mismatch: ref={a} fused={b}");
        }
    }

    #[test]
    fn metal_gemv_nt_matches_tiled_gemm_across_dtypes_and_tails() {
        let Ok(dev) = Device::new_metal(0) else {
            return;
        };
        for (dtype, tol) in [
            (DType::F32, 1e-4f32),
            (DType::F16, 5e-3f32),
            (DType::BF16, 2e-2f32),
        ] {
            // Non-multiples of 128 (K=307) and 16 (N=37) exercise both the 4-element
            // vectorized main loop and scalar tail in gemv_nt, while M=2 dispatches
            // standard tiled MLX GEMM as an exact reference on row 0.
            let x_f32 = Tensor::randn(0f32, 0.5f32, (2, 307), &dev).unwrap();
            let w_f32 = Tensor::randn(0f32, 0.5f32, (37, 307), &dev).unwrap();
            let x = x_f32.to_dtype(dtype).unwrap();
            let wt = w_f32.to_dtype(dtype).unwrap().t().unwrap();

            let x1 = x.narrow(0, 0, 1).unwrap().contiguous().unwrap();
            let y_gemv: Vec<f32> = x1
                .matmul(&wt)
                .unwrap()
                .to_dtype(DType::F32)
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1()
                .unwrap();
            let y_gemm: Vec<f32> = x
                .matmul(&wt)
                .unwrap()
                .narrow(0, 0, 1)
                .unwrap()
                .to_dtype(DType::F32)
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1()
                .unwrap();
            assert_eq!(y_gemv.len(), y_gemm.len());
            for (i, (a, b)) in y_gemv.iter().zip(y_gemm.iter()).enumerate() {
                assert!(
                    (a - b).abs() < tol,
                    "gemv_nt {dtype:?} col {i} mismatch: gemv={a} gemm={b}"
                );
            }
        }
    }

    #[test]
    fn metal_no_grad_cached_decode_matches_eager_reference() {
        let Ok(dev) = Device::new_metal(0) else {
            return;
        };
        let (d, n_heads, n_kv_heads, head_dim, d_mlp, n_layers, vocab) =
            (256usize, 4usize, 1usize, 64usize, 300usize, 2usize, 48usize);

        for (compute_dtype, act_dtype, tol) in [
            (
                qlora_rs::quantization::ComputeDType::F32,
                DType::F32,
                2e-4f32,
            ),
            (
                qlora_rs::quantization::ComputeDType::BF16,
                DType::BF16,
                4e-2f32,
            ),
        ] {
            let mut qcfg = qlora_rs::QLoraConfig::preset_all_bf16(4, 8);
            qcfg.quantization.compute_dtype = compute_dtype;
            qcfg.cache_dequantized = true;

            let qlin = |d_out: usize, d_in: usize| {
                let w = Tensor::randn(0f32, 0.02f32, (d_out, d_in), &Device::Cpu).unwrap();
                QuantizedLinear::from_weight(&w, None, &qcfg, &dev).unwrap()
            };
            let inv_freq = Tensor::ones(head_dim / 2, DType::F32, &dev).unwrap();
            let layers: Vec<Qwen35Layer> = (0..n_layers)
                .map(|_| {
                    let attn = Qwen2Attention {
                        q_proj: qlin(n_heads * head_dim, d),
                        k_proj: qlin(n_kv_heads * head_dim, d),
                        v_proj: qlin(n_kv_heads * head_dim, d),
                        o_proj: qlin(d, n_heads * head_dim),
                        q_bias: None,
                        k_bias: None,
                        v_bias: None,
                        n_heads,
                        n_kv_heads,
                        head_dim,
                        q_norm: Some(RmsNorm::new(
                            Tensor::ones(head_dim, DType::F32, &dev).unwrap(),
                            1e-6,
                        )),
                        k_norm: Some(RmsNorm::new(
                            Tensor::ones(head_dim, DType::F32, &dev).unwrap(),
                            1e-6,
                        )),
                    };
                    let mlp = Qwen2MLP {
                        gate_proj: qlin(d_mlp, d),
                        up_proj: qlin(d_mlp, d),
                        down_proj: qlin(d, d_mlp),
                    };
                    Qwen35Layer {
                        input_layernorm: RmsNorm::new(
                            Tensor::ones(d, DType::F32, &dev).unwrap(),
                            1e-6,
                        ),
                        attention: Qwen35AttentionBlock::Full(attn),
                        post_attention_layernorm: RmsNorm::new(
                            Tensor::ones(d, DType::F32, &dev).unwrap(),
                            1e-6,
                        ),
                        mlp,
                        inv_freq: Some(inv_freq.clone()),
                    }
                })
                .collect();

            let embed_f32 = Tensor::randn(0f32, 0.02f32, (vocab, d), &Device::Cpu).unwrap();
            let lm_head = QuantizedLinear::from_weight(&embed_f32, None, &qcfg, &dev).unwrap();
            let embed = embed_f32
                .to_device(&dev)
                .unwrap()
                .to_dtype(act_dtype)
                .unwrap();
            let model = Qwen35Model {
                embed_tokens: embed,
                layers,
                norm: RmsNorm::new(Tensor::ones(d, DType::F32, &dev).unwrap(), 1e-6),
                lm_head,
            };

            // Test both short prefill+decode (1-pass SDPA) and k_seq >= 1024 (2-pass SDPA).
            for prefill_len in [16usize, 1040usize] {
                let mut tokens: Vec<u32> = (0..prefill_len).map(|i| (i % vocab) as u32).collect();
                let mut cache = model.empty_cache();

                let prefill_ids = Tensor::from_vec(tokens.clone(), (1, prefill_len), &dev).unwrap();
                let cached_prefill: Vec<f32> = with_inference_fast_path(|| {
                    model.forward_cached_last_token(&prefill_ids, 0, &mut cache)
                })
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1()
                .unwrap();
                let eager_prefill: Vec<f32> = model
                    .forward(&prefill_ids)
                    .unwrap()
                    .narrow(1, prefill_len - 1, 1)
                    .unwrap()
                    .flatten_all()
                    .unwrap()
                    .to_vec1()
                    .unwrap();
                for (a, b) in cached_prefill.iter().zip(eager_prefill.iter()) {
                    assert!(
                        (a - b).abs() < tol,
                        "prefill({prefill_len}) {act_dtype:?} mismatch: fast={a} ref={b}"
                    );
                }

                for step in 0..3usize {
                    let pos = tokens.len();
                    let next_tok = ((pos + 7) % vocab) as u32;
                    tokens.push(next_tok);
                    let step_ids = Tensor::from_vec(vec![next_tok], (1, 1), &dev).unwrap();
                    let cached_step: Vec<f32> = with_inference_fast_path(|| {
                        model.forward_cached_last_token(&step_ids, pos, &mut cache)
                    })
                    .unwrap()
                    .flatten_all()
                    .unwrap()
                    .to_vec1()
                    .unwrap();

                    let full_ids =
                        Tensor::from_vec(tokens.clone(), (1, tokens.len()), &dev).unwrap();
                    let eager_step: Vec<f32> = model
                        .forward(&full_ids)
                        .unwrap()
                        .narrow(1, tokens.len() - 1, 1)
                        .unwrap()
                        .flatten_all()
                        .unwrap()
                        .to_vec1()
                        .unwrap();
                    for (a, b) in cached_step.iter().zip(eager_step.iter()) {
                        assert!(
                            (a - b).abs() < tol,
                            "decode({prefill_len}+{step}) {act_dtype:?} mismatch: fast={a} ref={b}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn metal_nf4_gemv_and_dequant_match_cpu_nf4_reference() {
        let Ok(dev) = Device::new_metal(0) else {
            return;
        };
        for (compute_dtype, act_dtype, tol) in [
            (
                qlora_rs::quantization::ComputeDType::F32,
                DType::F32,
                1e-4f32,
            ),
            (
                qlora_rs::quantization::ComputeDType::F16,
                DType::F16,
                1e-2f32,
            ),
            (
                qlora_rs::quantization::ComputeDType::BF16,
                DType::BF16,
                3e-2f32,
            ),
        ] {
            for k in [192usize, 256usize] {
                let mut qcfg = qlora_rs::QLoraConfig::preset_all_bf16(4, 8);
                qcfg.quantization.compute_dtype = compute_dtype;
                qcfg.cache_dequantized = true;

                let w = Tensor::randn(0f32, 0.25f32, (37, k), &Device::Cpu).unwrap();
                let qlin_metal = QuantizedLinear::from_weight(&w, None, &qcfg, &dev).unwrap();
                let mut qcfg_cpu = qcfg.clone();
                qcfg_cpu.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
                let qlin_cpu =
                    QuantizedLinear::from_weight(&w, None, &qcfg_cpu, &Device::Cpu).unwrap();

                for m in [1usize, 4usize] {
                    let x_cpu = Tensor::randn(0f32, 0.5f32, (1, m, k), &Device::Cpu)
                        .unwrap()
                        .to_dtype(act_dtype)
                        .unwrap();
                    let x_metal = x_cpu.to_device(&dev).unwrap();

                    let y_metal: Vec<f32> = qlin_metal
                        .forward(&x_metal)
                        .unwrap()
                        .to_dtype(DType::F32)
                        .unwrap()
                        .flatten_all()
                        .unwrap()
                        .to_vec1()
                        .unwrap();
                    let y_cpu: Vec<f32> = qlin_cpu
                        .forward(&x_cpu.to_dtype(DType::F32).unwrap())
                        .unwrap()
                        .to_dtype(DType::F32)
                        .unwrap()
                        .flatten_all()
                        .unwrap()
                        .to_vec1()
                        .unwrap();
                    for (i, (a, b)) in y_metal.iter().zip(y_cpu.iter()).enumerate() {
                        let denom = b.abs().max(1.0);
                        assert!(
                            (a - b).abs() / denom < tol,
                            "nf4 m={m} k={k} {act_dtype:?} idx {i} mismatch: metal={a} cpu={b}"
                        );
                    }
                }
            }
        }
    }

    #[cfg(any(feature = "metal", target_os = "macos"))]
    #[test]
    fn metal_nf4_gemv_batch_is_bitwise_identical_to_m1() {
        let Ok(dev) = Device::new_metal(0) else {
            return;
        };
        for (compute_dtype, act_dtype) in [
            (qlora_rs::quantization::ComputeDType::F32, DType::F32),
            (qlora_rs::quantization::ComputeDType::BF16, DType::BF16),
        ] {
            for k in [192usize, 256usize] {
                let mut qcfg = qlora_rs::QLoraConfig::preset_all_bf16(4, 8);
                qcfg.quantization.compute_dtype = compute_dtype;
                qcfg.cache_dequantized = true;

                let w = Tensor::randn(0f32, 0.25f32, (37, k), &Device::Cpu).unwrap();
                let qlin = QuantizedLinear::from_weight(&w, None, &qcfg, &dev).unwrap();

                for m in [2usize, 5, 8, 16] {
                    let x = Tensor::randn(0f32, 0.5f32, (m, 1, k), &Device::Cpu)
                        .unwrap()
                        .to_dtype(act_dtype)
                        .unwrap()
                        .to_device(&dev)
                        .unwrap();

                    let y_batch: Vec<u32> = qlin
                        .forward(&x)
                        .unwrap()
                        .to_dtype(DType::F32)
                        .unwrap()
                        .flatten_all()
                        .unwrap()
                        .to_vec1::<f32>()
                        .unwrap()
                        .into_iter()
                        .map(f32::to_bits)
                        .collect();

                    let mut y_single = Vec::with_capacity(y_batch.len());
                    for row in 0..m {
                        let xr = x.narrow(0, row, 1).unwrap();
                        let yr: Vec<u32> = qlin
                            .forward(&xr)
                            .unwrap()
                            .to_dtype(DType::F32)
                            .unwrap()
                            .flatten_all()
                            .unwrap()
                            .to_vec1::<f32>()
                            .unwrap()
                            .into_iter()
                            .map(f32::to_bits)
                            .collect();
                        y_single.extend(yr);
                    }
                    assert_eq!(
                        y_batch, y_single,
                        "nf4_gemv_batch must be bitwise identical to m=1 for m={m} k={k} {act_dtype:?}"
                    );
                }
            }
        }
    }

    fn seeded_2d(rows: usize, cols: usize, salt: f32) -> Tensor {
        let vals: Vec<f32> = (0..(rows * cols))
            .map(|i| ((i as f32 * 0.137 + salt).sin()) * 0.2)
            .collect();
        Tensor::from_vec(vals, (rows, cols), &Device::Cpu).unwrap()
    }

    #[test]
    fn bf16_rows_pad_to_gemm_rows() {
        assert_eq!(super::padded_rows(0, 8), 0);
        assert_eq!(super::padded_rows(1, 8), 8);
        assert_eq!(super::padded_rows(3, 8), 8);
        assert_eq!(super::padded_rows(8, 8), 8);
        assert_eq!(super::padded_rows(20, 8), 20);

        let dev = Device::Cpu;
        let mut qcfg = qlora_rs::QLoraConfig::preset_all_bf16(4, 8);
        qcfg.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
        qcfg.cache_dequantized = true;
        let w = seeded_2d(32, 64, 0.7);
        let qlin = QuantizedLinear::from_weight_padded(&w, None, &qcfg, &dev, 8).unwrap();

        let full = seeded_2d(8, 64, 1.3);
        let out8 = qlin.forward(&full).unwrap();
        let out3 = qlin.forward(&full.narrow(0, 0, 3).unwrap()).unwrap();
        let out1 = qlin.forward(&full.narrow(0, 0, 1).unwrap()).unwrap();

        let bits = |t: &Tensor| -> Vec<u32> {
            t.flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap()
                .into_iter()
                .map(f32::to_bits)
                .collect()
        };
        assert_eq!(bits(&out1), bits(&out8.narrow(0, 0, 1).unwrap()));
        assert_eq!(bits(&out3), bits(&out8.narrow(0, 0, 3).unwrap()));

        let prefill20 = seeded_2d(20, 64, 2.1);
        let out20 = qlin.forward(&prefill20).unwrap();
        assert_eq!(out20.dims(), &[20, 32]);
    }

    #[test]
    #[cfg(any(feature = "metal", target_os = "macos"))]
    fn metal_bf16_batched_equals_single() {
        let Ok(dev) = Device::new_metal(0) else {
            return;
        };
        let mut qcfg = qlora_rs::QLoraConfig::preset_all_bf16(4, 8);
        qcfg.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::BF16;
        qcfg.cache_dequantized = true;
        let w = seeded_2d(96, 128, 0.9);
        let qlin = QuantizedLinear::from_weight_padded(&w, None, &qcfg, &dev, 32).unwrap();

        let full = seeded_2d(32, 128, 1.7)
            .to_dtype(DType::BF16)
            .unwrap()
            .to_device(&dev)
            .unwrap()
            .reshape((32, 1, 128))
            .unwrap();
        let out32 = qlin.forward(&full).unwrap();
        let out5 = qlin.forward(&full.narrow(0, 0, 5).unwrap()).unwrap();
        let out1 = qlin.forward(&full.narrow(0, 0, 1).unwrap()).unwrap();

        let bits = |t: &Tensor| -> Vec<u32> {
            t.to_dtype(DType::F32)
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap()
                .into_iter()
                .map(f32::to_bits)
                .collect()
        };
        assert_eq!(
            bits(&out1),
            bits(&out32.narrow(0, 0, 1).unwrap()),
            "Metal BF16 padded GEMM M=1 must be bitwise identical to M=32 row 0"
        );
        assert_eq!(
            bits(&out5),
            bits(&out32.narrow(0, 0, 5).unwrap()),
            "Metal BF16 padded GEMM M=5 must be bitwise identical to M=32 rows 0..5"
        );

        // Also verify F32 head projection is bitwise identical across M=1, M=5, M=17, M=32.
        let mut head_cfg = qcfg.clone();
        head_cfg.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
        let head = QuantizedLinear::from_weight_f32_head(&w, None, &head_cfg, &dev).unwrap();
        let h32 = head.forward(&full).unwrap();
        assert_eq!(h32.dtype(), DType::F32);
        let h17 = head.forward(&full.narrow(0, 0, 17).unwrap()).unwrap();
        let h5 = head.forward(&full.narrow(0, 0, 5).unwrap()).unwrap();
        let h1 = head.forward(&full.narrow(0, 0, 1).unwrap()).unwrap();
        assert_eq!(bits(&h1), bits(&h32.narrow(0, 0, 1).unwrap()));
        assert_eq!(bits(&h5), bits(&h32.narrow(0, 0, 5).unwrap()));
        assert_eq!(bits(&h17), bits(&h32.narrow(0, 0, 17).unwrap()));
    }

    #[test]
    fn packed_forward_matches_separate_forwards_on_cpu() {
        use super::kv_cache_tests::{ids, model_with_rope};
        let model = model_with_rope(8, 2, 2, 16);
        let d0: [u32; 3] = [1, 7, 3];
        let d1: [u32; 2] = [12, 5];
        let d2: [u32; 4] = [4, 9, 2, 6];

        let sep0 = model.forward(&ids(&d0)).unwrap();
        let sep1 = model.forward(&ids(&d1)).unwrap();
        let sep2 = model.forward(&ids(&d2)).unwrap();
        let ref_cat = Tensor::cat(&[&sep0, &sep1, &sep2], 1).unwrap();

        let packed_ids = ids(&[1, 7, 3, 12, 5, 4, 9, 2, 6]);
        let doc_ids = [0u32, 0, 0, 1, 1, 2, 2, 2, 2];
        assert_eq!(
            super::packed_local_positions(&doc_ids),
            vec![0, 1, 2, 0, 1, 0, 1, 2, 3]
        );

        let packed_out = model.forward_packed(&packed_ids, &doc_ids).unwrap();
        assert_eq!(packed_out.dims(), ref_cat.dims());

        let diff = (packed_out - ref_cat)
            .unwrap()
            .abs()
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let max_diff = diff.into_iter().fold(0.0f32, f32::max);
        assert!(
            max_diff < 1e-5,
            "packed forward diverged from separate forwards: max_diff={max_diff}"
        );
        assert!(
            super::PackedAttnContext::new(&[0u32, 1, 0], &Device::Cpu).is_err(),
            "non-contiguous document runs must be rejected"
        );
    }

    #[test]
    fn checkpointed_packed_forward_matches_eager_and_propagates_boundary_grad() {
        use super::kv_cache_tests::{ids, model_with_rope};
        let model = model_with_rope(8, 2, 2, 16);
        let packed_ids = ids(&[1, 7, 3, 12, 5, 4, 9, 2, 6]);
        let doc_ids = [0u32, 0, 0, 1, 1, 2, 2, 2, 2];

        let eager = model.forward_packed(&packed_ids, &doc_ids).unwrap();
        let ckpt = model
            .forward_checkpointed_packed(&packed_ids, &doc_ids, 2)
            .unwrap();
        assert_eq!(ckpt.segments.len(), 2);
        assert_eq!(ckpt.logits.dims(), eager.dims());

        let diff = (ckpt.logits.clone() - eager)
            .unwrap()
            .abs()
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let max_diff = diff.into_iter().fold(0.0f32, f32::max);
        assert!(
            max_diff < 1e-5,
            "checkpointed packed forward diverged from eager packed forward: max_diff={max_diff}"
        );

        let loss = ckpt.logits.sum_all().unwrap();
        let grads = loss.backward().unwrap();
        let boundary_grad = grads
            .get(&ckpt.segments[1].input)
            .expect("final checkpoint boundary must receive gradient");
        let grad_norm: f32 = boundary_grad
            .sqr()
            .unwrap()
            .sum_all()
            .unwrap()
            .to_scalar()
            .unwrap();
        assert!(grad_norm > 0.0, "expected non-zero boundary grad");

        let (seg0_var, seg0_out) = model.recompute_segment(&ckpt.segments[0]).unwrap();
        let seg0_grads = (seg0_out * boundary_grad.detach())
            .unwrap()
            .sum_all()
            .unwrap()
            .backward()
            .unwrap();
        let seg0_grad_norm: f32 = seg0_grads
            .get(seg0_var.as_tensor())
            .expect("segment 0 recompute input must receive gradient")
            .sqr()
            .unwrap()
            .sum_all()
            .unwrap()
            .to_scalar()
            .unwrap();
        assert!(
            seg0_grad_norm > 0.0,
            "expected non-zero seg0 recompute grad"
        );
    }

    #[test]
    fn from_weight_frozen_propagates_input_grad_for_short_seq() {
        let mut qcfg = qlora_rs::QLoraConfig::preset_all_bf16(4, 8);
        qcfg.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
        qcfg.cache_dequantized = true;
        let w = seeded_2d(16, 64, 0.5);
        let frozen = QuantizedLinear::from_weight_frozen(&w, None, &qcfg, &Device::Cpu).unwrap();

        let x_init = seeded_2d(4, 64, 0.3).reshape((1, 4, 64)).unwrap();
        let x_var = candle_core::Var::from_tensor(&x_init).unwrap();
        let y = frozen.forward(x_var.as_tensor()).unwrap();
        let grads = y.sum_all().unwrap().backward().unwrap();
        let grad_norm: f32 = grads
            .get(x_var.as_tensor())
            .expect("from_weight_frozen must propagate input gradients for m <= 16")
            .sqr()
            .unwrap()
            .sum_all()
            .unwrap()
            .to_scalar()
            .unwrap();
        assert!(grad_norm > 0.0, "expected non-zero input grad");
    }

    #[test]
    fn nocache_nf4_propagates_lora_b_and_input_grads_cpu_and_metal() {
        let mut devices = vec![(
            Device::Cpu,
            qlora_rs::quantization::ComputeDType::F32,
            DType::F32,
        )];
        if let Ok(metal_dev) = Device::new_metal(0) {
            devices.push((
                metal_dev,
                qlora_rs::quantization::ComputeDType::BF16,
                DType::BF16,
            ));
        }

        let weight = seeded_2d(64, 64, 0.25);
        for (dev, compute_dt, in_dtype) in devices {
            let mut qcfg = qlora_rs::QLoraConfig::preset_all_bf16(4, 8);
            qcfg.cache_dequantized = false;
            qcfg.quantization.compute_dtype = compute_dt;

            let varmap = candle_nn::VarMap::new();
            let vb = candle_nn::VarBuilder::from_varmap(&varmap, DType::F32, &dev);
            let trainable = qlora_rs::qlora::QuantizedLinear::from_weight_with_varbuilder(
                &weight,
                None,
                &qcfg,
                vb.pp("p"),
            )
            .unwrap();
            let frozen =
                qlora_rs::qlora::QuantizedLinear::from_weight_frozen(&weight, None, &qcfg, &dev)
                    .unwrap();

            assert_eq!(trainable.is_weight_cached(), dev.is_metal());
            assert_eq!(frozen.is_weight_cached(), dev.is_metal());
            assert!(trainable.memory_bytes() > 0);
            assert!(frozen.memory_bytes() > 0);

            for seq_len in [4usize, 32usize] {
                let x_init = seeded_2d(seq_len, 64, 0.35)
                    .to_device(&dev)
                    .unwrap()
                    .to_dtype(in_dtype)
                    .unwrap()
                    .reshape((1, seq_len, 64))
                    .unwrap();

                let x_var = candle_core::Var::from_tensor(&x_init).unwrap();
                let y = trainable.forward(x_var.as_tensor()).unwrap();
                assert_eq!(y.dims(), &[1, seq_len, 64]);
                let grads = y
                    .to_dtype(DType::F32)
                    .unwrap()
                    .sum_all()
                    .unwrap()
                    .backward()
                    .unwrap();
                let x_grad_norm: f32 = grads
                    .get(x_var.as_tensor())
                    .expect("trainable nocache forward must backprop to input")
                    .to_dtype(DType::F32)
                    .unwrap()
                    .sqr()
                    .unwrap()
                    .sum_all()
                    .unwrap()
                    .to_scalar()
                    .unwrap();
                assert!(
                    x_grad_norm.is_finite() && x_grad_norm > 0.0,
                    "dev={dev:?} seq_len={seq_len}: expected positive input grad norm, got {x_grad_norm}"
                );

                let (_lora_a, lora_b) = trainable.lora_weights();
                let b_grad_norm: f32 = grads
                    .get(lora_b)
                    .expect("trainable nocache forward must backprop to lora_b")
                    .to_dtype(DType::F32)
                    .unwrap()
                    .sqr()
                    .unwrap()
                    .sum_all()
                    .unwrap()
                    .to_scalar()
                    .unwrap();
                assert!(
                    b_grad_norm.is_finite() && b_grad_norm > 0.0,
                    "dev={dev:?} seq_len={seq_len}: expected positive lora_b grad norm, got {b_grad_norm}"
                );

                let x_frozen = candle_core::Var::from_tensor(&x_init).unwrap();
                let y_frozen = frozen.forward(x_frozen.as_tensor()).unwrap();
                let grads_frozen = y_frozen
                    .to_dtype(DType::F32)
                    .unwrap()
                    .sum_all()
                    .unwrap()
                    .backward()
                    .unwrap();
                let frozen_grad_norm: f32 = grads_frozen
                    .get(x_frozen.as_tensor())
                    .expect("frozen nocache forward must backprop to input")
                    .to_dtype(DType::F32)
                    .unwrap()
                    .sqr()
                    .unwrap()
                    .sum_all()
                    .unwrap()
                    .to_scalar()
                    .unwrap();
                assert!(
                    frozen_grad_norm.is_finite() && frozen_grad_norm > 0.0,
                    "dev={dev:?} seq_len={seq_len}: expected positive frozen input grad norm, got {frozen_grad_norm}"
                );

                let x_head = candle_core::Var::from_tensor(&x_init).unwrap();
                let y_head = frozen.forward_f32_head(x_head.as_tensor()).unwrap();
                let grads_head = y_head.sum_all().unwrap().backward().unwrap();
                let head_grad_norm: f32 = grads_head
                    .get(x_head.as_tensor())
                    .expect("frozen nocache forward_f32_head must backprop to input")
                    .to_dtype(DType::F32)
                    .unwrap()
                    .sqr()
                    .unwrap()
                    .sum_all()
                    .unwrap()
                    .to_scalar()
                    .unwrap();
                assert!(
                    head_grad_norm.is_finite() && head_grad_norm > 0.0,
                    "dev={dev:?} seq_len={seq_len}: expected positive head input grad norm, got {head_grad_norm}"
                );
            }
        }
    }

    #[test]
    fn into_inference_matches_qlora_padded_output_for_2d_and_3d_inputs() {
        let mut qcfg = qlora_rs::QLoraConfig::preset_all_bf16(4, 8);
        qcfg.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
        qcfg.cache_dequantized = true;
        let w = seeded_2d(32, 64, 0.4);
        let bias = Tensor::randn(0f32, 0.1f32, 32, &Device::Cpu).unwrap();
        let qlin =
            QuantizedLinear::from_weight_padded(&w, Some(bias), &qcfg, &Device::Cpu, 8).unwrap();
        let x2d = seeded_2d(3, 64, 0.2);
        let x3d = seeded_2d(6, 64, 0.7).reshape((2, 3, 64)).unwrap();
        let ref2d = qlin.forward(&x2d).unwrap().to_vec2::<f32>().unwrap();
        let ref3d = qlin
            .forward(&x3d)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();

        let dense = qlin.into_inference().unwrap();
        assert!(matches!(dense, QuantizedLinear::Dense { .. }));
        let got2d = dense.forward(&x2d).unwrap().to_vec2::<f32>().unwrap();
        let got3d = dense
            .forward(&x3d)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();

        for (r_row, g_row) in ref2d.iter().zip(got2d.iter()) {
            for (r, g) in r_row.iter().zip(g_row.iter()) {
                assert!((r - g).abs() < 1e-5, "2d mismatch: ref={r} got={g}");
            }
        }
        for (r, g) in ref3d.iter().zip(got3d.iter()) {
            assert!((r - g).abs() < 1e-5, "3d mismatch: ref={r} got={g}");
        }
    }

    #[test]
    fn into_inference_rejects_nonzero_lora_b() {
        let mut qcfg = qlora_rs::QLoraConfig::preset_all_bf16(4, 8);
        qcfg.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
        qcfg.cache_dequantized = true;
        qcfg.lora.loftq_iterations = 1;
        let w = seeded_2d(16, 64, 0.4);
        let ql =
            qlora_rs::qlora::QuantizedLinear::from_weight(&w, None, &qcfg, &Device::Cpu).unwrap();
        let wrapped = QuantizedLinear::QLora(Box::new(ql));
        let err = wrapped
            .into_inference()
            .err()
            .expect("non-zero LoRA B must fail into_inference");
        assert!(err.to_string().contains("non-zero LoRA B"), "{err}");
    }

    fn ref_causal_depthwise_conv_silu_inner(
        x: &Tensor,
        conv_weight: &Tensor,
        conv_state: Option<&mut Option<Tensor>>,
        doc_ids: Option<&[u32]>,
    ) -> Result<Tensor> {
        let (b, s, c) = x.dims3()?;
        let k = conv_weight.dim(1)?;
        let dev = x.device();
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
        let mut steps = Vec::with_capacity(s);
        for t in offset..total_s {
            let mut acc = Tensor::zeros((b, c), DType::F32, dev)?;
            for j in 0..k {
                if t < j {
                    continue;
                }
                if let Some(ids) = doc_ids
                    && ids[t - j] != ids[t]
                {
                    break;
                }
                let x_t = full_x.narrow(1, t - j, 1)?.squeeze(1)?;
                let w = conv_weight.narrow(1, k - 1 - j, 1)?.squeeze(1)?;
                let prod = x_t.broadcast_mul(&w.unsqueeze(0)?)?;
                acc = (acc + prod)?;
            }
            steps.push(candle_nn::ops::silu(&acc)?);
        }

        if let Some(cs_ref) = conv_state {
            let keep = k.saturating_sub(1).min(total_s);
            *cs_ref = if keep > 0 {
                Some(full_x.narrow(1, total_s - keep, keep)?)
            } else {
                None
            };
        }

        Tensor::stack(&steps, 1)
    }

    fn ref_linear_attention_forward_inner(
        attn: &Qwen35LinearAttention,
        x: &Tensor,
        mut state_cache: Option<&mut Option<LinearStateCache>>,
        doc_ids: Option<&[u32]>,
    ) -> Result<Tensor> {
        let (b, seq_len, _d_model) = x.dims3()?;
        let device = x.device();
        let act_dtype = x.dtype();
        let qkv = attn
            .qkv_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?;

        let mut conv_state = state_cache
            .as_ref()
            .and_then(|sc| sc.as_ref().and_then(|s| s.conv_state.clone()));

        let mixed_qkv = if let Some(ids) = doc_ids {
            ref_causal_depthwise_conv_silu_inner(&qkv, &attn.conv_weight, None, Some(ids))?
        } else {
            ref_causal_depthwise_conv_silu_inner(
                &qkv,
                &attn.conv_weight,
                if state_cache.is_some() {
                    Some(&mut conv_state)
                } else {
                    None
                },
                None,
            )?
        };

        let key_dim = attn.num_k_heads * attn.head_k_dim;
        let value_dim = attn.num_v_heads * attn.head_v_dim;
        let query = mixed_qkv
            .narrow(candle_core::D::Minus1, 0, key_dim)?
            .reshape((b, seq_len, attn.num_k_heads, attn.head_k_dim))?
            .to_dtype(DType::F32)?;
        let key = mixed_qkv
            .narrow(candle_core::D::Minus1, key_dim, key_dim)?
            .reshape((b, seq_len, attn.num_k_heads, attn.head_k_dim))?
            .to_dtype(DType::F32)?;
        let value = mixed_qkv
            .narrow(candle_core::D::Minus1, key_dim + key_dim, value_dim)?
            .reshape((b, seq_len, attn.num_v_heads, attn.head_v_dim))?
            .to_dtype(DType::F32)?;

        let z = attn
            .z_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?
            .reshape((b, seq_len, attn.num_v_heads, attn.head_v_dim))?
            .to_dtype(DType::F32)?;
        let beta = candle_nn::ops::sigmoid(
            &attn
                .b_proj
                .forward(x)
                .map_err(|e| candle_core::Error::Msg(e.to_string()))?
                .to_dtype(DType::F32)?,
        )?
        .reshape((b, seq_len, attn.num_v_heads))?;
        let a = attn
            .a_proj
            .forward(x)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))?
            .reshape((b, seq_len, attn.num_v_heads))?
            .to_dtype(DType::F32)?;

        let a_log = attn.a_log.to_dtype(DType::F32)?;
        let dt_bias = attn.dt_bias.to_dtype(DType::F32)?;
        let g_pre =
            (a.broadcast_add(&dt_bias.reshape((1, 1, attn.num_v_heads))?)?).to_dtype(DType::F32)?;
        let g_soft = vox_plugin_mens_candle_core::attn_math::stable_softplus(&g_pre)?;
        let a_log_scale = a_log.exp()?;
        let g = g_soft
            .broadcast_mul(&a_log_scale.reshape((1, 1, attn.num_v_heads))?)?
            .neg()?;

        let mut query = vox_plugin_mens_candle_core::attn_math::l2norm_last_no_div(&query, 1e-6)?;
        let mut key = vox_plugin_mens_candle_core::attn_math::l2norm_last_no_div(&key, 1e-6)?;
        if attn.num_v_heads > attn.num_k_heads {
            let rep = attn.num_v_heads / attn.num_k_heads;
            query = Qwen35LinearAttention::repeat_heads_bshd(&query, rep)?;
            key = Qwen35LinearAttention::repeat_heads_bshd(&key, rep)?;
        }
        let query = (query * (1.0 / (attn.head_k_dim as f64).sqrt()))?;

        let mut state = if let Some(state_prev) = state_cache
            .as_ref()
            .and_then(|s| s.as_ref().map(|sc| &sc.recurrent_state))
        {
            state_prev.clone()
        } else {
            Tensor::zeros(
                (b, attn.num_v_heads, attn.head_k_dim, attn.head_v_dim),
                DType::F32,
                device,
            )?
        };
        let mut outs = Vec::with_capacity(seq_len);
        for t in 0..seq_len {
            if doc_ids.is_some_and(|ids| t > 0 && ids[t] != ids[t - 1]) {
                state = Tensor::zeros(
                    (b, attn.num_v_heads, attn.head_k_dim, attn.head_v_dim),
                    DType::F32,
                    device,
                )?;
            }
            let q_t = query.narrow(1, t, 1)?.squeeze(1)?;
            let k_t = key.narrow(1, t, 1)?.squeeze(1)?;
            let v_t = value.narrow(1, t, 1)?.squeeze(1)?;
            let g_t = g.narrow(1, t, 1)?.squeeze(1)?;
            let beta_t = beta.narrow(1, t, 1)?.squeeze(1)?;

            let g_scale = g_t.exp()?.reshape((b, attn.num_v_heads, 1, 1))?;
            state = state.broadcast_mul(&g_scale)?;

            let k_col = k_t.unsqueeze(candle_core::D::Minus1)?.contiguous()?;
            let kv_mem = state
                .transpose(2, 3)?
                .contiguous()?
                .matmul(&k_col)?
                .squeeze(candle_core::D::Minus1)?;
            let delta = v_t
                .broadcast_sub(&kv_mem)?
                .broadcast_mul(&beta_t.unsqueeze(candle_core::D::Minus1)?)?;
            let delta_row = delta.unsqueeze(2)?.contiguous()?;
            let upd = k_col.matmul(&delta_row)?;
            state = (state + upd)?;

            let q_col = q_t.unsqueeze(candle_core::D::Minus1)?.contiguous()?;
            let out_t = state
                .transpose(2, 3)?
                .contiguous()?
                .matmul(&q_col)?
                .squeeze(candle_core::D::Minus1)?;
            outs.push(out_t);
        }

        if let Some(ref mut sc) = state_cache {
            **sc = Some(LinearStateCache {
                recurrent_state: state,
                conv_state,
            });
        }

        let mut y = Tensor::stack(&outs, 1)?;
        let y_flat = y.reshape((b * seq_len * attn.num_v_heads, attn.head_v_dim))?;
        let z_flat = z.reshape((b * seq_len * attn.num_v_heads, attn.head_v_dim))?;
        let y_norm = super::rms_norm_f32(&attn.norm, &y_flat)?;
        let y_gate = y_norm.broadcast_mul(&candle_nn::ops::silu(&z_flat)?)?;
        y = y_gate
            .reshape((b, seq_len, value_dim))?
            .to_dtype(act_dtype)?;

        attn.out_proj
            .forward(&y)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }

    fn seeded_nd(shape: &[usize], salt: f32, dev: &Device) -> Tensor {
        let count: usize = shape.iter().product();
        let vals: Vec<f32> = (0..count)
            .map(|i| ((i as f32 * 0.173 + salt).sin()) * 0.25)
            .collect();
        Tensor::from_vec(vals, shape, dev).unwrap()
    }

    fn build_linear_attn_with_vars(
        dev: &Device,
    ) -> (
        Qwen35LinearAttention,
        candle_core::Var,
        candle_core::Var,
        candle_core::Var,
    ) {
        let d = 64usize;
        let num_k_heads = 2usize;
        let num_v_heads = 4usize;
        let head_k_dim = 16usize;
        let head_v_dim = 16usize;
        let key_dim = num_k_heads * head_k_dim;
        let value_dim = num_v_heads * head_v_dim;
        let qkv_dim = key_dim + key_dim + value_dim;
        let kernel = 4usize;

        let make_dense = |out_f: usize, in_f: usize, salt: f32| -> QuantizedLinear {
            let wt = seeded_nd(&[in_f, out_f], salt, dev);
            QuantizedLinear::Unquantized(wt)
        };

        let conv_var =
            candle_core::Var::from_tensor(&seeded_nd(&[qkv_dim, kernel], 1.1, dev)).unwrap();
        let dt_bias_var =
            candle_core::Var::from_tensor(&seeded_nd(&[num_v_heads], 2.2, dev)).unwrap();
        let a_log_var = candle_core::Var::from_tensor(
            &Tensor::from_vec(vec![-2.1f32, -1.7, -1.3, -2.4], num_v_heads, dev).unwrap(),
        )
        .unwrap();

        let attn = Qwen35LinearAttention {
            qkv_proj: make_dense(qkv_dim, d, 0.3),
            z_proj: make_dense(value_dim, d, 0.7),
            b_proj: make_dense(num_v_heads, d, 1.5),
            a_proj: make_dense(num_v_heads, d, 2.3),
            out_proj: make_dense(d, value_dim, 3.1),
            conv_weight: conv_var.as_tensor().clone(),
            dt_bias: dt_bias_var.as_tensor().clone(),
            a_log: a_log_var.as_tensor().clone(),
            norm: RmsNorm::new(Tensor::ones(head_v_dim, DType::F32, dev).unwrap(), 1e-6),
            num_k_heads,
            num_v_heads,
            head_k_dim,
            head_v_dim,
        };
        (attn, conv_var, a_log_var, dt_bias_var)
    }

    fn max_abs_diff(a: &Tensor, b: &Tensor) -> f32 {
        assert_eq!(a.dims(), b.dims());
        let diff = (a.to_dtype(DType::F32).unwrap() - b.to_dtype(DType::F32).unwrap())
            .unwrap()
            .abs()
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        diff.into_iter().fold(0.0f32, f32::max)
    }

    #[test]
    fn qwen35_linear_attention_chunked_matches_recurrent_forward_and_backward_unpacked() {
        let mut devs = vec![Device::Cpu];
        if let Ok(metal) = Device::new_metal(0) {
            devs.push(metal);
        }
        for dev in devs {
            let (b, seq_len, d) = (2usize, 150usize, 64usize);
            let (attn, conv_var, a_log_var, dt_bias_var) = build_linear_attn_with_vars(&dev);
            let qkv_dim =
                attn.num_k_heads * attn.head_k_dim * 2 + attn.num_v_heads * attn.head_v_dim;
            let x_data = seeded_nd(&[b, seq_len, d], 4.2, &dev);
            let loss_w = seeded_nd(&[b, seq_len, d], 7.7, &dev);

            let init_cache = LinearStateCache {
                recurrent_state: seeded_nd(
                    &[b, attn.num_v_heads, attn.head_k_dim, attn.head_v_dim],
                    5.5,
                    &dev,
                ),
                conv_state: Some(seeded_nd(&[b, 3, qkv_dim], 6.6, &dev)),
            };

            let x_chunk = candle_core::Var::from_tensor(&x_data).unwrap();
            let mut cache_chunk = Some(init_cache.clone());
            let y_chunk = attn
                .forward_inner(x_chunk.as_tensor(), Some(&mut cache_chunk), None)
                .unwrap();
            let g_chunk = (y_chunk.clone() * &loss_w)
                .unwrap()
                .sum_all()
                .unwrap()
                .backward()
                .unwrap();

            let x_ref = candle_core::Var::from_tensor(&x_data).unwrap();
            let mut cache_ref = Some(init_cache);
            let y_ref = ref_linear_attention_forward_inner(
                &attn,
                x_ref.as_tensor(),
                Some(&mut cache_ref),
                None,
            )
            .unwrap();
            let g_ref = (y_ref.clone() * &loss_w)
                .unwrap()
                .sum_all()
                .unwrap()
                .backward()
                .unwrap();

            let fwd_diff = max_abs_diff(&y_chunk, &y_ref);
            assert!(fwd_diff < 1e-4, "dev={dev:?} fwd max_abs_diff={fwd_diff}");

            let sc_chunk = cache_chunk.as_ref().unwrap();
            let sc_ref = cache_ref.as_ref().unwrap();
            let rec_diff = max_abs_diff(&sc_chunk.recurrent_state, &sc_ref.recurrent_state);
            assert!(
                rec_diff < 1e-4,
                "dev={dev:?} recurrent_state max_abs_diff={rec_diff}"
            );
            let conv_state_diff = max_abs_diff(
                sc_chunk.conv_state.as_ref().unwrap(),
                sc_ref.conv_state.as_ref().unwrap(),
            );
            assert!(
                conv_state_diff < 1e-4,
                "dev={dev:?} conv_state max_abs_diff={conv_state_diff}"
            );

            let gx_diff = max_abs_diff(
                g_chunk.get(x_chunk.as_tensor()).unwrap(),
                g_ref.get(x_ref.as_tensor()).unwrap(),
            );
            let gconv_diff = max_abs_diff(
                g_chunk.get(conv_var.as_tensor()).unwrap(),
                g_ref.get(conv_var.as_tensor()).unwrap(),
            );
            let galog_diff = max_abs_diff(
                g_chunk.get(a_log_var.as_tensor()).unwrap(),
                g_ref.get(a_log_var.as_tensor()).unwrap(),
            );
            let gdt_diff = max_abs_diff(
                g_chunk.get(dt_bias_var.as_tensor()).unwrap(),
                g_ref.get(dt_bias_var.as_tensor()).unwrap(),
            );
            assert!(gx_diff < 1e-4, "dev={dev:?} grad x max_abs_diff={gx_diff}");
            assert!(
                gconv_diff < 1e-4,
                "dev={dev:?} grad conv max_abs_diff={gconv_diff}"
            );
            assert!(
                galog_diff < 1e-4,
                "dev={dev:?} grad a_log max_abs_diff={galog_diff}"
            );
            assert!(
                gdt_diff < 1e-4,
                "dev={dev:?} grad dt_bias max_abs_diff={gdt_diff}"
            );
        }
    }

    #[test]
    fn qwen35_linear_attention_chunked_matches_recurrent_forward_and_backward_packed() {
        let mut devs = vec![Device::Cpu];
        if let Ok(metal) = Device::new_metal(0) {
            devs.push(metal);
        }
        let mut doc_ids = Vec::with_capacity(145);
        doc_ids.extend(std::iter::repeat_n(0u32, 37));
        doc_ids.push(1u32);
        doc_ids.extend(std::iter::repeat_n(2u32, 72));
        doc_ids.extend(std::iter::repeat_n(3u32, 35));
        assert_eq!(doc_ids.len(), 145);

        for dev in devs {
            let (b, seq_len, d) = (1usize, 145usize, 64usize);
            let (attn, conv_var, a_log_var, dt_bias_var) = build_linear_attn_with_vars(&dev);
            let x_data = seeded_nd(&[b, seq_len, d], 9.1, &dev);
            let loss_w = seeded_nd(&[b, seq_len, d], 3.4, &dev);

            let x_chunk = candle_core::Var::from_tensor(&x_data).unwrap();
            let y_chunk = attn
                .forward_inner(x_chunk.as_tensor(), None, Some(&doc_ids))
                .unwrap();
            let g_chunk = (y_chunk.clone() * &loss_w)
                .unwrap()
                .sum_all()
                .unwrap()
                .backward()
                .unwrap();

            let x_ref = candle_core::Var::from_tensor(&x_data).unwrap();
            let y_ref =
                ref_linear_attention_forward_inner(&attn, x_ref.as_tensor(), None, Some(&doc_ids))
                    .unwrap();
            let g_ref = (y_ref.clone() * &loss_w)
                .unwrap()
                .sum_all()
                .unwrap()
                .backward()
                .unwrap();

            let fwd_diff = max_abs_diff(&y_chunk, &y_ref);
            assert!(
                fwd_diff < 1e-4,
                "dev={dev:?} packed fwd max_abs_diff={fwd_diff}"
            );

            let gx_diff = max_abs_diff(
                g_chunk.get(x_chunk.as_tensor()).unwrap(),
                g_ref.get(x_ref.as_tensor()).unwrap(),
            );
            let gconv_diff = max_abs_diff(
                g_chunk.get(conv_var.as_tensor()).unwrap(),
                g_ref.get(conv_var.as_tensor()).unwrap(),
            );
            let galog_diff = max_abs_diff(
                g_chunk.get(a_log_var.as_tensor()).unwrap(),
                g_ref.get(a_log_var.as_tensor()).unwrap(),
            );
            let gdt_diff = max_abs_diff(
                g_chunk.get(dt_bias_var.as_tensor()).unwrap(),
                g_ref.get(dt_bias_var.as_tensor()).unwrap(),
            );
            assert!(
                gx_diff < 1e-4,
                "dev={dev:?} packed grad x max_abs_diff={gx_diff}"
            );
            assert!(
                gconv_diff < 1e-4,
                "dev={dev:?} packed grad conv max_abs_diff={gconv_diff}"
            );
            assert!(
                galog_diff < 1e-4,
                "dev={dev:?} packed grad a_log max_abs_diff={galog_diff}"
            );
            assert!(
                gdt_diff < 1e-4,
                "dev={dev:?} packed grad dt_bias max_abs_diff={gdt_diff}"
            );
        }
    }

    #[test]
    fn sdpa_causal_hd256_rem_lt_16_does_not_read_oob_kv_into_nan() {
        let Ok(dev) = Device::new_metal(0) else {
            return;
        };
        let seq_len = 35usize;
        let head_dim = 256usize;
        let n_heads = 4usize;
        let n_kv_heads = 1usize;

        let q = Tensor::full(0.1f32, (1, n_heads, seq_len, head_dim), &dev)
            .unwrap()
            .to_dtype(DType::BF16)
            .unwrap();
        let mut backing = vec![0.1f32; 64 * head_dim];
        backing[seq_len * head_dim..].fill(f32::NAN);
        let kv_backing = Tensor::from_vec(backing, (64, head_dim), &dev)
            .unwrap()
            .to_dtype(DType::BF16)
            .unwrap();
        let k = kv_backing
            .narrow(0, 0, seq_len)
            .unwrap()
            .reshape((1, n_kv_heads, seq_len, head_dim))
            .unwrap();
        let v = kv_backing
            .narrow(0, 0, seq_len)
            .unwrap()
            .reshape((1, n_kv_heads, seq_len, head_dim))
            .unwrap();

        let scale = 1.0 / (head_dim as f32).sqrt();
        let out = candle_nn::ops::sdpa(&q, &k, &v, None, true, scale, 1.0).unwrap();
        let out_vec = out
            .to_dtype(DType::F32)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        assert!(
            out_vec.iter().all(|x| x.is_finite()),
            "causal Metal SDPA produced non-finite output (OOB K/V block read)"
        );
    }
}
