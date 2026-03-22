//! Burn + wgpu LoRA trainer ([`TrainingBackend`] impl).

use std::path::Path;

use burn::backend::Autodiff;
use burn::tensor::{Int, Tensor, TensorData};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use tokenizers::Tokenizer;
use vox_tensor::data::{TrainingPair, VOCAB_SIZE, VoxTokenizer};
use vox_tensor::optim::{AdamW, LinearWarmupScheduler};
use vox_tensor::tensor::Tensor as Vt;
use vox_tensor::train::{Checkpoint, gradient_clip_norm};
use vox_tensor::vox_nn::cross_entropy_loss;

use super::backend::TrainingBackend;
use super::burn_hf_load::{try_load_gpt2_decoder_weights, try_load_token_embeddings};
use super::hf_load::{HfArchitecture, HfTransformerLayout};
use super::lora::LoraVoxTransformer;
use super::manifest;
use super::model_card;
use super::telemetry;
use super::telemetry_schema;
use super::train_log;
use super::training_config::PopuliTokenizerMode;
use super::training_text::hf_tokenize_chatml_supervised;
use crate::tensor::device::{
    DeviceKind, apply_backend_env, estimate_training_vram_mb, make_wgpu_device, probe_gpu,
};
use crate::tensor::training_config::LoraTrainingConfig;

type WgpuB = burn::backend::Wgpu;
type TrainBackend = Autodiff<WgpuB>;

/// Burn + wgpu LoRA on `VoxTokenizer` JSONL.
#[derive(Debug, Clone, Copy, Default)]
pub struct BurnLoraBackend;

fn sanitize_adapter_tag(tag: &str) -> String {
    tag.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
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

fn forward_loss(
    model: &LoraVoxTransformer<TrainBackend>,
    device: &burn::backend::wgpu::WgpuDevice,
    input_ids: &[i64],
    labels: &[i64],
) -> anyhow::Result<Tensor<TrainBackend, 1>> {
    let seq_len = input_ids.len();
    if seq_len == 0 {
        anyhow::bail!("empty sequence");
    }

    let input_tensor = Tensor::<TrainBackend, 2, Int>::from_data(
        TensorData::new(
            input_ids.iter().map(|&x| x as i32).collect::<Vec<_>>(),
            [1, seq_len],
        ),
        device,
    );

    let logits = model.forward(input_tensor);
    let [_, s, v] = logits.dims();

    let labels_len = labels.len();
    if labels_len != s {
        anyhow::bail!(
            "labels length {} != logits sequence dim {} (tokenization/model mismatch)",
            labels_len,
            s
        );
    }

    let mut loss_sum = Tensor::<TrainBackend, 1>::zeros([1], device);
    let mut count = 0f32;
    for (si, &lab) in labels.iter().enumerate() {
        if lab < 0 {
            continue;
        }
        let row = logits
            .clone()
            .slice([0..1, si..si + 1, 0..v])
            .reshape([1, v]);
        let t = Tensor::<TrainBackend, 1, Int>::from_data(
            TensorData::new(vec![lab as i32], [1]),
            device,
        );
        let ce = cross_entropy_loss(&Vt::D2(row), &Vt::D1Int(t));
        let Vt::D1(ten) = ce else {
            anyhow::bail!("unexpected loss tensor shape");
        };
        loss_sum = loss_sum + ten;
        count += 1.0;
    }
    if count < 1.0 {
        anyhow::bail!("no supervised tokens in sequence");
    }
    let scale = Tensor::<TrainBackend, 1>::from_floats([1.0 / count], device);
    let loss = loss_sum * scale;
    Ok(loss.mean())
}

impl TrainingBackend for BurnLoraBackend {
    fn run(
        &self,
        data_dir: &Path,
        output_dir: Option<&Path>,
        config: &LoraTrainingConfig,
        device_kind: DeviceKind,
        system_prompt: &str,
    ) -> anyhow::Result<()> {
        apply_backend_env(device_kind);

        if config.batch_size != 1 {
            train_log::warn(
                "batch_size > 1 is not used in the native LoRA loop (one sequence per forward). \
                 Effective throughput is batch_size × grad_accum in the CLI profile; only grad_accum scales optimizer steps.",
            );
        }

        let device = make_wgpu_device();
        tracing::info!(
            target: "vox_populi_gpu",
            event = "burn_wgpu_device",
            device = ?device,
            cli_device = ?device_kind,
            "Burn LoRA wgpu device (wgpu may pick integrated GPU or CPU backend per platform; see Burn/wgpu logs if performance is poor)"
        );
        train_log::info(&format!(
            "Burn LoRA wgpu device: {device:?} (cli `--device` {device_kind:?})"
        ));
        let train_path = config
            .train_file
            .clone()
            .unwrap_or_else(|| data_dir.join("train.jsonl"));
        if !train_path.is_file() {
            anyhow::bail!("training file not found: {}", train_path.display());
        }

        let out = output_dir.unwrap_or(data_dir).to_path_buf();
        std::fs::create_dir_all(&out)?;

        let gpu = probe_gpu();

        train_log::info(&format!(
            "LoRA train start — data={}, out={}, rank={}, alpha={}, seq={}, epochs={}, grad_accum={}, seed={}",
            train_path.display(),
            out.display(),
            config.rank,
            config.alpha,
            config.seq_len,
            config.epochs,
            config.grad_accum.max(1),
            config.seed
        ));

        let use_hf_tok = matches!(config.tokenizer_mode, PopuliTokenizerMode::Hf);
        let hf_tokenizer: Option<Tokenizer> = if use_hf_tok {
            let Some(ref tp) = config.tokenizer_path else {
                anyhow::bail!("HF tokenizer path missing (expected `tokenizer.json`).");
            };
            Some(
                Tokenizer::from_file(tp)
                    .map_err(|e| anyhow::anyhow!("load tokenizer {}: {e}", tp.display()))?,
            )
        } else if config.base_model_paths.is_some() {
            train_log::warn(
                "HF safetensors paths are recorded for provenance only when using Vox tokenizer; \
                 use `--tokenizer hf` to train with HF ChatML supervision + optional embed warm-start.",
            );
            None
        } else {
            None
        };

        let loaded = vox_tensor::data::load_all(&train_path, config.min_rating)?;
        tracing::debug!(
            train_file = %train_path.display(),
            min_rating = config.min_rating,
            context_filter = ?config.context_filter,
            pairs_after_rating = loaded.len(),
            "LoRA training data preflight (before context_filter)"
        );
        let mut pairs: Vec<TrainingPair> = loaded
            .into_iter()
            .filter(|p| pair_matches_filter(p, config.context_filter.as_deref()))
            .collect();
        tracing::debug!(
            pairs_after_context_filter = pairs.len(),
            context_filter = ?config.context_filter,
            "LoRA training data after context_filter"
        );
        if pairs.is_empty() {
            anyhow::bail!(
                "no training rows after rating ≥ {} and context filter {:?}",
                config.min_rating,
                config.context_filter
            );
        }

        let (vocab_size, d_model, n_heads, n_layers) = if use_hf_tok {
            let Some((_, ref cfg_path)) = config.base_model_paths else {
                anyhow::bail!(
                    "Burn + HF tokenizer requires downloaded HF weights (`--model`) for config.json."
                );
            };
            let layout = HfTransformerLayout::from_config_path(cfg_path)?;
            let d = &layout.dims;
            train_log::info(&format!(
                "Burn HF mode: vocab={}, d_model={}, n_heads={}, n_layers={} (from {})",
                d.vocab_size,
                d.n_embd,
                d.n_head,
                d.n_layer,
                cfg_path.display()
            ));
            (d.vocab_size, d.n_embd, d.n_head, d.n_layer)
        } else {
            (
                VOCAB_SIZE,
                super::lora::DEFAULT_D_MODEL,
                super::lora::DEFAULT_N_HEADS,
                super::lora::DEFAULT_N_LAYERS,
            )
        };

        if let Some(frac) = config.max_vram_fraction.filter(|f| *f > 0.0 && *f <= 1.0) {
            let est = estimate_training_vram_mb(
                d_model,
                n_heads,
                n_layers,
                vocab_size,
                config.batch_size.max(1),
                config.seq_len,
            );
            let budget = (gpu.vram_mb as f64 * frac as f64) as u64;
            if gpu.vram_mb > 0 && est > budget {
                train_log::warn(&format!(
                    "max_vram_fraction={frac}: est. training footprint ~{est} MB exceeds {budget} MB ({} MB × {frac}). Consider --preset safe or lower seq/batch.",
                    gpu.vram_mb
                ));
            }
        }

        let mut model = LoraVoxTransformer::<TrainBackend>::new(
            &device,
            vocab_size,
            d_model,
            n_heads,
            n_layers,
            config.rank,
            config.alpha,
        );

        if use_hf_tok && let Some((ref shards, ref cfg_path)) = config.base_model_paths {
            match try_load_token_embeddings(&mut model, shards, &device) {
                Ok(true) => {}
                Ok(false) => train_log::warn(
                    "HF embed warm-start: no matching wte/embed_tokens tensor in shards; training from random embeddings.",
                ),
                Err(e) => train_log::warn(&format!(
                    "HF embed warm-start failed (continuing from init): {e}"
                )),
            }
            match HfTransformerLayout::from_config_path(cfg_path) {
                Ok(layout) if layout.architecture == HfArchitecture::Gpt2 => {
                    match try_load_gpt2_decoder_weights(&mut model, shards, &layout, &device) {
                        Ok(rep) => {
                            if rep.layers_complete == 0
                                && !rep.pos_embedding
                                && !rep.final_norm
                                && !rep.lm_head
                            {
                                train_log::warn(
                                    "HF GPT-2 decoder warm-start: no decoder tensors applied (missing keys or shape mismatch).",
                                );
                            } else {
                                train_log::info(&format!(
                                    "HF GPT-2 decoder warm-start: {} layer(s) (attn+mlp+norms), wpe={}, ln_f={}, lm_head={}",
                                    rep.layers_complete,
                                    rep.pos_embedding,
                                    rep.final_norm,
                                    rep.lm_head
                                ));
                            }
                        }
                        Err(e) => train_log::warn(&format!(
                            "HF GPT-2 decoder warm-start failed (continuing): {e}"
                        )),
                    }
                }
                Ok(_) => {}
                Err(e) => train_log::warn(&format!(
                    "HF layout re-parse for decoder warm-start skipped: {e}"
                )),
            }
        }

        if let Some(ref resume) = config.resume_from {
            let p = resume.join("latest.bin");
            if p.is_file() {
                model = Checkpoint::load(model, &p).map_err(|e| anyhow::anyhow!("{e:?}"))?;
                train_log::info(&format!("Resumed from {}", p.display()));
            }
        }

        let mut optim = AdamW::new();
        let mut scheduler =
            LinearWarmupScheduler::new(1e-7, config.learning_rate, config.warmup_steps.max(1));

        let manifest_row = manifest::write_training_manifest(
            &out,
            manifest::initial_training_manifest(
                manifest::ArchParams {
                    vocab_size,
                    d_model,
                    n_heads,
                    n_layers,
                },
                train_path.display().to_string(),
                manifest::InitialManifestRun::from_lora_config(config),
                config
                    .tokenizer_path
                    .as_ref()
                    .map(|p| p.display().to_string()),
                manifest::InitialTrainingKernel::BurnLora,
            ),
        )?;

        let tag_suffix = config
            .adapter_tag
            .as_deref()
            .map(sanitize_adapter_tag)
            .filter(|s| !s.is_empty())
            .map(|s| format!("_{s}"))
            .unwrap_or_default();

        telemetry::append(
            &out,
            telemetry_schema::events::TRAIN_START,
            serde_json::json!({
                telemetry_schema::keys::TRAIN_FILE: train_path.display().to_string(),
                telemetry_schema::keys::OUTPUT_DIR: out.display().to_string(),
                telemetry_schema::keys::SEED: config.seed,
                telemetry_schema::keys::GRAD_ACCUM: config.grad_accum.max(1),
                "trainer": "native_lora_burn_wgpu",
                telemetry_schema::keys::EXECUTION_KERNEL: "burn_lora",
                telemetry_schema::keys::TELEMETRY_SCHEMA: telemetry_schema::TELEMETRY_SCHEMA_VERSION,
                "tokenizer_mode": format!("{:?}", config.tokenizer_mode),
                telemetry_schema::keys::PAIRS_LOADED: pairs.len(),
                "hf_embed_warm_start": use_hf_tok,
                "notes": if use_hf_tok {
                    "Burn LoRA + HF tokenizer (ChatML-supervised); optional HF embed warm-start. Not QLoRA/NF4."
                } else {
                    "Native LoRA on VoxTokenizer — not HF QLoRA."
                },
            }),
        )?;

        let ga = config.grad_accum.max(1);
        let inv_ga = Tensor::<TrainBackend, 1>::from_floats([1.0f32 / ga as f32], &device);
        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);

        for epoch in 1..=config.epochs {
            pairs.shuffle(&mut rng);
            let mut total_loss = 0f32;
            let mut opt_steps = 0u32;
            let mut micro_in_group = 0usize;
            let mut group_loss: Option<Tensor<TrainBackend, 1>> = None;

            for pair in &pairs {
                let (input_ids, labels) = if let Some(ref tok) = hf_tokenizer {
                    match hf_tokenize_chatml_supervised(
                        tok,
                        system_prompt,
                        &pair.prompt,
                        &pair.response,
                        config.seq_len,
                    ) {
                        Ok(x) => x,
                        Err(_) => continue,
                    }
                } else {
                    VoxTokenizer::tokenize_for_training(
                        system_prompt,
                        &pair.prompt,
                        &pair.response,
                        config.seq_len,
                    )
                };

                let loss_scalar = match forward_loss(&model, &device, &input_ids, &labels) {
                    Ok(l) => l,
                    Err(_) => continue,
                };

                let loss_val = loss_scalar
                    .clone()
                    .into_data()
                    .as_slice::<f32>()
                    .map(|s| s[0])
                    .unwrap_or(0.0);
                total_loss += loss_val;

                let scaled = loss_scalar * inv_ga.clone();
                group_loss = Some(match group_loss {
                    None => scaled,
                    Some(g) => g + scaled,
                });
                micro_in_group += 1;

                if micro_in_group >= ga {
                    let lr = scheduler.step();
                    let loss_step = group_loss.take().expect("group loss");
                    let loss_step_val = loss_step
                        .clone()
                        .into_data()
                        .as_slice::<f32>()
                        .map(|s| s[0])
                        .unwrap_or(0.0);
                    let mut grads = loss_step.backward();
                    gradient_clip_norm::<TrainBackend>(&mut grads, 1.0);
                    model = optim.step(lr, model, grads);
                    opt_steps += 1;
                    micro_in_group = 0;

                    if opt_steps.is_multiple_of(20) {
                        train_log::info(&format!(
                            "epoch {epoch} step {opt_steps} loss={loss_step_val:.4} lr={lr:.2e}"
                        ));
                        telemetry::append(
                            &out,
                            "step",
                            serde_json::json!({"epoch": epoch, "step": opt_steps, "loss": loss_step_val, "lr": lr}),
                        )?;
                    }
                }
            }

            if micro_in_group > 0 {
                let lr = scheduler.step();
                let partial = group_loss.take().expect("partial group");
                let rescale = Tensor::<TrainBackend, 1>::from_floats(
                    [ga as f32 / micro_in_group as f32],
                    &device,
                );
                let loss_step = partial * rescale;
                let loss_step_val = loss_step
                    .clone()
                    .into_data()
                    .as_slice::<f32>()
                    .map(|s| s[0])
                    .unwrap_or(0.0);
                let mut grads = loss_step.backward();
                gradient_clip_norm::<TrainBackend>(&mut grads, 1.0);
                model = optim.step(lr, model, grads);
                opt_steps += 1;
                let _ = loss_step_val;
            }

            let avg = total_loss / (pairs.len().max(1) as f32);
            train_log::info(&format!(
                "epoch {epoch} done — avg_micro_loss={avg:.6}, optimizer_steps={opt_steps}"
            ));
            let ckpt = out.join(format!("checkpoint_epoch_{epoch}{tag_suffix}.bin"));
            Checkpoint::save(&model, &ckpt).map_err(|e| anyhow::anyhow!("{e:?}"))?;
        }

        let final_p = out.join("model_final.bin");
        Checkpoint::save(&model, &final_p).map_err(|e| anyhow::anyhow!("{e:?}"))?;

        model_card::write(
            &out,
            &model_card::ModelCard {
                title: "Populi native LoRA checkpoint".into(),
                base_model: config.base_model.clone(),
                train_file: train_path.display().to_string(),
                vocab_size,
                d_model,
                n_layers,
                n_heads,
                notes: format!(
                    "manifest checksum reference: {}\nHF tokenizer path: {:?}\nadapter_tag: {:?}",
                    manifest_row.manifest_path.display(),
                    config.tokenizer_path,
                    config.adapter_tag
                ),
            },
        )?;

        telemetry::append(
            &out,
            telemetry_schema::events::TRAIN_COMPLETE,
            serde_json::json!({
                "ok": true,
                telemetry_schema::keys::EXECUTION_KERNEL: "burn_lora",
            }),
        )?;
        train_log::info(&format!("Training complete — {}", final_p.display()));
        Ok(())
    }
}
