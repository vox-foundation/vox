//! Logic bits for training loop.
//!
//! Ported verbatim from vox-populi (SP3 sub-batch C).

use crate::model::CheckpointSegment;
use candle_core::Tensor;
use qlora_rs::training::QLoraTrainer;

/// Gradient-checkpointed backward + optimizer step with detached-boundary
/// chunked `lm_head` backward on CUDA.
///
/// Chunked CE already computed `detached_head.head_grads` (`lm_head` LoRA grads
/// pre-scaled by `1 / accum_steps`) and `detached_head.d_final_hidden`. All `segments`
/// (`0..n_segments`) are recomputed in reverse via [`qlora_rs::backward_from_cotangent`]
/// and combined into one [`QLoraTrainer::optimizer_step_with_grads`] call.
pub fn checkpointed_backward_step(
    trainer: &mut QLoraTrainer,
    model: &crate::candle_qlora_train::TrainGraphModel,
    detached_head: super::types::DetachedHeadBackward,
    segments: Vec<CheckpointSegment>,
) -> anyhow::Result<()> {
    if segments.is_empty() {
        anyhow::bail!("checkpointed_backward_step: no segments");
    }

    let device = detached_head.d_final_hidden.device().clone();
    let all_vars = trainer.trainable_vars();
    let mut grads = detached_head.head_grads;
    let mut upstream = detached_head.d_final_hidden;

    for seg in segments.into_iter().rev() {
        let (input_var, seg_out) = model.recompute_segment(&seg)?;
        let mut seg_grads = qlora_rs::backward_from_cotangent(&seg_out, &upstream)?;
        upstream = seg_grads.remove(input_var.as_tensor()).ok_or_else(|| {
            anyhow::anyhow!("checkpointed backward: missing boundary grad for segment input")
        })?;
        if let Some(ref mut acc) = grads {
            qlora_rs::accumulate_grads_for_vars(acc, &seg_grads, &all_vars)?;
        } else {
            seg_grads.retain_vars(&all_vars);
            grads = Some(seg_grads);
        }
    }

    let combined_grads = match grads {
        Some(g) => g,
        None => Tensor::zeros((), candle_core::DType::F32, &device)?.backward()?,
    };
    trainer
        .optimizer_step_with_grads(combined_grads)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

// `trajectory_weight_for_pair` moved to `vox-plugin-mens-candle-core` — it was
// byte-for-byte duplicated with `vox-plugin-mens-candle-metal`'s entire (34-line)
// `logic.rs`. Re-exported here so `super::logic::trajectory_weight_for_pair`
// call sites in this crate are unaffected.
pub use vox_plugin_mens_candle_core::candle_qlora_train::training_loop::logic::trajectory_weight_for_pair;
