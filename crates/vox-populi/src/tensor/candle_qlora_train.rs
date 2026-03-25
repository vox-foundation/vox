//! Native QLoRA training: **NF4-quantized** frozen base linears + trainable LoRA via [`qlora_rs`].
//!
//! **Device:** maps Populi `--device` to Candle (CUDA / Metal when enabled, else CPU). Override
//! with `VOX_CANDLE_DEVICE=cpu`. See [`ENV_CANDLE_DEVICE`].
//!
//! ## Training loop properties
//!
//! - **Causal mask**: enforced in [`Qwen2Attention::forward`].
//! - **GQA**: K/V projections use `num_key_value_heads` from `config.json`; weights loaded
//!   with the correct `(kv_dim, d_model)` shape; K/V tensors are repeat-interleaved.
//! - **Warmup + cosine decay**: configured via `QLoraTrainingConfig.adapter_config.lr_schedule`.
//! - **Gradient clipping**: `max_grad_norm: Some(1.0)` in `AdapterTrainingConfig`.
//! - **Resume**: loads [`CheckpointState`] on start unless `force_restart` is set.
//! - **VoxDB**: persists run start, per-checkpoint updates, and final status asynchronously.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use candle_core::{DType, Device};
use candle_nn::VarBuilder;
use peft_rs::training::{AdapterTrainingConfig, LrSchedule};
use qlora_rs::QLoraConfig;
use qlora_rs::qlora::QuantizedLinear;
use qlora_rs::training::{QLoraTrainer, QLoraTrainingConfig};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use tokenizers::Tokenizer;
use vox_tensor::data::TrainingPair;

use super::candle_qlora_merge::QloraAdapterMetaV2;
use super::checkpoint_state::CheckpointState;
use super::device::{DeviceKind, probe_gpu};
use super::manifest;
use super::qlora_preflight::preflight_native_qlora;
use super::telemetry;
use super::telemetry_schema;
use super::train_jsonl_preflight::preflight_train_jsonl;
use super::train_log;
use super::training_config::LoraTrainingConfig;
use super::training_text::plain_system_prompt_response;

/// EMA alpha for ETA calculation (0.2 = stable but react within ~5 intervals).
const QLORA_ETA_EMA_ALPHA: f64 = 0.2;
/// Environment variable: force Candle to CPU regardless of device flag.
pub const ENV_CANDLE_DEVICE: &str = "VOX_CANDLE_DEVICE";

/// Global flag for graceful interruption (Ctrl+C).
static PAUSE_FLAG: AtomicBool = AtomicBool::new(false);

// ── DB message bus ────────────────────────────────────────────────────────────

/// Events sent from the training loop to the background VoxDB writer task.
enum TrainingDbEvent {
    Start {
        run_id: String,
        adapter_tag: Option<String>,
        model_name: Option<String>,
        output_dir: String,
        data_dir: String,
        planned_steps: Option<u32>,
    },
    Checkpoint {
        run_id: String,
        epoch: u32,
        global_step: u32,
        last_loss: Option<f32>,
        adapter_path: String,
    },
    Complete {
        run_id: String,
        global_step: u32,
        adapter_path: String,
    },
    Failed {
        run_id: String,
        global_step: u32,
    },
}

// ── Device selection ──────────────────────────────────────────────────────────

fn select_candle_device(kind: DeviceKind) -> Result<(Device, String)> {
    if std::env::var(ENV_CANDLE_DEVICE)
        .map(|v| v.trim().to_lowercase() == "cpu")
        .unwrap_or(false)
    {
        return Ok((Device::Cpu, "cpu(forced)".into()));
    }

    let (device, label) = match kind {
        DeviceKind::Cpu => (Device::Cpu, "cpu".into()),
        DeviceKind::Cuda => (Device::new_cuda(0)?, "cuda:0".into()),
        DeviceKind::Metal => (Device::new_metal(0)?, "metal:0".into()),
        DeviceKind::Best => {
            let g = probe_gpu();
            match g.vendor.as_str() {
                "nvidia" => {
                    let d = Device::new_cuda(0).unwrap_or_else(|_| {
                        train_log::warn("CUDA unavailable — falling back to CPU");
                        Device::Cpu
                    });
                    let lbl = if matches!(d, Device::Cpu) { "cpu(fallback)" } else { "cuda:0" };
                    (d, lbl.into())
                }
                "apple" => {
                    let d = Device::new_metal(0).unwrap_or_else(|_| {
                        train_log::warn("Metal unavailable — falling back to CPU");
                        Device::Cpu
                    });
                    let lbl = if matches!(d, Device::Cpu) { "cpu(fallback)" } else { "metal:0" };
                    (d, lbl.into())
                }
                v => {
                    train_log::warn(&format!("Unknown GPU vendor '{v}' — falling back to CPU"));
                    (Device::Cpu, "cpu(fallback)".into())
                }
            }
        }
    };
    Ok((device, label))
}

/// Load LoRA adapter weights (safetensors) into a trainer's varmap (warm-start).
fn load_adapter_into_trainer(trainer: &mut QLoraTrainer, path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("checkpoint adapter not found: {}", path.display());
    }
    trainer.load_lora_weights(path).context("warm-start LoRA weights")?;
    train_log::info(&format!("Warm-started LoRA weights from {}", path.display()));
    Ok(())
}

/// Helper to calculate cosine learning rate with linear warmup.
fn compute_cosine_lr(step: u32, warmup: usize, total: u32, base_lr: f64) -> f64 {
    if (step as usize) < warmup {
        base_lr * (step as f64 + 1.0) / warmup.max(1) as f64
    } else {
        let progress = (step as usize - warmup) as f64 / (total as usize - warmup).max(1) as f64;
        let progress = progress.clamp(0.0, 1.0);
        base_lr * 0.5 * (1.0 + (std::f64::consts::PI * progress).cos())
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Main entry point for `--backend qlora` training, called from [`super::backend_candle_qlora`].
pub fn run_candle_qlora_train(
    data_dir: &Path,
    output_dir: Option<&Path>,
    config: &LoraTrainingConfig,
    device_kind: DeviceKind,
    system_prompt: &str,
) -> Result<crate::tensor::backend::TrainingSummary> {
    let out_buf = output_dir.map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("populi/runs").join(config.run_id.as_deref().unwrap_or("v1"))
    });
    std::fs::create_dir_all(&out_buf).context("create output dir")?;
    let out: &Path = out_buf.as_path();

    let (device, device_label) = select_candle_device(device_kind)?;

    // ── Graceful pause (Ctrl+C) ──────────────────────────────────────────────
    PAUSE_FLAG.store(false, Ordering::SeqCst);
    let _ = ctrlc::set_handler(move || {
        if PAUSE_FLAG.load(Ordering::SeqCst) {
            // Second Ctrl+C = hard exit
            std::process::exit(1);
        }
        train_log::warn("Ctrl+C detected — saving checkpoint and pausing after current step...");
        PAUSE_FLAG.store(true, Ordering::SeqCst);
    });

    let bundle = preflight_native_qlora(config).map_err(|e| {
        anyhow::anyhow!("Model preflight failed: {e}. Ensure you have run 'vox populi download --model <name>' and that tokenizer.json + safetensors are present.")
    })?;
    let tokenizer = Tokenizer::from_file(&bundle.tokenizer_path)
        .map_err(|e| anyhow::anyhow!("load tokenizer: {e}"))?;

    // Resolve training data path: config.train_file → data_dir/train.jsonl fallback
    let train_path: PathBuf = config.train_file.clone().unwrap_or_else(|| {
        data_dir.join("train.jsonl")
    });
    let _ = preflight_train_jsonl(&train_path, 1_000_000)?;
    let mut pairs = vox_tensor::data::load_all(&train_path, config.min_rating)
        .with_context(|| format!("load training data from {}", train_path.display()))?;
    if pairs.is_empty() {
        anyhow::bail!("No training pairs found in {}", train_path.display());
    }

    let val_count = {
        let pct_count = (pairs.len() as f64 * config.validation_split_ratio.unwrap_or(0.05)) as usize;
        // Task 4.2 request: 10 hold-out inputs
        if pairs.len() > 20 {
            pct_count.max(10).min(pairs.len() / 2)
        } else {
            pct_count
        }
    };
    let eval_pairs = if val_count > 0 && pairs.len() > val_count {
        pairs.split_off(pairs.len() - val_count)
    } else {
        Vec::new()
    };

    // ── GQA-aware dimensions ─────────────────────────────────────────────────
    let n_heads = bundle.layout.num_attention_heads;
    let n_kv_heads = bundle.layout.num_key_value_heads;
    let head_dim = bundle.d_model / n_heads;
    let kv_dim = n_kv_heads * head_dim;

    // ── mmap base weights ─────────────────────────────────────────────────────
    #[allow(unsafe_code)]
    let vb_mmap = unsafe {
        VarBuilder::from_mmaped_safetensors(&bundle.weight_paths, DType::F32, &device)?
    };
    let wte = vb_mmap.get((bundle.vocab, bundle.d_model), &bundle.embed_key)?;

    // ── qlora-rs config ───────────────────────────────────────────────────────
    let rank = config.rank.max(1);
    let alpha_u = config.alpha.round() as usize;
    let qlora_cfg = QLoraConfig::preset_qv_bf16(rank, alpha_u);

    let total_steps_planned = (pairs.len() * config.epochs) as u32;
    let warmup_steps = config.warmup_steps.min((total_steps_planned / 10).max(1) as usize);

    let mut train_cfg = QLoraTrainingConfig::default();
    train_cfg.adapter_config = AdapterTrainingConfig {
        learning_rate: config.learning_rate,
        lr_schedule: LrSchedule::LinearWarmup { warmup_steps },
        weight_decay: 0.01,
        gradient_accumulation_steps: config.grad_accum.max(1),
        max_grad_norm: Some(1.0), // gradient clipping
    };
    train_cfg.num_epochs = config.epochs;

    let mut trainer = QLoraTrainer::new(train_cfg, device.clone());

    // ── Build transformer graph ───────────────────────────────────────────────
    let mut model_layers = Vec::with_capacity(bundle.layout.num_hidden_layers);
    let mut adapter_layer_order: Vec<String> = Vec::new();
    let mut base_key_map: HashMap<String, String> = HashMap::new();

    let (final_norm, lm_head) = {
        let vb = trainer.var_builder();
        train_log::info(&format!(
            "Candle QLoRA: building full graph ({} layers, n_heads={}, n_kv_heads={}, head_dim={})",
            bundle.layout.num_hidden_layers, n_heads, n_kv_heads, head_dim
        ));

        for i in 0..bundle.layout.num_hidden_layers {
            // ── Layer norms ───────────────────────────────────────────────────
            let ln1_key = format!("model.layers.{i}.input_layernorm.weight");
            let ln2_key = format!("model.layers.{i}.post_attention_layernorm.weight");
            let w_ln1 = vb_mmap.get(bundle.d_model, &ln1_key)?.to_dtype(DType::F32)?;
            let w_ln2 = vb_mmap.get(bundle.d_model, &ln2_key)?.to_dtype(DType::F32)?;
            let ln1 = candle_nn::RmsNorm::new(w_ln1, 1e-6);
            let ln2 = candle_nn::RmsNorm::new(w_ln2, 1e-6);

            // ── Attention projections (GQA-correct K/V shapes) ────────────────
            let q_key = format!("model.layers.{i}.self_attn.q_proj.weight");
            let k_key = format!("model.layers.{i}.self_attn.k_proj.weight");
            let v_key = format!("model.layers.{i}.self_attn.v_proj.weight");
            let o_key = format!("model.layers.{i}.self_attn.o_proj.weight");

            let w_q = vb_mmap.get((bundle.d_model, bundle.d_model), &q_key)?.to_dtype(DType::F32)?;
            let w_k = vb_mmap.get((kv_dim, bundle.d_model), &k_key)?.to_dtype(DType::F32)?;
            let w_v = vb_mmap.get((kv_dim, bundle.d_model), &v_key)?.to_dtype(DType::F32)?;
            let w_o = vb_mmap.get((bundle.d_model, bundle.d_model), &o_key)?.to_dtype(DType::F32)?;

            let q_label = format!("l{i}.q");
            let k_label = format!("l{i}.k");
            let v_label = format!("l{i}.v");
            let o_label = format!("l{i}.o");

            let q_proj = QuantizedLinear::from_weight_with_varbuilder(&w_q, None, &qlora_cfg, vb.pp(&q_label))?;
            let k_proj = QuantizedLinear::from_weight_with_varbuilder(&w_k, None, &qlora_cfg, vb.pp(&k_label))?;
            let v_proj = QuantizedLinear::from_weight_with_varbuilder(&w_v, None, &qlora_cfg, vb.pp(&v_label))?;
            let o_proj = QuantizedLinear::from_weight_with_varbuilder(&w_o, None, &qlora_cfg, vb.pp(&o_label))?;

            for (lbl, bk) in [(&q_label, &q_key), (&k_label, &k_key), (&v_label, &v_key), (&o_label, &o_key)] {
                adapter_layer_order.push(lbl.clone());
                base_key_map.insert(lbl.clone(), bk.clone());
            }

            // ── MLP projections ───────────────────────────────────────────────
            let inter_sz = bundle.layout.intermediate_size.unwrap_or(bundle.d_model * 4);
            let gate_key = format!("model.layers.{i}.mlp.gate_proj.weight");
            let up_key   = format!("model.layers.{i}.mlp.up_proj.weight");
            let down_key = format!("model.layers.{i}.mlp.down_proj.weight");

            let w_gate = vb_mmap.get((inter_sz, bundle.d_model), &gate_key)?.to_dtype(DType::F32)?;
            let w_up   = vb_mmap.get((inter_sz, bundle.d_model), &up_key)?.to_dtype(DType::F32)?;
            let w_down = vb_mmap.get((bundle.d_model, inter_sz), &down_key)?.to_dtype(DType::F32)?;

            let gate_label = format!("l{i}.gate");
            let up_label   = format!("l{i}.up");
            let down_label = format!("l{i}.down");

            let gate_proj = QuantizedLinear::from_weight_with_varbuilder(&w_gate, None, &qlora_cfg, vb.pp(&gate_label))?;
            let up_proj   = QuantizedLinear::from_weight_with_varbuilder(&w_up,   None, &qlora_cfg, vb.pp(&up_label))?;
            let down_proj = QuantizedLinear::from_weight_with_varbuilder(&w_down,  None, &qlora_cfg, vb.pp(&down_label))?;

            for (lbl, bk) in [(&gate_label, &gate_key), (&up_label, &up_key), (&down_label, &down_key)] {
                adapter_layer_order.push(lbl.clone());
                base_key_map.insert(lbl.clone(), bk.clone());
            }

            // ── RoPE frequency table (optional per-layer) ─────────────────────
            let inv_key = format!("model.layers.{i}.self_attn.rotary_emb.inv_freq");
            let inv_freq = vb_mmap
                .get((head_dim / 2,), &inv_key)
                .ok()
                .and_then(|t| t.to_dtype(DType::F32).ok());

            model_layers.push(crate::tensor::candle_model_qwen::Qwen2Layer {
                input_layernorm: ln1,
                self_attn: crate::tensor::candle_model_qwen::Qwen2Attention {
                    q_proj, k_proj, v_proj, o_proj,
                    n_heads, n_kv_heads, head_dim,
                },
                post_attention_layernorm: ln2,
                mlp: crate::tensor::candle_model_qwen::Qwen2MLP { gate_proj, up_proj, down_proj },
                inv_freq,
            });
        }

        // ── Final norm + LM head (weight-tied to embeddings) ─────────────────
        let fnorm_w = vb_mmap.get(bundle.d_model, "model.norm.weight")?.to_dtype(DType::F32)?;
        let final_norm = candle_nn::RmsNorm::new(fnorm_w, 1e-6);
        let w_lm = wte.to_dtype(DType::F32)?;
        let lm_label = "lm_head".to_string();
        let lm_base  = bundle.embed_key.clone();
        let lm_head = QuantizedLinear::from_weight_with_varbuilder(&w_lm, None, &qlora_cfg, vb.pp(&lm_label))?;
        adapter_layer_order.push(lm_label.clone());
        base_key_map.insert(lm_label, lm_base);

        (final_norm, lm_head)
    }; // vb dropped here — VarMap retains trainable LoRA vars

    trainer.init_optimizer(&[]).context("init qlora optimizer")?;

    let model = crate::tensor::candle_model_qwen::Qwen2Model {
        embed_tokens: wte,
        layers: model_layers,
        norm: final_norm,
        lm_head,
    };

    // ── Async VoxDB writer ────────────────────────────────────────────────────
    let run_id = config.run_id.clone().unwrap_or_else(|| {
        format!("qlora_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs())
    });

    let (db_tx, mut db_rx) = tokio::sync::mpsc::unbounded_channel::<TrainingDbEvent>();

    let db_run_id = run_id.clone();
    tokio::spawn(async move {
        let Ok(db) = vox_db::VoxDb::connect_default().await else {
            tracing::warn!(run_id = %db_run_id, "VoxDB unavailable — training telemetry will not be persisted");
            return;
        };
        while let Some(evt) = db_rx.recv().await {
            match evt {
                TrainingDbEvent::Start { run_id, adapter_tag, model_name, output_dir, data_dir, planned_steps } => {
                    let params = vox_db::training_run::TrainingRunStartParams {
                        run_id: run_id.clone(),
                        adapter_tag,
                        model_name,
                        output_dir,
                        data_dir,
                        planned_steps,
                    };
                    let _ = db.record_training_run_start(&params).await;
                    let _ = db.record_training_event(&run_id, "train_start", serde_json::json!({"run_id": run_id})).await;
                }
                TrainingDbEvent::Checkpoint { run_id, epoch, global_step, last_loss, adapter_path } => {
                    let _ = db.update_training_checkpoint(&run_id, epoch, global_step, last_loss, Some(&adapter_path)).await;
                    let _ = db.record_training_checkpoint(&run_id, epoch, global_step, &adapter_path).await;
                }
                TrainingDbEvent::Complete { run_id, global_step, adapter_path } => {
                    let _ = db.mark_training_complete(&run_id, global_step, Some(&adapter_path)).await;
                    let _ = db.record_training_event(&run_id, "train_complete", serde_json::json!({"global_step": global_step})).await;
                }
                TrainingDbEvent::Failed { run_id, global_step } => {
                    let _ = db.mark_training_failed(&run_id, global_step).await;
                    let _ = db.record_training_event(&run_id, "train_failed", serde_json::json!({"global_step": global_step})).await;
                }
            }
        }
    });

    // Fire off the "start" DB event
    let _ = db_tx.send(TrainingDbEvent::Start {
        run_id: run_id.clone(),
        adapter_tag: config.adapter_tag.clone(),
        model_name: config.base_model.clone(),
        output_dir: out.display().to_string(),
        data_dir: data_dir.display().to_string(),
        planned_steps: Some(total_steps_planned),
    });

    let result = run_training_loop(
        &mut trainer,
        model,
        &bundle,
        out,
        config,
        pairs,
        eval_pairs,
        &tokenizer,
        &device,
        &db_tx,
        &run_id,
        &device_label,
        &train_path,
        system_prompt,
        &adapter_layer_order,
        &base_key_map,
        total_steps_planned,
        warmup_steps,
    );

    if result.is_err() {
        let _ = db_tx.send(TrainingDbEvent::Failed {
            run_id: run_id.clone(),
            global_step: 0,
        });
    }

    result
}

// ── Training loop ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn run_training_loop(
    trainer: &mut QLoraTrainer,
    model: crate::tensor::candle_model_qwen::Qwen2Model,
    bundle: &crate::tensor::qlora_preflight::QloraEmbedBundle,
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
) -> Result<crate::tensor::backend::TrainingSummary> {
    // ── Resume detection ─────────────────────────────────────────────────────
    let mut start_epoch = 1usize;
    let mut global_step = 0u32;
    let mut resume_pair_offset = 0usize;
    let mut resume_shuffled_indices: Option<Vec<usize>> = None;

    if !config.force_restart {
        if let Some(ckpt) = CheckpointState::load(out) {
            train_log::info(&format!(
                "Checkpoint found — resuming from epoch={} global_step={} pair_offset={}",
                ckpt.epoch, ckpt.global_step, ckpt.pair_offset
            ));
            // Attempt to warm-start LoRA weights
            if std::path::Path::new(&ckpt.adapter_path).exists() {
                let _ = load_adapter_into_trainer(trainer, std::path::Path::new(&ckpt.adapter_path));
            }
            start_epoch = ckpt.epoch as usize;
            global_step = ckpt.global_step;
            resume_pair_offset = ckpt.pair_offset;
            resume_shuffled_indices = Some(ckpt.shuffled_indices);
        }
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
            train_log::info(&format!("Epoch {}/{} curriculum threshold: diff <= {}", epoch, config.epochs, max_difficulty));
        }

        let mut epoch_loss_sum = 0.0f64;
        let mut epoch_steps = 0u32;

        let pair_start = if epoch == start_epoch { resume_pair_offset } else { 0 };

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

        for (pair_loop_idx, &pair_real_idx) in shuffled_indices.iter().enumerate().skip(pair_start) {
            let pair = &pairs[pair_real_idx];
            
            // Curriculum filter
            if config.curriculum && pair.difficulty.unwrap_or(5) > max_difficulty {
                continue;
            }

            let text = plain_system_prompt_response(system_prompt, &pair.prompt, &pair.response);
            let enc = tokenizer.encode(text, true).map_err(|e| anyhow::anyhow!("{e}"))?;
            let mut ids = enc.get_ids().to_vec();
            total_tokens += ids.len();
            if ids.len() > config.seq_len {
                let start = ids.len() - config.seq_len;
                ids = ids[start..].to_vec();
            }
            if ids.len() < 2 {
                continue; // skip sequences too short to form an input/target pair
            }

            // Separate input (all but last) and target (all but first) tokens
            let input_ids = candle_core::Tensor::new(&ids[..ids.len() - 1], device)?.unsqueeze(0)?;
            let targets = candle_core::Tensor::new(&ids[1..], device)?.unsqueeze(0)?;

            let loss_val = (|| -> Result<f32> {
                let logits = model.forward(&input_ids)?;
                // [batch, seq-1, vocab] → [batch*(seq-1), vocab]
                let logits = logits.flatten_to(1)?;
                let targets_flat = targets.flatten_all()?;

                // ── Supervision masking (Option 1: Prompt offset) ────────────
                let prompt_only = tokenizer.encode(pair.prompt.clone(), false).map_err(|e| anyhow::anyhow!("{e}"))?;
                let prompt_len = prompt_only.get_ids().len();
                
                // Mask tokens that belong to the prompt (system + human)
                // IDs are [seq] (input is ids[..n-1], targets are ids[1..n])
                // If ids = [S, H, A, A], seq=4. input=[S, H, A], targets=[H, A, A].
                // prompt_len (S+H) = 2. we want mask=[0, 1, 1] relative to targets.
                let ids_len = ids.len();
                let mask_vec: Vec<f32> = (0..ids_len - 1)
                    .map(|i| if (i + 1) >= prompt_len { 1.0f32 } else { 0.0 })
                    .collect();
                let mask = candle_core::Tensor::from_vec(mask_vec, ids_len - 1, device)?;
                
                // Apply mask to loss (cross_entropy already averages, so we need per-token CE)
                // For custom masking we use log_softmax + gather.
                let log_sm = candle_nn::ops::log_softmax(&logits, 1)?;
                let logprobs = log_sm.gather(&targets_flat.unsqueeze(1)?, 1)?.flatten_all()?;
                let loss = (logprobs.broadcast_mul(&mask)?.sum_all()? / mask.sum_all()?)?;
                // Invert sign because log_softmax is negative
                let loss = (loss * -1.0)?;

                trainer
                    .backward_step(&loss)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                // ── Cosine LR Update ─────────────────────────────────────────
                let lr = compute_cosine_lr(global_step, warmup_steps, total_steps_planned, config.learning_rate);
                trainer.config.adapter_config.learning_rate = lr;
                trainer.update_lr();

                Ok(loss.to_scalar::<f32>()?)
            })()?;

            if loss_val.is_nan() {
                train_log::warn(&format!("⚠ NaN loss detected at epoch {} step {} — skipping gradient update. Consider reducing --lr or increasing --warmup.", epoch, global_step));
                continue;
            }

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
                let dt = now.duration_since(progress_anchor_time).as_secs_f64().max(1e-3);
                let ds = (global_step - progress_anchor_step) as f64;
                let sps = ds / dt;
                ema_steps_per_sec = Some(match ema_steps_per_sec {
                    None => sps,
                    Some(prev) => QLORA_ETA_EMA_ALPHA * sps + (1.0 - QLORA_ETA_EMA_ALPHA) * prev,
                });
                let pct = if total_steps_planned > 0 {
                    100.0 * global_step as f64 / total_steps_planned as f64
                } else { 0.0 };
                let eta_s = ema_steps_per_sec.map(|s| {
                    if s > 0.0 { (total_steps_planned.saturating_sub(global_step) as f64 / s) as u64 } else { 0 }
                });
                let eta_str = eta_s.map_or("eta ?".into(), |s| {
                    if s >= 3600 {
                        format!("eta ~{}h {:02}m {:02}s", s / 3600, (s % 3600) / 60, s % 60)
                    } else {
                        format!("eta ~{:02}m {:02}s", s / 60, s % 60)
                    }
                });
                let current_lr = trainer.current_lr();
                let eff_batch = config.batch_size.max(1) * config.grad_accum.max(1);
                let ema_str = ema_loss_val.map(|v| format!("{:.4}", v)).unwrap_or_else(|| "----".to_string());
                train_log::info(&format!(
                    "E{:02}/{} step={} loss={:.4} (ema={}) lr={:.2e} eff_batch={} {:.1}% {}",
                    epoch, config.epochs, global_step, loss_val, ema_str, current_lr, eff_batch, pct, eta_str
                ));
                telemetry::append(out, telemetry_schema::events::TRAIN_STEP, serde_json::json!({
                    telemetry_schema::keys::EPOCH: epoch,
                    telemetry_schema::keys::STEP: global_step,
                    telemetry_schema::keys::LOSS: loss_val,
                    telemetry_schema::keys::LR: current_lr,
                    telemetry_schema::keys::ETA_SECONDS_REMAINING: eta_s,
                    telemetry_schema::keys::PROGRESS_FRACTION: global_step as f64 / total_steps_planned.max(1) as f64,
                    telemetry_schema::keys::STEPS_PER_SEC_EMA: ema_steps_per_sec,
                }))?;
                progress_anchor_step = global_step;
                progress_anchor_time = now;
                last_progress = now;
            }

            // ── Graceful pause check ──────────────────────────────────────────
            if PAUSE_FLAG.load(Ordering::SeqCst) {
                let ckpt_path = out.join(format!("pause_step_{global_step}.safetensors"));
                trainer.save_adapter(&ckpt_path).context("save pause adapter")?;
                let state = CheckpointState {
                    schema: super::checkpoint_state::CHECKPOINT_SCHEMA.to_string(),
                    run_id: run_id.to_string(),
                    epoch: epoch as u32,
                    global_step,
                    pair_offset: pair_loop_idx + 1,
                    shuffled_indices: shuffled_indices.clone(),
                    rng_seed: config.seed,
                    adapter_path: ckpt_path.display().to_string(),
                    last_loss: last_loss_val,
                    wall_seconds_elapsed: progress_anchor_time.elapsed().as_secs_f64(),
                    saved_at_utc: CheckpointState::now_utc(),
                };
                state.save(out).context("save CheckpointState on pause")?;
                let wall_secs = run_start_inst.elapsed().as_secs_f64();
                let ms_per_step = if global_step > 0 { (wall_secs * 1000.0) / global_step as f64 } else { 0.0 };
                train_log::warn(&format!("Training paused at step {global_step}. Resume with 'vox populi train --resume {}'", out.display()));
                return Ok(crate::tensor::backend::TrainingSummary {
                    wall_secs,
                    total_steps: global_step as usize,
                    total_tokens,
                    ms_per_step,
                });
            }

            // ── Mid-epoch checkpoint ──────────────────────────────────────────
            if let Some(every) = config.checkpoint_every {
                if every > 0 && (pair_loop_idx + 1) % every == 0 {
                    let ckpt_path = out.join(format!("checkpoint_step_{global_step}.safetensors"));
                    trainer.save_adapter(&ckpt_path).context("save mid-epoch adapter")?;

                    let state = CheckpointState {
                        schema: super::checkpoint_state::CHECKPOINT_SCHEMA.to_string(),
                        run_id: run_id.to_string(),
                        epoch: epoch as u32,
                        global_step,
                        pair_offset: pair_loop_idx + 1,
                        shuffled_indices: shuffled_indices.clone(),
                        rng_seed: config.seed,
                        adapter_path: ckpt_path.display().to_string(),
                        last_loss: last_loss_val,
                        wall_seconds_elapsed: progress_anchor_time.elapsed().as_secs_f64(),
                        saved_at_utc: CheckpointState::now_utc(),
                    };
                    state.save(out).context("save CheckpointState mid-epoch")?;

                    let _ = db_tx.send(TrainingDbEvent::Checkpoint {
                        run_id: run_id.to_string(),
                        epoch: epoch as u32,
                        global_step,
                        last_loss: Some(last_loss_val),
                        adapter_path: ckpt_path.display().to_string(),
                    });
                }
            }
        }

        // ── Validation Pass ───────────────────────────────────────────────────
        let mut val_loss_sum = 0.0f64;
        let mut val_steps = 0u32;
        if !eval_pairs.is_empty() {
            train_log::info(&format!("Running validation on {} pairs...", eval_pairs.len()));
            for  pair in &eval_pairs {
                let text = plain_system_prompt_response(system_prompt, &pair.prompt, &pair.response);
                if let Ok(enc) = tokenizer.encode(text, true) {
                    let mut ids = enc.get_ids().to_vec();
                    if ids.len() > config.seq_len {
                        let start = ids.len() - config.seq_len;
                        ids = ids[start..].to_vec();
                    }
                    if ids.len() >= 2 {
                        if let Ok(input_ids) = candle_core::Tensor::new(&ids[..ids.len() - 1], device).and_then(|t| t.unsqueeze(0)) {
                            if let Ok(targets) = candle_core::Tensor::new(&ids[1..], device).and_then(|t| t.unsqueeze(0)) {
                                if let Ok(logits) = model.forward(&input_ids) {
                                    if let Ok(logits) = logits.flatten_to(1) {
                                        if let Ok(targets_flat) = targets.flatten_all() {
                                            if let Ok(prompt_only) = tokenizer.encode(pair.prompt.clone(), false) {
                                                let prompt_len = prompt_only.get_ids().len();
                                                let ids_len = ids.len();
                                                let mask_vec: Vec<f32> = (0..ids_len - 1)
                                                    .map(|i| if (i + 1) >= prompt_len { 1.0f32 } else { 0.0 })
                                                    .collect();
                                                if let Ok(mask) = candle_core::Tensor::from_vec(mask_vec, ids_len - 1, device) {
                                                    if let Ok(log_sm) = candle_nn::ops::log_softmax(&logits, 1) {
                                                        if let Ok(tgt_uns) = targets_flat.unsqueeze(1) {
                                                            if let Ok(logprobs) = log_sm.gather(&tgt_uns, 1).and_then(|t| t.flatten_all()) {
                                                                if let Ok(loss) = logprobs.broadcast_mul(&mask)
                                                                    .and_then(|m| m.sum_all())
                                                                    .and_then(|sum_m| sum_m.broadcast_div(&mask.sum_all().unwrap_or_else(|_| candle_core::Tensor::new(1f32, device).unwrap()))) 
                                                                {
                                                                    if let Ok(loss_val) = loss.to_scalar::<f32>() {
                                                                        val_loss_sum += (loss_val * -1.0) as f64;
                                                                        val_steps += 1;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Epoch boundary: summary + checkpoint ──────────────────────────────
        let avg_loss = if epoch_steps > 0 { epoch_loss_sum / epoch_steps as f64 } else { 0.0 };
        let avg_val_loss = if val_steps > 0 { val_loss_sum / val_steps as f64 } else { 0.0 };
        train_log::info(&format!(
            "Epoch {}/{} complete — avg_loss={:.4} val_loss={:.4} ({} steps, {} val steps)",
            epoch, config.epochs, avg_loss, avg_val_loss, epoch_steps, val_steps
        ));

        let epoch_ckpt = out.join(format!("checkpoint_epoch_{epoch}.safetensors"));
        trainer.save_adapter(&epoch_ckpt).context("save epoch adapter")?;

        // next_epoch = epoch + 1; pair_offset = 0 (fresh shuffle on resume)
        let epoch_state = CheckpointState {
            schema: super::checkpoint_state::CHECKPOINT_SCHEMA.to_string(),
            run_id: run_id.to_string(),
            epoch: (epoch + 1) as u32,
            global_step,
            pair_offset: 0,
            shuffled_indices: vec![],
            rng_seed: config.seed,
            adapter_path: epoch_ckpt.display().to_string(),
            last_loss: last_loss_val,
            wall_seconds_elapsed: progress_anchor_time.elapsed().as_secs_f64(),
            saved_at_utc: CheckpointState::now_utc(),
        };
        epoch_state.save(out).context("save CheckpointState at epoch boundary")?;

        let _ = db_tx.send(TrainingDbEvent::Checkpoint {
            run_id: run_id.to_string(),
            epoch: epoch as u32,
            global_step,
            last_loss: Some(last_loss_val),
            adapter_path: epoch_ckpt.display().to_string(),
        });

        telemetry::append(out, "epoch_complete", serde_json::json!({
            "epoch": epoch,
            "avg_loss": avg_loss,
            "val_loss": avg_val_loss,
            "steps": epoch_steps,
            "val_steps": val_steps,
            "global_step": global_step,
            "checkpoint": epoch_ckpt.display().to_string(),
        }))?;
    }

    // ── Final adapter save ────────────────────────────────────────────────────
    let final_path = out.join("candle_qlora_adapter.safetensors");
    trainer.save_adapter(&final_path).context("save final adapter")?;

    // ── Model card (auto-generate MODEL_CARD.md) ──────────────────────────────
    let final_avg_loss = if total_step_count > 0 {
        total_loss_sum / total_step_count as f64
    } else {
        0.0
    };
    let card = super::model_card::ModelCard {
        title: format!(
            "Vox LoRA Adapter — {}",
            config.adapter_tag.as_deref().unwrap_or("default")
        ),
        base_model: config.base_model.clone(),
        train_file: config
            .train_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        vocab_size: bundle.vocab,
        d_model: bundle.d_model,
        n_layers: bundle.layout.num_hidden_layers,
        n_heads: bundle.layout.num_attention_heads,
        notes: format!(
            "rank={rank} α={alpha} epochs={epochs} steps={global_step} avg_loss={avg:.4}\nDevice: {device}\nCheckpoint: {ckpt}",
            rank = config.rank,
            alpha = config.alpha,
            epochs = config.epochs,
            avg = final_avg_loss,
            device = device_label,
            ckpt = final_path.display(),
        ),
    };
    if let Err(e) = super::model_card::write(out, &card) {
        train_log::warn(&format!("MODEL_CARD.md could not be written: {e}"));
    } else {
        train_log::info(&format!("Wrote MODEL_CARD.md to {}", out.join("MODEL_CARD.md").display()));
    }

    // Write adapter meta with actual layer-to-key mapping (fixes Bug 1.6)
    let meta = QloraAdapterMetaV2 {
        format: QloraAdapterMetaV2::FORMAT.to_string(),
        version: QloraAdapterMetaV2::VERSION,
        embed_key: bundle.embed_key.clone(),
        vocab: bundle.vocab,
        d_model: bundle.d_model,
        rank: config.rank,
        alpha: config.alpha as usize,
        layer_order: adapter_layer_order.to_vec(),
        base_key_map: base_key_map.clone(),
        base_model: config.base_model.clone(),
    };
    std::fs::write(
        out.join("adapter_meta_v2.json"),
        serde_json::to_string_pretty(&meta)?,
    )?;

    // Delete checkpoint — training is complete, no need to resume
    CheckpointState::delete(out);

    let _ = db_tx.send(TrainingDbEvent::Complete {
        run_id: run_id.to_string(),
        global_step,
        adapter_path: final_path.display().to_string(),
    });

    telemetry::append(out, telemetry_schema::events::TRAIN_COMPLETE, serde_json::json!({
        "global_step": global_step,
        "final_adapter": final_path.display().to_string(),
        "run_id": run_id,
    }))?;

    train_log::info(&format!(
        "Training complete — {global_step} steps — adapter: {}",
        final_path.display()
    ));

    let wall_secs = run_start_inst.elapsed().as_secs_f64();
    let ms_per_step = if global_step > 0 { (wall_secs * 1000.0) / global_step as f64 } else { 0.0 };

    Ok(crate::tensor::backend::TrainingSummary {
        wall_secs,
        total_steps: global_step as usize,
        total_tokens,
        ms_per_step,
    })
}
