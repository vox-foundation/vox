//! `PopuliAction::Train` implementation (corpus preflight + `schola::train`).

use std::path::PathBuf;

use super::action::{MensTokenizerCli, PopuliTrainBackendCli, TrainingDeploymentTargetCli};
use crate::commands::schola::train;

#[allow(clippy::too_many_arguments)]
pub async fn run_train(
    model: Option<String>,
    device: String,
    backend: PopuliTrainBackendCli,
    data_dir: PathBuf,
    output_dir: PathBuf,
    rank: Option<usize>,
    alpha: Option<f32>,
    seq_len: usize,
    batch_size: Option<usize>,
    grad_accum: Option<usize>,
    resume: Option<PathBuf>,
    epochs: Option<usize>,
    lr: Option<f64>,
    warmup: Option<usize>,
    seed: u64,
    min_rating: Option<u8>,
    preset: Option<String>,
    deployment_target: TrainingDeploymentTargetCli,
    process_priority: String,
    vram_limit_fraction: Option<f32>,
    background: bool,
    log_dir: Option<PathBuf>,
    adapter_tag: Option<String>,
    context_filter: Option<String>,
    tokenizer: MensTokenizerCli,
    qlora_no_double_quant: bool,
    qlora_require_full_proxy_stack: bool,
    qlora_lm_head_only: bool,
    qlora_max_skip_rate: Option<f32>,
    qlora_proxy_max_layers: Option<usize>,
    qlora_ce_last_k: usize,
    checkpoint_every: Option<usize>,
    force_restart: bool,
    require_gpu: bool,
    allow_cpu_fallback: bool,
    cloud: String,
    _max_budget: Option<f64>,
    _train_data_hf: Option<String>,
    _adapter_upload_hf: Option<String>,
    _max_runtime_secs: Option<u64>,
    validation_split_ratio: f64,
    curriculum: bool,
) -> anyhow::Result<()> {
    if cloud != "local" {
        #[cfg(feature = "cloud")]
        {
            use vox_populi::mens::cloud::{CloudJobSpec, CloudResolver};
            let config = vox_populi::mens::cloud::CloudProviderConfig::default();
            let mut spec = CloudJobSpec::new_train(&config);
            spec.model_id = model.unwrap_or_else(|| vox_populi::mens::DEFAULT_MODEL_ID.to_string());
            spec.train_data_hf = _train_data_hf;
            spec.adapter_upload_hf = _adapter_upload_hf;
            spec.max_budget_usd = _max_budget;
            spec.max_runtime_secs = _max_runtime_secs;
            spec.preset = preset.clone().unwrap_or_else(|| "auto".to_string());
            spec.seq_len = seq_len;
            spec.batch_size = batch_size.unwrap_or(4);
            spec.epochs = epochs.unwrap_or(3);
            spec.num_samples = 5000;

            let resolver = CloudResolver::new_from_env().await?;
            return resolver.dispatch(spec, &cloud).await;
        }
        #[cfg(not(feature = "cloud"))]
        {
            anyhow::bail!(
                "Cloud dispatch requires the 'cloud' feature. Rebuild with: cargo build -p vox-cli --features cloud"
            );
        }
    }
    let process_priority = if background {
        "low".to_string()
    } else {
        process_priority
    };
    let vram_limit_fraction = if background {
        vram_limit_fraction.or(Some(0.8))
    } else {
        vram_limit_fraction
    };

    // Preflight auto-regen check
    let workspace_root = vox_corpus::training::contract::find_workspace_root();
    if let Some(ref root) = workspace_root {
        use owo_colors::OwoColorize;
        let current_fp = vox_corpus::corpus::preflight::compute_corpus_fingerprint(root);

        let is_fresh = if let Ok(db) = vox_db::VoxDb::connect_default().await {
            db.is_corpus_fresh(&current_fp).await.unwrap_or(false)
        } else {
            let fp_file = vox_corpus::corpus::preflight::fingerprint_cache_path(root);
            vox_corpus::corpus::preflight::corpus_is_fresh(root, &fp_file)
        };

        let skip_regen = std::env::var("VOX_TRAIN_SKIP_CORPUS_MIX")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if !is_fresh && !skip_regen {
            eprintln!(
                "  {} Stale corpus detected (fingerprint: {}). Regenerating...",
                "🔄".cyan(),
                current_fp
            );
            let _ = vox_corpus::corpus::preflight::clean_corpus_targets(root);

            let cfg = vox_corpus::synthetic_gen::SyntheticGenConfig::default();
            let out_path = root.join("mens/data/synthetic.jsonl");
            let mut pairs = 0;
            if let Ok(count) = vox_corpus::synthetic_gen::generate_all(&cfg, &out_path) {
                eprintln!("  {} Regenerated {} synthetic pairs", "✓".green(), count);
                pairs = count;
            }

            eprintln!("  {} Running corpus extraction pipeline...", "🔄".cyan());
            if let Err(e) = crate::commands::mens::pipeline::run(
                data_dir.clone(),
                output_dir.clone(),
                true,  // skip_train
                false, // strict_gate
                None,  // device
                None,  // model
                None,  // epochs
                None,  // preset
                None,  // stages
                false, // dry_run
                false, // curriculum
            )
            .await
            {
                eprintln!("  {} Pipeline error: {}", "⚠️".yellow(), e);
            } else {
                eprintln!("  {} Corpus extraction pipeline completed.", "✓".green());
            }

            let mix_yaml = root.join("mens/config/mix.yaml");
            if mix_yaml.exists() {
                eprintln!("  {} Running corpus mix...", "🔄".cyan());
                if let Err(e) = vox_corpus::corpus::run_mix(&mix_yaml) {
                    eprintln!("  {} Mix failed: {}", "⚠️".yellow(), e);
                } else if let Ok(mix_cfg) = vox_corpus::corpus::MixConfigSchema::load(&mix_yaml) {
                    let mixed_path = root.join(&mix_cfg.output);
                    let final_train_path = data_dir.join("train.jsonl");
                    if mixed_path.exists() {
                        if let Err(e) = std::fs::copy(&mixed_path, &final_train_path) {
                            eprintln!(
                                "  {} Failed to copy mix to {}: {}",
                                "⚠️".yellow(),
                                final_train_path.display(),
                                e
                            );
                        } else {
                            eprintln!(
                                "  {} Mixed data ready at: {}",
                                "✓".green(),
                                final_train_path.display()
                            );
                            #[allow(unsafe_code)]
                            unsafe {
                                std::env::set_var("VOX_TRAIN_SKIP_CORPUS_MIX", "1");
                            }
                        }
                    }
                }
            }

            if let Ok(db) = vox_db::VoxDb::connect_default().await {
                let _ = db
                    .record_corpus_snapshot(
                        &current_fp,
                        env!("CARGO_PKG_VERSION"),
                        pairs as i64,
                        None,
                    )
                    .await;
            } else {
                let fp_file = vox_corpus::corpus::preflight::fingerprint_cache_path(root);
                let _ = vox_corpus::corpus::preflight::write_fingerprint_snapshot(root, &fp_file);
            }
        }
    }

    let context_filter = context_filter.or_else(|| adapter_tag.clone());
    if let Some(ref log_dir) = log_dir {
        return crate::commands::schola::train::spawn_train_with_log(log_dir.clone());
    }
    let deployment_target = if preset.as_deref() == Some("mobile_edge") {
        vox_populi::mens::TrainingDeploymentTarget::MobileEdge
    } else {
        deployment_target.into()
    };
    train::run_train(
        backend.into(),
        model,
        device,
        data_dir,
        output_dir,
        rank,
        alpha,
        Some(seq_len),
        batch_size,
        grad_accum,
        resume,
        epochs,
        lr,
        warmup,
        seed,
        min_rating,
        preset,
        deployment_target,
        process_priority,
        vram_limit_fraction,
        adapter_tag,
        context_filter,
        Some(validation_split_ratio),
        tokenizer.into(),
        qlora_no_double_quant,
        qlora_require_full_proxy_stack,
        qlora_max_skip_rate,
        qlora_lm_head_only,
        qlora_proxy_max_layers,
        qlora_ce_last_k,
        checkpoint_every,
        force_restart,
        curriculum,
        require_gpu,
        allow_cpu_fallback,
    )
    .await
}
