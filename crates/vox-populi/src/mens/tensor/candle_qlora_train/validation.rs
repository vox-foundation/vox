//! Hold-out validation pass (cross-entropy on last-k labels).

use candle_core::Device;
use tokenizers::Tokenizer;
use vox_tensor::data::TrainingPair;

use crate::mens::tensor::{train_log, training_config::LoraTrainingConfig, training_text::plain_system_prompt_response};
use crate::mens::tensor::candle_model_qwen::Qwen2Model;

/// Returns `(val_loss_sum, val_steps)` where loss is **negative** log-prob summed for averaging.
pub(super) fn run_validation_pass(
    eval_pairs: &[TrainingPair],
    tokenizer: &Tokenizer,
    device: &Device,
    model: &Qwen2Model,
    system_prompt: &str,
    config: &LoraTrainingConfig,
) -> (f64, u32) {
    let mut val_loss_sum = 0.0f64;
    let mut val_steps = 0u32;
    if eval_pairs.is_empty() {
        return (val_loss_sum, val_steps);
    }
    train_log::info(&format!(
        "Running validation on {} pairs...",
        eval_pairs.len()
    ));
    for pair in eval_pairs {
        let text = plain_system_prompt_response(system_prompt, &pair.prompt, &pair.response);
        let prefix_text = plain_system_prompt_response(system_prompt, &pair.prompt, "");
        if let Ok(enc) = tokenizer.encode(text, true) {
            let prefix_len = tokenizer
                .encode(prefix_text, true)
                .map(|e| e.get_ids().len())
                .unwrap_or(0);
            let mut ids = enc.get_ids().to_vec();
            let mut trunc_offset = 0usize;
            if ids.len() > config.seq_len {
                trunc_offset = ids.len() - config.seq_len;
                ids = ids[trunc_offset..].to_vec();
            }
            if ids.len() >= 2
                && let Ok(input_ids) =
                    candle_core::Tensor::new(&ids[..ids.len() - 1], device).and_then(|t| t.unsqueeze(0))
                && let Ok(targets) =
                    candle_core::Tensor::new(&ids[1..], device).and_then(|t| t.unsqueeze(0))
                && let Ok(logits) = model.forward(&input_ids)
                && let Ok(logits) = logits.flatten_to(1)
                && let Ok(targets_flat) = targets.flatten_all()
            {
                let prompt_len = prefix_len.saturating_sub(trunc_offset);
                let ids_len = ids.len();
                let ce_last_k = config.qlora_ce_last_k.max(1);
                let last_k_start = ids_len.saturating_sub(ce_last_k);
                let mask_vec: Vec<f32> = (0..ids_len - 1)
                    .map(|i| {
                        let target_idx = i + 1;
                        if target_idx >= prompt_len && target_idx >= last_k_start {
                            1.0f32
                        } else {
                            0.0
                        }
                    })
                    .collect();
                if let Ok(mask) = candle_core::Tensor::from_vec(mask_vec, ids_len - 1, device)
                    && let Ok(log_sm) = candle_nn::ops::log_softmax(&logits, 1)
                    && let Ok(tgt_uns) = targets_flat.unsqueeze(1)
                    && let Ok(logprobs) =
                        log_sm.gather(&tgt_uns, 1).and_then(|t| t.flatten_all())
                    && let Ok(loss) = logprobs
                        .broadcast_mul(&mask)
                        .and_then(|m| m.sum_all())
                        .and_then(|sum_m| {
                            sum_m.broadcast_div(&mask.sum_all().unwrap_or_else(|_| {
                                candle_core::Tensor::new(1f32, device).unwrap()
                            }))
                        })
                    && let Ok(loss_val) = loss.to_scalar::<f32>()
                {
                    val_loss_sum += -loss_val as f64;
                    val_steps += 1;
                }
            }
        }
    }
    (val_loss_sum, val_steps)
}
