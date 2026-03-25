use super::{PipelineProgress, PipelineStage};
use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;

/// Run the dogfood pipeline: corpus extract → validate → pairs → eval → optional native train.
pub async fn run(
    data_dir: PathBuf,
    output_dir: PathBuf,
    skip_train: bool,
    strict_gate: bool,
    device: Option<String>,
    model: Option<String>,
    epochs: Option<usize>,
    preset: Option<String>,
    stages: Option<String>,
    dry_run: bool,
    curriculum: bool,
) -> Result<()> {
    #[cfg(not(feature = "gpu"))]
    {
        let _ = (
            strict_gate,
            device.as_ref(),
            model.as_ref(),
            epochs,
            preset.as_ref(),
            curriculum,
        );
    }

    let run_id = vox_corpus::training::timestamp_string();

    let all_possible_stages = [
        PipelineStage::Generate,
        PipelineStage::Extract,
        PipelineStage::Replay,
        PipelineStage::Validate,
        PipelineStage::Pairs,
        PipelineStage::Eval,
        PipelineStage::Mix,
        PipelineStage::Train,
    ];

    let mut planned_stages = Vec::new();
    if let Some(s) = stages {
        let requested: HashSet<String> = s.split(',').map(|x| x.trim().to_lowercase()).collect();
        for stage in all_possible_stages {
            if requested.contains(stage.as_str()) {
                if stage == PipelineStage::Train && skip_train {
                    continue;
                }
                planned_stages.push(stage);
            }
        }
    } else {
        for stage in all_possible_stages {
            if stage == PipelineStage::Train && skip_train {
                continue;
            }
            planned_stages.push(stage);
        }
    }

    let total_stages = planned_stages.len();
    let validated = PathBuf::from("mens/data/validated.jsonl");
    let train_jsonl = data_dir.join("train.jsonl");
    let eval_out = output_dir.join("eval_results.json");

    tracing::info!(
        run_id = %run_id,
        stages = ?planned_stages,
        dry_run,
        "mens pipeline: start"
    );

    if !dry_run {
        if let Some(p) = validated.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&output_dir)?;
        std::fs::create_dir_all("mens/data/mix_sources")?;
    }

    for (completed_stages, stage) in planned_stages.into_iter().enumerate() {
        let progress = PipelineProgress {
            run_id: run_id.clone(),
            current_stage: stage,
            total_stages,
            completed_stages,
            progress_pct: (completed_stages as f64 / total_stages as f64) * 100.0,
        };

        // Report progress to telemetry/logs
        tracing::info!(
            stage = stage.as_str(),
            progress = %format!("{:.0}%", progress.progress_pct),
            "--- Pipeline Stage: {} ---",
            stage.as_str().to_uppercase()
        );

        match stage {
            PipelineStage::Generate => {
                if !dry_run {
                    crate::commands::corpus::run(crate::commands::corpus::CorpusAction::Generate {
                        output: PathBuf::from("mens/data/synthetic.jsonl"),
                        force_regen: false,
                        dry_run: false,
                    })
                    .await?;
                }
            }
            PipelineStage::Extract => {
                if !dry_run {
                    // Extract from .vox examples
                    let examples_dir = PathBuf::from("examples");
                    if examples_dir.is_dir() {
                        let _ = crate::commands::corpus::run(
                            crate::commands::corpus::CorpusAction::Extract {
                                dir: examples_dir,
                                output: validated.clone(),
                            },
                        )
                        .await;
                    }

                    // Extract from Rust source
                    let crates_dir = PathBuf::from("crates");
                    if crates_dir.is_dir() {
                        let _ = crate::commands::corpus::run(
                            crate::commands::corpus::CorpusAction::ExtractRs {
                                dir: crates_dir,
                                output: PathBuf::from("mens/data/mix_sources/rust_source.jsonl"),
                            },
                        )
                        .await;
                    }

                    // Extract from documentation
                    let docs_dir = PathBuf::from("docs/src");
                    if docs_dir.is_dir() {
                        let _ = crate::commands::corpus::run(
                            crate::commands::corpus::CorpusAction::ExtractDocs {
                                dir: docs_dir,
                                output: PathBuf::from("mens/data/mix_sources/docs.jsonl"),
                            },
                        )
                        .await;
                    }
                }
            }
            PipelineStage::Validate => {
                if !dry_run {
                    if validated.is_file() {
                        crate::commands::corpus::run(
                            crate::commands::corpus::CorpusAction::Validate {
                                input: validated.clone(),
                                output: Some(validated.clone()),
                                no_recheck: true,
                            },
                        )
                        .await?;
                    }
                }
            }
            PipelineStage::Replay => {
                if !dry_run {
                    crate::commands::corpus::run(crate::commands::corpus::CorpusAction::Replay {
                        chatml: true,
                        min_score: 4.0, // High quality only for auto-replay
                        output: PathBuf::from("mens/data/mix_sources/autofeedback.jsonl"),
                        limit: 1000,
                    })
                    .await?;
                }
            }
            PipelineStage::Pairs => {
                if !dry_run {
                    if validated.is_file() {
                        crate::commands::corpus::run(
                            crate::commands::corpus::CorpusAction::Pairs {
                                input: validated.clone(),
                                output: train_jsonl.clone(),
                                docs: Some(PathBuf::from("docs/src")),
                            },
                        )
                        .await?;
                    }
                }
            }
            PipelineStage::Eval => {
                if !dry_run {
                    if train_jsonl.is_file() {
                        crate::commands::corpus::run(crate::commands::corpus::CorpusAction::Eval {
                            input: train_jsonl.clone(),
                            output: eval_out.clone(),
                            print_summary: false,
                        })
                        .await?;
                    }
                }
            }
            PipelineStage::Mix => {
                if !dry_run {
                    let mix_config = PathBuf::from("mens/config/mix.yaml");
                    if mix_config.is_file() {
                        crate::commands::corpus::run(crate::commands::corpus::CorpusAction::Mix {
                            config: mix_config,
                        })
                        .await?;
                    }
                }
            }
            PipelineStage::Train => {
                if !dry_run {
                    #[cfg(feature = "gpu")]
                    {
                        let device = device.clone().unwrap_or_else(|| "best".into());
                        let target_model = model
                            .clone()
                            .unwrap_or_else(|| "Qwen/Qwen2.5-Coder-3B-Instruct".into());
                        let target_preset = preset.clone().or_else(|| Some("qwen_4080_16g".into()));

                        // SAFETY: CLI process; no concurrent `getenv` readers rely on these during this block.
                        #[allow(unsafe_code)]
                        unsafe {
                            std::env::set_var("VOX_BENCHMARK", "1");
                            if strict_gate {
                                std::env::set_var("VOX_EVAL_STRICT", "1");
                                std::env::set_var("VOX_BENCHMARK_MIN_PASS_RATE", "0.80");
                            } else {
                                std::env::set_var("VOX_EVAL_STRICT", "0");
                                std::env::set_var("VOX_BENCHMARK_MIN_PASS_RATE", "0.0");
                            }
                        }

                        crate::commands::schola::train::run_train(
                            crate::commands::mens::PopuliTrainBackendCli::Qlora.into(),
                            Some(target_model),
                            device,
                            data_dir.clone(),
                            output_dir.clone(),
                            None, // rank (auto from preset)
                            None, // alpha
                            None, // seq_len
                            None, // batch_size
                            None, // grad_accum
                            None, // resume
                            epochs,
                            None, // lr
                            None, // warmup
                            42,   // seed
                            None, // min_rating
                            target_preset,
                            vox_mens::TrainingDeploymentTarget::Workstation,
                            "normal".into(),
                            None, // vram_limit_fraction
                            None, // adapter_tag
                            None, // context_filter
                            None, // validation_split_ratio (use default 5%)
                            crate::commands::mens::MensTokenizerCli::Hf.into(),
                            false, // qlora_no_double_quant
                            false, // qlora_require_full_proxy_stack
                            None,  // qlora_max_skip_rate
                            false, // qlora_lm_head_only
                            None,  // qlora_proxy_max_layers
                            64,    // qlora_ce_last_k
                            None,  // checkpoint_every
                            false, // force_restart
                            curriculum,
                            false, // require_gpu
                            true,  // allow_cpu_fallback
                        )
                        .await?;
                    }

                    #[cfg(not(feature = "gpu"))]
                    {
                        anyhow::bail!(
                            "mens pipeline: native train was requested but this `vox` binary was built without the `gpu` feature; pass `--skip-train` or rebuild with `--features gpu`"
                        );
                    }
                }
            }
        }
    }

    tracing::info!(
        run_id = %run_id,
        "mens pipeline: complete"
    );

    Ok(())
}
