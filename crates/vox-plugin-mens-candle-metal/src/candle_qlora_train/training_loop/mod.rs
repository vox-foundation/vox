//! Main epoch loop for QLoRA training.
//!
//! Ported verbatim from vox-populi (SP3 sub-batch C).

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use candle_core::Device;
use owo_colors::OwoColorize;
use std::sync::atomic::Ordering;
use tokenizers::Tokenizer;
use vox_tensor::data::TrainingPair;

use qlora_rs::training::QLoraTrainer;
use rand::SeedableRng;

use super::{
    PAUSE_FLAG, QLORA_ETA_EMA_ALPHA, TrainingDbEvent, TrainingLoopStats, compute_cosine_lr,
};
use crate::{
    config::LoraTrainingConfig, manifest, qlora_preflight::QloraEmbedBundle, telemetry,
    telemetry_schema, train_log, training_summary::TrainingSummary,
};

// checkpoint / curriculum / encoding / logic / telem_helpers moved to
// vox-plugin-mens-candle-core — byte-for-byte identical to the CUDA plugin's
// copies (logic.rs here held only `trajectory_weight_for_pair`).
pub use vox_plugin_mens_candle_core::candle_qlora_train::training_loop::{
    checkpoint, curriculum, encoding, forward, telem_helpers,
};

pub mod logic;
pub mod types;
pub mod validation;

pub use self::checkpoint::apply_checkpoint_resume;
pub use self::validation::preflight_masked_ce_finite;

use self::types::*;

/// On a real OOM: flush the adapter checkpoint so the run resumes, append an
/// observed-peak calibration record, and return an actionable error.
///
/// Refuse-and-resume, not auto-retry with a smaller batch: halving the batch
/// mid-run silently changes the effective batch size and the LR schedule for
/// the remainder of the run — a silent substitution. See Task 7 brief.
// ponytail: refuse + checkpoint, not mid-run batch retreat. Add
// --on-oom=shrink as an explicit flag only if the OOM rate justifies it.
#[allow(clippy::too_many_arguments)]
fn checkpoint_and_bail_on_oom(
    err: anyhow::Error,
    trainer: &mut QLoraTrainer,
    out: &Path,
    config: &LoraTrainingConfig,
    db_tx: &tokio::sync::mpsc::UnboundedSender<TrainingDbEvent>,
    run_id: &str,
    epoch: usize,
    global_step: u32,
    pair_loop_idx: usize,
    shuffled_indices: &[usize],
    last_loss_val: f32,
    run_start_inst: Instant,
    total_tokens: usize,
    micro_steps_executed: u32,
) -> anyhow::Error {
    if !super::oom::is_oom(&err) {
        return err;
    }

    let ckpt_path = out.join(format!("oom_step_{global_step}.safetensors"));
    let checkpoint_error = trainer.save_adapter(&ckpt_path).err().map(|e| {
        train_log::warn(&format!("OOM adapter checkpoint save failed: {e}"));
        e.to_string()
    });
    let state = crate::checkpoint_state::CheckpointState {
        schema: crate::checkpoint_state::CHECKPOINT_SCHEMA.to_string(),
        run_id: run_id.to_string(),
        epoch: epoch as u32,
        global_step,
        pair_offset: pair_loop_idx + 1,
        shuffled_indices: shuffled_indices.to_vec(),
        rng_seed: config.seed,
        adapter_path: ckpt_path.display().to_string(),
        last_loss: last_loss_val,
        wall_seconds_elapsed: run_start_inst.elapsed().as_secs_f64(),
        saved_at_utc: crate::checkpoint_state::CheckpointState::now_utc(),
        data_fingerprint: config.data_fingerprint.clone(),
        total_tokens,
        micro_steps_executed,
    };
    if let Err(e) = state.save(out) {
        train_log::warn(&format!("OOM CheckpointState save failed: {e}"));
    }
    let _ = db_tx.send(TrainingDbEvent::Checkpoint {
        run_id: run_id.to_string(),
        epoch: epoch as u32,
        global_step,
        last_loss: Some(last_loss_val),
        adapter_path: ckpt_path.display().to_string(),
    });

    // ponytail: this plugin crate has no live VRAM telemetry hookup yet
    // (see training_loop/telem_helpers.rs — HardwareRegistry::monitor() is
    // vox-populi-internal); 0 records "unknown observed peak", not a real
    // reading. Wire a real reading when the host-capability reconnect lands.
    let ev = super::oom::OomEvent {
        step: global_step,
        observed_bytes: 0,
        batch_size: config.batch_size.max(1),
        seq_len: config.seq_len,
        predicted_bytes: 0,
        checkpoint_error,
    };
    let msg = super::oom::render_oom(&ev);
    if let Err(e) = telemetry::append(
        out,
        telemetry_schema::events::OOM,
        serde_json::json!({
            "step": ev.step,
            "batch_size": ev.batch_size,
            "seq_len": ev.seq_len,
            "observed_bytes": ev.observed_bytes,
            "predicted_bytes": ev.predicted_bytes,
        }),
    ) {
        train_log::warn(&format!("OOM telemetry record failed: {e}"));
    }
    anyhow::anyhow!(msg)
}

fn effective_opt_steps_planned(
    pack_enabled: bool,
    packed_pairs_consumed: u64,
    packed_bins_emitted: u64,
    total_steps_planned: u32,
    grad_accum: u32,
    total_optimizer_steps_planned: u32,
    optimizer_step_count: u32,
) -> u32 {
    if pack_enabled && packed_bins_emitted >= 8 {
        let avg_pairs = (packed_pairs_consumed as f64 / packed_bins_emitted as f64).max(1.0);
        let est_micro = ((total_steps_planned as f64) / avg_pairs).ceil().max(1.0) as u32;
        est_micro
            .div_ceil(grad_accum.max(1))
            .max(optimizer_step_count)
    } else {
        total_optimizer_steps_planned
    }
}

fn compute_pair_reward(pair: &TrainingPair, reward_hook: Option<&str>) -> f32 {
    let mut computed_reward = pair.rating.unwrap_or(0) as f32;
    if matches!(reward_hook, Some("cargo_build") | Some("cargo_test")) {
        let mut snippet_opt = pair.response.as_deref();
        if snippet_opt.is_none()
            && let Some(turns) = &pair.messages
            && let Some(last) = turns.last()
        {
            snippet_opt = Some(last.content.as_str());
        }
        if let Some(resp) = snippet_opt {
            let code = vox_eval::extract_vox_code(resp).unwrap_or_else(|| resp.to_string());
            let ast_report = vox_compiler::ast_eval(&code);
            let r_syntax = if ast_report.parse_success { 1.0 } else { 0.0 };
            let r_test = vox_eval::cargo_build_reward(resp);
            let r_coverage = ast_report.coverage_score();
            let r = r_syntax * (r_test + r_coverage);
            computed_reward = if r_syntax == 0.0 { -1.0 } else { r as f32 };
        }
    }
    computed_reward
}

#[allow(clippy::too_many_arguments)]
pub fn run_training_loop(
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
    contamination_score: Option<f32>,
) -> Result<TrainingSummary> {
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

    let proxy_stack_complete = match crate::qlora_weights::tensor_keys_union(&bundle.weight_paths) {
        Ok(present) => {
            let cov = crate::qlora_weights::middle_projection_coverage(&bundle.layout, &present);
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
        total_tokens: resume_total_tokens,
        micro_steps_executed: resume_micro_steps_executed,
    } = resume;

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
            {
                let mut run = manifest::InitialManifestRun::from_lora_config(config);
                run.contamination_score = contamination_score;
                run
            },
            Some(bundle.tokenizer_path.display().to_string()),
            manifest::InitialTrainingKernel::CandleQlora {
                proxy_stack_complete,
                middle_layers_active: bundle.layout.num_hidden_layers,
                ce_last_k: config.qlora_ce_last_k,
                architecture: match bundle.layout.architecture {
                    crate::hf_layout::HfArchitecture::Qwen35 => "qwen3_5".to_string(),
                    crate::hf_layout::HfArchitecture::Gpt2 => "gpt2".to_string(),
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

    let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);
    let mut last_progress = Instant::now();
    let progress_every = vox_config::timeouts::D_5S;
    let mut ema_steps_per_sec: Option<f64> = None;
    let mut optimizer_step_count: u32 = global_step / config.grad_accum.max(1) as u32;
    // Optimizer step `n` (0-based) runs at `compute_cosine_lr(n)`. Set it before the
    // first step too: step 0 used to run at the full base LR (no warmup) and every
    // later step lagged the schedule by one.
    trainer.config.adapter_config.learning_rate = compute_cosine_lr(
        optimizer_step_count,
        warmup_steps,
        total_optimizer_steps_planned,
        config.learning_rate,
    );
    trainer.update_lr();
    let mut progress_anchor_step = optimizer_step_count;
    let mut progress_anchor_time = Instant::now();
    let mut last_loss_val: f32 = 0.0;
    let mut ema_loss_val: Option<f64> = None;
    let mut total_loss_sum = 0.0f64;
    let mut total_step_count: u32 = resume_micro_steps_executed;
    let mut total_tokens: usize = resume_total_tokens;
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
    let mut total_syntax_weight: f64 = 0.0;
    let mut skip_non_finite: u64 = 0;
    // Human stderr progress line (cadence: train_log::progress_line_due).
    let loop_start_opt_step = optimizer_step_count;
    let mut line_printed_any = false;
    let mut line_opt_step = optimizer_step_count;
    let mut line_time = Instant::now();
    let mut line_tokens: u64 = 0;
    let mut first_backward_logged = false;

    struct BufferedEncoded {
        pair_loop_idx: usize,
        pair_real_idx: usize,
        enc:
            vox_plugin_mens_candle_core::candle_qlora_train::training_loop::types::EncodedTrainStep,
        mask_sum: f32,
    }

    let pack_enabled = config.sequence_packing;
    let pack_cap = config.seq_len.max(2);
    let mut packed_pairs_consumed: u64 = 0;
    let mut packed_bins_emitted: u64 = 0;

    // Started before the first step so its peak covers the whole run,
    // including the first-step allocation spike; read after the last step.
    // drift-allow(timeout-literal,duplicate-numeric-literal): local wait/poll bound, not an HTTP request timeout; a shared constant needs a vox-config edge
    let peak_sampler = super::PeakSampler::start(device, std::time::Duration::from_millis(200));
    let run_start_inst = Instant::now();
    let mut capped = false;
    'epoch_loop: for epoch in start_epoch..=config.epochs {
        let shuffled_indices = checkpoint::build_epoch_shuffled_indices(
            epoch,
            start_epoch,
            &pairs,
            &resume_shuffled_indices,
            &mut rng,
            config.curriculum,
        );

        trainer.start_epoch();

        if config.curriculum {
            let max_difficulty = curriculum::max_difficulty_for_epoch(epoch, config);
            let phase_label = config
                .curriculum_schedule
                .as_ref()
                .and_then(|s| s.curriculum_phases.as_ref())
                .and_then(|p| p.get(epoch - 1))
                .cloned()
                .unwrap_or_else(|| "auto".to_string());

            train_log::info(&format!(
                "Epoch {}/{} [phase: {}] curriculum threshold: diff <= {}",
                epoch,
                config.epochs,
                phase_label.cyan(),
                max_difficulty
            ));
        }

        let mut epoch_loss_sum = 0.0f64;
        let mut epoch_steps = 0u32;
        let pair_start = if epoch == start_epoch {
            resume_pair_offset.min(shuffled_indices.len())
        } else {
            0
        };

        let max_difficulty = curriculum::max_difficulty_for_epoch(epoch, config);

        let mut pending_bin: Vec<BufferedEncoded> = Vec::new();
        let mut pending_bin_tokens: usize = 0;
        let mut norm_queue: Vec<Vec<BufferedEncoded>> = Vec::new();

        for entry in shuffled_indices
            .iter()
            .enumerate()
            .skip(pair_start)
            .map(Some)
            .chain(std::iter::once(None))
        {
            let force_flush = if let Some((pair_loop_idx, &pair_real_idx)) = entry {
                let since_line = line_time.elapsed();
                if train_log::progress_line_due(
                    line_printed_any,
                    optimizer_step_count - line_opt_step,
                    since_line,
                ) {
                    let window_tokens = total_valid_tokens - line_tokens;
                    let planned_opt_now = effective_opt_steps_planned(
                        pack_enabled,
                        packed_pairs_consumed,
                        packed_bins_emitted,
                        total_steps_planned,
                        grad_accum,
                        total_optimizer_steps_planned,
                        optimizer_step_count,
                    );
                    train_log::info(&train_log::format_progress_line(&train_log::ProgressLine {
                        epoch,
                        epochs: config.epochs,
                        opt_step: optimizer_step_count,
                        opt_steps_planned: planned_opt_now,
                        micro_step: global_step,
                        loss_ema: ema_loss_val,
                        loss_last: ema_loss_val.map(|_| last_loss_val),
                        lr: trainer.current_lr(),
                        tokens_per_sec: (window_tokens > 0)
                            .then(|| window_tokens as f64 / since_line.as_secs_f64().max(1e-3)),
                        eta_secs: train_log::eta_secs(
                            optimizer_step_count - loop_start_opt_step,
                            planned_opt_now.saturating_sub(optimizer_step_count),
                            run_start_inst.elapsed(),
                        ),
                        skips: train_log::SkipCounts {
                            curriculum: skip_curriculum,
                            short_seq: skip_short_seq,
                            no_supervision: skip_no_supervised_positions,
                            non_finite: skip_non_finite,
                            token_oob: skip_token_id_oob,
                        },
                    }));
                    line_printed_any = true;
                    line_opt_step = optimizer_step_count;
                    line_time = Instant::now();
                    line_tokens = total_valid_tokens;
                }

                let pair = &pairs[pair_real_idx];
                let (sample_weight, was_clamped) = logic::trajectory_weight_for_pair(pair, config);
                if config.trajectory_weighting_enabled
                    && (sample_weight - 1.0_f64).abs() > f64::EPSILON
                {
                    trajectory_weighted_pairs += 1;
                }
                if was_clamped {
                    trajectory_clamped_pairs += 1;
                }

                let enc = match encoding::try_encode_training_step(
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

                if !encoding::token_ids_in_model_vocab(&enc.ids, bundle.vocab) {
                    skip_token_id_oob += 1;
                    if !token_oob_warned {
                        token_oob_warned = true;
                        let max_id = enc.ids.iter().copied().max().unwrap_or(0);
                        train_log::warn(&format!(
                            "token id out of model vocab range (max_id={max_id}, vocab_size={}, pair_real_idx={pair_real_idx}); skipping pairs with OOV ids",
                            bundle.vocab
                        ));
                    }
                    continue;
                }

                let mask_sum = forward::supervised_mask_sum_preview(
                    enc.ids.len(),
                    enc.prefix_len,
                    enc.trunc_offset,
                    enc.token_weights.as_deref(),
                    config.qlora_ce_last_k,
                );
                if mask_sum <= 0.0 || !mask_sum.is_finite() {
                    skip_no_supervised_positions += 1;
                    continue;
                }

                let item = BufferedEncoded {
                    pair_loop_idx,
                    pair_real_idx,
                    enc,
                    mask_sum,
                };
                if pack_enabled {
                    if !pending_bin.is_empty() && pending_bin_tokens + item.enc.ids.len() > pack_cap
                    {
                        norm_queue.push(std::mem::take(&mut pending_bin));
                        pending_bin_tokens = 0;
                    }
                    pending_bin_tokens += item.enc.ids.len();
                    pending_bin.push(item);
                } else {
                    norm_queue.push(vec![item]);
                }
                false
            } else {
                if !pending_bin.is_empty() {
                    norm_queue.push(std::mem::take(&mut pending_bin));
                    pending_bin_tokens = 0;
                }
                true
            };

            let steps_until_opt = (grad_accum as usize)
                .saturating_sub(trainer.pending_accumulation_steps())
                .max(1);
            let should_flush = !norm_queue.is_empty()
                && (force_flush
                    || !config.token_normalized_grad_accum
                    || norm_queue.len() >= steps_until_opt);
            if !should_flush {
                continue;
            }

            let bins = std::mem::take(&mut norm_queue);
            let window_len = bins.len();
            let window_mask_sum: f32 = bins.iter().flat_map(|b| b.iter()).map(|s| s.mask_sum).sum();

            for bin in bins {
                let last_item = bin.last().expect("non-empty bin");
                let pair_loop_idx = last_item.pair_loop_idx;
                let bin_mask_sum: f32 = bin.iter().map(|s| s.mask_sum).sum();
                let norm_scale: f64 = if config.token_normalized_grad_accum {
                    forward::token_norm_window_scale(bin_mask_sum, window_mask_sum, window_len)
                        as f64
                } else {
                    1.0
                };

                let mut lr_applied_this_step = 0.0_f64;
                let mut completed_opt_step_this_micro = false;
                let trainable_vars_for_head = if config.gradient_checkpointing {
                    Some(trainer.trainable_vars())
                } else {
                    None
                };
                let head_mode = match trainable_vars_for_head.as_deref() {
                    Some(vars) => forward::HeadCeMode::Checkpointed {
                        trainable_vars: vars,
                        grad_accum_steps: trainer
                            .config
                            .adapter_config
                            .gradient_accumulation_steps
                            .max(1),
                    },
                    None => forward::HeadCeMode::EagerTape,
                };
                let forward_res = if bin.len() == 1 {
                    forward::forward_masked_ce(
                        &model,
                        &last_item.enc.ids,
                        last_item.enc.prefix_len,
                        last_item.enc.trunc_offset,
                        last_item.enc.sample_weight * norm_scale,
                        last_item.enc.token_weights.as_deref(),
                        config,
                        head_mode,
                        device,
                    )
                } else {
                    let packed_refs: Vec<forward::PackedSampleRef<'_>> = bin
                        .iter()
                        .map(|s| forward::PackedSampleRef {
                            ids: &s.enc.ids,
                            prefix_len: s.enc.prefix_len,
                            trunc_offset: s.enc.trunc_offset,
                            sample_weight: s.enc.sample_weight,
                            token_weights: s.enc.token_weights.as_deref(),
                        })
                        .collect();
                    forward::forward_packed_masked_ce(
                        &model,
                        &packed_refs,
                        norm_scale,
                        config,
                        head_mode,
                        device,
                    )
                };
                let loss_result = match forward_res {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(checkpoint_and_bail_on_oom(
                            e,
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
                            total_tokens,
                            total_step_count,
                        ));
                    }
                };

                let loss_val_micro = match loss_result {
                    MaskedCeForward::NoSupervision => {
                        skip_no_supervised_positions += 1;
                        None
                    }
                    MaskedCeForward::NonFinite { kind, mask_sum } => {
                        skip_non_finite += 1;
                        train_log::warn(&format!(
                            "Non-finite loss ({kind}) before backward at epoch {epoch} micro_step {global_step} (skip update); mask_sum={mask_sum:.3}",
                        ));
                        None
                    }
                    MaskedCeForward::Finite {
                        loss,
                        loss_scalar,
                        supervised_tokens,
                        theoretical_tokens,
                        syntax_weight_sum,
                        segments,
                        detached_head,
                    } => {
                        let backward_result = match (segments, detached_head, loss) {
                            (Some(segs), Some(dh), _) => {
                                logic::checkpointed_backward_step(trainer, &model, dh, segs)
                                    .map_err(|e| anyhow::anyhow!("{e}"))
                            }
                            (None, None, Some(loss_t)) => trainer
                                .backward_step(&loss_t)
                                .map_err(|e| anyhow::anyhow!("{e}")),
                            _ => Err(anyhow::anyhow!(
                                "inconsistent MaskedCeForward::Finite head/segment state"
                            )),
                        };
                        if let Err(e) = backward_result {
                            return Err(checkpoint_and_bail_on_oom(
                                e,
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
                                total_tokens,
                                total_step_count,
                            ));
                        }
                        total_valid_tokens = total_valid_tokens.saturating_add(supervised_tokens);
                        if !first_backward_logged {
                            first_backward_logged = true;
                            train_log::info(&format!(
                                "Training started: first backward pass done after {:.1}s ({} optimizer steps planned, grad_accum={grad_accum}).",
                                run_start_inst.elapsed().as_secs_f64(),
                                total_optimizer_steps_planned,
                            ));
                        }
                        total_theoretical_tokens =
                            total_theoretical_tokens.saturating_add(theoretical_tokens);
                        total_syntax_weight += syntax_weight_sum as f64;

                        lr_applied_this_step = trainer.current_lr();
                        if pack_enabled {
                            packed_pairs_consumed =
                                packed_pairs_consumed.saturating_add(bin.len() as u64);
                            packed_bins_emitted = packed_bins_emitted.saturating_add(1);
                        }

                        if trainer.pending_accumulation_steps() == 0 {
                            optimizer_step_count += 1;
                            completed_opt_step_this_micro = true;
                            let planned_opt_now = effective_opt_steps_planned(
                                pack_enabled,
                                packed_pairs_consumed,
                                packed_bins_emitted,
                                total_steps_planned,
                                grad_accum,
                                total_optimizer_steps_planned,
                                optimizer_step_count,
                            );
                            trainer.config.adapter_config.learning_rate = compute_cosine_lr(
                                optimizer_step_count,
                                warmup_steps,
                                planned_opt_now,
                                config.learning_rate,
                            );
                            trainer.update_lr();
                        }
                        Some(loss_scalar)
                    }
                };

                let Some(loss_val) = loss_val_micro else {
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

                let computed_reward = bin
                    .iter()
                    .map(|it| {
                        compute_pair_reward(&pairs[it.pair_real_idx], config.reward_hook.as_deref())
                    })
                    .sum::<f32>()
                    / bin.len().max(1) as f32;

                let _ = db_tx.send(TrainingDbEvent::GrpoStep {
                    run_id: run_id.to_string(),
                    step: global_step,
                    mean_reward: computed_reward,
                    policy_loss: loss_val,
                    clip_fraction: 0.0,
                    parse_rate: 1.0,
                });

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
                        Some(prev) => {
                            QLORA_ETA_EMA_ALPHA * sps + (1.0 - QLORA_ETA_EMA_ALPHA) * prev
                        }
                    });
                    const ETA_CALIBRATION_MIN_STEPS: u32 = 8;
                    let planned_opt_now = effective_opt_steps_planned(
                        pack_enabled,
                        packed_pairs_consumed,
                        packed_bins_emitted,
                        total_steps_planned,
                        grad_accum,
                        total_optimizer_steps_planned,
                        optimizer_step_count,
                    );
                    let eta_s_telem: Option<u64> = if optimizer_step_count
                        >= ETA_CALIBRATION_MIN_STEPS
                    {
                        ema_steps_per_sec.and_then(|s| {
                            if s > 1e-6 {
                                Some(
                                    (planned_opt_now.saturating_sub(optimizer_step_count) as f64
                                        / s) as u64,
                                )
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    };
                    let step_payload = telem_helpers::build_train_step_payload(
                        epoch,
                        global_step,
                        optimizer_step_count,
                        loss_val,
                        lr_applied_this_step,
                        eta_s_telem,
                        planned_opt_now,
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
                        grad_accum as u64,
                        total_syntax_weight,
                    );
                    telemetry::append(out, telemetry_schema::events::TRAIN_STEP, step_payload)?;
                    progress_anchor_step = optimizer_step_count;
                    progress_anchor_time = now;
                    last_progress = now;
                }

                if PAUSE_FLAG.load(Ordering::SeqCst) {
                    let ckpt_path = out.join(format!("pause_step_{global_step}.safetensors"));
                    trainer
                        .save_adapter(&ckpt_path)
                        .context("save pause adapter")?;
                    let state = crate::checkpoint_state::CheckpointState {
                        schema: crate::checkpoint_state::CHECKPOINT_SCHEMA.to_string(),
                        run_id: run_id.to_string(),
                        epoch: epoch as u32,
                        global_step,
                        pair_offset: pair_loop_idx + 1,
                        shuffled_indices: shuffled_indices.clone(),
                        rng_seed: config.seed,
                        adapter_path: ckpt_path.display().to_string(),
                        last_loss: last_loss_val,
                        wall_seconds_elapsed: run_start_inst.elapsed().as_secs_f64(),
                        saved_at_utc: crate::checkpoint_state::CheckpointState::now_utc(),
                        data_fingerprint: config.data_fingerprint.clone(),
                        total_tokens,
                        micro_steps_executed: total_step_count,
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
                    return Ok(TrainingSummary {
                        wall_secs,
                        total_steps: global_step as usize,
                        total_tokens,
                        ms_per_step,
                    });
                }

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
                    total_tokens,
                    total_step_count,
                )?;

                if completed_opt_step_this_micro
                    && optimizer_step_count >= 1
                    && config
                        .max_runtime_secs
                        .is_some_and(|cap| run_start_inst.elapsed().as_secs() >= cap)
                {
                    capped = true;
                    train_log::info(&format!(
                        "Max runtime cap ({}s) reached after optimizer step {optimizer_step_count} ({:.1}s elapsed); stopping cleanly.",
                        config.max_runtime_secs.unwrap_or_default(),
                        run_start_inst.elapsed().as_secs_f64(),
                    ));
                    break 'epoch_loop;
                }
            }
        }

        if trainer
            .flush_pending_grads(true)
            .map_err(|e| anyhow::anyhow!("flush epoch pending grads: {e}"))?
        {
            optimizer_step_count += 1;
            let planned_opt_now = effective_opt_steps_planned(
                pack_enabled,
                packed_pairs_consumed,
                packed_bins_emitted,
                total_steps_planned,
                grad_accum,
                total_optimizer_steps_planned,
                optimizer_step_count,
            );
            trainer.config.adapter_config.learning_rate = compute_cosine_lr(
                optimizer_step_count,
                warmup_steps,
                planned_opt_now,
                config.learning_rate,
            );
            trainer.update_lr();
        }

        if optimizer_step_count >= 1
            && config
                .max_runtime_secs
                .is_some_and(|cap| run_start_inst.elapsed().as_secs() >= cap)
        {
            capped = true;
            train_log::info(&format!(
                "Max runtime cap ({}s) reached at epoch {epoch} flush (optimizer step {optimizer_step_count}, {:.1}s elapsed); stopping before validation.",
                config.max_runtime_secs.unwrap_or_default(),
                run_start_inst.elapsed().as_secs_f64(),
            ));
            break 'epoch_loop;
        }

        let (val_loss_sum, val_steps) = validation::run_validation_pass(
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
            total_tokens,
            total_step_count,
        )?;
    }

    // Read after the last step, before the sampler thread is torn down.
    let peak_metal_bytes = peak_sampler.observed_peak_bytes();
    drop(peak_sampler);

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
        ema_loss_val,
        last_avg_val_loss,
        TrainingLoopStats {
            skip_no_supervised_positions,
            skip_short_seq,
            skip_curriculum,
            skip_token_id_oob,
        },
        run_start_inst,
        peak_metal_bytes,
        capped,
    )
}
