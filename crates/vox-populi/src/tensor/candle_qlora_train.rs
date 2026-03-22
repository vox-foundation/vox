//! Native QLoRA training: **NF4-quantized** frozen base linears + trainable LoRA via [`qlora_rs`].
//! When HF shards list every expected block **output projection** (`o_proj` / GPT-2 `c_proj`), we stack
//! those [`QuantizedLinear`] layers **before** the tied LM head and call `QLoraTrainer::training_step_lm`
//! on the full slice (sequential forward). Otherwise **LM head only**. Context embeddings use the mmap
//! `f32` table (`index_select`).
//!
//! **Device:** maps Populi `--device` to Candle (CUDA / Metal when enabled, else CPU). Override
//! with `VOX_CANDLE_DEVICE=cpu`. See [`super::ENV_CANDLE_DEVICE`](super::ENV_CANDLE_DEVICE).

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Context;
use candle_core::{DType, Device, Tensor};
use qlora_rs::QLoraConfig;
use qlora_rs::qlora::QuantizedLinear;
use qlora_rs::training::{QLoraTrainer, QLoraTrainingConfig};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use safetensors::serialize;
use safetensors::tensor::{Dtype, TensorView};
use tokenizers::Tokenizer;
use vox_tensor::data::{TrainingPair, VOCAB_SIZE};

use super::candle_qlora_graph::{adapter_names_for_stack, stacked_lm_logits_shape};
use super::candle_qlora_merge::QloraAdapterMetaV2;
use super::candle_qlora_weights::{
    filter_keys_in_shard, log_key_inventory_from_present, middle_projection_coverage,
    missing_middle_keys_report, ordered_middle_projection_keys, tensor_keys_union,
};
use super::device::{DeviceKind, apply_backend_env, estimate_training_vram_mb_qlora, probe_gpu};
use super::manifest;
use super::model_card;
use super::qlora_preflight::preflight_native_qlora;
use super::telemetry;
use super::telemetry_schema;
use super::train_jsonl_preflight::preflight_train_jsonl;
use super::train_log;
use super::training_config::LoraTrainingConfig;
use super::training_text::plain_system_prompt_response;

/// Environment: force Candle qlora to CPU (`VOX_CANDLE_DEVICE=cpu`).
pub const ENV_CANDLE_DEVICE: &str = "VOX_CANDLE_DEVICE";

#[derive(Debug, Clone)]
struct GpuFallbackReport {
    requested_device_kind: String,
    causes: Vec<String>,
}

struct CandleDeviceSelection {
    device: Device,
    label: &'static str,
    gpu_fallback: Option<GpuFallbackReport>,
}

fn wants_candle_acceleration(kind: DeviceKind) -> bool {
    !matches!(kind, DeviceKind::Cpu)
}

/// EMA weight on **per-interval** steps/sec (recent window vs history).
const QLORA_ETA_EMA_ALPHA: f64 = 0.22;
/// Do not print ETA until this many `global_step`s (warm-up is noisy on CUDA).
const QLORA_ETA_MIN_STEPS: u32 = 24;

/// Upper-bound count of `global_step` increments per epoch if no rows are skipped for
/// vocab / hidden reasons (encode + trim + `qlora_ce_last_k` suffix positions only).
fn count_planned_qlora_steps_per_epoch(
    pairs: &[TrainingPair],
    config: &LoraTrainingConfig,
    tokenizer: &Tokenizer,
    system_prompt: &str,
) -> anyhow::Result<u64> {
    let k_ce = config.qlora_ce_last_k.max(1);
    let mut total = 0u64;
    for pair in pairs {
        let text = plain_system_prompt_response(system_prompt, &pair.prompt, &pair.response);
        let enc = tokenizer
            .encode(text.as_str(), true)
            .map_err(|e| anyhow::anyhow!("tokenizer encode (planned step count): {e}"))?;
        let mut ids: Vec<u32> = enc.get_ids().to_vec();
        if ids.len() > config.seq_len {
            let start = ids.len() - config.seq_len;
            ids = ids[start..].to_vec();
        }
        if ids.len() < 2 {
            continue;
        }
        let l = ids.len();
        let start_t = l.saturating_sub(k_ce).max(1);
        total = total.saturating_add((l - start_t) as u64);
    }
    Ok(total)
}

#[must_use]
fn format_eta_hms(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "?".to_string();
    }
    let s = seconds.round() as u64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{h}h{m}m{sec}s")
    } else if m > 0 {
        format!("{m}m{sec}s")
    } else {
        format!("{sec}s")
    }
}

fn pick_candle_device(device_kind: DeviceKind) -> CandleDeviceSelection {
    if std::env::var(ENV_CANDLE_DEVICE)
        .ok()
        .as_deref()
        .is_some_and(|s| s.eq_ignore_ascii_case("cpu"))
    {
        if wants_candle_acceleration(device_kind) {
            tracing::info!(
                target: "vox_populi_gpu",
                event = "gpu_intentional_cpu",
                component = "candle_qlora",
                reason = "VOX_CANDLE_DEVICE=cpu",
                cli_device = ?device_kind,
            );
        }
        return CandleDeviceSelection {
            device: Device::Cpu,
            label: "cpu (VOX_CANDLE_DEVICE=cpu)",
            gpu_fallback: None,
        };
    }

    if matches!(device_kind, DeviceKind::Cpu) {
        tracing::info!(
            target: "vox_populi_gpu",
            event = "gpu_intentional_cpu",
            component = "candle_qlora",
            reason = "--device_cpu",
        );
        return CandleDeviceSelection {
            device: Device::Cpu,
            label: "cpu",
            gpu_fallback: None,
        };
    }

    let mut causes: Vec<String> = Vec::new();

    #[cfg(feature = "candle-qlora-cuda")]
    {
        match Device::new_cuda(0) {
            Ok(d) => {
                tracing::info!(
                    target: "vox_populi_gpu",
                    event = "gpu_selected",
                    component = "candle_qlora",
                    device = "cuda:0",
                );
                return CandleDeviceSelection {
                    device: d,
                    label: "cuda:0",
                    gpu_fallback: None,
                };
            }
            Err(e) => causes.push(format!("CUDA device 0: {e}")),
        }
    }
    #[cfg(not(feature = "candle-qlora-cuda"))]
    {
        causes.push(
            "build has no `candle-qlora-cuda` (rebuild with `--features populi-candle-cuda` for NVIDIA)"
                .to_string(),
        );
    }

    #[cfg(all(feature = "candle-qlora-metal", target_os = "macos"))]
    {
        match Device::new_metal(0) {
            Ok(d) => {
                tracing::info!(
                    target: "vox_populi_gpu",
                    event = "gpu_selected",
                    component = "candle_qlora",
                    device = "metal:0",
                );
                return CandleDeviceSelection {
                    device: d,
                    label: "metal:0",
                    gpu_fallback: None,
                };
            }
            Err(e) => causes.push(format!("Metal device 0: {e}")),
        }
    }
    #[cfg(all(target_os = "macos", not(feature = "candle-qlora-metal")))]
    {
        causes.push(
            "macOS build has no `candle-qlora-metal` (rebuild with `--features populi-candle-metal`)"
                .to_string(),
        );
    }

    let summary = format!(
        "`--device` requests a GPU-capable stack ({device_kind:?}); using CPU. {}",
        causes.join(" | ")
    );
    let fb = GpuFallbackReport {
        requested_device_kind: format!("{device_kind:?}"),
        causes,
    };
    train_log::gpu_fallback("candle_qlora", &summary);

    CandleDeviceSelection {
        device: Device::Cpu,
        label: "cpu",
        gpu_fallback: Some(fb),
    }
}

fn pair_matches_filter(pair: &TrainingPair, filter: Option<&str>) -> bool {
    let Some(f) = filter.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    let needle = f.to_lowercase();
    pair.category
        .as_deref()
        .map(|c| c.to_lowercase().contains(needle.as_str()))
        .unwrap_or(false)
}

/// Embedding of `ids[token_idx - 1]` predicting `ids[token_idx]` (next-token LM); `[d_model]`.
fn hidden_before_predicted_token(
    wte: &Tensor,
    ids: &[u32],
    token_idx: usize,
) -> candle_core::Result<Tensor> {
    if token_idx == 0 {
        candle_core::bail!("token_idx must be >= 1 for next-token prediction");
    }
    if token_idx >= ids.len() {
        candle_core::bail!("token_idx out of range for ids");
    }
    let dev = wte.device();
    let ctx = &ids[..token_idx];
    let idx = Tensor::new(ctx, dev)?;
    let emb = wte.index_select(&idx, 0)?;
    emb.narrow(0, ctx.len().saturating_sub(1), 1)?.squeeze(0)
}

/// Last token hidden state `[d_model]` from frozen `f32` embedding table.
#[allow(dead_code)] // Used by `#[cfg(test)]` module; training loop uses [`hidden_before_predicted_token`].
fn last_hidden_from_wte(wte: &Tensor, ids: &[u32]) -> candle_core::Result<Tensor> {
    let sl = ids.len();
    if sl < 2 {
        candle_core::bail!("encoded sequence too short (< 2 tokens)");
    }
    hidden_before_predicted_token(wte, ids, sl - 1)
}

fn f32_tensor_to_le_bytes(t: &Tensor) -> candle_core::Result<(Vec<usize>, Vec<u8>)> {
    let shape: Vec<usize> = t.dims().to_vec();
    let v = t.flatten_all()?.to_vec1::<f32>()?;
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    Ok((shape, bytes))
}

struct OwnedF32Tensor {
    name: String,
    shape: Vec<usize>,
    data: Vec<u8>,
}

/// Persist **LoRA-only** weights (A/B) for every stacked `QuantizedLinear` (format v2).
fn save_qlora_adapter_v2(
    layers: &[QuantizedLinear],
    logical_names: &[String],
    path: &Path,
) -> anyhow::Result<()> {
    if layers.len() != logical_names.len() {
        anyhow::bail!(
            "adapter save: {} layers vs {} names",
            layers.len(),
            logical_names.len()
        );
    }
    let mut owned: Vec<OwnedF32Tensor> = Vec::new();
    for (logical, layer) in logical_names.iter().zip(layers.iter()) {
        let (a, b) = layer.lora_weights();
        let (shape_a, bytes_a) = f32_tensor_to_le_bytes(a).context("flatten lora_a")?;
        let (shape_b, bytes_b) = f32_tensor_to_le_bytes(b).context("flatten lora_b")?;
        owned.push(OwnedF32Tensor {
            name: format!("{logical}.lora_a"),
            shape: shape_a,
            data: bytes_a,
        });
        owned.push(OwnedF32Tensor {
            name: format!("{logical}.lora_b"),
            shape: shape_b,
            data: bytes_b,
        });
    }
    let mut map: HashMap<String, TensorView<'_>> = HashMap::new();
    for ot in &owned {
        let view = TensorView::new(Dtype::F32, ot.shape.clone(), ot.data.as_slice())
            .with_context(|| format!("TensorView {}", ot.name))?;
        map.insert(ot.name.clone(), view);
    }
    let payload =
        serialize(&map, &None).map_err(|e| anyhow::anyhow!("safetensors serialize: {e}"))?;
    std::fs::write(path, payload).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// Run Candle + qlora-rs QLoRA training (NF4 base + LoRA on LM head).
pub(super) fn run_candle_qlora_train(
    data_dir: &Path,
    output_dir: Option<&Path>,
    config: &LoraTrainingConfig,
    device_kind: DeviceKind,
    system_prompt: &str,
) -> anyhow::Result<()> {
    apply_backend_env(device_kind);
    let selection = pick_candle_device(device_kind);
    let device = selection.device;
    let device_label = selection.label;
    train_log::info(&format!("Candle qlora compute device: {device_label}"));

    let bundle = preflight_native_qlora(config)?;
    let train_path = config
        .train_file
        .clone()
        .unwrap_or_else(|| data_dir.join("train.jsonl"));
    if !train_path.is_file() {
        anyhow::bail!("training file not found: {}", train_path.display());
    }
    let max_line = config
        .seq_len
        .saturating_mul(4096)
        .clamp(65_536, 16_777_216);
    preflight_train_jsonl(&train_path, max_line)
        .with_context(|| format!("train JSONL preflight {}", train_path.display()))?;
    let out = output_dir.unwrap_or(data_dir).to_path_buf();
    std::fs::create_dir_all(&out)?;

    let key_union = tensor_keys_union(&bundle.weight_paths)
        .context("inventory HF safetensors keys (middle projections)")?;
    let middle_candidates_full = ordered_middle_projection_keys(&bundle.layout);
    let loaded_middle_keys = filter_keys_in_shard(&middle_candidates_full, &key_union);
    let cov = middle_projection_coverage(&bundle.layout, &key_union);
    debug_assert_eq!(
        loaded_middle_keys.len(),
        cov.matched,
        "middle projection coverage must match filter_keys_in_shard"
    );
    let middle_candidates: Vec<String> = match config.qlora_proxy_max_layers {
        None => middle_candidates_full.clone(),
        Some(cap) => middle_candidates_full
            .iter()
            .take(cap.min(middle_candidates_full.len()))
            .cloned()
            .collect(),
    };
    if middle_candidates.len() < middle_candidates_full.len() {
        train_log::info(&format!(
            "Candle QLoRA: proxy stack depth capped — {} / {} middle projection layer(s) (`qlora_proxy_max_layers`)",
            middle_candidates.len(),
            middle_candidates_full.len()
        ));
    }
    log_key_inventory_from_present(&key_union, "vox_populi_candle_qlora");
    if loaded_middle_keys.len() < middle_candidates_full.len() && !middle_candidates_full.is_empty()
    {
        let missing = missing_middle_keys_report(&bundle.layout, &key_union, 24);
        train_log::warn(&format!(
            "Candle QLoRA: shard has {}/{} expected middle projection keys (sample missing: {:?})",
            loaded_middle_keys.len(),
            middle_candidates_full.len(),
            missing
        ));
    }

    if let Some(ref fb) = selection.gpu_fallback {
        telemetry::append(
            &out,
            telemetry_schema::events::GPU_FALLBACK,
            serde_json::json!({
                "component": "candle_qlora",
                "requested_device_kind": fb.requested_device_kind,
                "causes": fb.causes,
            }),
        )?;
    }

    if let Some(frac) = config.max_vram_fraction.filter(|f| *f > 0.0 && *f <= 1.0) {
        let gpu = probe_gpu();
        let est = estimate_training_vram_mb_qlora(
            bundle.d_model,
            1,
            1,
            bundle.vocab.max(VOCAB_SIZE),
            1,
            config.seq_len,
        );
        let budget = (gpu.vram_mb as f64 * frac as f64) as u64;
        if gpu.vram_mb > 0 && est > budget {
            train_log::warn(&format!(
                "max_vram_fraction={frac}: est. ~{est} MB vs budget {budget} MB (heuristic; qlora-rs uses NF4 base)"
            ));
        }
    }

    let loaded = vox_tensor::data::load_all(&train_path, config.min_rating)?;
    tracing::debug!(
        train_file = %train_path.display(),
        min_rating = config.min_rating,
        context_filter = ?config.context_filter,
        pairs_after_rating = loaded.len(),
        "Candle qlora preflight (before context_filter)"
    );
    let mut pairs: Vec<TrainingPair> = loaded
        .into_iter()
        .filter(|p| pair_matches_filter(p, config.context_filter.as_deref()))
        .collect();
    tracing::debug!(
        pairs_after_context_filter = pairs.len(),
        context_filter = ?config.context_filter,
        "Candle qlora after context_filter"
    );
    if pairs.is_empty() {
        anyhow::bail!(
            "no training rows after rating ≥ {} and context filter {:?}",
            config.min_rating,
            config.context_filter
        );
    }

    let tokenizer = Tokenizer::from_file(&bundle.tokenizer_path)
        .map_err(|e| anyhow::anyhow!("load tokenizer {}: {e}", bundle.tokenizer_path.display()))?;

    let planned_steps_per_epoch =
        count_planned_qlora_steps_per_epoch(&pairs, config, &tokenizer, system_prompt)?;
    if planned_steps_per_epoch == 0 {
        train_log::warn(
            "Candle qlora-rs: planned_steps_per_epoch=0 (no tokenized suffix positions); ETA unavailable",
        );
    }
    let planned_steps_total = planned_steps_per_epoch.saturating_mul(config.epochs.max(1) as u64);

    #[allow(unsafe_code)]
    let vb_mmap = unsafe {
        candle_nn::VarBuilder::from_mmaped_safetensors(&bundle.weight_paths, DType::F32, &device)
            .context("mmap HF safetensors for frozen embeddings")?
    };
    let wte = vb_mmap
        .get((bundle.vocab, bundle.d_model), bundle.embed_key.as_str())
        .with_context(|| format!("load tensor {} from safetensors", bundle.embed_key))?;

    let rank = config.rank.max(1);
    let alpha_u = config.alpha.round() as usize;
    let alpha_u = alpha_u.max(1);

    let mut qlora_cfg = QLoraConfig::preset_qv_bf16(rank, alpha_u);
    qlora_cfg.quantization.double_quant = config.qlora_double_quant;
    qlora_cfg
        .validate_for_training()
        .map_err(|e| anyhow::anyhow!("QLoraConfig invalid for training: {e}"))?;

    // Deep o_proj proxy stacks are a supported bounded graph; qlora-rs LM CE can still be stiff on some CUDA stacks.
    // Prefer `--qlora-lm-head-only` (wired through `LoraTrainingConfig`); env remains for ad-hoc runs.
    let lm_head_only_env = std::env::var("VOX_QLORA_LM_HEAD_ONLY")
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);
    let lm_head_only = config.qlora_lm_head_only || lm_head_only_env;
    let use_o_proj_stack = !lm_head_only && cov.complete && !middle_candidates.is_empty();
    if lm_head_only {
        train_log::warn(
            "Candle QLoRA LM-head-only mode — skipping o_proj proxy stack; training tied LM-head QuantizedLinear only (stable CE; see populi-training-ssot.md).",
        );
    }

    let mut train_cfg = QLoraTrainingConfig::default();
    train_cfg.adapter_config.learning_rate = config.learning_rate;
    train_cfg.adapter_config.gradient_accumulation_steps = config.grad_accum.max(1);
    train_cfg.warmup_steps = config.warmup_steps;
    train_cfg.num_epochs = config.epochs;
    train_cfg.batch_size = 1;
    train_cfg.use_paged_optimizer = false;

    let mut trainer = QLoraTrainer::new(train_cfg, device.clone());
    let mut quant_stack: Vec<QuantizedLinear> = Vec::new();

    {
        let vb = trainer.var_builder();
        if use_o_proj_stack {
            train_log::info(&format!(
                "Candle QLoRA: sequential o_proj proxy stack — {} layer(s) + tied LM head (qlora-rs)",
                middle_candidates.len()
            ));
            for (i, key) in middle_candidates.iter().enumerate() {
                let w = vb_mmap
                    .get((bundle.d_model, bundle.d_model), key.as_str())
                    .with_context(|| format!("load stack weight `{key}` from safetensors"))?;
                let w = w
                    .to_device(&device)?
                    .to_dtype(DType::F32)
                    .with_context(|| format!("middle `{key}` to device/dtype"))?;
                let layer = QuantizedLinear::from_weight_with_varbuilder(
                    &w,
                    None,
                    &qlora_cfg,
                    vb.pp(format!("mid{i}")),
                )
                .with_context(|| format!("QuantizedLinear stack layer mid{i} ({key})"))?;
                quant_stack.push(layer);
            }
        } else if !middle_candidates.is_empty() {
            train_log::warn(
                "Candle QLoRA: not all block output-projection tensors are in the weight shards — \
                 using LM-head-only training (see telemetry `middle_projections_loaded`).",
            );
        }

        let w_lm = wte
            .to_device(&device)?
            .to_dtype(DType::F32)
            .context("LM head weight to device/dtype")?;

        let lm_head =
            QuantizedLinear::from_weight_with_varbuilder(&w_lm, None, &qlora_cfg, vb.pp("lm_head"))
                .context("QuantizedLinear::from_weight_with_varbuilder (lm_head)")?;
        quant_stack.push(lm_head);
    }

    let layer_refs: Vec<&QuantizedLinear> = quant_stack.iter().collect();
    trainer
        .init_optimizer(&layer_refs)
        .context("QLoraTrainer::init_optimizer")?;

    let manifest_row = manifest::write_training_manifest(
        &out,
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
                proxy_stack_complete: use_o_proj_stack,
                middle_layers_active: middle_candidates.len(),
                ce_last_k: config.qlora_ce_last_k.max(1),
            },
        ),
    )?;

    telemetry::append(
        &out,
        telemetry_schema::events::TRAIN_START,
        serde_json::json!({
            telemetry_schema::keys::TRAIN_FILE: train_path.display().to_string(),
            telemetry_schema::keys::OUTPUT_DIR: out.display().to_string(),
            telemetry_schema::keys::SEED: config.seed,
            telemetry_schema::keys::GRAD_ACCUM: config.grad_accum.max(1),
            "trainer": "candle_qlora_qlora_rs_nf4_stack",
            telemetry_schema::keys::EXECUTION_KERNEL: "candle_qlora",
            telemetry_schema::keys::TELEMETRY_SCHEMA: telemetry_schema::TELEMETRY_SCHEMA_VERSION,
            telemetry_schema::keys::CANDLE_COMPAT_MODE: true,
            "compute_device": device_label,
            "gpu_fallback": selection.gpu_fallback.as_ref().map(|fb| serde_json::json!({
                "requested_device_kind": fb.requested_device_kind,
                "causes": fb.causes,
            })),
            telemetry_schema::keys::PAIRS_LOADED: pairs.len(),
            "embed_key": bundle.embed_key,
            "hf_config_path": bundle.config_path.display().to_string(),
            "qlora": "nf4_quantized_base + trainable_lora (qlora-rs)",
            "middle_projections_loaded": loaded_middle_keys.len(),
            "middle_projections_expected": middle_candidates_full.len(),
            "middle_projections_stack_active": middle_candidates.len(),
            "qlora_proxy_max_layers": config.qlora_proxy_max_layers,
            "candle_qlora_graph_id": if use_o_proj_stack && !middle_candidates.is_empty() {
                "proxy_stack_v1_residual"
            } else {
                "lm_head_only"
            },
            "candle_qlora_middle_layers_active": middle_candidates.len(),
            "candle_qlora_ce_last_k": config.qlora_ce_last_k.max(1),
            "o_proj_proxy_stack": use_o_proj_stack,
            "trainable_quant_layers": quant_stack.len(),
            "double_quant": config.qlora_double_quant,
            "hf_model_type": bundle.layout.model_type,
            "determinism": {
                "seed": config.seed,
                "pairs_loaded": pairs.len(),
                "train_file": train_path.display().to_string(),
            },
            telemetry_schema::keys::EPOCHS: config.epochs,
            telemetry_schema::keys::PLANNED_STEPS_PER_EPOCH: planned_steps_per_epoch,
            telemetry_schema::keys::PLANNED_STEPS_TOTAL: planned_steps_total,
            "planned_steps_note": "upper bound if no vocab/hidden skips; actual global_step may be lower",
        }),
    )?;

    train_log::info(&format!(
        "Candle qlora-rs start — data={}, out={}, rank={}, alpha={}, seq={}, epochs={}, grad_accum={}, seed={}, planned_steps≈{planned_steps_per_epoch}/epoch × {} = {planned_steps_total} total (upper bound; skips reduce)",
        train_path.display(),
        out.display(),
        config.rank,
        config.alpha,
        config.seq_len,
        config.epochs,
        config.grad_accum.max(1),
        config.seed,
        config.epochs.max(1),
    ));

    let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);
    let training_wall_start = Instant::now();
    let mut global_step: u32 = 0;
    // Heartbeat + ETA: 5s avoids log spam; ETA uses interval EMA (not raw average).
    let progress_every = Duration::from_secs(5);
    let mut last_progress = Instant::now();
    let mut progress_anchor_step = 0u32;
    let mut progress_anchor_time = training_wall_start;
    let mut ema_steps_per_sec: Option<f64> = None;
    let mut steps_executed: u64 = 0;
    let mut skips_bad_vocab: u64 = 0;
    let mut skips_last_hidden: u64 = 0;
    let mut skips_short_seq: u64 = 0;

    for epoch in 1..=config.epochs {
        let mut epoch_visits: u64 = 0;
        let mut epoch_skips: u64 = 0;
        trainer.start_epoch();
        pairs.shuffle(&mut rng);

        'pair: for pair in &pairs {
            epoch_visits += 1;
            let text = plain_system_prompt_response(system_prompt, &pair.prompt, &pair.response);
            let enc = tokenizer
                .encode(text.as_str(), true)
                .map_err(|e| anyhow::anyhow!("tokenizer encode: {e}"))?;
            let mut ids: Vec<u32> = enc.get_ids().to_vec();
            if ids.len() > config.seq_len {
                let start = ids.len() - config.seq_len;
                ids = ids[start..].to_vec();
            }

            if ids.len() < 2 {
                skips_short_seq += 1;
                epoch_skips += 1;
                continue;
            }

            let l = ids.len();
            let k_ce = config.qlora_ce_last_k.max(1);
            let start_t = l.saturating_sub(k_ce).max(1);
            let mut bad_vocab_in_suffix = false;
            for id in &ids[start_t..l] {
                if *id as usize >= bundle.vocab {
                    bad_vocab_in_suffix = true;
                    break;
                }
            }
            if bad_vocab_in_suffix {
                skips_bad_vocab += 1;
                epoch_skips += 1;
                continue;
            }

            let mut hiddens: Vec<Tensor> = Vec::new();
            for ti in start_t..l {
                match hidden_before_predicted_token(&wte, &ids, ti) {
                    Ok(h) => hiddens.push(h),
                    Err(_) => {
                        skips_last_hidden += 1;
                        epoch_skips += 1;
                        continue 'pair;
                    }
                }
            }
            for (i, ti) in (start_t..l).enumerate() {
                let h = &hiddens[i];
                let tid = ids[ti] as usize;
                let inp = h.unsqueeze(0)?.unsqueeze(0)?;
                let tgt = Tensor::new(&[[tid as u32]], &device)?;

                let loss = match trainer.training_step_lm(&layer_refs, &inp, &tgt) {
                    Ok(step_loss) => step_loss,
                    Err(e) => {
                        let es = e.to_string();
                        let low = es.to_lowercase();
                        if low.contains("out of memory") || low.contains("oom") {
                            anyhow::bail!(
                                "Candle QLoRA OOM in training_step_lm: {es}. \
                                 For ~16GB (e.g. RTX 4080): lower --seq-len, use `--preset safe` / `4080_safe` / `qwen_4080_16g`, raise --grad-accum, lower --rank, or set VOX_CANDLE_DEVICE=cpu."
                            );
                        }
                        return Err(anyhow::anyhow!("QLoraTrainer::training_step_lm: {e}"));
                    }
                };

                if !loss.is_finite() {
                    anyhow::bail!(
                        "non-finite loss {loss} at epoch {epoch} after global_step {global_step} (try lower LR or check tokenizer/data)"
                    );
                }

                global_step = global_step.saturating_add(1);
                steps_executed += 1;

                if last_progress.elapsed() >= progress_every {
                    last_progress = Instant::now();
                    let loss_s = train_log::format_loss_for_log(loss);
                    let now = Instant::now();
                    let dt = now
                        .duration_since(progress_anchor_time)
                        .as_secs_f64()
                        .max(1e-3);
                    let ds = global_step.saturating_sub(progress_anchor_step) as f64;
                    let interval_sps = ds / dt;
                    progress_anchor_step = global_step;
                    progress_anchor_time = now;
                    ema_steps_per_sec = Some(match ema_steps_per_sec {
                        None => interval_sps,
                        Some(prev) => {
                            QLORA_ETA_EMA_ALPHA * interval_sps + (1.0 - QLORA_ETA_EMA_ALPHA) * prev
                        }
                    });
                    let eta_suffix = if planned_steps_total > 0 {
                        let pct = 100.0 * global_step as f64 / planned_steps_total as f64;
                        let remaining = planned_steps_total.saturating_sub(global_step as u64);
                        if global_step < QLORA_ETA_MIN_STEPS {
                            format!(
                                " warming up (step {global_step}, {pct:.2}% of planned)"
                            )
                        } else if let Some(sps) = ema_steps_per_sec
                            && sps > 1e-6
                        {
                            let eta_sec = remaining as f64 / sps;
                            let eta_hms = format_eta_hms(eta_sec);
                            let tps = sps * config.seq_len as f64;
                            format!(
                                " ETA≈{eta_hms} • {sps:.2} step/s • ~{tps:.1} tok/s ({pct:.2}%)"
                            )
                        } else {
                            format!(" calibrating… ({pct:.2}% of planned)")
                        }
                    } else {
                        String::new()
                    };
                    
                    // Non-spammy \r output direct to stderr
                    use std::io::Write;
                    let _ = write!(
                        std::io::stderr(),
                        "\r\x1b[2K[Epoch {}/{} Step {}] Loss: {} | Skips: VCB:{} HID:{} SEQ:{} |{}",
                        epoch, config.epochs, global_step, loss_s, skips_bad_vocab, skips_last_hidden, skips_short_seq, eta_suffix
                    );
                    let _ = std::io::stderr().flush();
                }

                if global_step.is_multiple_of(20) {
                    let (eta_val, frac_val, sps_ema_val) =
                        if planned_steps_total > 0 && global_step >= QLORA_ETA_MIN_STEPS {
                            let remaining = planned_steps_total.saturating_sub(global_step as u64);
                            let frac = global_step as f64 / planned_steps_total as f64;
                            if let Some(sps) = ema_steps_per_sec
                                && sps > 1e-6
                            {
                                let eta_sec = remaining as f64 / sps;
                                (
                                    if eta_sec.is_finite() {
                                        serde_json::Value::from(eta_sec)
                                    } else {
                                        serde_json::Value::Null
                                    },
                                    serde_json::Value::from(frac),
                                    serde_json::Value::from(sps),
                                )
                            } else {
                                (
                                    serde_json::Value::Null,
                                    serde_json::Value::from(frac),
                                    serde_json::Value::Null,
                                )
                            }
                        } else {
                            (
                                serde_json::Value::Null,
                                if planned_steps_total > 0 {
                                    serde_json::Value::from(
                                        global_step as f64 / planned_steps_total as f64,
                                    )
                                } else {
                                    serde_json::Value::Null
                                },
                                serde_json::Value::Null,
                            )
                        };
                    let loss_f = loss;
                    let sps_val = ema_steps_per_sec.unwrap_or(0.0);
                    let tps_val = sps_val * config.seq_len as f64;
                    telemetry::append(
                        &out,
                        telemetry_schema::events::TRAIN_STEP,
                        serde_json::json!({
                            "epoch": epoch,
                            telemetry_schema::keys::STEP: global_step,
                            "loss": loss_f,
                            "tokens_per_sec": tps_val,
                            "eta_sec": eta_val,
                            "fraction": frac_val,
                            "sps_ema": sps_ema_val,
                        }),
                    )?;
                }
            }
        }
        if let Some(max_sr) = config.qlora_max_skip_rate
            && epoch_visits > 0
        {
            let sr = epoch_skips as f32 / epoch_visits as f32;
            if sr > max_sr {
                anyhow::bail!(
                    "Candle QLoRA skip rate {sr:.3} exceeds --qlora-max-skip-rate {max_sr:.3} \
                     (epoch {epoch}, visits {epoch_visits}, skips {epoch_skips})."
                );
            }
        }

        train_log::info(&format!("candle qlora-rs epoch {epoch} complete"));
    }

    let wall_seconds = training_wall_start.elapsed().as_secs_f64();
    let mean_steps_per_sec = steps_executed as f64 / wall_seconds.max(1e-6);

    manifest::finalize_candle_qlora_training_manifest(
        &out,
        steps_executed,
        skips_bad_vocab,
        skips_last_hidden,
        skips_short_seq,
        use_o_proj_stack,
    )
    .context("finalize Candle QLoRA training manifest")?;

    let adapter_path = out.join("candle_qlora_adapter.safetensors");
    let n_middle = if use_o_proj_stack {
        middle_candidates.len()
    } else {
        0
    };
    let adapter_logical_names = adapter_names_for_stack(n_middle);
    save_qlora_adapter_v2(&quant_stack, &adapter_logical_names, &adapter_path)?;

    let mut base_key_map: HashMap<String, String> = HashMap::new();
    if use_o_proj_stack {
        for (i, key) in middle_candidates.iter().enumerate() {
            base_key_map.insert(format!("mid{i}"), key.clone());
        }
    }
    base_key_map.insert("lm_head".into(), bundle.embed_key.clone());

    let meta = QloraAdapterMetaV2 {
        format: QloraAdapterMetaV2::FORMAT.to_string(),
        version: QloraAdapterMetaV2::VERSION,
        embed_key: bundle.embed_key.clone(),
        vocab: bundle.vocab,
        d_model: bundle.d_model,
        rank,
        alpha: alpha_u,
        layer_order: adapter_logical_names.clone(),
        base_key_map,
    };

    let meta_path = out.join("candle_qlora_adapter_meta.json");
    std::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).with_context(|| "serialize adapter meta v2")?,
    )
    .with_context(|| format!("write {}", meta_path.display()))?;

    let mut v3 = super::adapter_schema_v3::from_qlora_meta_v2(&meta);
    v3.quant.double_quant = config.qlora_double_quant;
    let v3_path = out.join("populi_adapter_manifest_v3.json");
    std::fs::write(
        &v3_path,
        serde_json::to_string_pretty(&v3).context("serialize adapter manifest v3")?,
    )
    .with_context(|| format!("write {}", v3_path.display()))?;

    model_card::write(
        &out,
        &model_card::ModelCard {
            title: format!("Populi Candle QLoRA (qlora-rs NF4, {device_label})"),
            base_model: config.base_model.clone(),
            train_file: train_path.display().to_string(),
            vocab_size: bundle.vocab,
            d_model: bundle.d_model,
            n_layers: bundle.layout.dims.n_layer,
            n_heads: bundle.layout.dims.n_head,
            notes: format!(
                "Frozen embed key `{}` (f32 mmap for context); stacked NF4 projections + LM head via qlora-rs (ADR 006; bounded proxy v1).\nSuffix CE: `--qlora-ce-last-k` = {} (last K positions per row).\nMiddle stack active: {} / {} model `o_proj` slots; shard keys loaded: {}\nLoRA adapter v2: {}\nSidecar v2: {}\nAdapter manifest v3: {}\nTraining manifest: {}",
                bundle.embed_key,
                config.qlora_ce_last_k.max(1),
                middle_candidates.len(),
                middle_candidates_full.len(),
                loaded_middle_keys.len(),
                adapter_path.display(),
                meta_path.display(),
                v3_path.display(),
                manifest_row.manifest_path.display()
            ),
        },
    )?;

    telemetry::append(
        &out,
        telemetry_schema::events::TRAIN_COMPLETE,
        serde_json::json!({
            "ok": true,
            telemetry_schema::keys::EXECUTION_KERNEL: "candle_qlora",
            "qlora_rs": true,
            "adapter_manifest_v3": v3_path.display().to_string(),
            "training_steps_executed": steps_executed,
            "candle_qlora_ce_last_k": config.qlora_ce_last_k.max(1),
            "skips_bad_vocab": skips_bad_vocab,
            "skips_last_hidden": skips_last_hidden,
            "skips_short_seq": skips_short_seq,
            "pairs_loaded": pairs.len(),
            "seed": config.seed,
            "eval_holdout": "not_run_use_separate_jsonl_or_future_flag",
            "wall_seconds": wall_seconds,
            "mean_steps_per_sec": mean_steps_per_sec,
        }),
    )?;
    train_log::info(&format!(
        "Candle qlora-rs training complete — adapter {} (wall {:.1}s, mean {:.3} step/s)",
        adapter_path.display(),
        wall_seconds,
        mean_steps_per_sec
    ));
    Ok(())
}

#[cfg(all(test, feature = "candle-qlora"))]
mod tests {
    use candle_core::{Device, Tensor};

    use super::{hidden_before_predicted_token, last_hidden_from_wte};

    #[test]
    fn last_hidden_shape_is_d_model() {
        let dev = Device::Cpu;
        let vocab = 12usize;
        let d = 5usize;
        let w: Vec<f32> = (0..vocab * d).map(|i| i as f32 * 0.01).collect();
        let wte = Tensor::from_vec(w, (vocab, d), &dev).unwrap();
        let ids: Vec<u32> = vec![1, 2, 3, 4, 5];
        let h = last_hidden_from_wte(&wte, &ids).unwrap();
        assert_eq!(h.dims(), &[d]);
    }

    #[test]
    fn hidden_before_predicted_token_matches_last_hidden_for_final_index() {
        let dev = Device::Cpu;
        let vocab = 12usize;
        let d = 5usize;
        let w: Vec<f32> = (0..vocab * d).map(|i| i as f32 * 0.01).collect();
        let wte = Tensor::from_vec(w, (vocab, d), &dev).unwrap();
        let ids: Vec<u32> = vec![1, 2, 3, 4, 5];
        let sl = ids.len();
        let a = last_hidden_from_wte(&wte, &ids).unwrap();
        let b = hidden_before_predicted_token(&wte, &ids, sl - 1).unwrap();
        assert_eq!(a.to_vec1::<f32>().unwrap(), b.to_vec1::<f32>().unwrap());
    }
}
