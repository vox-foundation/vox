//! Native QLoRA training: **NF4-quantized** frozen base linears + trainable LoRA via [`qlora_rs`].
//!
//! **Device:** maps Mens `--device` to Candle (CUDA / Metal when enabled, else CPU). Override
//! with `VOX_CANDLE_DEVICE=cpu`. See [`ENV_CANDLE_DEVICE`].
//!
//! ## Training loop properties
//!
//! - **Causal mask**: enforced in [`Qwen2Attention::forward`].
//! - **GQA**: K/V projections use `num_key_value_heads` from `config.json`; weights loaded
//!   with the correct `(kv_dim, d_model)` shape; K/V tensors are repeat-interleaved.
//! - **Warmup + cosine decay**: applied in-loop via `compute_cosine_lr`, synced into
//!   `trainer.config.adapter_config.learning_rate` and `QLoraTrainer::update_lr` (the live field the
//!   optimizer reads; not the stale `AdapterTrainingState` clone).
//! - **Gradient clipping**: `max_grad_norm: Some(1.0)` in `AdapterTrainingConfig`.
//! - **Resume**: loads [`CheckpointState`] on start unless `force_restart` is set.
//! - **VoxDB**: persists run start, per-checkpoint updates, and final status asynchronously.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

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

pub(super) use super::candle_qlora_merge::QloraAdapterMetaV2;
use super::device::DeviceKind;
use super::qlora_preflight::preflight_native_qlora;
use super::train_jsonl_preflight::preflight_train_jsonl;
use super::train_log;
use super::training_config::LoraTrainingConfig;

/// EMA alpha for ETA calculation (0.2 = stable but react within ~5 intervals).
pub(super) const QLORA_ETA_EMA_ALPHA: f64 = 0.2;
/// Environment variable: force Candle to CPU regardless of device flag.
pub const ENV_CANDLE_DEVICE: &str = "VOX_CANDLE_DEVICE";

/// Global flag for graceful interruption (Ctrl+C).
pub(super) static PAUSE_FLAG: AtomicBool = AtomicBool::new(false);

// ── DB message bus ────────────────────────────────────────────────────────────

/// Events sent from the training loop to the background VoxDB writer task.
pub(super) enum TrainingDbEvent {
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

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct TrainingLoopStats {
    pub skip_no_supervised_positions: u64,
    pub skip_short_seq: u64,
    pub skip_curriculum: u64,
}

/// Load LoRA adapter weights (safetensors) into a trainer's varmap (warm-start).
fn load_adapter_into_trainer(trainer: &mut QLoraTrainer, path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("checkpoint adapter not found: {}", path.display());
    }
    trainer
        .load_lora_weights(path)
        .context("warm-start LoRA weights")?;
    train_log::info(&format!(
        "Warm-started LoRA weights from {}",
        path.display()
    ));
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
) -> Result<crate::mens::tensor::backend::TrainingSummary> {
    let out_buf = output_dir.map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("mens/runs").join(config.run_id.as_deref().unwrap_or("v1"))
    });
    std::fs::create_dir_all(&out_buf).context("create output dir")?;
    let out: &Path = out_buf.as_path();

    if config.force_restart
        && let Err(e) = device_select::purge_fresh_start_artifacts(out)
    {
        train_log::warn(&format!(
            "Could not remove previous run artifacts in {}: {e}",
            out.display()
        ));
    }

    let (device, device_label) =
        device_select::select_candle_device(device_kind, config.allow_cpu_fallback)?;
    if config.require_gpu && matches!(device, Device::Cpu) {
        anyhow::bail!(
            "GPU execution was required, but Candle selected CPU device '{}'. \
             Re-run with --device cuda (or metal on macOS) after fixing local accelerator setup.",
            device_label
        );
    }

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
        anyhow::anyhow!("Model preflight failed: {e}. Ensure you have run 'vox mens download --model <name>' and that tokenizer.json + safetensors are present.")
    })?;
    let tokenizer = Tokenizer::from_file(&bundle.tokenizer_path)
        .map_err(|e| anyhow::anyhow!("load tokenizer: {e}"))?;

    // Resolve training data path: config.train_file → data_dir/train.jsonl fallback
    let train_path: PathBuf = config
        .train_file
        .clone()
        .unwrap_or_else(|| data_dir.join("train.jsonl"));
    let _ = preflight_train_jsonl(&train_path, 1_000_000)?;
    let mut pairs = vox_tensor::data::load_all(&train_path, config.min_rating)
        .with_context(|| format!("load training data from {}", train_path.display()))?;
    if let Some(filter) = config.context_filter.as_deref() {
        let needle = filter.trim().to_ascii_lowercase();
        if !needle.is_empty() {
            let before = pairs.len();
            pairs.retain(|p| {
                p.category
                    .as_deref()
                    .map(|c| c.to_ascii_lowercase().contains(&needle))
                    .unwrap_or(false)
            });
            train_log::info(&format!(
                "Applied --context-filter={filter:?}: {} -> {} rows",
                before,
                pairs.len()
            ));
        }
    }
    if pairs.is_empty() {
        anyhow::bail!("No training pairs found in {}", train_path.display());
    }

    let val_count = {
        let pct_count =
            (pairs.len() as f64 * config.validation_split_ratio.unwrap_or(0.05)) as usize;
        // Task 4.2 request: 10 hold-out inputs
        if pairs.len() > 20 {
            pct_count.max(10).min(pairs.len() / 2)
        } else {
            pct_count
        }
    };
    let eval_pairs = if val_count > 0 && pairs.len() > val_count {
        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed ^ 0xA1B2_C3D4_E5F6_1122);
        pairs.shuffle(&mut rng);
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
    let vb_mmap =
        unsafe { VarBuilder::from_mmaped_safetensors(&bundle.weight_paths, DType::F32, &device)? };
    let wte = vb_mmap.get((bundle.vocab, bundle.d_model), &bundle.embed_key)?;

    // ── qlora-rs config ───────────────────────────────────────────────────────
    let rank = config.rank.max(1);
    let alpha_u = config.alpha.round() as usize;
    let qlora_cfg = QLoraConfig::preset_qv_bf16(rank, alpha_u);

    let total_steps_planned = (pairs.len() * config.epochs) as u32;
    let grad_accum = config.grad_accum.max(1) as u32;
    let total_optimizer_steps_planned = total_steps_planned.div_ceil(grad_accum);
    let warmup_steps = config
        .warmup_steps
        .min((total_optimizer_steps_planned / 10).max(1) as usize);

    let train_cfg = QLoraTrainingConfig {
        adapter_config: AdapterTrainingConfig {
            learning_rate: config.learning_rate,
            lr_schedule: LrSchedule::LinearWarmup { warmup_steps },
            weight_decay: 0.01,
            gradient_accumulation_steps: config.grad_accum.max(1),
            max_grad_norm: Some(1.0), // gradient clipping
        },
        num_epochs: config.epochs,
        ..Default::default()
    };

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
            let w_ln1 = vb_mmap
                .get(bundle.d_model, &ln1_key)?
                .to_dtype(DType::F32)?;
            let w_ln2 = vb_mmap
                .get(bundle.d_model, &ln2_key)?
                .to_dtype(DType::F32)?;
            let ln1 = candle_nn::RmsNorm::new(w_ln1, 1e-6);
            let ln2 = candle_nn::RmsNorm::new(w_ln2, 1e-6);

            // ── Attention projections (GQA-correct K/V shapes) ────────────────
            let q_key = format!("model.layers.{i}.self_attn.q_proj.weight");
            let k_key = format!("model.layers.{i}.self_attn.k_proj.weight");
            let v_key = format!("model.layers.{i}.self_attn.v_proj.weight");
            let o_key = format!("model.layers.{i}.self_attn.o_proj.weight");

            let w_q = vb_mmap
                .get((bundle.d_model, bundle.d_model), &q_key)?
                .to_dtype(DType::F32)?;
            let w_k = vb_mmap
                .get((kv_dim, bundle.d_model), &k_key)?
                .to_dtype(DType::F32)?;
            let w_v = vb_mmap
                .get((kv_dim, bundle.d_model), &v_key)?
                .to_dtype(DType::F32)?;
            let w_o = vb_mmap
                .get((bundle.d_model, bundle.d_model), &o_key)?
                .to_dtype(DType::F32)?;

            let q_label = format!("l{i}.q");
            let k_label = format!("l{i}.k");
            let v_label = format!("l{i}.v");
            let o_label = format!("l{i}.o");

            let q_proj = QuantizedLinear::from_weight_with_varbuilder(
                &w_q,
                None,
                &qlora_cfg,
                vb.pp(&q_label),
            )?;
            let k_proj = QuantizedLinear::from_weight_with_varbuilder(
                &w_k,
                None,
                &qlora_cfg,
                vb.pp(&k_label),
            )?;
            let v_proj = QuantizedLinear::from_weight_with_varbuilder(
                &w_v,
                None,
                &qlora_cfg,
                vb.pp(&v_label),
            )?;
            let o_proj = QuantizedLinear::from_weight_with_varbuilder(
                &w_o,
                None,
                &qlora_cfg,
                vb.pp(&o_label),
            )?;

            for (lbl, bk) in [
                (&q_label, &q_key),
                (&k_label, &k_key),
                (&v_label, &v_key),
                (&o_label, &o_key),
            ] {
                adapter_layer_order.push(lbl.clone());
                base_key_map.insert(lbl.clone(), bk.clone());
            }

            // ── MLP projections ───────────────────────────────────────────────
            let inter_sz = bundle
                .layout
                .intermediate_size
                .unwrap_or(bundle.d_model * 4);
            let gate_key = format!("model.layers.{i}.mlp.gate_proj.weight");
            let up_key = format!("model.layers.{i}.mlp.up_proj.weight");
            let down_key = format!("model.layers.{i}.mlp.down_proj.weight");

            let w_gate = vb_mmap
                .get((inter_sz, bundle.d_model), &gate_key)?
                .to_dtype(DType::F32)?;
            let w_up = vb_mmap
                .get((inter_sz, bundle.d_model), &up_key)?
                .to_dtype(DType::F32)?;
            let w_down = vb_mmap
                .get((bundle.d_model, inter_sz), &down_key)?
                .to_dtype(DType::F32)?;

            let gate_label = format!("l{i}.gate");
            let up_label = format!("l{i}.up");
            let down_label = format!("l{i}.down");

            let gate_proj = QuantizedLinear::from_weight_with_varbuilder(
                &w_gate,
                None,
                &qlora_cfg,
                vb.pp(&gate_label),
            )?;
            let up_proj = QuantizedLinear::from_weight_with_varbuilder(
                &w_up,
                None,
                &qlora_cfg,
                vb.pp(&up_label),
            )?;
            let down_proj = QuantizedLinear::from_weight_with_varbuilder(
                &w_down,
                None,
                &qlora_cfg,
                vb.pp(&down_label),
            )?;

            for (lbl, bk) in [
                (&gate_label, &gate_key),
                (&up_label, &up_key),
                (&down_label, &down_key),
            ] {
                adapter_layer_order.push(lbl.clone());
                base_key_map.insert(lbl.clone(), bk.clone());
            }

            // ── RoPE frequency table (optional per-layer) ─────────────────────
            let inv_key = format!("model.layers.{i}.self_attn.rotary_emb.inv_freq");
            let inv_freq = vb_mmap
                .get((head_dim / 2,), &inv_key)
                .ok()
                .and_then(|t| t.to_dtype(DType::F32).ok());

            model_layers.push(crate::mens::tensor::candle_model_qwen::Qwen2Layer {
                input_layernorm: ln1,
                self_attn: crate::mens::tensor::candle_model_qwen::Qwen2Attention {
                    q_proj,
                    k_proj,
                    v_proj,
                    o_proj,
                    n_heads,
                    n_kv_heads,
                    head_dim,
                },
                post_attention_layernorm: ln2,
                mlp: crate::mens::tensor::candle_model_qwen::Qwen2MLP {
                    gate_proj,
                    up_proj,
                    down_proj,
                },
                inv_freq,
            });
        }

        // ── Final norm + LM head (weight-tied to embeddings) ─────────────────
        let fnorm_w = vb_mmap
            .get(bundle.d_model, "model.norm.weight")?
            .to_dtype(DType::F32)?;
        let final_norm = candle_nn::RmsNorm::new(fnorm_w, 1e-6);
        let w_lm = wte.to_dtype(DType::F32)?;
        let lm_label = "lm_head".to_string();
        let lm_base = bundle.embed_key.clone();
        let lm_head = QuantizedLinear::from_weight_with_varbuilder(
            &w_lm,
            None,
            &qlora_cfg,
            vb.pp(&lm_label),
        )?;
        adapter_layer_order.push(lm_label.clone());
        base_key_map.insert(lm_label, lm_base);

        (final_norm, lm_head)
    }; // vb dropped here — VarMap retains trainable LoRA vars

    trainer
        .init_optimizer(&[])
        .context("init qlora optimizer")?;

    let model = crate::mens::tensor::candle_model_qwen::Qwen2Model {
        embed_tokens: wte,
        layers: model_layers,
        norm: final_norm,
        lm_head,
    };

    // ── Async VoxDB writer ────────────────────────────────────────────────────
    let run_id = config.run_id.clone().unwrap_or_else(|| {
        format!(
            "qlora_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        )
    });

    let db_tx = db_thread::spawn_training_db_writer(run_id.clone());

    // Fire off the "start" DB event
    let _ = db_tx.send(TrainingDbEvent::Start {
        run_id: run_id.clone(),
        adapter_tag: config.adapter_tag.clone(),
        model_name: config.base_model.clone(),
        output_dir: out.display().to_string(),
        data_dir: data_dir.display().to_string(),
        planned_steps: Some(total_steps_planned),
    });

    let result = training_loop::run_training_loop(
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
        total_optimizer_steps_planned,
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

mod checkpoint_mid;
mod db_thread;
mod device_select;
mod epoch_boundary;
mod finalize;
mod training_loop;
mod validation;
