//! Logic bits for training loop.

use crate::model::CheckpointSegment;
use candle_core::Tensor;
use qlora_rs::training::QLoraTrainer;

/// Gradient-checkpointed backward + optimizer step.
///
/// The checkpointed forward severed the autograd tape at each segment boundary,
/// so `loss`'s graph spans only the **final** segment + head. To get correct
/// grads for every layer's LoRA params without ever materializing the full-model
/// backward graph (the single-backward peak that OOMs large models), we:
///
/// 1. `loss.backward()` — grads for the last segment's LoRA Vars **and**
///    `dL/d(input)` at the last segment's boundary activation.
/// 2. Walk segments in reverse (excluding the last): recompute the segment's
///    forward from its stored detached boundary (tape live), then inject the
///    upstream cotangent via [`qlora_rs::backward_from_cotangent`]. Fold the
///    segment's Var grads into the accumulator and take `dL/d(seg_input)` as the
///    next upstream cotangent.
/// 3. One [`QLoraTrainer::optimizer_step_with_grads`] (clip + AdamW), honoring
///    grad-accumulation cadence identically to the eager path.
pub fn checkpointed_backward_step(
    trainer: &mut QLoraTrainer,
    model: &crate::candle_qlora_train::TrainGraphModel,
    loss: Tensor,
    mut segments: Vec<CheckpointSegment>,
) -> anyhow::Result<()> {
    if segments.is_empty() {
        anyhow::bail!("checkpointed_backward_step: no segments");
    }

    let device = loss.device().clone();

    // Mirror backward_step's grad-accumulation loss scaling so numerics match.
    let accum_steps = trainer
        .config
        .adapter_config
        .gradient_accumulation_steps
        .max(1);
    let scaled_loss = if accum_steps > 1 {
        let scale = Tensor::new(1.0f32 / accum_steps as f32, &device)?;
        loss.broadcast_mul(&scale)?
    } else {
        loss
    };

    let all_vars = trainer.trainable_vars();

    // Step 1: backward over the final segment + head, then immediately drop the final
    // segment's forward graph (`scaled_loss` / `last_seg`) and prune non-Var grads so
    // earlier segment recomputes never stack on top of the final segment's tensors.
    let mut grads = scaled_loss.backward()?;
    drop(scaled_loss);

    let last_seg = segments.pop().expect("non-empty");
    let mut upstream = grads.remove(&last_seg.input).ok_or_else(|| {
        anyhow::anyhow!("checkpointed backward: missing boundary grad for final segment input")
    })?;
    drop(last_seg);
    grads.retain_vars(&all_vars);
    device.synchronize()?;

    // Step 2: recompute-and-backward each earlier segment in reverse, dropping the
    // segment forward graph and temporary GradStore BEFORE `device.synchronize()` so
    // all transient buffers have strong_count == 1 when the Metal pool prunes.
    for seg in segments.into_iter().rev() {
        let (input_var, seg_out) = model.recompute_segment(&seg)?;
        drop(seg);
        let mut seg_grads = qlora_rs::backward_from_cotangent(&seg_out, &upstream)?;
        drop(seg_out);
        qlora_rs::accumulate_grads_for_vars(&mut grads, &seg_grads, &all_vars)?;
        upstream = seg_grads.remove(input_var.as_tensor()).ok_or_else(|| {
            anyhow::anyhow!("checkpointed backward: missing boundary grad for segment input")
        })?;
        drop(input_var);
        drop(seg_grads);
        device.synchronize()?;
    }
    drop(upstream);
    device.synchronize()?;

    // Step 3: clip + optimizer step from the combined grads.
    trainer
        .optimizer_step_with_grads(grads)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    device.synchronize()?;
    Ok(())
}

pub use vox_plugin_mens_candle_core::candle_qlora_train::training_loop::logic::trajectory_weight_for_pair;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candle_qlora_train::{TrainGraphModel, build_train_cfg};
    use crate::config::LoraTrainingConfig;
    use crate::model::{
        QuantizedLinear, Qwen2Attention, Qwen2MLP, Qwen35AttentionBlock, Qwen35Layer, Qwen35Model,
    };
    use candle_core::{DType, Device, Tensor};
    use candle_nn::{RmsNorm, VarBuilder};
    use qlora_rs::QLoraConfig;

    #[test]
    fn checkpointed_backward_step_rejects_empty_segments() {
        let dev = Device::Cpu;
        let cfg = LoraTrainingConfig::default();
        let train_cfg = build_train_cfg(&cfg, 0);
        let mut trainer = QLoraTrainer::new(train_cfg, dev.clone());
        let model = {
            let vb = trainer.var_builder();
            let mut qcfg = QLoraConfig::preset_all_bf16(4, 8);
            qcfg.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
            qcfg.cache_dequantized = true;

            let d = 8usize;
            let vocab = 16usize;
            let qlin = |vb: VarBuilder, d_out: usize, d_in: usize| {
                let w = Tensor::randn(0f32, 0.02f32, (d_out, d_in), &dev).unwrap();
                QuantizedLinear::from_weight_with_varbuilder(&w, None, &qcfg, vb).unwrap()
            };
            let layer = Qwen35Layer {
                input_layernorm: RmsNorm::new(Tensor::ones(d, DType::F32, &dev).unwrap(), 1e-6),
                attention: Qwen35AttentionBlock::Full(Qwen2Attention {
                    q_proj: qlin(vb.pp("q"), d, d),
                    k_proj: qlin(vb.pp("k"), d, d),
                    v_proj: qlin(vb.pp("v"), d, d),
                    o_proj: qlin(vb.pp("o"), d, d),
                    q_bias: None,
                    k_bias: None,
                    v_bias: None,
                    n_heads: 2,
                    n_kv_heads: 2,
                    head_dim: d / 2,
                    q_norm: None,
                    k_norm: None,
                }),
                post_attention_layernorm: RmsNorm::new(
                    Tensor::ones(d, DType::F32, &dev).unwrap(),
                    1e-6,
                ),
                mlp: Qwen2MLP {
                    gate_proj: qlin(vb.pp("g"), d * 2, d),
                    up_proj: qlin(vb.pp("u"), d * 2, d),
                    down_proj: qlin(vb.pp("dn"), d, d * 2),
                },
                inv_freq: None,
            };
            let embed = Tensor::randn(0f32, 0.02f32, (vocab, d), &dev).unwrap();
            let lm_head =
                QuantizedLinear::from_weight_with_varbuilder(&embed, None, &qcfg, vb.pp("lm_head"))
                    .unwrap();
            TrainGraphModel::Qwen35(Qwen35Model {
                embed_tokens: embed,
                layers: vec![layer],
                norm: RmsNorm::new(Tensor::ones(d, DType::F32, &dev).unwrap(), 1e-6),
                lm_head,
            })
        };

        let loss = Tensor::new(1.0f32, &dev).unwrap();
        let err = checkpointed_backward_step(&mut trainer, &model, loss, Vec::new()).unwrap_err();
        assert!(err.to_string().contains("no segments"));
    }
}
