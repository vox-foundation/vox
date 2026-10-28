//! `forward_masked_ce` — masked cross-entropy forward pass, and the segmented
//! backward for its checkpointed variant.
//!
//! Ported verbatim from vox-populi (SP3 sub-batch C).

use super::types::{CheckpointedCe, MaskedCeForward};
use crate::candle_qlora_train::TrainGraphModel;
use crate::config::LoraTrainingConfig;
use anyhow::{Context, Result};
use candle_core::backprop::GradStore;
use candle_core::{Device, Tensor, Var};
use qlora_rs::training::QLoraTrainer;

/// Head rows run at once on the checkpointed path. A chunk's CE backward holds
/// about a dozen `[rows, vocab]` F32 buffers; for a whole 1024-token row that
/// was ~36 GB of Metal pool (320 S2), for 256 rows x 151936 it is a few GB.
const HEAD_CHUNK_ROWS: usize = if cfg!(test) { 4 } else { 256 };

pub fn forward_masked_ce(
    model: &crate::candle_qlora_train::TrainGraphModel,
    ids: &[u32],
    prefix_len: usize,
    trunc_offset: usize,
    sample_weight: f64,
    token_weights: Option<&[f32]>,
    config: &LoraTrainingConfig,
    device: &Device,
) -> Result<MaskedCeForward> {
    let ids_len = ids.len();
    if ids_len < 2 {
        return Ok(MaskedCeForward::NoSupervision);
    }

    let prompt_len = prefix_len.saturating_sub(trunc_offset);
    let ce_last_k = if config.qlora_ce_last_k == 0 {
        ids_len
    } else {
        config.qlora_ce_last_k
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

    let mask_sum: f32 = mask_vec.iter().sum();
    if mask_sum <= 0.0 || !mask_sum.is_finite() {
        return Ok(MaskedCeForward::NoSupervision);
    }

    let input_ids = Tensor::new(&ids[..ids_len - 1], device)?.unsqueeze(0)?;
    let targets = Tensor::new(&ids[1..], device)?;
    let max_target = ids[1..].iter().copied().max().unwrap_or(0) as usize;
    let w = sample_weight as f32;

    // Checkpointed: one layer per segment and the head in row chunks, so the
    // backward never holds more than one layer's activations or one chunk's
    // logits. The loss here is a plain scalar; the backward re-runs the head.
    // Report the plain per-token CE; only the backward loss carries the row's
    // mix/trajectory weight (otherwise a 6x-weighted row logs 6x its real loss).
    let (loss_scalar, loss, ckpt) = if config.gradient_checkpointing {
        let (hidden, segments) = model.checkpointed_hidden(&input_ids, usize::MAX)?;
        let untracked = hidden.detach();
        let mut nll = 0f32;
        for (r0, n) in head_chunks(&mask_vec) {
            nll += head_chunk_nll(
                model,
                &untracked,
                &targets,
                &mask_vec[r0..r0 + n],
                r0,
                max_target,
            )?
            .to_scalar::<f32>()?;
        }
        let ce = nll / mask_sum;
        let ckpt = CheckpointedCe {
            segments,
            hidden,
            targets,
            mask: mask_vec,
            mask_sum,
            weight: w,
            max_target,
        };
        (ce, Tensor::new(ce * w, device)?, Some(ckpt))
    } else {
        let logits = model.forward(&input_ids)?.flatten_to(1)?;
        check_vocab(max_target, &logits)?;
        let mask = Tensor::from_vec(mask_vec, ids_len - 1, device)?;
        let logprobs = candle_nn::ops::log_softmax(&logits, 1)?
            .gather(&targets.unsqueeze(1)?, 1)?
            .flatten_all()?;
        let unweighted = (logprobs.broadcast_mul(&mask)?.sum_all()? / mask.sum_all()?)?.neg()?;
        let loss = unweighted.broadcast_mul(&Tensor::new(&[w], device)?)?;
        (unweighted.to_scalar::<f32>()?, loss, None)
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
        ckpt,
    })
}

fn check_vocab(max_target: usize, logits: &Tensor) -> Result<()> {
    let vocab_dim = logits.dim(1)?;
    if max_target >= vocab_dim {
        anyhow::bail!(
            "forward_masked_ce: max target token id {max_target} >= logits vocab dimension {vocab_dim}"
        );
    }
    Ok(())
}

/// `(first_row, rows)` of each head chunk holding at least one supervised row.
fn head_chunks(mask: &[f32]) -> impl Iterator<Item = (usize, usize)> + '_ {
    (0..mask.len())
        .step_by(HEAD_CHUNK_ROWS)
        .map(move |r0| (r0, HEAD_CHUNK_ROWS.min(mask.len() - r0)))
        .filter(move |&(r0, n)| mask[r0..r0 + n].iter().any(|&m| m != 0.0))
}

/// `-sum(mask * log p(target))` over head rows `r0..r0 + mask.len()` of
/// `hidden` (`[1, seq, d]`).
fn head_chunk_nll(
    model: &TrainGraphModel,
    hidden: &Tensor,
    targets: &Tensor,
    mask: &[f32],
    r0: usize,
    max_target: usize,
) -> Result<Tensor> {
    let n = mask.len();
    let logits = model
        .head_forward(&hidden.narrow(1, r0, n)?)?
        .flatten_to(1)?;
    check_vocab(max_target, &logits)?;
    let logprobs = candle_nn::ops::log_softmax(&logits, 1)?
        .gather(&targets.narrow(0, r0, n)?.unsqueeze(1)?, 1)?
        .flatten_all()?;
    let mask = Tensor::new(mask, hidden.device())?;
    Ok(logprobs.broadcast_mul(&mask)?.sum_all()?.neg()?)
}

/// Backward + optimizer step for a checkpointed forward. Port of the CUDA
/// plugin's `logic::checkpointed_backward_step`, with the same loss scaling for
/// gradient accumulation.
pub fn checkpointed_backward_step(
    trainer: &mut QLoraTrainer,
    model: &TrainGraphModel,
    ckpt: CheckpointedCe,
) -> Result<()> {
    let accum = trainer
        .config
        .adapter_config
        .gradient_accumulation_steps
        .max(1);
    let grads = checkpointed_grads(model, ckpt, &trainer.trainable_vars(), 1.0 / accum as f32)?;
    trainer
        .optimizer_step_with_grads(grads)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// LoRA grads of `scale * weight * CE`: the head in row chunks (each chunk's
/// graph freed before the next), then the last segment, then each earlier
/// segment recomputed and backpropagated in reverse.
pub fn checkpointed_grads(
    model: &TrainGraphModel,
    ckpt: CheckpointedCe,
    vars: &[Var],
    scale: f32,
) -> Result<GradStore> {
    let CheckpointedCe {
        segments,
        hidden,
        targets,
        mask,
        mask_sum,
        weight,
        max_target,
    } = ckpt;
    let last = segments
        .last()
        .context("checkpointed backward: no segments")?;
    // A fresh leaf, not a bare detach (see `model::independent_boundary`).
    let head_in = Var::from_tensor(&hidden.detach())?;
    // candle's `GradStore::new` is private; an untracked scalar's backward is an
    // empty store (plus that scalar's own seed).
    let mut grads = Tensor::new(0f32, hidden.device())?.backward()?;
    let mut upstream = head_in.as_tensor().zeros_like()?;
    let k = f64::from(scale * weight / mask_sum);
    for (r0, n) in head_chunks(&mask) {
        let part = head_chunk_nll(
            model,
            head_in.as_tensor(),
            &targets,
            &mask[r0..r0 + n],
            r0,
            max_target,
        )?
        .affine(k, 0.0)?;
        let chunk = part.backward()?;
        fold_on_host(&mut grads, &chunk, vars)?;
        if let Some(g) = chunk.get(head_in.as_tensor()) {
            upstream = (upstream + g)?;
        }
    }
    let seg_grads = qlora_rs::backward_from_cotangent(&hidden, &upstream)?;
    drop(hidden);
    fold_on_host(&mut grads, &seg_grads, vars)?;
    upstream = seg_grads
        .get(&last.input)
        .cloned()
        .context("checkpointed backward: no grad at the last segment input")?;
    drop(seg_grads);
    for seg in segments.iter().rev().skip(1) {
        let (input_var, seg_out) = model.recompute_segment(seg)?;
        let seg_grads = qlora_rs::backward_from_cotangent(&seg_out, &upstream)?;
        fold_on_host(&mut grads, &seg_grads, vars)?;
        upstream = seg_grads
            .get(input_var.as_tensor())
            .cloned()
            .context("checkpointed backward: no grad at a segment input")?;
    }
    Ok(grads)
}

/// `qlora_rs::accumulate_grads_for_vars`, summed on the host. On Metal a LoRA
/// grad computed mid-backward takes whatever free pooled buffer is at least its
/// size (often 16-256 MB for a few hundred KB), and candle's pool never shrinks;
/// held on the device through the remaining segments and the accumulation
/// window, those buffers stayed pinned (320-S3: +11 GB per-row peak at 0.6B
/// accum 8). Copied out, the device grad is freed with its segment.
fn fold_on_host(dst: &mut GradStore, src: &GradStore, vars: &[Var]) -> Result<()> {
    for var in vars {
        if let Some(g) = src.get(var.as_tensor()) {
            let g = g.to_device(&Device::Cpu)?;
            let merged = match dst.get(var.as_tensor()) {
                Some(existing) => existing.add(&g)?,
                None => g,
            };
            dst.insert(var.as_tensor(), merged);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::DType;
    use candle_nn::{VarBuilder, VarMap};

    /// 9 positions, prompt of 6: with 4-row head chunks the first chunk has no
    /// supervised row and the last chunk is partial.
    const IDS: [u32; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const PREFIX: usize = 6;
    const WEIGHT: f64 = 2.0;

    fn tiny(n_layers: usize) -> (VarMap, TrainGraphModel) {
        tiny_on(&Device::Cpu, n_layers)
    }

    fn tiny_on(dev: &Device, n_layers: usize) -> (VarMap, TrainGraphModel) {
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, dev);
        let m = crate::model::gradient_checkpoint_tests::build_model(vb, 8, n_layers, 16, dev);
        (varmap, TrainGraphModel::Qwen35(m))
    }

    fn run(model: &TrainGraphModel, gradient_checkpointing: bool) -> MaskedCeForward {
        run_on(model, gradient_checkpointing, &Device::Cpu)
    }

    fn run_on(
        model: &TrainGraphModel,
        gradient_checkpointing: bool,
        dev: &Device,
    ) -> MaskedCeForward {
        let config = LoraTrainingConfig {
            gradient_checkpointing,
            ..Default::default()
        };
        forward_masked_ce(model, &IDS, PREFIX, 0, WEIGHT, None, &config, dev).unwrap()
    }

    /// 320-S3: on Metal a LoRA grad computed mid-backward takes whatever free
    /// pooled buffer is at least its size, and candle's pool never shrinks
    /// (0.6B: 394 grads, 25 MB, sat in 6-13 GB of 16-512 MB buffers). Held
    /// through the remaining segments and the accumulation window, they pinned
    /// those buffers: +11 GB per-row peak at accum 8. The segmented backward
    /// hands over host grads, equal to the eager ones.
    #[test]
    fn checkpointed_grads_leave_the_metal_pool_and_match_the_eager_backward() {
        let Ok(dev) = Device::new_metal(0) else {
            return; // no Metal device (e.g. a macOS CI VM)
        };
        let (varmap, model) = tiny_on(&dev, 3);
        let vars = varmap.all_vars();
        let MaskedCeForward::Finite { loss, .. } = run_on(&model, false, &dev) else {
            panic!("eager forward not finite");
        };
        let want = loss.backward().unwrap();
        let MaskedCeForward::Finite {
            ckpt: Some(ckpt), ..
        } = run_on(&model, true, &dev)
        else {
            panic!("checkpointed forward carried no checkpoint state");
        };
        let got = checkpointed_grads(&model, ckpt, &vars, 0.25).unwrap();
        for v in &vars {
            let (Some(a), Some(b)) = (want.get(v.as_tensor()), got.get(v.as_tensor())) else {
                panic!("a LoRA var is missing a grad on one side");
            };
            assert!(
                b.device().is_cpu(),
                "a LoRA grad is still held in a Metal buffer"
            );
            let a = (a.to_device(&Device::Cpu).unwrap() * 0.25).unwrap();
            let diff = (a - b).unwrap().abs().unwrap().max_all().unwrap();
            assert!(diff.to_scalar::<f32>().unwrap() < 1e-4);
        }
    }

    /// The trainer holds those host grads across micro-steps; closing the
    /// window must give exactly the update of one step on their sum taken on
    /// the device (same summation order as the trainer: newest + held).
    #[test]
    fn host_held_micro_step_grads_step_exactly_like_their_device_sum() {
        let Ok(dev) = Device::new_metal(0) else {
            return; // no Metal device (e.g. a macOS CI VM)
        };
        let trainer = |accum: usize| {
            let mut config = qlora_rs::training::QLoraTrainingConfig::default();
            config.adapter_config.gradient_accumulation_steps = accum;
            let mut t = QLoraTrainer::new(config, dev.clone());
            let w = t
                .var_builder()
                .get_with_hints(64, "w", candle_nn::Init::Const(1.0))
                .unwrap();
            t.init_optimizer(&[]).unwrap();
            (t, w)
        };
        let grad = |w: &Tensor, mean: f32| {
            let x = Tensor::randn(mean, 1f32, 64, &dev).unwrap();
            let loss = (w * x).unwrap().sqr().unwrap().sum_all().unwrap();
            loss.backward().unwrap().get(w).unwrap().clone()
        };
        let (mut held, w) = trainer(2);
        let (mut once, w_ref) = trainer(1);
        let (g1, g2) = (grad(&w, 0.3), grad(&w, -1.1));
        for g in [&g1, &g2] {
            let mut store = Tensor::new(0f32, &Device::Cpu).unwrap().backward().unwrap();
            store.insert(&w, g.to_device(&Device::Cpu).unwrap());
            held.optimizer_step_with_grads(store).unwrap();
        }
        let mut sum = Tensor::new(0f32, &dev).unwrap().backward().unwrap();
        sum.insert(&w_ref, (&g2 + &g1).unwrap());
        once.optimizer_step_with_grads(sum).unwrap();
        let (a, b) = (w.to_vec1::<f32>().unwrap(), w_ref.to_vec1::<f32>().unwrap());
        assert_ne!(a, vec![1f32; 64], "the window closed without an update");
        assert_eq!(
            a, b,
            "held host grads stepped differently from their device sum"
        );
    }

    /// The Metal loop must honor `gradient_checkpointing` (S2: an eager 28-layer
    /// graph at 2333 tokens reached ~374 GB): one segment per layer, same loss.
    #[test]
    fn checkpointing_splits_per_layer_and_keeps_the_eager_loss() {
        let (_vm, model) = tiny(3);
        let (
            MaskedCeForward::Finite {
                loss_scalar: eager,
                ckpt: None,
                ..
            },
            MaskedCeForward::Finite {
                loss_scalar: chunked,
                ckpt: Some(ckpt),
                ..
            },
        ) = (run(&model, false), run(&model, true))
        else {
            panic!("eager must carry no checkpoint state and checkpointed must carry it");
        };
        assert_eq!(ckpt.segments.len(), 3, "one checkpoint segment per layer");
        assert!((eager - chunked).abs() < 1e-5, "loss {eager} vs {chunked}");
    }

    /// S2: under checkpointing the whole-row `[seq, vocab]` head was the
    /// remaining ~35 MB/token. The head runs in bounded chunks, skipping
    /// chunks with no supervised row.
    #[test]
    fn head_chunks_are_bounded_and_skip_unsupervised_rows() {
        let mask = [0., 0., 0., 0., 0., 1., 1., 1., 1.];
        assert_eq!(head_chunks(&mask).collect::<Vec<_>>(), [(4, 4), (8, 1)]);
    }

    #[test]
    fn checkpointed_grads_match_the_eager_backward() {
        let (varmap, model) = tiny(3);
        let vars = varmap.all_vars();
        let MaskedCeForward::Finite { loss, .. } = run(&model, false) else {
            panic!("eager forward not finite");
        };
        let want = loss.backward().unwrap();
        let MaskedCeForward::Finite {
            ckpt: Some(ckpt), ..
        } = run(&model, true)
        else {
            panic!("checkpointed forward carried no checkpoint state");
        };
        // The loop passes 1 / grad_accum; grads are linear in it.
        let got = checkpointed_grads(&model, ckpt, &vars, 0.25).unwrap();
        for v in &vars {
            let (Some(a), Some(b)) = (want.get(v.as_tensor()), got.get(v.as_tensor())) else {
                panic!("a LoRA var is missing a grad on one side");
            };
            let diff = ((a * 0.25).unwrap() - b)
                .unwrap()
                .abs()
                .unwrap()
                .max_all()
                .unwrap();
            assert!(diff.to_scalar::<f32>().unwrap() < 1e-4);
        }
        assert_eq!(
            vars.len(),
            3 * 14 + 2,
            "7 LoRA linears per layer + the head"
        );
    }
}
