//! Hold-out validation pass.
//!
//! Ported verbatim from vox-populi (SP3 sub-batch C).

use anyhow::Result;
use candle_core::Device;
use tokenizers::Tokenizer;
use vox_tensor::data::TrainingPair;

use super::types::{MaskedCeForward, TryEncodeOutcome};
use crate::{config::LoraTrainingConfig, qlora_preflight::QloraEmbedBundle, train_log};

fn qlora_forward_logits_smoke(
    model: &crate::candle_qlora_train::TrainGraphModel,
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
            "QLoRA forward smoke test: {bad} non-finite values in first logits row on trivial input_ids=[0,1] — check Metal/NF4 path or VOX_CANDLE_DEVICE=cpu to isolate"
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn preflight_masked_ce_finite(
    model: &crate::candle_qlora_train::TrainGraphModel,
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
    // Each probe is a full forward pass; scanning all rows of a large corpus that
    // can never supervise (e.g. a prompt that fills the window) took ~40 minutes
    // before failing. Give up after this many encoded-but-unsupervised rows.
    const MAX_UNSUPERVISED_PROBES: usize = 256;
    let mut unsupervised = 0usize;
    // Heartbeat: this scan runs a full forward per row until one row has
    // supervised tokens, and used to be silent — a corpus whose rows are all
    // masked out looked like a hang for the whole scan.
    let mut last_beat = std::time::Instant::now();
    for (pair_idx, pair) in pairs.iter().enumerate() {
        if last_beat.elapsed() >= train_log::PROGRESS_MAX_INTERVAL {
            train_log::info(&format!(
                "masked CE preflight: scanned {pair_idx}/{} rows, none with supervised tokens yet (seq_len={})",
                pairs.len(),
                config.seq_len
            ));
            last_beat = std::time::Instant::now();
        }
        let enc = match super::encoding::try_encode_training_step(
            pair,
            system_prompt,
            tokenizer,
            config,
            max_diff,
        )? {
            TryEncodeOutcome::SkipCurriculum => continue,
            TryEncodeOutcome::SkipShortSeq => continue,
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
            None,
            config,
            device,
        )? {
            MaskedCeForward::NoSupervision => {
                unsupervised += 1;
                if unsupervised >= MAX_UNSUPERVISED_PROBES {
                    anyhow::bail!(
                        "masked CE preflight: the first {unsupervised} encodable rows had no supervised \
                         answer tokens inside --seq-len {} (last row: {} prompt tokens of {} kept). \
                         Raise --seq-len or shorten the system prompt.",
                        config.seq_len,
                        enc.prefix_len.saturating_sub(enc.trunc_offset),
                        enc.ids.len()
                    );
                }
                continue;
            }
            MaskedCeForward::NonFinite { kind, mask_sum } => {
                anyhow::bail!(
                    "masked CE preflight failed: non-finite loss ({kind}) before training (mask_sum={mask_sum:.6}, pair_idx={pair_idx}); \
                     trivial forward smoke on input_ids=[0,1] already passed — inspect this row, try a smaller --seq-len, or compare VOX_CANDLE_DEVICE=cpu vs Metal"
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

/// Returns `(val_loss_sum, val_steps)`.
pub fn run_validation_pass(
    eval_pairs: &[TrainingPair],
    tokenizer: &Tokenizer,
    device: &Device,
    model: &crate::candle_qlora_train::TrainGraphModel,
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
        let messages = pair
            .messages
            .as_deref()
            .map(|t| crate::training_text::with_system_turn(t, system_prompt, &config.chatml));
        let text = if let Some(ref turns) = messages {
            crate::training_text::chatml_turns_text(turns, &config.chatml)
        } else if let (Some(p), Some(r)) = (pair.effective_prompt(), pair.effective_response()) {
            crate::training_text::chatml_supervised_text(system_prompt, p, r, &config.chatml)
        } else {
            continue;
        };
        let prefix_text = if let Some(ref turns) = messages {
            crate::training_text::chatml_turns_prefix_open_assistant(turns, &config.chatml)
        } else if let (Some(p), Some(r)) = (pair.effective_prompt(), pair.effective_response()) {
            crate::training_text::chatml_prefix_open_assistant(system_prompt, p, r, &config.chatml)
        } else {
            continue;
        };
        let prefix_len = crate::candle_qlora_train::ce_mask_align::aligned_prefix_token_len(
            tokenizer,
            prefix_text.as_str(),
            text.as_str(),
        )
        .unwrap_or(0);
        if let Ok(enc) = tokenizer.encode(text.as_str(), true) {
            let (ids, trunc_offset) = crate::training_text::fit_to_seq_len(
                enc.get_ids().to_vec(),
                prefix_len,
                config.seq_len,
            );
            // The training CE: under checkpointing it keeps one layer's graph
            // and one head chunk, not the whole row's tape and `[seq, vocab]`
            // logits (320 S2).
            if let Ok(MaskedCeForward::Finite { loss_scalar, .. }) =
                super::forward::forward_masked_ce(
                    model,
                    &ids,
                    prefix_len,
                    trunc_offset,
                    1.0,
                    None,
                    config,
                    device,
                )
            {
                val_loss_sum += loss_scalar as f64;
                val_steps += 1;
            }
        }
    }
    (val_loss_sum, val_steps)
}
