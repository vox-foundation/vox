//! `forward_masked_ce` — device-agnostic masked cross-entropy forward pass.

use super::types::{DetachedHeadBackward, MaskedCeForward};
use crate::config::LoraTrainingConfig;
use anyhow::Result;
use candle_core::Device;

/// Controls whether `lm_head` + cross-entropy runs under no-grad preflight,
/// eager full-tape autograd, or detached-boundary chunked `lm_head` backward.
#[derive(Clone, Copy)]
pub enum HeadCeMode<'a> {
    /// Preflight / validation pass (`with_no_grad`): projects active chunks via
    /// [`ForwardCeGraph::head_forward_rows`] without retaining autograd graphs.
    NoGradPreflight,
    /// Eager autograd pass (`!gradient_checkpointing`): builds live graph over
    /// active chunks and returns `loss: Some(loss)`.
    EagerTape,
    /// Detached-boundary chunked head backward (`gradient_checkpointing`):
    /// computes `head_forward_rows` + `.backward()` inside each 256-row chunk and
    /// immediately frees chunk logits before threading `d_final_hidden` back through segments.
    Checkpointed {
        trainable_vars: &'a [candle_core::Var],
        grad_accum_steps: usize,
    },
}

/// Backend model graph abstraction implemented by each device plugin (`TrainGraphModel`).
pub trait ForwardCeGraph {
    type Segment;

    /// Full transformer stack before final `norm` + `lm_head`, returning `[b, seq, d_model]`.
    fn forward_hidden_eager(&self, input_ids: &candle_core::Tensor) -> Result<candle_core::Tensor>;
    /// Packed transformer stack before final `norm` + `lm_head`, returning `[1, seq, d_model]`.
    fn forward_packed_hidden_eager(
        &self,
        input_ids: &candle_core::Tensor,
        doc_ids: &[u32],
    ) -> Result<candle_core::Tensor>;
    /// Checkpointed transformer stack returning detached final hidden state `[b, seq, d_model]`
    /// and all segment boundary inputs (`0..n_segments`) for full reverse recomputation.
    fn forward_checkpointed_hidden_segments(
        &self,
        input_ids: &candle_core::Tensor,
        n_segments: usize,
    ) -> Result<(candle_core::Tensor, Vec<Self::Segment>)>;
    /// Checkpointed packed transformer stack returning detached final hidden state
    /// `[1, seq, d_model]` and all segment boundary inputs (`0..n_segments`).
    fn forward_checkpointed_packed_hidden_segments(
        &self,
        input_ids: &candle_core::Tensor,
        doc_ids: &[u32],
        n_segments: usize,
    ) -> Result<(candle_core::Tensor, Vec<Self::Segment>)>;
    /// Final RMSNorm + `lm_head` projection + F32 cast on `[chunk_rows, d_model]` -> `[chunk_rows, vocab]`.
    fn head_forward_rows(&self, hidden_2d: &candle_core::Tensor) -> Result<candle_core::Tensor>;

    /// Synchronize accelerator command queue between chunk backwards so Metal/CUDA
    /// buffer pools prune transient `[256, vocab]` logits buffers immediately.
    fn sync_device(&self) -> Result<()> {
        Ok(())
    }

    /// Transformer layer count used to auto-scale gradient checkpoint segments.
    fn num_layers(&self) -> usize {
        0
    }
}

/// Input slice for one encoded example inside a packed training bin.
pub struct PackedSampleRef<'a> {
    pub ids: &'a [u32],
    pub prefix_len: usize,
    pub trunc_offset: usize,
    pub sample_weight: f64,
    pub token_weights: Option<&'a [f32]>,
}

/// Build the shifted next-token supervision mask (`len = ids.len() - 1`) for one sample.
pub fn build_token_mask_vec(
    ids_len: usize,
    prefix_len: usize,
    trunc_offset: usize,
    token_weights: Option<&[f32]>,
    qlora_ce_last_k: usize,
) -> Vec<f32> {
    if ids_len < 2 {
        return Vec::new();
    }
    let prompt_len = prefix_len.saturating_sub(trunc_offset);
    let ce_last_k = if qlora_ce_last_k == 0 {
        ids_len
    } else {
        qlora_ce_last_k
    };
    let last_k_start = ids_len.saturating_sub(ce_last_k);

    let mut mask_vec: Vec<f32> = (0..ids_len - 1)
        .map(|i| {
            let target_idx = i + 1;
            if target_idx >= prompt_len && target_idx >= last_k_start {
                1.0f32
            } else {
                0.0
            }
        })
        .collect();

    if let Some(tw) = token_weights {
        for (i, &w) in tw.iter().enumerate() {
            let target_idx = i + 1;
            if target_idx < ids_len && i < mask_vec.len() {
                mask_vec[i] *= w;
            }
        }
    }
    mask_vec
}

/// Total supervised mask weight for an encoded step prior to backward.
pub fn supervised_mask_sum_preview(
    ids_len: usize,
    prefix_len: usize,
    trunc_offset: usize,
    token_weights: Option<&[f32]>,
    qlora_ce_last_k: usize,
) -> f32 {
    build_token_mask_vec(
        ids_len,
        prefix_len,
        trunc_offset,
        token_weights,
        qlora_ce_last_k,
    )
    .into_iter()
    .sum()
}

/// Scale factor correcting length bias across a `grad_accum` window.
///
/// Unnormalized gradient accumulation divides each micro-step loss equally by
/// `window_len`, giving tokens in short responses `>10x` larger gradient updates
/// than tokens in long responses. Multiplying each item's loss by
/// `(item_mask_sum / window_mask_sum) * window_len` makes the accumulated
/// gradient equal the exact token-weighted mean CE across the window.
pub fn token_norm_window_scale(item_mask_sum: f32, window_mask_sum: f32, window_len: usize) -> f32 {
    if window_len == 0
        || item_mask_sum <= 0.0
        || window_mask_sum <= 0.0
        || !item_mask_sum.is_finite()
        || !window_mask_sum.is_finite()
    {
        return 1.0;
    }
    (item_mask_sum / window_mask_sum) * (window_len as f32)
}

/// Concatenate multiple encoded samples into one packed
/// `(ids, doc_ids, mask_vec, unweighted_mask_sum, weighted_mask_sum)` sequence,
/// zeroing `mask_vec` on every cross-document target transition (`doc_end - 1`).
pub fn build_packed_mask_and_doc_ids(
    samples: &[PackedSampleRef<'_>],
    qlora_ce_last_k: usize,
) -> (Vec<u32>, Vec<u32>, Vec<f32>, f32, f32) {
    let valid: Vec<&PackedSampleRef<'_>> = samples.iter().filter(|s| s.ids.len() >= 2).collect();
    if valid.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new(), 0.0, 0.0);
    }
    let total_tokens: usize = valid.iter().map(|s| s.ids.len()).sum();
    let mut full_ids = Vec::with_capacity(total_tokens);
    let mut full_doc_ids = Vec::with_capacity(total_tokens);
    let mut full_mask = Vec::with_capacity(total_tokens.saturating_sub(1));
    let mut unweighted_mask_sum = 0.0f32;
    let mut weighted_mask_sum = 0.0f32;

    for (doc_idx, sample) in valid.iter().enumerate() {
        if doc_idx > 0 {
            // Last token of the previous doc must never be trained to predict the first
            // token of the next doc.
            full_mask.push(0.0f32);
        }
        full_ids.extend_from_slice(sample.ids);
        full_doc_ids.extend(std::iter::repeat_n(doc_idx as u32, sample.ids.len()));
        let mut mask = build_token_mask_vec(
            sample.ids.len(),
            sample.prefix_len,
            sample.trunc_offset,
            sample.token_weights,
            qlora_ce_last_k,
        );
        let doc_unweighted: f32 = mask.iter().copied().sum();
        unweighted_mask_sum += doc_unweighted;
        let sample_w = sample.sample_weight.max(0.0) as f32;
        if (sample_w - 1.0).abs() > f32::EPSILON {
            for w in &mut mask {
                *w *= sample_w;
            }
        }
        let doc_weighted: f32 = mask.iter().copied().sum();
        weighted_mask_sum += doc_weighted;
        full_mask.extend(mask);
    }

    (
        full_ids,
        full_doc_ids,
        full_mask,
        unweighted_mask_sum,
        weighted_mask_sum,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn forward_masked_ce<M: ForwardCeGraph>(
    model: &M,
    ids: &[u32],
    prefix_len: usize,
    trunc_offset: usize,
    sample_weight: f64,
    token_weights: Option<&[f32]>,
    config: &LoraTrainingConfig,
    head_mode: HeadCeMode<'_>,
    device: &Device,
) -> Result<MaskedCeForward<M::Segment>> {
    let ids_len = ids.len();
    if ids_len < 2 {
        return Ok(MaskedCeForward::NoSupervision);
    }

    let mask_vec = build_token_mask_vec(
        ids_len,
        prefix_len,
        trunc_offset,
        token_weights,
        config.qlora_ce_last_k,
    );

    let mask_sum_cpu: f32 = mask_vec.iter().copied().sum();
    if mask_sum_cpu <= 0.0 || !mask_sum_cpu.is_finite() {
        return Ok(MaskedCeForward::NoSupervision);
    }

    run_masked_ce_with_head(
        model,
        ids,
        None,
        mask_vec,
        sample_weight,
        config,
        head_mode,
        device,
    )
}

/// Forward pass for a packed sequence bin (`sequence_packing`).
///
/// Uses `ForwardCeGraph::forward_checkpointed_packed_hidden_segments` (with
/// detached-boundary 256-row chunked `lm_head` backward when `HeadCeMode::Checkpointed`)
/// or `ForwardCeGraph::forward_packed_hidden_eager` so attention stays block-diagonal
/// per document and RoPE positions reset cleanly at every document boundary.
#[allow(clippy::too_many_arguments)]
pub fn forward_packed_masked_ce<M: ForwardCeGraph>(
    model: &M,
    samples: &[PackedSampleRef<'_>],
    loss_scale_multiplier: f64,
    config: &LoraTrainingConfig,
    head_mode: HeadCeMode<'_>,
    device: &Device,
) -> Result<MaskedCeForward<M::Segment>> {
    let (ids, doc_ids, mask_vec, unweighted_mask_sum, weighted_mask_sum) =
        build_packed_mask_and_doc_ids(samples, config.qlora_ce_last_k);
    let ids_len = ids.len();
    if ids_len < 2
        || unweighted_mask_sum <= 0.0
        || weighted_mask_sum <= 0.0
        || !unweighted_mask_sum.is_finite()
        || !weighted_mask_sum.is_finite()
    {
        return Ok(MaskedCeForward::NoSupervision);
    }

    // `run_masked_ce_with_head` divides the weighted summed logprobs by `weighted_mask_sum`;
    // scaling by `(weighted / unweighted) * loss_scale_multiplier` cancels `weighted` so
    // the resulting gradient is identically `-sum(w_i * m * logp) / unweighted * loss_scale_multiplier`.
    let effective_sample_weight =
        (weighted_mask_sum as f64 / unweighted_mask_sum as f64) * loss_scale_multiplier;

    run_masked_ce_with_head(
        model,
        &ids,
        Some(&doc_ids[..ids_len - 1]),
        mask_vec,
        effective_sample_weight,
        config,
        head_mode,
        device,
    )
}

/// Resolve checkpoint segment count from an optional `VOX_MENS_GC_SEGMENTS`
/// value and the model's layer count.
fn gradient_checkpoint_segments_with(env_raw: Option<&str>, num_layers: usize) -> usize {
    if let Some(n) = env_raw
        .map(str::trim)
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n >= 1)
    {
        return n;
    }
    if num_layers >= 48 {
        (num_layers / 2).clamp(16, num_layers)
    } else {
        4
    }
}

/// Number of checkpoint segments to split the layer stack into.
fn gradient_checkpoint_segments(_config: &LoraTrainingConfig, num_layers: usize) -> usize {
    gradient_checkpoint_segments_with(
        std::env::var("VOX_MENS_GC_SEGMENTS").ok().as_deref(),
        num_layers,
    )
}

const CE_ROW_CHUNK_SIZE: usize = 256;

fn scalar_f32(t: &candle_core::Tensor) -> Result<f32> {
    match t.rank() {
        0 => Ok(t.to_scalar::<f32>()?),
        1 if t.dim(0)? == 1 => Ok(t.squeeze(0)?.to_scalar::<f32>()?),
        r => anyhow::bail!("unexpected loss rank: expected scalar or [1], got rank={r}"),
    }
}

fn check_max_target_vocab(max_tgt: usize, vocab_dim: usize) -> Result<()> {
    if max_tgt >= vocab_dim {
        anyhow::bail!(
            "forward_masked_ce: max target token id {max_tgt} >= logits vocab dimension {vocab_dim}"
        );
    }
    Ok(())
}

fn masked_chunk_logprob_sum(
    logits_c: &candle_core::Tensor,
    targets_c: &candle_core::Tensor,
    mask_slice: &[f32],
) -> Result<candle_core::Tensor> {
    let len = mask_slice.len();
    let mask_c = candle_core::Tensor::from_slice(mask_slice, len, logits_c.device())?;
    let log_sm = candle_nn::ops::log_softmax(logits_c, 1)?;
    let logprobs = log_sm.gather(&targets_c.unsqueeze(1)?, 1)?.flatten_all()?;
    Ok(logprobs.broadcast_mul(&mask_c)?.sum_all()?)
}

#[cfg(test)]
fn chunked_masked_cross_entropy_sum(
    logits: &candle_core::Tensor,
    targets_flat: &candle_core::Tensor,
    mask_vec: &[f32],
    chunk_rows: usize,
) -> Result<candle_core::Tensor> {
    let n_rows = logits.dim(0)?;
    let chunk = chunk_rows.max(1);
    let mut total: Option<candle_core::Tensor> = None;
    for start in (0..n_rows).step_by(chunk) {
        let end = (start + chunk).min(n_rows);
        let mask_slice = &mask_vec[start..end];
        if mask_slice.iter().all(|&m| m == 0.0) {
            continue;
        }
        let len = end - start;
        let logits_c = logits.narrow(0, start, len)?;
        let targets_c = targets_flat.narrow(0, start, len)?;
        let sum_c = masked_chunk_logprob_sum(&logits_c, &targets_c, mask_slice)?;
        total = Some(match total {
            Some(acc) => (acc + sum_c)?,
            None => sum_c,
        });
    }
    total.ok_or_else(|| anyhow::anyhow!("chunked_masked_cross_entropy_sum: no active mask rows"))
}

#[allow(clippy::too_many_arguments)]
fn run_masked_ce_with_head<M: ForwardCeGraph>(
    model: &M,
    ids: &[u32],
    packed_doc_ids: Option<&[u32]>,
    mask_vec: Vec<f32>,
    effective_sample_weight: f64,
    config: &LoraTrainingConfig,
    head_mode: HeadCeMode<'_>,
    device: &Device,
) -> Result<MaskedCeForward<M::Segment>> {
    let ids_len = ids.len();
    let mask_sum: f32 = mask_vec.iter().copied().sum();
    if ids_len < 2 || mask_sum <= 0.0 || !mask_sum.is_finite() {
        return Ok(MaskedCeForward::NoSupervision);
    }

    let max_tgt = ids[1..].iter().copied().max().unwrap_or(0) as usize;
    let input_ids = candle_core::Tensor::new(&ids[..ids_len - 1], device)?.unsqueeze(0)?;
    let targets_flat = candle_core::Tensor::new(&ids[1..], device)?;
    let supervised_tokens = mask_sum.max(0.0) as u64;
    let theoretical_tokens = (ids_len.saturating_sub(1)) as u64;

    match head_mode {
        HeadCeMode::NoGradPreflight => {
            let hidden_3d = candle_core::with_no_grad(|| match packed_doc_ids {
                Some(doc_ids) => model.forward_packed_hidden_eager(&input_ids, doc_ids),
                None => model.forward_hidden_eager(&input_ids),
            })?;
            let hidden_2d = hidden_3d.flatten_to(1)?;
            let n_rows = hidden_2d.dim(0)?;
            let mut checked_vocab = false;
            let mut logprob_sum_f64 = 0.0f64;
            for start in (0..n_rows).step_by(CE_ROW_CHUNK_SIZE) {
                let end = (start + CE_ROW_CHUNK_SIZE).min(n_rows);
                let mask_slice = &mask_vec[start..end];
                if mask_slice.iter().all(|&m| m == 0.0) {
                    continue;
                }
                let len = end - start;
                let h_slice = hidden_2d.narrow(0, start, len)?.contiguous()?;
                let logits_c = candle_core::with_no_grad(|| model.head_forward_rows(&h_slice))?;
                if !checked_vocab {
                    check_max_target_vocab(max_tgt, logits_c.dim(1)?)?;
                    checked_vocab = true;
                }
                let targets_c = targets_flat.narrow(0, start, len)?;
                let sum_c = candle_core::with_no_grad(|| {
                    masked_chunk_logprob_sum(&logits_c, &targets_c, mask_slice)
                })?;
                let sum_val = scalar_f32(&sum_c)?;
                if !sum_val.is_finite() {
                    let kind = if sum_val.is_nan() { "nan" } else { "inf" };
                    return Ok(MaskedCeForward::NonFinite { kind, mask_sum });
                }
                logprob_sum_f64 += sum_val as f64;
            }
            let loss_scalar = (-logprob_sum_f64 / (mask_sum as f64)) as f32;
            if !loss_scalar.is_finite() {
                let kind = if loss_scalar.is_nan() { "nan" } else { "inf" };
                return Ok(MaskedCeForward::NonFinite { kind, mask_sum });
            }
            Ok(MaskedCeForward::Finite {
                loss: None,
                loss_scalar,
                supervised_tokens,
                theoretical_tokens,
                syntax_weight_sum: mask_sum,
                segments: None,
                detached_head: None,
            })
        }
        HeadCeMode::EagerTape => {
            let hidden_3d = match packed_doc_ids {
                Some(doc_ids) => model.forward_packed_hidden_eager(&input_ids, doc_ids)?,
                None => model.forward_hidden_eager(&input_ids)?,
            };
            let hidden_2d = hidden_3d.flatten_to(1)?;
            let n_rows = hidden_2d.dim(0)?;
            let mut checked_vocab = false;
            let mut total: Option<candle_core::Tensor> = None;
            for start in (0..n_rows).step_by(CE_ROW_CHUNK_SIZE) {
                let end = (start + CE_ROW_CHUNK_SIZE).min(n_rows);
                let mask_slice = &mask_vec[start..end];
                if mask_slice.iter().all(|&m| m == 0.0) {
                    continue;
                }
                let len = end - start;
                let h_slice = hidden_2d.narrow(0, start, len)?;
                let logits_c = model.head_forward_rows(&h_slice)?;
                if !checked_vocab {
                    check_max_target_vocab(max_tgt, logits_c.dim(1)?)?;
                    checked_vocab = true;
                }
                let targets_c = targets_flat.narrow(0, start, len)?;
                let sum_c = masked_chunk_logprob_sum(&logits_c, &targets_c, mask_slice)?;
                total = Some(match total {
                    Some(acc) => (acc + sum_c)?,
                    None => sum_c,
                });
            }
            let logprob_sum = total
                .ok_or_else(|| anyhow::anyhow!("run_masked_ce_with_head: no active mask rows"))?;
            let unweighted = (logprob_sum / (mask_sum as f64))?.neg()?;
            let w_t = candle_core::Tensor::new(&[effective_sample_weight as f32], device)?;
            let loss = unweighted.broadcast_mul(&w_t)?;
            let loss_scalar = scalar_f32(&unweighted)?;
            if !loss_scalar.is_finite() {
                let kind = if loss_scalar.is_nan() { "nan" } else { "inf" };
                return Ok(MaskedCeForward::NonFinite { kind, mask_sum });
            }
            Ok(MaskedCeForward::Finite {
                loss: Some(loss),
                loss_scalar,
                supervised_tokens,
                theoretical_tokens,
                syntax_weight_sum: mask_sum,
                segments: None,
                detached_head: None,
            })
        }
        HeadCeMode::Checkpointed {
            trainable_vars,
            grad_accum_steps,
        } => {
            let n_segments = gradient_checkpoint_segments(config, model.num_layers());
            let (final_hidden_3d, segments) = match packed_doc_ids {
                Some(doc_ids) => model
                    .forward_checkpointed_packed_hidden_segments(&input_ids, doc_ids, n_segments)?,
                None => model.forward_checkpointed_hidden_segments(&input_ids, n_segments)?,
            };
            let hidden_shape = final_hidden_3d.dims().to_vec();
            let final_hidden_2d = final_hidden_3d.flatten_to(1)?;
            let n_rows = final_hidden_2d.dim(0)?;
            let hidden_dtype = final_hidden_2d.dtype();
            let d_model = final_hidden_2d.dim(1)?;
            let accum = grad_accum_steps.max(1) as f64;
            let scale_coeff = -(effective_sample_weight / ((mask_sum as f64) * accum)) as f32;
            let scale_t = candle_core::Tensor::new(&[scale_coeff], device)?;

            let mut checked_vocab = false;
            let mut unweighted_nll_sum = 0.0f64;
            let mut head_grads: Option<candle_core::backprop::GradStore> = None;
            let mut d_hidden_chunks: Vec<candle_core::Tensor> =
                Vec::with_capacity(n_rows.div_ceil(CE_ROW_CHUNK_SIZE));

            for start in (0..n_rows).step_by(CE_ROW_CHUNK_SIZE) {
                let end = (start + CE_ROW_CHUNK_SIZE).min(n_rows);
                let len = end - start;
                let mask_slice = &mask_vec[start..end];
                if mask_slice.iter().all(|&m| m == 0.0) {
                    d_hidden_chunks.push(candle_core::Tensor::zeros(
                        (len, d_model),
                        hidden_dtype,
                        device,
                    )?);
                    continue;
                }

                let h_slice = final_hidden_2d.narrow(0, start, len)?.contiguous()?;
                let h_var = candle_core::Var::from_tensor(&h_slice)?;
                let logits_c = model.head_forward_rows(h_var.as_tensor())?;
                if !checked_vocab {
                    check_max_target_vocab(max_tgt, logits_c.dim(1)?)?;
                    checked_vocab = true;
                }
                let targets_c = targets_flat.narrow(0, start, len)?;
                let sum_c = masked_chunk_logprob_sum(&logits_c, &targets_c, mask_slice)?;
                let sum_val = scalar_f32(&sum_c)?;
                if !sum_val.is_finite() {
                    let kind = if sum_val.is_nan() { "nan" } else { "inf" };
                    return Ok(MaskedCeForward::NonFinite { kind, mask_sum });
                }
                unweighted_nll_sum += -(sum_val as f64);

                let chunk_loss = sum_c.broadcast_mul(&scale_t)?;
                let mut chunk_grads = chunk_loss.backward()?;
                let d_h_c = chunk_grads.remove(h_var.as_tensor()).ok_or_else(|| {
                    anyhow::anyhow!(
                        "chunked head backward: missing gradient for hidden chunk [{start}..{end}]"
                    )
                })?;
                d_hidden_chunks.push(d_h_c);

                if let Some(ref mut acc) = head_grads {
                    qlora_rs::accumulate_grads_for_vars(acc, &chunk_grads, trainable_vars)?;
                    drop(chunk_grads);
                } else {
                    chunk_grads.retain_vars(trainable_vars);
                    head_grads = Some(chunk_grads);
                }

                drop(chunk_loss);
                drop(sum_c);
                drop(logits_c);
                drop(h_var);
                drop(h_slice);
                model.sync_device()?;
            }

            let loss_scalar = (unweighted_nll_sum / (mask_sum as f64)) as f32;
            if !loss_scalar.is_finite() {
                let kind = if loss_scalar.is_nan() { "nan" } else { "inf" };
                return Ok(MaskedCeForward::NonFinite { kind, mask_sum });
            }
            let d_final_hidden =
                candle_core::Tensor::cat(&d_hidden_chunks, 0)?.reshape(hidden_shape)?;
            Ok(MaskedCeForward::Finite {
                loss: None,
                loss_scalar,
                supervised_tokens,
                theoretical_tokens,
                syntax_weight_sum: mask_sum,
                segments: Some(segments),
                detached_head: Some(DetachedHeadBackward {
                    head_grads,
                    d_final_hidden,
                }),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_norm_window_scale_reweights_length_bias() {
        let s_short = token_norm_window_scale(10.0, 100.0, 2);
        let s_long = token_norm_window_scale(90.0, 100.0, 2);
        assert!((s_short - 0.2).abs() < 1e-6);
        assert!((s_long - 1.8).abs() < 1e-6);
        assert!((s_short + s_long - 2.0).abs() < 1e-6);
        assert!((token_norm_window_scale(0.0, 100.0, 2) - 1.0).abs() < 1e-6);
        assert!((token_norm_window_scale(10.0, 0.0, 2) - 1.0).abs() < 1e-6);
        assert!((token_norm_window_scale(10.0, 100.0, 0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn packed_mask_zeroes_cross_document_boundary() {
        let doc0_ids = [10u32, 11, 12, 13];
        let doc1_ids = [20u32, 21, 22];
        let samples = [
            PackedSampleRef {
                ids: &doc0_ids,
                prefix_len: 2,
                trunc_offset: 0,
                sample_weight: 1.0,
                token_weights: None,
            },
            PackedSampleRef {
                ids: &doc1_ids,
                prefix_len: 1,
                trunc_offset: 0,
                sample_weight: 1.0,
                token_weights: None,
            },
        ];

        let (ids, doc_ids, mask, unweighted_sum, weighted_sum) =
            build_packed_mask_and_doc_ids(&samples, 0);
        assert_eq!(ids, vec![10, 11, 12, 13, 20, 21, 22]);
        assert_eq!(doc_ids, vec![0, 0, 0, 0, 1, 1, 1]);
        assert!((unweighted_sum - 4.0).abs() < 1e-6);
        assert!((weighted_sum - 4.0).abs() < 1e-6);
        assert_eq!(mask, vec![0.0, 1.0, 1.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn packed_mask_preserves_heterogeneous_sample_weights() {
        let doc0_ids = [1u32, 2, 3];
        let doc1_ids = [4u32, 5, 6, 7, 8];
        let samples = [
            PackedSampleRef {
                ids: &doc0_ids,
                prefix_len: 0,
                trunc_offset: 0,
                sample_weight: 1.0,
                token_weights: None,
            },
            PackedSampleRef {
                ids: &doc1_ids,
                prefix_len: 0,
                trunc_offset: 0,
                sample_weight: 3.0,
                token_weights: None,
            },
        ];

        let (_ids, _doc_ids, mask, unweighted_sum, weighted_sum) =
            build_packed_mask_and_doc_ids(&samples, 0);
        assert!((unweighted_sum - 6.0).abs() < 1e-6);
        assert!((weighted_sum - 14.0).abs() < 1e-6);
        assert_eq!(mask, vec![1.0, 1.0, 0.0, 3.0, 3.0, 3.0, 3.0]);
    }

    #[test]
    fn flush_pending_grads_steps_trailing_partial_window() {
        let mut cfg = qlora_rs::training::QLoraTrainingConfig::default();
        cfg.adapter_config.gradient_accumulation_steps = 4;
        cfg.adapter_config.weight_decay = 0.0;
        cfg.adapter_config.max_grad_norm = None;
        let mut trainer = qlora_rs::training::QLoraTrainer::new(cfg, Device::Cpu);
        let w = trainer
            .var_builder()
            .get_with_hints(1, "w", candle_nn::Init::Const(1.0))
            .unwrap();
        trainer.init_optimizer(&[]).unwrap();

        let scalar = |t: &candle_core::Tensor| -> f32 {
            t.flatten_all().unwrap().to_vec1::<f32>().unwrap()[0]
        };
        let before = scalar(&w);
        assert_eq!(trainer.pending_accumulation_steps(), 0);

        trainer
            .backward_step(&w.affine(2.0, 0.0).unwrap().sum_all().unwrap())
            .unwrap();
        assert_eq!(trainer.pending_accumulation_steps(), 1);
        assert!(
            (scalar(&w) - before).abs() < 1e-9,
            "optimizer must not step before accumulation window closes"
        );

        let flushed = trainer.flush_pending_grads(true).unwrap();
        assert!(
            flushed,
            "expected pending grads to trigger an optimizer step"
        );
        assert_eq!(trainer.pending_accumulation_steps(), 0);
        assert!(
            scalar(&w) < before - 1e-6,
            "flush_pending_grads did not update weight: before={before}, after={}",
            scalar(&w)
        );
        assert!(!trainer.flush_pending_grads(true).unwrap());
    }

    #[test]
    fn chunked_masked_ce_matches_dense_loss_and_logits_grad() {
        let dev = Device::Cpu;
        let rows = 600usize;
        let vocab = 37usize;
        let logits_data: Vec<f32> = (0..(rows * vocab))
            .map(|i| (((i * 17 + 3) % 97) as f32 - 48.0) * 0.05)
            .collect();
        let logits_base = candle_core::Tensor::from_vec(logits_data, (rows, vocab), &dev).unwrap();
        let targets_data: Vec<u32> = (0..rows).map(|i| ((i * 7 + 5) % vocab) as u32).collect();
        let targets = candle_core::Tensor::from_vec(targets_data, rows, &dev).unwrap();
        let mask_vec: Vec<f32> = (0..rows)
            .map(|i| {
                if i < 280 {
                    0.0
                } else {
                    ((i % 5) as f32 + 1.0) * 0.25
                }
            })
            .collect();
        let mask_sum: f32 = mask_vec.iter().sum();

        let v_dense = candle_core::Var::from_tensor(&logits_base).unwrap();
        let mask_t = candle_core::Tensor::from_slice(&mask_vec, rows, &dev).unwrap();
        let dense_loss = (candle_nn::ops::log_softmax(v_dense.as_tensor(), 1)
            .unwrap()
            .gather(&targets.unsqueeze(1).unwrap(), 1)
            .unwrap()
            .flatten_all()
            .unwrap()
            .broadcast_mul(&mask_t)
            .unwrap()
            .sum_all()
            .unwrap()
            / mask_t.sum_all().unwrap())
        .unwrap()
        .neg()
        .unwrap();
        let grads_dense = dense_loss.backward().unwrap();

        let v_chunk = candle_core::Var::from_tensor(&logits_base).unwrap();
        let chunk_sum = chunked_masked_cross_entropy_sum(
            v_chunk.as_tensor(),
            &targets,
            &mask_vec,
            CE_ROW_CHUNK_SIZE,
        )
        .unwrap();
        let chunk_loss = (chunk_sum / (mask_sum as f64)).unwrap().neg().unwrap();
        let grads_chunk = chunk_loss.backward().unwrap();

        let d_s = dense_loss.to_scalar::<f32>().unwrap();
        let c_s = chunk_loss.to_scalar::<f32>().unwrap();
        assert!(
            (d_s - c_s).abs() < 1e-5,
            "loss mismatch: dense={d_s}, chunked={c_s}"
        );

        let gd = grads_dense
            .get(v_dense.as_tensor())
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let gc = grads_chunk
            .get(v_chunk.as_tensor())
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        for (i, (&a, &b)) in gd.iter().zip(gc.iter()).enumerate() {
            assert!(
                (a - b).abs() < 1e-5,
                "grad mismatch at {i}: dense={a}, chunked={b}"
            );
        }
    }

    #[test]
    fn gradient_checkpoint_segments_auto_scales_and_honors_env_override() {
        assert_eq!(gradient_checkpoint_segments_with(None, 0), 4);
        assert_eq!(gradient_checkpoint_segments_with(None, 36), 4);
        assert_eq!(gradient_checkpoint_segments_with(None, 48), 24);
        assert_eq!(gradient_checkpoint_segments_with(None, 64), 32);

        // Explicit `VOX_MENS_GC_SEGMENTS` override wins across all layer counts.
        assert_eq!(gradient_checkpoint_segments_with(Some("12"), 64), 12);
        assert_eq!(gradient_checkpoint_segments_with(Some(" 8 "), 0), 8);
        // Invalid/zero env values fall back to layer-based scaling.
        assert_eq!(gradient_checkpoint_segments_with(Some("0"), 64), 32);
        assert_eq!(gradient_checkpoint_segments_with(Some("invalid"), 36), 4);
    }

    #[test]
    fn detached_chunked_head_matches_eager_and_nograd_across_multiple_chunks() {
        use candle_core::{DType, Tensor, Var};
        struct ToySeg {
            input: Tensor,
            start: usize,
            end: usize,
        }
        struct ToyModel {
            embed: Tensor,
            layers: Vec<Var>,
            lm_head: Var,
        }
        impl ToyModel {
            fn apply_layers(&self, h: &Tensor, start: usize, end: usize) -> Tensor {
                let mut cur = h.clone();
                for w in &self.layers[start..end] {
                    let delta = cur.broadcast_matmul(w.as_tensor()).unwrap().tanh().unwrap();
                    cur = (&cur + &delta).unwrap();
                }
                cur
            }
            fn recompute(&self, seg: &ToySeg) -> (Var, Tensor) {
                let iv = Var::from_tensor(&seg.input).unwrap();
                let out = self.apply_layers(iv.as_tensor(), seg.start, seg.end);
                (iv, out)
            }
        }
        impl ForwardCeGraph for ToyModel {
            type Segment = ToySeg;

            fn forward_hidden_eager(&self, input_ids: &Tensor) -> Result<Tensor> {
                let (b, s) = input_ids.dims2()?;
                let d = self.embed.dim(1)?;
                let x = self
                    .embed
                    .index_select(&input_ids.flatten_all()?, 0)?
                    .reshape((b, s, d))?;
                Ok(self.apply_layers(&x, 0, self.layers.len()))
            }

            fn forward_packed_hidden_eager(
                &self,
                input_ids: &Tensor,
                _doc_ids: &[u32],
            ) -> Result<Tensor> {
                self.forward_hidden_eager(input_ids)
            }

            fn forward_checkpointed_hidden_segments(
                &self,
                input_ids: &Tensor,
                n_segments: usize,
            ) -> Result<(Tensor, Vec<Self::Segment>)> {
                let (b, s) = input_ids.dims2()?;
                let d = self.embed.dim(1)?;
                let x = self
                    .embed
                    .index_select(&input_ids.flatten_all()?, 0)?
                    .reshape((b, s, d))?;
                let mut segs = Vec::new();
                let mut cur = x.detach();
                let n = self.layers.len();
                let segs_n = n_segments.clamp(1, n);
                let base = n / segs_n;
                let rem = n % segs_n;
                let mut idx = 0;
                for i in 0..segs_n {
                    let end = idx + base + usize::from(i < rem);
                    segs.push(ToySeg {
                        input: cur.clone(),
                        start: idx,
                        end,
                    });
                    cur = candle_core::with_no_grad(|| {
                        candle_core::Result::Ok(self.apply_layers(&cur, idx, end))
                    })?
                    .detach();
                    idx = end;
                }
                Ok((cur, segs))
            }

            fn forward_checkpointed_packed_hidden_segments(
                &self,
                input_ids: &Tensor,
                _doc_ids: &[u32],
                n_segments: usize,
            ) -> Result<(Tensor, Vec<Self::Segment>)> {
                self.forward_checkpointed_hidden_segments(input_ids, n_segments)
            }

            fn head_forward_rows(&self, hidden_2d: &Tensor) -> Result<Tensor> {
                Ok(hidden_2d.matmul(self.lm_head.as_tensor())?)
            }

            fn num_layers(&self) -> usize {
                self.layers.len()
            }
        }

        let dev = Device::Cpu;
        let d = 6usize;
        let vocab = 19usize;
        let embed_vec: Vec<f32> = (0..(vocab * d))
            .map(|i| (((i * 13 + 7) % 53) as f32 - 26.0) * 0.04)
            .collect();
        let l0_vec: Vec<f32> = (0..(d * d))
            .map(|i| (((i * 11 + 1) % 31) as f32 - 15.0) * 0.03)
            .collect();
        let l1_vec: Vec<f32> = (0..(d * d))
            .map(|i| (((i * 17 + 5) % 37) as f32 - 18.0) * 0.03)
            .collect();
        let head_vec: Vec<f32> = (0..(d * vocab))
            .map(|i| (((i * 19 + 3) % 41) as f32 - 20.0) * 0.04)
            .collect();

        let l0 = Var::from_tensor(&Tensor::from_vec(l0_vec, (d, d), &dev).unwrap()).unwrap();
        let l1 = Var::from_tensor(&Tensor::from_vec(l1_vec, (d, d), &dev).unwrap()).unwrap();
        let lm_head =
            Var::from_tensor(&Tensor::from_vec(head_vec, (d, vocab), &dev).unwrap()).unwrap();
        let model = ToyModel {
            embed: Tensor::from_vec(embed_vec, (vocab, d), &dev).unwrap(),
            layers: vec![l0.clone(), l1.clone()],
            lm_head: lm_head.clone(),
        };

        let seq_tokens = 601usize;
        let ids: Vec<u32> = (0..seq_tokens)
            .map(|i| ((i * 7 + 3) % vocab) as u32)
            .collect();
        let prefix_len = 270usize; // > 256 -> chunk 0 completely skipped, chunks 1 & 2 active
        let sample_weight = 1.75f64;
        let grad_accum_steps = 3usize;
        let mut cfg = LoraTrainingConfig::default();
        cfg.qlora_ce_last_k = 0;

        // 1. Eager reference
        let eager_res = forward_masked_ce(
            &model,
            &ids,
            prefix_len,
            0,
            sample_weight,
            None,
            &cfg,
            HeadCeMode::EagerTape,
            &dev,
        )
        .unwrap();
        let (eager_loss_scalar, eager_grads) = match eager_res {
            MaskedCeForward::Finite {
                loss: Some(loss),
                loss_scalar,
                ..
            } => {
                let scale = Tensor::new(1.0f32 / grad_accum_steps as f32, &dev).unwrap();
                let scaled = loss.broadcast_mul(&scale).unwrap();
                (loss_scalar, scaled.backward().unwrap())
            }
            _ => panic!("expected Finite EagerTape"),
        };

        // 2. NoGradPreflight loss must match
        let preflight_res = forward_masked_ce(
            &model,
            &ids,
            prefix_len,
            0,
            sample_weight,
            None,
            &cfg,
            HeadCeMode::NoGradPreflight,
            &dev,
        )
        .unwrap();
        let preflight_loss_scalar = match preflight_res {
            MaskedCeForward::Finite {
                loss: None,
                detached_head: None,
                segments: None,
                loss_scalar,
                ..
            } => loss_scalar,
            _ => panic!("expected Finite NoGradPreflight"),
        };
        assert!((eager_loss_scalar - preflight_loss_scalar).abs() < 1e-5);

        // 3. Detached-boundary chunked head + segment recompute backward
        let trainable_vars = vec![l0.clone(), l1.clone(), lm_head.clone()];
        let ckpt_res = forward_masked_ce(
            &model,
            &ids,
            prefix_len,
            0,
            sample_weight,
            None,
            &cfg,
            HeadCeMode::Checkpointed {
                trainable_vars: &trainable_vars,
                grad_accum_steps,
            },
            &dev,
        )
        .unwrap();
        let (ckpt_loss_scalar, mut grads, mut upstream, segments) = match ckpt_res {
            MaskedCeForward::Finite {
                loss: None,
                loss_scalar,
                segments: Some(segs),
                detached_head: Some(dh),
                ..
            } => (
                loss_scalar,
                dh.head_grads.unwrap_or_else(|| {
                    Tensor::zeros((), DType::F32, &dev)
                        .unwrap()
                        .backward()
                        .unwrap()
                }),
                dh.d_final_hidden,
                segs,
            ),
            _ => panic!("expected Finite Checkpointed"),
        };
        assert!((eager_loss_scalar - ckpt_loss_scalar).abs() < 1e-5);
        for seg in segments.into_iter().rev() {
            let (iv, seg_out) = model.recompute(&seg);
            let mut sg = qlora_rs::backward_from_cotangent(&seg_out, &upstream).unwrap();
            upstream = sg.remove(iv.as_tensor()).unwrap();
            qlora_rs::accumulate_grads_for_vars(&mut grads, &sg, &trainable_vars).unwrap();
        }
        let _ = upstream;

        for (name, var) in [("l0", &l0), ("l1", &l1), ("lm_head", &lm_head)] {
            let g_ref = eager_grads
                .get(var.as_tensor())
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap();
            let g_got = grads
                .get(var.as_tensor())
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap();
            let max_diff = g_ref
                .iter()
                .zip(g_got.iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            assert!(
                max_diff < 1e-5,
                "detached chunked head grad mismatch for {name}: max_diff={max_diff}"
            );
        }
    }
}
