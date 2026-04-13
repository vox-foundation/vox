use anyhow::Result;
use candle_core::Device;
use tokenizers::Tokenizer;
use vox_tensor::data::TrainingPair;

use crate::mens::tensor::{
    train_log, qlora_preflight::QloraEmbedBundle, training_config::LoraTrainingConfig,
};
use super::types::{MaskedCeForward, TryEncodeOutcome};

pub fn qlora_forward_logits_smoke(
    model: &crate::mens::tensor::candle_qlora_train::TrainGraphModel,
    vocab: usize,
    device: &Device,
) -> Result<()> {
    if vocab < 2 {
        return Ok(());
    }
    let input_ids = candle_core::Tensor::new(&[0u32, 1u32], device)?.unsqueeze(0)?;
    let logits = model.forward(&input_ids)?;
    let sample = logits
        .narrow(0, 0, 1)?
        .narrow(1, 0, 1)?
        .flatten_all()?
        .to_vec1::<f32>()?;
    let bad = sample
        .iter()
        .take(16_384)
        .filter(|x| !x.is_finite())
        .count();
    if bad > 0 {
        anyhow::bail!(
            "QLoRA forward smoke test: {bad} non-finite values in first logits row on trivial input_ids=[0,1] — check CUDA/NF4 path or VOX_CANDLE_DEVICE=cpu to isolate"
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn preflight_masked_ce_finite(
    model: &crate::mens::tensor::candle_qlora_train::TrainGraphModel,
    bundle: &QloraEmbedBundle,
    pairs: &[TrainingPair],
    tokenizer: &Tokenizer,
    device: &Device,
    config: &LoraTrainingConfig,
    system_prompt: &str,
    start_epoch: usize,
) -> Result<()> {
    qlora_forward_logits_smoke(model, bundle.vocab, device)?;

    let max_diff = super::curriculum::max_difficulty_for_epoch(start_epoch, config);
    let vocab = bundle.vocab;
    for (pair_idx, pair) in pairs.iter().enumerate() {
        let enc = match super::encoding::try_encode_training_step(pair, system_prompt, tokenizer, config, max_diff)?
        {
            TryEncodeOutcome::SkipCurriculum | TryEncodeOutcome::SkipShortSeq => continue,
            TryEncodeOutcome::Encoded(enc) => enc,
        };

        if !super::encoding::token_ids_in_model_vocab(&enc.ids, vocab) {
            let max_id = enc.ids.iter().copied().max().unwrap_or(0);
            anyhow::bail!(
                "masked CE preflight: token id out of model vocab range (max_id={max_id}, vocab_size={vocab}) at pair index {pair_idx}; \
                 align the HF tokenizer with the base checkpoint or set VOX_MENS_TRAIN_JSONL_STRICT=1 for earlier JSONL validation"
            );
        }

        match super::forward::forward_masked_ce(
            model,
            &enc.ids,
            enc.prefix_len,
            enc.trunc_offset,
            enc.sample_weight,
            config,
            device,
        )? {
            MaskedCeForward::NoSupervision => continue,
            MaskedCeForward::NonFinite { kind, mask_sum } => {
                anyhow::bail!(
                    "masked CE preflight failed: non-finite loss ({kind}) before training (mask_sum={mask_sum:.6}, pair_idx={pair_idx}); \
                     trivial forward smoke on input_ids=[0,1] already passed — inspect this row, try a smaller --seq-len, or compare VOX_CANDLE_DEVICE=cpu vs cuda"
                );
            }
            MaskedCeForward::Finite { .. } => {
                train_log::info(
                    "Masked CE numeric preflight passed (finite loss on first eligible batch).",
                );
                return Ok(());
            }
        }
    }
    anyhow::bail!(
        "masked CE preflight: no eligible pair produced supervised CE — add assistant tokens in the CE window or relax curriculum"
    )
}
