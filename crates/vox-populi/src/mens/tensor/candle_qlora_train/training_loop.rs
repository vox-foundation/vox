use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use candle_core::Device;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use tokenizers::Tokenizer;
use vox_tensor::data::TrainingPair;

use qlora_rs::training::QLoraTrainer;

use super::{
    PAUSE_FLAG, QLORA_ETA_EMA_ALPHA, TrainingDbEvent, TrainingLoopStats, compute_cosine_lr,
    load_adapter_into_trainer,
};
use crate::mens::tensor::{
    backend, checkpoint_state::CheckpointState, manifest, qlora_preflight::QloraEmbedBundle,
    telemetry, telemetry_schema, train_log, training_config::LoraTrainingConfig,
    training_text::plain_system_prompt_response,
};

#[derive(Debug, Clone)]
pub(super) struct QloraTrainingResume {
    pub start_epoch: usize,
    pub global_step: u32,
    pub resume_pair_offset: usize,
    pub resume_shuffled_indices: Option<Vec<usize>>,
}

pub(super) fn apply_checkpoint_resume(
    trainer: &mut QLoraTrainer,
    config: &LoraTrainingConfig,
    out: &Path,
    pairs_len: usize,
) -> Result<QloraTrainingResume> {
    let mut start_epoch = 1usize;
    let mut global_step = 0u32;
    let mut resume_pair_offset = 0usize;
    let mut resume_shuffled_indices: Option<Vec<usize>> = None;

    let checkpoint_root = config.resume_from.as_deref().unwrap_or(out);
    if !config.force_restart
        && let Some(ckpt) = CheckpointState::load(checkpoint_root)
    {
        train_log::info(&format!(
            "Checkpoint found in {} — resuming from epoch={} global_step={} pair_offset={}",
            checkpoint_root.display(),
            ckpt.epoch,
            ckpt.global_step,
            ckpt.pair_offset
        ));
        if std::path::Path::new(&ckpt.adapter_path).exists() {
            if let Err(err) =
                load_adapter_into_trainer(trainer, std::path::Path::new(&ckpt.adapter_path))
            {
                train_log::warn(&format!(
                    "Resume adapter load failed for {}: {err}",
                    ckpt.adapter_path
                ));
            }
        } else {
            train_log::warn(&format!(
                "Resume checkpoint references missing adapter {}; continuing with fresh adapter weights.",
                ckpt.adapter_path
            ));
        }
        start_epoch = ckpt.epoch as usize;
        global_step = ckpt.global_step;
        resume_pair_offset = ckpt.pair_offset;
        if ckpt.shuffled_indices.is_empty() {
            train_log::warn(
                "Resume checkpoint did not include shuffled_indices (epoch-boundary checkpoint); reshuffling for resume epoch.",
            );
            resume_shuffled_indices = None;
            resume_pair_offset = 0;
        } else {
            let (validated_indices, dropped_bad_indices) =
                sanitize_resume_indices(&ckpt.shuffled_indices, pairs_len);
            if dropped_bad_indices > 0 {
                train_log::warn(&format!(
                    "Resume checkpoint shuffled_indices dropped {} out-of-range/duplicate entries; reshuffling current epoch.",
                    dropped_bad_indices
                ));
                resume_shuffled_indices = None;
                resume_pair_offset = 0;
            } else if validated_indices.len() != pairs_len {
                train_log::warn(&format!(
                    "Resume checkpoint shuffled_indices length {} does not match current dataset size {}; reshuffling current epoch.",
                    validated_indices.len(),
                    pairs_len
                ));
                resume_shuffled_indices = None;
                resume_pair_offset = 0;
            } else {
                resume_shuffled_indices = Some(validated_indices);
            }
        }
    }

    Ok(QloraTrainingResume {
        start_epoch,
        global_step,
        resume_pair_offset,
        resume_shuffled_indices,
    })
}

fn max_difficulty_for_epoch(epoch: usize, config: &LoraTrainingConfig) -> u8 {
    if !config.curriculum {
        return 10;
    }
    if config.epochs > 1 {
        let progress = (epoch - 1) as f32 / (config.epochs - 1) as f32;
        (3.0 + progress * 7.0).ceil() as u8
    } else {
        10
    }
}

struct EncodedTrainStep {
    raw_token_len: usize,
    ids: Vec<u32>,
    prefix_len: usize,
    trunc_offset: usize,
    sample_weight: f64,
}

enum TryEncodeOutcome {
    Encoded(EncodedTrainStep),
    SkipCurriculum,
    SkipShortSeq,
}

fn try_encode_training_step(
    pair: &TrainingPair,
    system_prompt: &str,
    tokenizer: &Tokenizer,
    config: &LoraTrainingConfig,
    max_difficulty: u8,
) -> Result<TryEncodeOutcome> {
    if config.curriculum && pair.difficulty.unwrap_or(5) > max_difficulty {
        return Ok(TryEncodeOutcome::SkipCurriculum);
    }
    let text = plain_system_prompt_response(system_prompt, &pair.prompt, &pair.response);
    let prefix_text = plain_system_prompt_response(system_prompt, &pair.prompt, "");
    let prefix_enc = tokenizer
        .encode(prefix_text, true)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let prefix_len = prefix_enc.get_ids().len();
    let enc = tokenizer
        .encode(text, true)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut ids = enc.get_ids().to_vec();
    let raw_token_len = ids.len();
    let mut trunc_offset = 0usize;
    if ids.len() > config.seq_len {
        trunc_offset = ids.len() - config.seq_len;
        ids = ids[trunc_offset..].to_vec();
    }
    if ids.len() < 2 {
        return Ok(TryEncodeOutcome::SkipShortSeq);
    }
    let (sample_weight, _) = trajectory_weight_for_pair(pair, config);
    Ok(TryEncodeOutcome::Encoded(EncodedTrainStep {
        raw_token_len,
        ids,
        prefix_len,
        trunc_offset,
        sample_weight,
    }))
}

pub(super) fn token_ids_in_model_vocab(ids: &[u32], vocab: usize) -> bool {
    if vocab == 0 {
        return false;
    }
    ids.iter().all(|&id| (id as usize) < vocab)
}

pub(super) enum MaskedCeForward {
    NoSupervision,
    NonFinite {
        kind: &'static str,
        mask_sum: f32,
    },
    Finite {
        loss: candle_core::Tensor,
        loss_scalar: f32,
        supervised_tokens: u64,
        theoretical_tokens: u64,
    },
}

pub(super) fn forward_masked_ce(
    model: &super::TrainGraphModel,
    ids: &[u32],
    prefix_len: usize,
    trunc_offset: usize,
    sample_weight: f64,
    config: &LoraTrainingConfig,
    device: &Device,
) -> Result<MaskedCeForward> {
    let ids_len = ids.len();
    if ids_len < 2 {
        return Ok(MaskedCeForward::NoSupervision);
    }
    let input_ids = candle_core::Tensor::new(&ids[..ids_len - 1], device)?.unsqueeze(0)?;
    let targets = candle_core::Tensor::new(&ids[1..], device)?.unsqueeze(0)?;

    let logits = model.forward(&input_ids)?;
    let logits = logits.flatten_to(1)?;
    let targets_flat = targets.flatten_all()?;

    let prompt_len = prefix_len.saturating_sub(trunc_offset);
    let ce_last_k = if config.qlora_ce_last_k == 0 {
        ids_len
    } else {
        config.qlora_ce_last_k
    };
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
    let mask = candle_core::Tensor::from_vec(mask_vec, ids_len - 1, device)?;

    let mask_sum = mask.sum_all()?.to_scalar::<f32>()?;
    if mask_sum <= 0.0 || !mask_sum.is_finite() {
        return Ok(MaskedCeForward::NoSupervision);
    }

    let log_sm = candle_nn::ops::log_softmax(&logits, 1)?;
    let logprobs = log_sm
        .gather(&targets_flat.unsqueeze(1)?, 1)?
        .flatten_all()?;
    let loss = (logprobs.broadcast_mul(&mask)?.sum_all()? / mask.sum_all()?)?;
    let w = -sample_weight as f32;
    let w_t = candle_core::Tensor::new(&[w], device)?;
    let loss = loss.broadcast_mul(&w_t)?;

    let loss_scalar = match loss.rank() {
        0 => loss.to_scalar::<f32>()?,
        1 if loss.dim(0)? == 1 => loss.squeeze(0)?.to_scalar::<f32>()?,
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
    })
}

pub(super) fn qlora_forward_logits_smoke(
    model: &super::TrainGraphModel,
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

pub(super) fn preflight_masked_ce_finite(
    model: &super::TrainGraphModel,
    bundle: &QloraEmbedBundle,
    pairs: &[TrainingPair],
    tokenizer: &Tokenizer,
    device: &Device,
    config: &LoraTrainingConfig,
    system_prompt: &str,
    start_epoch: usize,
) -> Result<()> {
    qlora_forward_logits_smoke(model, bundle.vocab, device)?;

    let max_diff = max_difficulty_for_epoch(start_epoch, config);
    let vocab = bundle.vocab;
    for (pair_idx, pair) in pairs.iter().enumerate() {
        let enc = match try_encode_training_step(pair, system_prompt, tokenizer, config, max_diff)?
        {
            TryEncodeOutcome::SkipCurriculum | TryEncodeOutcome::SkipShortSeq => continue,
            TryEncodeOutcome::Encoded(enc) => enc,
        };

        if !token_ids_in_model_vocab(&enc.ids, vocab) {
            let max_id = enc.ids.iter().copied().max().unwrap_or(0);
            anyhow::bail!(
                "masked CE preflight: token id out of model vocab range (max_id={max_id}, vocab_size={vocab}) at pair index {pair_idx}; \
                 align the HF tokenizer with the base checkpoint or set VOX_MENS_TRAIN_JSONL_STRICT=1 for earlier JSONL validation"
            );
        }

        match forward_masked_ce(
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

#[allow(clippy::too_many_arguments)]
pub(super) fn run_training_loop(
    trainer: &mut QLoraTrainer,
    model: super::TrainGraphModel,
    bundle: &QloraEmbedBundle,
    resume: QloraTrainingResume,
    out: &Path,
    config: &LoraTrainingConfig,
    pairs: Vec<TrainingPair>,
    eval_pairs: Vec<TrainingPair>,
    tokenizer: &Tokenizer,
    device: &Device,
    db_tx: &tokio::sync::mpsc::UnboundedSender<TrainingDbEvent>,
    run_id: &str,
    device_label: &str,
    train_path: &Path,
    system_prompt: &str,
    adapter_layer_order: &[String],
    base_key_map: &HashMap<String, String>,
    total_steps_planned: u32,
    total_optimizer_steps_planned: u32,
    warmup_steps: usize,
) -> Result<backend::TrainingSummary> {
    if !config.qlora_double_quant {
        train_log::info(
            "qlora_double_quant=false: Candle QLoRA uses qlora-rs preset quantization; flag is recorded for contract/manifest parity only.",
        );
    }
    if config.qlora_max_skip_rate.is_some() {
        train_log::info(
            "qlora_max_skip_rate is reserved for future skip accounting; value is stored in config/manifest for observability only.",
        );
    }

    // The trainer always runs a full forward graph; this flag tracks middle-projection key completeness in base shards.
    let proxy_stack_complete =
        match crate::mens::tensor::candle_qlora_weights::tensor_keys_union(&bundle.weight_paths) {
            Ok(present) => {
                let cov = crate::mens::tensor::candle_qlora_weights::middle_projection_coverage(
                    &bundle.layout,
                    &present,
                );
                cov.expected == 0 || cov.complete
            }
            Err(err) => {
                train_log::warn(&format!(
                    "Could not recompute middle projection coverage for manifest: {err}"
                ));
                false
            }
        };

    let QloraTrainingResume {
        start_epoch,
        mut global_step,
        resume_pair_offset,
        resume_shuffled_indices,
    } = resume;

    // ── Training manifest ─────────────────────────────────────────────────────
    manifest::write_training_manifest(
        out,
        manifest::initial_training_manifest(
            manifest::ArchParams {
                vocab_size: bundle.vocab,
                d_model: bundle.d_model,
                n_heads: bundle.layout.num_attention_heads,
                n_layers: bundle.layout.num_hidden_layers,
            },
            train_path.display().to_string(),
            manifest::InitialManifestRun::from_lora_config(config),
            Some(bundle.tokenizer_path.display().to_string()),
            manifest::InitialTrainingKernel::CandleQlora {
                proxy_stack_complete,
                middle_layers_active: bundle.layout.num_hidden_layers,
                ce_last_k: config.qlora_ce_last_k,
                architecture: match bundle.layout.architecture {
                    crate::mens::tensor::hf_load::HfArchitecture::Qwen35 => "qwen3_5".to_string(),
                    crate::mens::tensor::hf_load::HfArchitecture::Qwen2 => "qwen2".to_string(),
                    crate::mens::tensor::hf_load::HfArchitecture::Gpt2 => "gpt2".to_string(),
                },
                linear_layers: Some(
                    bundle
                        .layout
                        .layer_types
                        .iter()
                        .filter(|t| t.as_str() == "linear_attention")
                        .count(),
                ),
                full_layers: Some(
                    bundle
                        .layout
                        .layer_types
                        .iter()
                        .filter(|t| t.as_str() == "full_attention")
                        .count(),
                ),
            },
        ),
    )?;

    telemetry::append(
        out,
        telemetry_schema::events::TRAIN_START,
        serde_json::json!({
            telemetry_schema::keys::TRAIN_FILE: train_path.display().to_string(),
            telemetry_schema::keys::OUTPUT_DIR: out.display().to_string(),
            telemetry_schema::keys::SEED: config.seed,
            telemetry_schema::keys::GRAD_ACCUM: config.grad_accum.max(1),
            telemetry_schema::keys::EPOCHS: config.epochs,
            telemetry_schema::keys::PLANNED_STEPS_TOTAL: total_steps_planned,
            "planned_optimizer_steps_total": total_optimizer_steps_planned,
            "compute_device": device_label,
            "warmup_steps": warmup_steps,
            "n_heads": bundle.layout.num_attention_heads,
            "n_kv_heads": bundle.layout.num_key_value_heads,
            "run_id": run_id,
        }),
    )?;

    // ── Training state ────────────────────────────────────────────────────────
    let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);
    let mut last_progress = Instant::now();
    let progress_every = Duration::from_secs(5);
    let mut ema_steps_per_sec: Option<f64> = None;
    let mut optimizer_step_count: u32 = global_step / config.grad_accum.max(1) as u32;
    let mut progress_anchor_step = optimizer_step_count;
    let mut progress_anchor_time = Instant::now();
    let mut last_loss_val: f32 = 0.0;
    let mut ema_loss_val: Option<f64> = None;
    let mut total_loss_sum = 0.0f64;
    let mut total_step_count: u32 = 0;
    let mut total_tokens: usize = 0;
    let mut last_avg_val_loss: Option<f64> = None;
    let grad_accum = config.grad_accum.max(1) as u32;
    let mut skip_no_supervised_positions: u64 = 0;
    let mut skip_short_seq: u64 = 0;
    let mut skip_curriculum: u64 = 0;
    let mut skip_token_id_oob: u64 = 0;
    let mut token_oob_warned = false;
    let mut trajectory_weighted_pairs: u64 = 0;
    let mut trajectory_clamped_pairs: u64 = 0;
    let mut total_valid_tokens: u64 = 0;
    let mut total_theoretical_tokens: u64 = 0;

    let run_start_inst = Instant::now();
    for epoch in start_epoch..=config.epochs {
        // ── Epoch shuffle (or restore from checkpoint on resume epoch) ────────
        let shuffled_indices: Vec<usize> = build_epoch_shuffled_indices(
            epoch,
            start_epoch,
            pairs.len(),
            &resume_shuffled_indices,
            &mut rng,
        );

        trainer.start_epoch();

        if config.curriculum {
            let max_difficulty = if config.epochs > 1 {
                let progress = (epoch - 1) as f32 / (config.epochs - 1) as f32;
                (3.0 + progress * 7.0).ceil() as u8
            } else {
                10
            };
            train_log::info(&format!(
                "Epoch {}/{} curriculum threshold: diff <= {}",
                epoch, config.epochs, max_difficulty
            ));
        }

        let mut epoch_loss_sum = 0.0f64;
        let mut epoch_steps = 0u32;

        let pair_start = if epoch == start_epoch {
            resume_pair_offset.min(shuffled_indices.len())
        } else {
            0
        };

        // Curriculum learning: compute max allowed difficulty for this epoch
        let max_difficulty = if config.curriculum {
            // Linear ramp-up: epoch 1 -> difficulty 3, final epoch -> difficulty 10
            if config.epochs > 1 {
                let progress = (epoch - 1) as f32 / (config.epochs - 1) as f32;
                (3.0 + progress * 7.0).ceil() as u8
            } else {
                10
            }
        } else {
            10
        };

        for (pair_loop_idx, &pair_real_idx) in shuffled_indices.iter().enumerate().skip(pair_start)
        {
            let pair = &pairs[pair_real_idx];
            let (sample_weight, was_clamped) = trajectory_weight_for_pair(pair, config);
            if config.trajectory_weighting_enabled && (sample_weight - 1.0_f64).abs() > f64::EPSILON
            {
                trajectory_weighted_pairs += 1;
            }
            if was_clamped {
                trajectory_clamped_pairs += 1;
            }

            let enc = match try_encode_training_step(
                pair,
                system_prompt,
                tokenizer,
                config,
                max_difficulty,
            )? {
                TryEncodeOutcome::SkipCurriculum => {
                    skip_curriculum += 1;
                    continue;
                }
                TryEncodeOutcome::SkipShortSeq => {
                    skip_short_seq += 1;
                    continue;
                }
                TryEncodeOutcome::Encoded(enc) => enc,
            };

            total_tokens += enc.raw_token_len;

            if !token_ids_in_model_vocab(&enc.ids, bundle.vocab) {
                skip_token_id_oob += 1;
                if !token_oob_warned {
                    token_oob_warned = true;
                    let max_id = enc.ids.iter().copied().max().unwrap_or(0);
                    train_log::warn(&format!(
                        "token id out of model vocab range (max_id={max_id}, vocab_size={}, pair_real_idx={pair_real_idx}); \
                         skipping pairs with OOV ids — align HF tokenizer with base checkpoint or set VOX_MENS_TRAIN_JSONL_STRICT=1 for earlier JSONL validation",
                        bundle.vocab
                    ));
                }
                continue;
            }

            let mut lr_applied_this_step = 0.0_f64;
            let loss_val = (|| -> Result<Option<f32>> {
                match forward_masked_ce(
                    &model,
                    &enc.ids,
                    enc.prefix_len,
                    enc.trunc_offset,
                    enc.sample_weight,
                    config,
                    device,
                )? {
                    MaskedCeForward::NoSupervision => {
                        skip_no_supervised_positions += 1;
                        let prompt_len = enc.prefix_len.saturating_sub(enc.trunc_offset);
                        let ids_len = enc.ids.len();
                        let ce_last_k = if config.qlora_ce_last_k == 0 {
                            ids_len
                        } else {
                            config.qlora_ce_last_k
                        };
                        let last_k_start = ids_len.saturating_sub(ce_last_k);
                        train_log::debug(&format!(
                            "skip pair: no supervised CE positions (prompt_len={} last_k_start={} seq={})",
                            prompt_len, last_k_start, ids_len
                        ));
                        Ok(None)
                    }
                    MaskedCeForward::NonFinite { kind, mask_sum } => {
                        let prompt_len = enc.prefix_len.saturating_sub(enc.trunc_offset);
                        let ids_len = enc.ids.len();
                        train_log::warn(&format!(
                            "⚠ Non-finite loss ({kind}) before backward at epoch {epoch} micro_step {global_step} (skip update); \
                             mask_sum={mask_sum:.3} sample_weight={:.3} seq_len={ids_len} prompt_len_adj={prompt_len} — \
                             often indicates NaN logits from the forward pass (try smaller --seq-len, lower --lr, or CPU check) or bad token ids vs vocab",
                            enc.sample_weight,
                        ));
                        Ok(None)
                    }
                    MaskedCeForward::Finite {
                        loss,
                        loss_scalar,
                        supervised_tokens,
                        theoretical_tokens,
                    } => {
                        trainer
                            .backward_step(&loss)
                            .map_err(|e| anyhow::anyhow!("{e}"))?;
                        total_valid_tokens = total_valid_tokens.saturating_add(supervised_tokens);
                        total_theoretical_tokens =
                            total_theoretical_tokens.saturating_add(theoretical_tokens);

                        lr_applied_this_step = trainer.current_lr();

                        let lr_next = compute_cosine_lr(
                            optimizer_step_count,
                            warmup_steps,
                            total_optimizer_steps_planned,
                            config.learning_rate,
                        );
                        let micro_step_after_backward = global_step + 1;
                        if micro_step_after_backward.is_multiple_of(grad_accum) {
                            optimizer_step_count += 1;
                            trainer.config.adapter_config.learning_rate = lr_next;
                            trainer.update_lr();
                        }

                        Ok(Some(loss_scalar))
                    }
                }
            })()?;

            let Some(loss_val) = loss_val else {
                continue;
            };

            global_step += 1;
            epoch_steps += 1;
            epoch_loss_sum += loss_val as f64;
            total_loss_sum += loss_val as f64;
            total_step_count += 1;
            last_loss_val = loss_val;

            ema_loss_val = Some(match ema_loss_val {
                None => loss_val as f64,
                Some(prev) => 0.1 * (loss_val as f64) + 0.9 * prev,
            });

            // ── Progress reporting every 5s ───────────────────────────────────
            let elapsed_since_progress = last_progress.elapsed();
            if elapsed_since_progress >= progress_every {
                let now = Instant::now();
                let dt = now
                    .duration_since(progress_anchor_time)
                    .as_secs_f64()
                    .max(1e-3);
                let ds = (optimizer_step_count - progress_anchor_step) as f64;
                let sps = ds / dt;
                ema_steps_per_sec = Some(match ema_steps_per_sec {
                    None => sps,
                    Some(prev) => QLORA_ETA_EMA_ALPHA * sps + (1.0 - QLORA_ETA_EMA_ALPHA) * prev,
                });
                let pct = if total_optimizer_steps_planned > 0 {
                    100.0 * optimizer_step_count as f64 / total_optimizer_steps_planned as f64
                } else {
                    0.0
                };
                let eta_s = ema_steps_per_sec.map(|s| {
                    if s > 0.0 {
                        (total_optimizer_steps_planned.saturating_sub(optimizer_step_count) as f64
                            / s) as u64
                    } else {
                        0
                    }
                });
                let eta_str = eta_s.map_or("eta ?".into(), |s| {
                    if s >= 3600 {
                        format!("eta ~{}h {:02}m {:02}s", s / 3600, (s % 3600) / 60, s % 60)
                    } else {
                        format!("eta ~{:02}m {:02}s", s / 60, s % 60)
                    }
                });
                let eff_batch = config.batch_size.max(1) * config.grad_accum.max(1);
                let ema_str = ema_loss_val
                    .map(|v| format!("{:.4}", v))
                    .unwrap_or_else(|| "----".to_string());
                train_log::info(&format!(
                    "E{:02}/{} step={} opt_step={} loss={:.4} (ema={}) lr={:.2e} eff_batch={} {:.1}% {} skips(no_sup={},short={},curric={},oob={}) traj(weighted_pairs={},clamped_pairs={})",
                    epoch,
                    config.epochs,
                    global_step,
                    optimizer_step_count,
                    loss_val,
                    ema_str,
                    lr_applied_this_step,
                    eff_batch,
                    pct,
                    eta_str,
                    skip_no_supervised_positions,
                    skip_short_seq,
                    skip_curriculum,
                    skip_token_id_oob,
                    trajectory_weighted_pairs,
                    trajectory_clamped_pairs
                ));
                let step_payload = build_train_step_payload(
                    epoch,
                    global_step,
                    optimizer_step_count,
                    loss_val,
                    lr_applied_this_step,
                    eta_s,
                    total_optimizer_steps_planned,
                    skip_no_supervised_positions,
                    skip_short_seq,
                    skip_curriculum,
                    skip_token_id_oob,
                    trajectory_weighted_pairs,
                    trajectory_clamped_pairs,
                    ema_steps_per_sec,
                    total_valid_tokens,
                    total_theoretical_tokens,
                    config.batch_size.max(1) as u64,
                    config.seq_len as u64,
                );
                telemetry::append(out, telemetry_schema::events::TRAIN_STEP, step_payload)?;
                progress_anchor_step = optimizer_step_count;
                progress_anchor_time = now;
                last_progress = now;
            }

            // ── Graceful pause check ──────────────────────────────────────────
            if PAUSE_FLAG.load(Ordering::SeqCst) {
                let ckpt_path = out.join(format!("pause_step_{global_step}.safetensors"));
                trainer
                    .save_adapter(&ckpt_path)
                    .context("save pause adapter")?;
                let state = CheckpointState {
                    schema: crate::mens::tensor::checkpoint_state::CHECKPOINT_SCHEMA.to_string(),
                    run_id: run_id.to_string(),
                    epoch: epoch as u32,
                    global_step,
                    pair_offset: pair_loop_idx + 1,
                    shuffled_indices: shuffled_indices.clone(),
                    rng_seed: config.seed,
                    adapter_path: ckpt_path.display().to_string(),
                    last_loss: last_loss_val,
                    wall_seconds_elapsed: run_start_inst.elapsed().as_secs_f64(),
                    saved_at_utc: CheckpointState::now_utc(),
                };
                state.save(out).context("save CheckpointState on pause")?;
                let wall_secs = run_start_inst.elapsed().as_secs_f64();
                let ms_per_step = if global_step > 0 {
                    (wall_secs * 1000.0) / global_step as f64
                } else {
                    0.0
                };
                train_log::warn(&format!(
                    "Training paused at step {global_step}. Resume with 'vox mens train --resume {}'",
                    out.display()
                ));
                return Ok(crate::mens::tensor::backend::TrainingSummary {
                    wall_secs,
                    total_steps: global_step as usize,
                    total_tokens,
                    ms_per_step,
                });
            }

            // ── Mid-epoch checkpoint ──────────────────────────────────────────
            super::checkpoint_mid::maybe_save_mid_epoch_checkpoint(
                trainer,
                out,
                config,
                db_tx,
                run_id,
                epoch,
                global_step,
                pair_loop_idx,
                &shuffled_indices,
                last_loss_val,
                run_start_inst,
            )?;
        }

        // ── Validation Pass ───────────────────────────────────────────────────
        let (val_loss_sum, val_steps) = super::validation::run_validation_pass(
            &eval_pairs,
            tokenizer,
            device,
            &model,
            system_prompt,
            config,
        );
        if val_steps > 0 {
            last_avg_val_loss = Some(val_loss_sum / val_steps as f64);
        }

        // ── Epoch boundary: summary + checkpoint ──────────────────────────────
        super::epoch_boundary::finish_epoch(
            trainer,
            out,
            config,
            db_tx,
            run_id,
            epoch,
            global_step,
            epoch_steps,
            epoch_loss_sum,
            val_loss_sum,
            val_steps,
            last_loss_val,
            progress_anchor_time,
        )?;
    }

    super::finalize::finalize_training_run(
        trainer,
        bundle,
        out,
        config,
        db_tx,
        run_id,
        device_label,
        adapter_layer_order,
        base_key_map,
        global_step,
        optimizer_step_count,
        total_tokens,
        total_step_count,
        total_loss_sum,
        last_avg_val_loss,
        TrainingLoopStats {
            skip_no_supervised_positions,
            skip_short_seq,
            skip_curriculum,
            skip_token_id_oob,
        },
        run_start_inst,
    )
}

fn build_epoch_shuffled_indices(
    epoch: usize,
    start_epoch: usize,
    pair_count: usize,
    resume_shuffled_indices: &Option<Vec<usize>>,
    rng: &mut rand::rngs::StdRng,
) -> Vec<usize> {
    if epoch == start_epoch
        && let Some(idx) = resume_shuffled_indices
        && !idx.is_empty()
    {
        return idx.clone();
    }
    let mut idx: Vec<usize> = (0..pair_count).collect();
    idx.shuffle(rng);
    idx
}

fn sanitize_resume_indices(indices: &[usize], pair_count: usize) -> (Vec<usize>, usize) {
    if indices.is_empty() {
        return (Vec::new(), 0);
    }
    let mut seen = vec![false; pair_count];
    let mut out = Vec::with_capacity(indices.len());
    let mut dropped = 0usize;
    for &idx in indices {
        if idx >= pair_count || seen[idx] {
            dropped += 1;
            continue;
        }
        seen[idx] = true;
        out.push(idx);
    }
    (out, dropped)
}

fn trajectory_weight_for_pair(pair: &TrainingPair, config: &LoraTrainingConfig) -> (f64, bool) {
    if !config.trajectory_weighting_enabled {
        return (1.0, false);
    }
    let mut weight = 1.0_f64;
    if let Some(category) = pair.category.as_deref() {
        let c = category.to_ascii_lowercase();
        if c.contains("tool_trace") || c.contains("trajectory") {
            weight *= config.trajectory_tool_trace_boost.max(0.0) as f64;
        }
        if c.contains("fail") || c.contains("error") {
            weight *= config.trajectory_failure_category_boost.max(0.0) as f64;
        }
    }
    if let (Some(floor), Some(rating)) = (config.trajectory_quality_floor, pair.rating)
        && rating >= floor
    {
        weight *= config.trajectory_quality_boost.max(0.0) as f64;
    }
    if !weight.is_finite() {
        return (1.0, true);
    }
    const MAX_TRAJECTORY_WEIGHT: f64 = 8.0;
    let clamped = weight.clamp(0.0, MAX_TRAJECTORY_WEIGHT);
    let was_clamped = (clamped - weight).abs() > f64::EPSILON;
    (clamped, was_clamped)
}

#[allow(clippy::too_many_arguments)]
fn build_train_step_payload(
    epoch: usize,
    global_step: u32,
    optimizer_step_count: u32,
    loss_val: f32,
    lr_applied_this_step: f64,
    eta_s: Option<u64>,
    total_optimizer_steps_planned: u32,
    skip_no_supervised_positions: u64,
    skip_short_seq: u64,
    skip_curriculum: u64,
    skip_token_id_oob: u64,
    trajectory_weighted_pairs: u64,
    trajectory_clamped_pairs: u64,
    ema_steps_per_sec: Option<f64>,
    total_valid_tokens: u64,
    total_theoretical_tokens: u64,
    batch_size: u64,
    seq_len: u64,
) -> serde_json::Value {
    let supervised_ratio_pct = if total_theoretical_tokens == 0 {
        0.0
    } else {
        (total_valid_tokens as f64 / total_theoretical_tokens as f64) * 100.0
    };
    let token_throughput_proxy = ema_steps_per_sec.map(|s| s * batch_size as f64 * seq_len as f64);
    serde_json::json!({
        telemetry_schema::keys::EPOCH: epoch,
        telemetry_schema::keys::STEP: global_step,
        "optimizer_step": optimizer_step_count,
        telemetry_schema::keys::LOSS: loss_val,
        telemetry_schema::keys::LR: lr_applied_this_step,
        telemetry_schema::keys::LEARNING_RATE: lr_applied_this_step,
        telemetry_schema::keys::ETA_SECONDS_REMAINING: eta_s,
        telemetry_schema::keys::PROGRESS_FRACTION: optimizer_step_count as f64 / total_optimizer_steps_planned.max(1) as f64,
        telemetry_schema::keys::STEPS_PER_SEC_EMA: ema_steps_per_sec,
        telemetry_schema::keys::TOKENS_PER_SEC: token_throughput_proxy,
        telemetry_schema::keys::TOKENS_PER_SEC_IS_PROXY: true,
        telemetry_schema::keys::VALID_TOKENS: total_valid_tokens,
        telemetry_schema::keys::THEORETICAL_TOKENS: total_theoretical_tokens,
        telemetry_schema::keys::SUPERVISED_RATIO_PCT: supervised_ratio_pct,
        "skip_no_supervised_positions": skip_no_supervised_positions,
        "skip_short_seq": skip_short_seq,
        "skip_curriculum": skip_curriculum,
        "skip_token_id_oob": skip_token_id_oob,
        "trajectory_weighted_pairs": trajectory_weighted_pairs,
        "trajectory_clamped_pairs": trajectory_clamped_pairs,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_epoch_shuffled_indices, build_train_step_payload, sanitize_resume_indices,
        token_ids_in_model_vocab, trajectory_weight_for_pair,
    };
    use rand::SeedableRng;
    use vox_tensor::data::TrainingPair;

    use crate::mens::tensor::training_config::LoraTrainingConfig;

    #[test]
    fn token_ids_in_model_vocab_accepts_in_range() {
        assert!(token_ids_in_model_vocab(&[0, 10, 99], 100));
    }

    #[test]
    fn token_ids_in_model_vocab_rejects_when_id_ge_vocab() {
        assert!(!token_ids_in_model_vocab(&[0, 100], 100));
    }

    #[test]
    fn token_ids_in_model_vocab_empty_allowed_when_vocab_nonzero() {
        assert!(token_ids_in_model_vocab(&[], 500));
    }

    #[test]
    fn token_ids_in_model_vocab_rejects_zero_vocab() {
        assert!(!token_ids_in_model_vocab(&[0], 0));
    }

    #[test]
    fn uses_resume_indices_when_present_and_nonempty() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let got = build_epoch_shuffled_indices(3, 3, 5, &Some(vec![4, 2, 1, 3, 0]), &mut rng);
        assert_eq!(got, vec![4, 2, 1, 3, 0]);
    }

    #[test]
    fn reshuffles_when_resume_indices_are_empty() {
        let mut rng_a = rand::rngs::StdRng::seed_from_u64(42);
        let mut rng_b = rand::rngs::StdRng::seed_from_u64(42);
        let got = build_epoch_shuffled_indices(2, 2, 6, &Some(vec![]), &mut rng_a);
        let expect = build_epoch_shuffled_indices(2, 1, 6, &None, &mut rng_b);
        assert_eq!(got, expect);
        assert!(!got.is_empty());
    }

    #[test]
    fn sanitize_resume_indices_rejects_out_of_bounds_and_duplicates() {
        let (indices, dropped) = sanitize_resume_indices(&[3, 1, 3, 5, 0], 4);
        assert_eq!(indices, vec![3, 1, 0]);
        assert_eq!(dropped, 2);
    }

    #[test]
    fn trajectory_weighting_defaults_to_identity_when_disabled() {
        let pair = TrainingPair {
            prompt: "p".into(),
            response: "r".into(),
            rating: Some(5),
            category: Some("tool_trace".into()),
            difficulty: None,
            lane: None,
            response_mode: None,
            task_family: None,
        };
        let cfg = LoraTrainingConfig::default();
        assert_eq!(trajectory_weight_for_pair(&pair, &cfg), (1.0, false));
    }

    #[test]
    fn trajectory_weighting_applies_category_and_quality_boosts() {
        let pair = TrainingPair {
            prompt: "p".into(),
            response: "r".into(),
            rating: Some(5),
            category: Some("tool_trace_failure".into()),
            difficulty: None,
            lane: None,
            response_mode: None,
            task_family: None,
        };
        let cfg = LoraTrainingConfig {
            trajectory_weighting_enabled: true,
            trajectory_tool_trace_boost: 1.2,
            trajectory_failure_category_boost: 1.1,
            trajectory_quality_floor: Some(4),
            trajectory_quality_boost: 1.05,
            ..Default::default()
        };
        let (w, clamped) = trajectory_weight_for_pair(&pair, &cfg);
        assert!(!clamped);
        let expected = (1.2_f32 as f64) * (1.1_f32 as f64) * (1.05_f32 as f64);
        assert!(
            (w - expected).abs() < 1e-9,
            "w={w} expected {expected} (f32 boost chain)"
        );
    }

    #[test]
    fn trajectory_weighting_clamps_pathological_values() {
        let pair = TrainingPair {
            prompt: "p".into(),
            response: "r".into(),
            rating: Some(5),
            category: Some("trajectory_failure".into()),
            difficulty: None,
            lane: None,
            response_mode: None,
            task_family: None,
        };
        let cfg = LoraTrainingConfig {
            trajectory_weighting_enabled: true,
            trajectory_tool_trace_boost: 1000.0,
            trajectory_failure_category_boost: 1000.0,
            trajectory_quality_floor: Some(1),
            trajectory_quality_boost: 1000.0,
            ..Default::default()
        };
        let (w, clamped) = trajectory_weight_for_pair(&pair, &cfg);
        assert!(clamped);
        assert!(w <= 8.0);
    }

    #[test]
    fn trajectory_telemetry_payload_reports_clamped_pairs() {
        let payload = build_train_step_payload(
            1,
            10,
            4,
            0.55,
            1e-4,
            Some(90),
            25,
            0,
            0,
            0,
            2,
            6,
            3,
            Some(2.2),
            128,
            256,
            2,
            128,
        );
        assert_eq!(
            payload.get("skip_token_id_oob").and_then(|v| v.as_u64()),
            Some(2)
        );
        assert_eq!(
            payload
                .get("trajectory_weighted_pairs")
                .and_then(|v| v.as_u64()),
            Some(6)
        );
        assert_eq!(
            payload
                .get("trajectory_clamped_pairs")
                .and_then(|v| v.as_u64()),
            Some(3)
        );
    }
}
