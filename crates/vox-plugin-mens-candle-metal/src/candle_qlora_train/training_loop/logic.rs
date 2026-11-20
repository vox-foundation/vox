//! Logic bits for training loop.

use crate::model::CheckpointSegment;
use qlora_rs::training::QLoraTrainer;

/// Gradient-checkpointed backward + optimizer step with detached-boundary
/// chunked `lm_head` backward.
///
/// Chunked CE already computed `detached_head.head_grads` (`lm_head` LoRA grads
/// accumulated across active 256-row chunks, pre-scaled by `1 / accum_steps`) and
/// `detached_head.d_final_hidden` (`dL/d(final_hidden)`). All `segments` (`0..n_segments`)
/// were run under `with_no_grad`, so we walk all segments in reverse, inject `upstream`
/// via [`qlora_rs::backward_from_cotangent`], fold each segment's LoRA grads, and invoke
/// one [`QLoraTrainer::optimizer_step_with_grads`].
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
        drop(seg);
        let mut seg_grads = qlora_rs::backward_from_cotangent(&seg_out, &upstream)?;
        drop(seg_out);
        upstream = seg_grads.remove(input_var.as_tensor()).ok_or_else(|| {
            anyhow::anyhow!("checkpointed backward: missing boundary grad for segment input")
        })?;
        if let Some(ref mut acc) = grads {
            qlora_rs::accumulate_grads_for_vars(acc, &seg_grads, &all_vars)?;
            drop(seg_grads);
        } else {
            seg_grads.retain_vars(&all_vars);
            grads = Some(seg_grads);
        }
        drop(input_var);
        device.synchronize()?;
    }
    drop(upstream);
    device.synchronize()?;

    let combined_grads = match grads {
        Some(g) => g,
        None => candle_core::Tensor::zeros((), candle_core::DType::F32, &device)?.backward()?,
    };
    trainer
        .optimizer_step_with_grads(combined_grads)
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

        let dh = super::super::types::DetachedHeadBackward {
            head_grads: None,
            d_final_hidden: Tensor::zeros((1, 4, 8), DType::F32, &dev).unwrap(),
        };
        let err = checkpointed_backward_step(&mut trainer, &model, dh, Vec::new()).unwrap_err();
        assert!(err.to_string().contains("no segments"));
    }

    #[test]
    fn checkpointed_chunked_head_matches_eager_on_qwen35_model() {
        use super::super::forward::{HeadCeMode, forward_masked_ce};
        use super::super::types::MaskedCeForward;

        let dev = Device::Cpu;
        let d = 8usize;
        let vocab = 16usize;

        let make_weights = |seed_off: usize| -> Vec<f32> {
            (0..(d * d))
                .map(|i| (((i * 13 + seed_off) % 37) as f32 - 18.0) * 0.02)
                .collect()
        };
        let embed_vec: Vec<f32> = (0..(vocab * d))
            .map(|i| (((i * 7 + 5) % 41) as f32 - 20.0) * 0.03)
            .collect();
        let mlp_up_vec: Vec<f32> = (0..(d * 2 * d))
            .map(|i| (((i * 11 + 3) % 29) as f32 - 14.0) * 0.02)
            .collect();
        let mlp_dn_vec: Vec<f32> = (0..(d * d * 2))
            .map(|i| (((i * 17 + 9) % 31) as f32 - 15.0) * 0.02)
            .collect();

        let build_pair = |grad_checkpointing: bool| {
            let cfg = LoraTrainingConfig {
                seq_len: 24,
                grad_accum: 2,
                gradient_checkpointing: grad_checkpointing,
                lora_exclude_lm_head: false,
                rank: 2,
                alpha: 4.0,
                ..Default::default()
            };
            let mut train_cfg = build_train_cfg(&cfg, 4);
            train_cfg.adapter_config.learning_rate = 1e-3;
            train_cfg.adapter_config.gradient_accumulation_steps = 2;
            train_cfg.adapter_config.weight_decay = 0.0;
            train_cfg.adapter_config.max_grad_norm = None;
            train_cfg.use_paged_optimizer = false;
            let trainer = QLoraTrainer::new(train_cfg, dev.clone());
            let model = {
                let vb = trainer.var_builder();
                let mut qcfg = QLoraConfig::preset_all_bf16(2, 4);
                qcfg.quantization.compute_dtype = qlora_rs::quantization::ComputeDType::F32;
                qcfg.cache_dequantized = true;

                let qlin = |vb: VarBuilder, w_vec: &[f32], rows: usize, cols: usize| {
                    let w = Tensor::from_vec(w_vec.to_vec(), (rows, cols), &dev).unwrap();
                    QuantizedLinear::from_weight_with_varbuilder(&w, None, &qcfg, vb).unwrap()
                };
                let mut layers = Vec::new();
                for l_idx in 0..2usize {
                    let lvb = vb.pp(format!("l{l_idx}"));
                    layers.push(Qwen35Layer {
                        input_layernorm: RmsNorm::new(
                            Tensor::ones(d, DType::F32, &dev).unwrap(),
                            1e-6,
                        ),
                        attention: Qwen35AttentionBlock::Full(Qwen2Attention {
                            q_proj: qlin(lvb.pp("q"), &make_weights(l_idx + 1), d, d),
                            k_proj: qlin(lvb.pp("k"), &make_weights(l_idx + 2), d, d),
                            v_proj: qlin(lvb.pp("v"), &make_weights(l_idx + 3), d, d),
                            o_proj: qlin(lvb.pp("o"), &make_weights(l_idx + 4), d, d),
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
                            gate_proj: qlin(lvb.pp("g"), &mlp_up_vec, d * 2, d),
                            up_proj: qlin(lvb.pp("u"), &mlp_up_vec, d * 2, d),
                            down_proj: qlin(lvb.pp("dn"), &mlp_dn_vec, d, d * 2),
                        },
                        inv_freq: None,
                    });
                }
                let embed = Tensor::from_vec(embed_vec.clone(), (vocab, d), &dev).unwrap();
                let lm_head = QuantizedLinear::from_weight_with_varbuilder(
                    &embed,
                    None,
                    &qcfg,
                    vb.pp("lm_head"),
                )
                .unwrap();
                TrainGraphModel::Qwen35(Qwen35Model {
                    embed_tokens: embed,
                    layers,
                    norm: RmsNorm::new(Tensor::ones(d, DType::F32, &dev).unwrap(), 1e-6),
                    lm_head,
                })
            };
            (trainer, model, cfg)
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let adapter_path = temp_dir.path().join("init_adapter.safetensors");
        let (mut trainer_eager, model_eager, cfg_eager) = build_pair(false);
        trainer_eager.save_adapter(&adapter_path).unwrap();
        let (mut trainer_ckpt, model_ckpt, cfg_ckpt) = build_pair(true);
        trainer_ckpt.load_lora_weights(&adapter_path).unwrap();
        trainer_eager.init_optimizer(&[]).unwrap();
        trainer_ckpt.init_optimizer(&[]).unwrap();

        let ids_batch = [
            vec![1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            vec![2u32, 4, 6, 8, 10, 12, 14, 1, 3, 5, 7],
        ];
        for (step_idx, ids) in ids_batch.iter().enumerate() {
            let res_e = forward_masked_ce(
                &model_eager,
                ids,
                4,
                0,
                1.0,
                None,
                &cfg_eager,
                HeadCeMode::EagerTape,
                &dev,
            )
            .unwrap();
            let loss_e = match res_e {
                MaskedCeForward::Finite {
                    loss: Some(l),
                    loss_scalar,
                    segments: None,
                    detached_head: None,
                    ..
                } => {
                    trainer_eager.backward_step(&l).unwrap();
                    loss_scalar
                }
                _ => panic!("expected EagerTape finite result"),
            };

            let vars = trainer_ckpt.trainable_vars();
            let res_c = forward_masked_ce(
                &model_ckpt,
                ids,
                4,
                0,
                1.0,
                None,
                &cfg_ckpt,
                HeadCeMode::Checkpointed {
                    trainable_vars: &vars,
                    grad_accum_steps: 2,
                },
                &dev,
            )
            .unwrap();
            let loss_c = match res_c {
                MaskedCeForward::Finite {
                    loss: None,
                    loss_scalar,
                    segments: Some(segs),
                    detached_head: Some(dh),
                    ..
                } => {
                    checkpointed_backward_step(&mut trainer_ckpt, &model_ckpt, dh, segs).unwrap();
                    loss_scalar
                }
                _ => panic!("expected Checkpointed finite result"),
            };

            assert!(
                (loss_e - loss_c).abs() < 1e-5,
                "step {step_idx} loss mismatch: eager={loss_e} vs ckpt={loss_c}"
            );
        }

        // After 2 steps (completing grad_accum=2 AdamW update), compare named LoRA parameters.
        // (`trainable_vars()` iterates an unordered HashMap so zipping two trainers directly
        // mixes parameter keys; compare via saved safetensors dictionaries instead.)
        let after_eager_path = temp_dir.path().join("after_eager.safetensors");
        let after_ckpt_path = temp_dir.path().join("after_ckpt.safetensors");
        trainer_eager.save_adapter(&after_eager_path).unwrap();
        trainer_ckpt.save_adapter(&after_ckpt_path).unwrap();
        let init_map = candle_core::safetensors::load(&adapter_path, &dev).unwrap();
        let eager_map = candle_core::safetensors::load(&after_eager_path, &dev).unwrap();
        let ckpt_map = candle_core::safetensors::load(&after_ckpt_path, &dev).unwrap();
        assert_eq!(eager_map.len(), ckpt_map.len());
        let mut saw_l0_update = false;
        let mut saw_lm_head_update = false;
        for (name, te) in &eager_map {
            let tc = ckpt_map
                .get(name)
                .unwrap_or_else(|| panic!("missing key {name} in checkpointed adapter"));
            let ti = init_map.get(name).unwrap();
            let a = te.flatten_all().unwrap().to_vec1::<f32>().unwrap();
            let b = tc.flatten_all().unwrap().to_vec1::<f32>().unwrap();
            let i_vec = ti.flatten_all().unwrap().to_vec1::<f32>().unwrap();
            let max_diff = a
                .iter()
                .zip(b.iter())
                .map(|(x, y)| (x - y).abs())
                .fold(0.0f32, f32::max);
            assert!(
                max_diff < 1e-5,
                "param {name} parity mismatch after optimizer step: max_diff={max_diff}"
            );
            let moved = a
                .iter()
                .zip(i_vec.iter())
                .map(|(x, y)| (x - y).abs())
                .fold(0.0f32, f32::max);
            if moved > 1e-6 {
                if name.starts_with("l0.") {
                    saw_l0_update = true;
                }
                if name.starts_with("lm_head.") {
                    saw_lm_head_update = true;
                }
            }
        }
        assert!(
            saw_l0_update,
            "expected l0 LoRA weights to update through segment recompute"
        );
        assert!(
            saw_lm_head_update,
            "expected lm_head LoRA weights to update through chunked CE"
        );
    }
}
