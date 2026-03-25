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

use super::{compute_cosine_lr, load_adapter_into_trainer, TrainingDbEvent, PAUSE_FLAG, QLORA_ETA_EMA_ALPHA};
use crate::mens::tensor::{
    backend, checkpoint_state::CheckpointState, manifest, qlora_preflight::QloraEmbedBundle, telemetry,
    telemetry_schema, train_log, training_config::LoraTrainingConfig, training_text::plain_system_prompt_response,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn run_training_loop(
    trainer: &mut QLoraTrainer,
    model: crate::mens::tensor::candle_model_qwen::Qwen2Model,
    bundle: &QloraEmbedBundle,
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
    warmup_steps: usize,
) -> Result<backend::TrainingSummary> {
    if !config.qlora_double_quant {
        train_log::warn(
            "qlora_double_quant=false requested, but Candle QLoRA currently uses preset quantization; flag is accepted for compatibility only.",
        );
    }
    if config.qlora_lm_head_only || config.qlora_proxy_max_layers.is_some() {
        train_log::warn(
            "qlora_lm_head_only / qlora_proxy_max_layers are not yet wired into the Candle training graph; proceeding with full graph.",
        );
    }
    if config.qlora_max_skip_rate.is_some() {
        train_log::warn(
            "qlora_max_skip_rate is not yet enforced in Candle training; keep this as observability-only until skip accounting lands.",
        );
    }

    // ── Resume detection ─────────────────────────────────────────────────────
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
        // Attempt to warm-start LoRA weights
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
        resume_shuffled_indices = Some(ckpt.shuffled_indices);
    }

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
                proxy_stack_complete: true,
                middle_layers_active: bundle.layout.num_hidden_layers,
                ce_last_k: config.qlora_ce_last_k.max(1),
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
    let mut progress_anchor_step = global_step;
    let mut progress_anchor_time = Instant::now();
    let mut last_loss_val: f32 = 0.0;
    let mut ema_loss_val: Option<f64> = None;
    let mut total_loss_sum = 0.0f64;
    let mut total_step_count: u32 = 0;
    let mut total_tokens: usize = 0;

    let run_start_inst = Instant::now();
    for epoch in start_epoch..=config.epochs {
        // ── Epoch shuffle (or restore from checkpoint on resume epoch) ────────
        let shuffled_indices: Vec<usize> = if epoch == start_epoch {
            if let Some(ref idx) = resume_shuffled_indices {
                idx.clone()
            } else {
                let mut idx: Vec<usize> = (0..pairs.len()).collect();
                idx.shuffle(&mut rng);
                idx
            }
        } else {
            let mut idx: Vec<usize> = (0..pairs.len()).collect();
            idx.shuffle(&mut rng);
            idx
        };

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
            resume_pair_offset
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

            // Curriculum filter
            if config.curriculum && pair.difficulty.unwrap_or(5) > max_difficulty {
                continue;
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
            let mut trunc_offset = 0usize;
            total_tokens += ids.len();
            if ids.len() > config.seq_len {
                trunc_offset = ids.len() - config.seq_len;
                ids = ids[trunc_offset..].to_vec();
            }
            if ids.len() < 2 {
                continue; // skip sequences too short to form an input/target pair
            }

            // Separate input (all but last) and target (all but first) tokens
            let input_ids =
                candle_core::Tensor::new(&ids[..ids.len() - 1], device)?.unsqueeze(0)?;
            let targets = candle_core::Tensor::new(&ids[1..], device)?.unsqueeze(0)?;

            let mut lr_applied_this_step = 0.0_f64;
            let loss_val = (|| -> Result<Option<f32>> {
                let logits = model.forward(&input_ids)?;
                // [batch, seq-1, vocab] → [batch*(seq-1), vocab]
                let logits = logits.flatten_to(1)?;
                let targets_flat = targets.flatten_all()?;

                // ── Supervision masking ───────────────────────────────────────
                // `plain_system_prompt_response` builds "system\\nprompt\\nresponse", so we
                // derive the mask boundary from the exact tokenized prefix and then adjust it
                // for left-truncation.
                let prompt_len = prefix_len.saturating_sub(trunc_offset);
                let ids_len = ids.len();
                let ce_last_k = config.qlora_ce_last_k.max(1);
                let last_k_start = ids_len.saturating_sub(ce_last_k);

                // Mask tokens that belong to the prompt (system + human)
                // IDs are [seq] (input is ids[..n-1], targets are ids[1..n])
                // If ids = [S, H, A, A], seq=4. input=[S, H, A], targets=[H, A, A].
                // prompt_len (S+H) = 2. we want mask=[0, 1, 1] relative to targets.
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
                    // No overlap between last-K CE window and assistant tokens (e.g. truncated
                    // away or empty response) — avoid 0/0 NaN and do not call backward.
                    train_log::debug(&format!(
                        "skip pair: no supervised CE positions (prompt_len={} last_k_start={} seq={})",
                        prompt_len, last_k_start, ids_len
                    ));
                    return Ok(None);
                }

                // Apply mask to loss (cross_entropy already averages, so we need per-token CE)
                // For custom masking we use log_softmax + gather.
                let log_sm = candle_nn::ops::log_softmax(&logits, 1)?;
                let logprobs = log_sm
                    .gather(&targets_flat.unsqueeze(1)?, 1)?
                    .flatten_all()?;
                let loss = (logprobs.broadcast_mul(&mask)?.sum_all()? / mask.sum_all()?)?;
                // Invert sign because log_softmax is negative
                let loss = (loss * -1.0)?;

                let loss_scalar = loss.to_scalar::<f32>()?;
                if !loss_scalar.is_finite() {
                    train_log::warn(&format!(
                        "⚠ Non-finite loss before backward at epoch {} step {} (skip update); try --lr or check data",
                        epoch, global_step
                    ));
                    return Ok(None);
                }

                trainer
                    .backward_step(&loss)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                // LR that was in effect for the backward/optimizer step above (before we bump for next step).
                lr_applied_this_step = trainer.current_lr();

                // ── Cosine LR schedule for the *next* step ────────────────────
                let lr_next = compute_cosine_lr(
                    global_step,
                    warmup_steps,
                    total_steps_planned,
                    config.learning_rate,
                );
                trainer.config.adapter_config.learning_rate = lr_next;
                trainer.update_lr();

                Ok(Some(loss_scalar))
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
                let ds = (global_step - progress_anchor_step) as f64;
                let sps = ds / dt;
                ema_steps_per_sec = Some(match ema_steps_per_sec {
                    None => sps,
                    Some(prev) => QLORA_ETA_EMA_ALPHA * sps + (1.0 - QLORA_ETA_EMA_ALPHA) * prev,
                });
                let pct = if total_steps_planned > 0 {
                    100.0 * global_step as f64 / total_steps_planned as f64
                } else {
                    0.0
                };
                let eta_s = ema_steps_per_sec.map(|s| {
                    if s > 0.0 {
                        (total_steps_planned.saturating_sub(global_step) as f64 / s) as u64
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
                    "E{:02}/{} step={} loss={:.4} (ema={}) lr={:.2e} eff_batch={} {:.1}% {}",
                    epoch,
                    config.epochs,
                    global_step,
                    loss_val,
                    ema_str,
                    lr_applied_this_step,
                    eff_batch,
                    pct,
                    eta_str
                ));
                telemetry::append(
                    out,
                    telemetry_schema::events::TRAIN_STEP,
                    serde_json::json!({
                        telemetry_schema::keys::EPOCH: epoch,
                        telemetry_schema::keys::STEP: global_step,
                        telemetry_schema::keys::LOSS: loss_val,
                        telemetry_schema::keys::LR: lr_applied_this_step,
                        telemetry_schema::keys::ETA_SECONDS_REMAINING: eta_s,
                        telemetry_schema::keys::PROGRESS_FRACTION: global_step as f64 / total_steps_planned.max(1) as f64,
                        telemetry_schema::keys::STEPS_PER_SEC_EMA: ema_steps_per_sec,
                    }),
                )?;
                progress_anchor_step = global_step;
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
                    "Training paused at step {global_step}. Resume with 'vox schola train --resume {}'",
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
        total_tokens,
        total_step_count,
        total_loss_sum,
        run_start_inst,
    )
}
