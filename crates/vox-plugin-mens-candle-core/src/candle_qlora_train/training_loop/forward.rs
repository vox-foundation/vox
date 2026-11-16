//! `forward_masked_ce` — device-agnostic masked cross-entropy forward pass.

use super::types::MaskedCeForward;
use crate::config::LoraTrainingConfig;
use anyhow::Result;
use candle_core::Device;

/// Backend model graph abstraction implemented by each device plugin (`TrainGraphModel`).
pub trait ForwardCeGraph {
    type Segment;

    fn forward_eager(&self, input_ids: &candle_core::Tensor) -> Result<candle_core::Tensor>;
    fn forward_packed_eager(
        &self,
        input_ids: &candle_core::Tensor,
        doc_ids: &[u32],
    ) -> Result<candle_core::Tensor>;
    fn forward_checkpointed_segments(
        &self,
        input_ids: &candle_core::Tensor,
        n_segments: usize,
    ) -> Result<(candle_core::Tensor, Vec<Self::Segment>)>;
    fn forward_checkpointed_packed_segments(
        &self,
        input_ids: &candle_core::Tensor,
        doc_ids: &[u32],
        n_segments: usize,
    ) -> Result<(candle_core::Tensor, Vec<Self::Segment>)>;

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

    let input_ids = candle_core::Tensor::new(&ids[..ids_len - 1], device)?.unsqueeze(0)?;
    let targets = candle_core::Tensor::new(&ids[1..], device)?.unsqueeze(0)?;

    // Activation/gradient checkpointing: when enabled, the forward severs the
    // autograd tape at segment boundaries so only ~1 segment's activations are
    // retained for backward (bounds the single-backward peak that OOMs large models).
    if config.gradient_checkpointing {
        let n_segments = gradient_checkpoint_segments(config, model.num_layers());
        let (logits_raw, segments) = model.forward_checkpointed_segments(&input_ids, n_segments)?;
        let logits = logits_raw.flatten_to(1)?;
        let targets_flat = targets.flatten_all()?;
        return finalize_masked_ce(
            logits,
            targets_flat,
            ids_len,
            mask_vec,
            sample_weight,
            device,
            Some(segments),
        );
    }

    let logits = model.forward_eager(&input_ids)?;
    let logits = logits.flatten_to(1)?;
    let targets_flat = targets.flatten_all()?;
    finalize_masked_ce(
        logits,
        targets_flat,
        ids_len,
        mask_vec,
        sample_weight,
        device,
        None,
    )
}

/// Forward pass for a packed sequence bin (`sequence_packing`).
///
/// Uses `ForwardCeGraph::forward_checkpointed_packed_segments` (when
/// `gradient_checkpointing` is enabled) or `ForwardCeGraph::forward_packed_eager`
/// so attention stays block-diagonal per document and RoPE positions reset cleanly
/// at every document boundary.
pub fn forward_packed_masked_ce<M: ForwardCeGraph>(
    model: &M,
    samples: &[PackedSampleRef<'_>],
    loss_scale_multiplier: f64,
    config: &LoraTrainingConfig,
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

    let input_ids = candle_core::Tensor::new(&ids[..ids_len - 1], device)?.unsqueeze(0)?;
    let targets = candle_core::Tensor::new(&ids[1..], device)?.unsqueeze(0)?;
    let input_doc_ids = &doc_ids[..ids_len - 1];
    // `finalize_masked_ce` divides the weighted summed logprobs by `weighted_mask_sum`;
    // scaling by `(weighted / unweighted) * loss_scale_multiplier` cancels `weighted` so
    // the resulting gradient is identically `-sum(w_i * m * logp) / unweighted * loss_scale_multiplier`.
    let effective_sample_weight =
        (weighted_mask_sum as f64 / unweighted_mask_sum as f64) * loss_scale_multiplier;

    if config.gradient_checkpointing {
        let n_segments = gradient_checkpoint_segments(config, model.num_layers());
        let (logits_raw, segments) =
            model.forward_checkpointed_packed_segments(&input_ids, input_doc_ids, n_segments)?;
        let logits = logits_raw.flatten_to(1)?;
        let targets_flat = targets.flatten_all()?;
        return finalize_masked_ce(
            logits,
            targets_flat,
            ids_len,
            mask_vec,
            effective_sample_weight,
            device,
            Some(segments),
        );
    }

    let logits = model.forward_packed_eager(&input_ids, input_doc_ids)?;
    let logits = logits.flatten_to(1)?;
    let targets_flat = targets.flatten_all()?;
    finalize_masked_ce(
        logits,
        targets_flat,
        ids_len,
        mask_vec,
        effective_sample_weight,
        device,
        None,
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
        let mask_c = candle_core::Tensor::from_slice(mask_slice, len, logits.device())?;
        let log_sm = candle_nn::ops::log_softmax(&logits_c, 1)?;
        let logprobs = log_sm.gather(&targets_c.unsqueeze(1)?, 1)?.flatten_all()?;
        let sum_c = logprobs.broadcast_mul(&mask_c)?.sum_all()?;
        total = Some(match total {
            Some(acc) => (acc + sum_c)?,
            None => sum_c,
        });
    }
    total.ok_or_else(|| anyhow::anyhow!("chunked_masked_cross_entropy_sum: no active mask rows"))
}

#[allow(clippy::too_many_arguments)]
fn finalize_masked_ce<Seg>(
    logits: candle_core::Tensor,
    targets_flat: candle_core::Tensor,
    ids_len: usize,
    mask_vec: Vec<f32>,
    sample_weight: f64,
    device: &Device,
    segments: Option<Vec<Seg>>,
) -> Result<MaskedCeForward<Seg>> {
    let vocab_dim = logits.dim(1)?;
    let max_tgt = targets_flat.max(0)?.to_scalar::<u32>()? as usize;
    if max_tgt >= vocab_dim {
        anyhow::bail!(
            "forward_masked_ce: max target token id {max_tgt} >= logits vocab dimension {vocab_dim}"
        );
    }

    let mask_sum: f32 = mask_vec.iter().copied().sum();
    if mask_sum <= 0.0 || !mask_sum.is_finite() {
        return Ok(MaskedCeForward::NoSupervision);
    }
    let _ = ids_len;

    let logprob_sum =
        chunked_masked_cross_entropy_sum(&logits, &targets_flat, &mask_vec, CE_ROW_CHUNK_SIZE)?;
    let unweighted = (logprob_sum / (mask_sum as f64))?.neg()?;
    let w = sample_weight as f32;
    let w_t = candle_core::Tensor::new(&[w], device)?;
    let loss = unweighted.broadcast_mul(&w_t)?;

    let loss_scalar = match unweighted.rank() {
        0 => unweighted.to_scalar::<f32>()?,
        1 if unweighted.dim(0)? == 1 => unweighted.squeeze(0)?.to_scalar::<f32>()?,
        r => {
            anyhow::bail!("unexpected loss rank: expected scalar or [1], got rank={r}")
        }
    };
    if !loss_scalar.is_finite() {
        let kind = if loss_scalar.is_nan() { "nan" } else { "inf" };
        return Ok(MaskedCeForward::NonFinite { kind, mask_sum });
    }

    Ok(MaskedCeForward::Finite {
        loss,
        loss_scalar,
        supervised_tokens: mask_sum.max(0.0) as u64,
        theoretical_tokens: (ids_len.saturating_sub(1)) as u64,
        syntax_weight_sum: mask_sum,
        segments,
    })
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
}
