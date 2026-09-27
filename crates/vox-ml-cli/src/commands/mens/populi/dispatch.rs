//! Dispatch for [`PopuliAction`](crate::commands::mens::PopuliAction).

use anyhow::Result;

use super::PopuliAction;

#[cfg(feature = "gpu")]
use super::{
    MensTokenizerCli, OptimizerExperimentModeCli, PopuliTrainBackendCli,
    TrainingDeploymentTargetCli,
};

use crate::commands::mens::bench_completion;
use crate::commands::mens::eval_gate;
use crate::commands::mens::pipeline;
use crate::commands::mens::status;

#[cfg(feature = "gpu")]
use crate::commands::mens::eval_local;
use crate::commands::mens::probe;
#[cfg(feature = "gpu")]
use crate::commands::schola::merge_qlora;

#[cfg(feature = "gpu")]
use std::path::PathBuf;

/// Dispatch `vox mens` subcommands to their feature-gated implementations.
pub async fn run(action: PopuliAction, _global_json: bool, _global_verbose: bool) -> Result<()> {
    match action {
        #[cfg(feature = "mens-base")]
        PopuliAction::Pipeline {
            data_dir,
            output_dir,
            skip_train,
            strict_gate,
            device,
            model,
            epochs,
            preset,
            stages,
            dry_run,
            curriculum,
            profile,
        } => {
            pipeline::run(
                data_dir,
                output_dir,
                skip_train,
                strict_gate,
                device,
                model,
                epochs,
                preset,
                stages,
                dry_run,
                curriculum,
                profile,
            )
            .await
        }
        PopuliAction::TrainUv {
            model: _,
            data_dir: _,
            output_dir: _,
            rank: _,
            alpha: _,
            epochs: _,
        } => {
            anyhow::bail!(
                "`vox mens train-uv` is retired: `quantized_train.py` is not shipped in this repository.\n\
                 Use **`vox mens train --backend qlora --tokenizer hf`** (see docs/src/architecture/mens-training-ssot.md)."
            );
        }
        #[cfg(feature = "gpu")]
        PopuliAction::Dogfood {
            output_dir,
            checkpoint_every,
            force_restart,
        } => {
            let data_dir = PathBuf::from(vox_corpus::training::CANONICAL_TRAIN_DATA_DIR);

            crate::commands::schola::train::run_train(
                PopuliTrainBackendCli::Qlora.into(),
                None,
                "cuda".into(),
                data_dir,
                output_dir,
                None,                         // rank
                None,                         // alpha
                None,                         // seq_len
                None,                         // batch_size
                None,                         // grad_accum
                None,                         // budget_seq_len
                None,                         // budget_batch_size
                None,                         // budget_grad_accum
                None,                         // resume
                None,                         // epochs
                None,                         // lr
                None,                         // warmup
                42,                           // seed
                None,                         // min_rating
                Some("qwen_4080_16g".into()), // preset
                TrainingDeploymentTargetCli::Workstation.into(),
                "normal".into(),                   // process_priority
                None,                              // vram_limit_fraction
                Some("vox_dogfood_gpu_v1".into()), // adapter_tag
                Some(vox_populi::mens::tensor::training_config::ContextFilter {
                    categories: Some(vec!["vox".to_string()]),
                    ..Default::default()
                }), // context_filter
                Some(0.05),                        // validation_split_ratio
                MensTokenizerCli::Hf.into(),
                false, // qlora_no_double_quant
                true,  // qlora_require_full_proxy_stack
                false, // qlora_allow_partial_proxy_stack
                None,  // qlora_max_skip_rate
                false, // qlora_lm_head_only
                None,  // qlora_proxy_max_layers
                0,     // qlora_ce_last_k (whole assistant response)
                Some(checkpoint_every),
                force_restart,
                false, // curriculum (dogfood default: off)
                OptimizerExperimentModeCli::Off.into(),
                true,               // require_gpu
                false,              // allow_cpu_fallback
                None,               // base_model_family
                None,               // upstream_model_id
                None,               // license_class
                false,              // attribution_required
                false,              // trajectory_weighting_enabled
                1.1,                // trajectory_tool_trace_boost
                1.15,               // trajectory_failure_category_boost
                None,               // trajectory_quality_floor
                1.05,               // trajectory_quality_boost
                None,               // curriculum_schedule
                Default::default(), // chatml_config
                None,               // mix_config
            )
            .await?;
            Ok(())
        }
        #[cfg(feature = "gpu")]
        PopuliAction::Train {
            model,
            device,
            backend,
            data_dir,
            output_dir,
            rank,
            alpha,
            seq_len,
            batch_size,
            grad_accum,
            resume,
            epochs,
            lr,
            warmup,
            seed,
            min_rating,
            preset,
            domain,
            deployment_target,
            process_priority,
            vram_limit_fraction,
            background,
            log_dir,
            adapter_tag,
            context_filter,
            tokenizer,
            qlora_no_double_quant,
            qlora_require_full_proxy_stack,
            qlora_allow_partial_proxy_stack,
            qlora_lm_head_only,
            qlora_max_skip_rate,
            qlora_proxy_max_layers,
            qlora_ce_last_k,
            checkpoint_every,
            force_restart,
            gradient_checkpointing,
            no_auto_heal,
            require_gpu,
            allow_cpu_fallback,
            base_model_family,
            upstream_model_id,
            license_class,
            attribution_required,
            trajectory_weighting_enabled,
            trajectory_tool_trace_boost,
            trajectory_failure_category_boost,
            trajectory_quality_floor,
            trajectory_quality_boost,
            cloud,
            max_budget,
            train_data_hf,
            adapter_upload_hf,
            max_runtime_secs,
            validation_split_ratio,
            curriculum,
            optimizer_experiment_mode,
            data_mode,
            fast_corpus,
            persistent,
        } => {
            // The plugin self-heal preflight (run_train.rs, cuda path) reads this
            // env var. Setting it here keeps the opt-out from threading a bool
            // through the ~60-arg train dispatch chain.
            if no_auto_heal {
                // SAFETY: single-threaded CLI startup, before any training threads spawn.
                #[allow(unsafe_code)]
                unsafe {
                    std::env::set_var("VOX_MENS_NO_AUTO_HEAL", "1");
                }
            }
            // Activation/gradient checkpointing flag → env var (read at config build
            // in schola::train::gpu), same pattern as `--no-auto-heal` above to avoid
            // threading another bool through the ~60-arg train dispatch chain.
            if gradient_checkpointing {
                // SAFETY: single-threaded CLI startup, before any training threads spawn.
                #[allow(unsafe_code)]
                unsafe {
                    std::env::set_var("VOX_MENS_GRADIENT_CHECKPOINTING", "1");
                }
            }
            super::train_arm::run_train(
                model,
                device,
                backend,
                data_dir,
                output_dir,
                rank,
                alpha,
                seq_len,
                batch_size,
                grad_accum,
                resume,
                epochs,
                lr,
                warmup,
                seed,
                min_rating,
                preset,
                domain,
                deployment_target,
                process_priority,
                vram_limit_fraction,
                background,
                log_dir,
                adapter_tag,
                context_filter,
                tokenizer,
                qlora_no_double_quant,
                qlora_require_full_proxy_stack,
                qlora_allow_partial_proxy_stack,
                qlora_lm_head_only,
                qlora_max_skip_rate,
                qlora_proxy_max_layers,
                qlora_ce_last_k,
                checkpoint_every,
                force_restart,
                require_gpu,
                allow_cpu_fallback,
                base_model_family,
                upstream_model_id,
                license_class,
                attribution_required,
                trajectory_weighting_enabled,
                trajectory_tool_trace_boost,
                trajectory_failure_category_boost,
                trajectory_quality_floor,
                trajectory_quality_boost,
                cloud,
                max_budget,
                train_data_hf,
                adapter_upload_hf,
                max_runtime_secs,
                validation_split_ratio,
                curriculum,
                optimizer_experiment_mode.into(),
                data_mode,
                fast_corpus,
                persistent,
            )
            .await
        }

        #[cfg(not(feature = "gpu"))]
        PopuliAction::TrainStub { .. }
        | PopuliAction::DogfoodStub { .. }
        | PopuliAction::ServeStub { .. } => {
            // This `gpu` is a vox-ml-cli **cargo feature**, not a runtime
            // plugin. Old wording told users to `vox plugin install
            // tensor-burn-wgpu`, which doesn't fix anything — the binary
            // itself was compiled without the gpu feature, so the dispatch
            // arm above never gets reached. The real fix is to rebuild.
            //
            // Auto-installing a runtime plugin can't help here either: the
            // training code path is `#[cfg]`-gated out of this binary.
            anyhow::bail!(
                "vox mens {{train, dogfood, serve}} requires the `gpu` cargo feature, \
                 which was not enabled when this binary was built.\n\n\
                 To enable, rebuild from the workspace root with:\n\n  \
                 cargo build -p vox-ml-cli --release --features gpu,mens-candle-cuda\n\n\
                 Then re-install the runtime CUDA plugin (catalog or workspace):\n\n  \
                 vox plugin install mens-candle-cuda\n\
                 or, from this workspace:\n  \
                 vox plugin install --path crates/vox-plugin-mens-candle-cuda --yes\n\n\
                 See: docs/src/reference/plugins.md and \
                 docs/src/reference/mens-training.md"
            );
        }

        #[cfg(feature = "gpu")]
        PopuliAction::Serve {
            model,
            port,
            host,
            max_tokens,
            temperature,
            cloud,
            max_budget: _max_budget,
            model_hf: _model_hf,
            max_runtime_secs: _max_runtime_secs,
            persistent: _persistent,
        } => {
            if cloud != "local" {
                #[cfg(feature = "cloud")]
                {
                    use vox_populi::mens::cloud::CloudJobSpec;
                    let config = vox_populi::mens::cloud::CloudProviderConfig::default();
                    let rt = _max_runtime_secs.ok_or_else(|| {
                        anyhow::anyhow!("--max-runtime-secs is REQUIRED for cloud serve")
                    })?;
                    let mut spec = CloudJobSpec::new_serve(&config, rt);
                    spec.model_id = _model_hf.unwrap_or_else(vox_populi::mens::default_model_id);
                    spec.max_budget_usd = _max_budget;
                    spec.serve_port = port;
                    spec.persistent = _persistent;

                    let resolver = vox_populi::mens::cloud::CloudResolver::new_from_env().await?;
                    return resolver.dispatch(spec, &cloud).await;
                }
                #[cfg(not(feature = "cloud"))]
                {
                    anyhow::bail!(
                        "Cloud dispatch requires the 'cloud' feature. Rebuild with: cargo build -p vox-cli --features cloud"
                    );
                }
            }

            let model = model
                .ok_or_else(|| anyhow::anyhow!("--model <path> is required for local serve"))?;

            // Gate: Ensure collateral damage eval has run and passed if we're serving an adapter
            let manifest_path = model.join("training_manifest.json");
            let adapter_path = model.join("candle_qlora_adapter.safetensors");
            if manifest_path.exists() || adapter_path.exists() {
                let report_path = model.join("collateral_damage_report.json");
                if !report_path.exists() {
                    anyhow::bail!(
                        "eval_collateral_damage check not found! Run `vox mens eval-collateral-damage --pre-score <baseline.json> --post-adapter <adapter>` before serving this adapter."
                    );
                }

                let report_raw = std::fs::read_to_string(&report_path)?;
                let report_json: serde_json::Value = serde_json::from_str(&report_raw)?;
                let status = report_json.get("status").and_then(|s| s.as_str());
                if status != Some("pass") {
                    anyhow::bail!(
                        "eval_collateral_damage check FAILED. The adapter degraded performance beyond acceptable thresholds and cannot be served."
                    );
                }

                assert_serve_preconditions(&model)?;
            }

            // Serve via the built-in Axum server, gated behind the execution-api feature.
            // There is no standalone "vox-schola" binary in this workspace to fall back
            // to, so a binary built without execution-api can't serve locally at all.
            #[cfg(feature = "execution-api")]
            {
                let cfg = crate::commands::ai::serve::ServeConfig {
                    model_path: model,
                    port,
                    host,
                    max_tokens,
                    temperature,
                    system_prompt: None,
                };
                // run_serve creates its own Tokio runtime; call it from a blocking thread
                // so it doesn't conflict with the outer async executor.
                #[allow(clippy::needless_return)] // load-bearing without execution-api
                return tokio::task::block_in_place(|| crate::commands::ai::serve::run_serve(&cfg));
            }

            #[cfg(not(feature = "execution-api"))]
            {
                let _ = (model, port, host, max_tokens, temperature);
                anyhow::bail!(
                    "vox mens serve requires the `execution-api` cargo feature, which was \
                     not enabled when this binary was built.\n\n\
                     To enable, rebuild from the workspace root with:\n\n  \
                     cargo build -p vox-ml-cli --release --features gpu,execution-api,mens-candle-cuda\n\n\
                     (swap `mens-candle-cuda` for the ML backend plugin matching this host)."
                );
            }
        }

        PopuliAction::Corpus(action) => crate::commands::corpus::run(action).await,

        #[cfg(feature = "gpu")]
        PopuliAction::Models => crate::commands::mens::models::run_models(_global_verbose),

        PopuliAction::Probe {
            detailed,
            measure,
            sweep,
            model_dir,
            seq_len,
            gradient_checkpointing,
            model,
        } => {
            let v = detailed || _global_verbose;
            if measure {
                probe::run_measure()
            } else if sweep {
                probe::run_sweep(model_dir, seq_len, gradient_checkpointing)
            } else {
                probe::run_probe(v, model, seq_len, gradient_checkpointing).await
            }
        }

        PopuliAction::WatchTelemetry {
            telemetry,
            err_log,
            interval_ms,
        } => crate::commands::mens::watch_telemetry::run(telemetry, err_log, interval_ms),
        PopuliAction::Status {
            run_dir,
            quotas,
            config,
            cloud,
            db,
        } => {
            if cloud {
                #[cfg(feature = "codex")]
                {
                    use owo_colors::OwoColorize;
                    let db = vox_db::VoxDb::connect_default().await?;
                    let summary = db.cloud_cost_summary().await?;

                    println!("\n  {}", "Cloud GPU Dispatch Summary".bold().cyan());
                    println!(
                        "  Jobs:      {}",
                        summary.running_jobs + summary.completed_jobs
                    );
                    println!("  Spent:     ${:.2}", summary.total_spent_usd);
                    println!("  Accruing:  ${:.2}", summary.accrued_usd);
                    println!(
                        "  Efficiency: {:.0} tokens/$",
                        summary.avg_tokens_per_dollar
                    );
                    return Ok(());
                }
                #[cfg(not(feature = "codex"))]
                {
                    anyhow::bail!("Cloud status requires the 'codex' feature (VoxDb access).");
                }
            }
            let _ = _global_json;
            status::run_status(run_dir, _global_json, quotas, config, db).await
        }

        #[cfg(feature = "gpu")]
        PopuliAction::MergeQlora {
            base_shard,
            adapter,
            meta,
            output,
            quantize,
            keep_merged,
            gguf_out,
            llama_cpp,
            license_class,
        } => merge_qlora::run_merge_qlora(
            base_shard,
            adapter,
            meta,
            output,
            quantize,
            keep_merged,
            gguf_out,
            llama_cpp,
            license_class,
        ),

        PopuliAction::ExportGguf { input, output } => {
            anyhow::bail!(export_gguf_not_implemented_message(&input, &output));
        }

        #[cfg(feature = "mens-dei")]
        PopuliAction::Generate {
            prompt,
            output,
            no_validate,
            server_url,
            max_retries,
            output_mode,
            schema,
            context_mode,
            conversation_id,
            queue,
            mode,
        } => {
            if let Some(ref m) = mode {
                // SAFETY: isolated env var for this process; no other threads read it during this block
                #[allow(unsafe_code)]
                unsafe {
                    std::env::set_var("VOX_DEI_MODE_PROFILE", m);
                }
            }
            // Run generate in a dedicated thread with its own runtime to avoid
            // "Cannot drop a runtime in a context where blocking is not allowed" during shutdown.
            let prompt = prompt.clone();
            let output = output.clone();
            let server_url = server_url.clone();
            let output_mode = output_mode.as_deref();
            let schema = schema.as_deref();
            let context_mode = context_mode.clone();
            tokio::task::block_in_place(|| {
                let rt = tokio::runtime::Runtime::new().expect("create runtime for generate");
                rt.block_on(crate::commands::ai::generate::run(
                    &prompt,
                    output,
                    no_validate,
                    server_url.as_deref(),
                    max_retries,
                    output_mode,
                    schema,
                    Some(&context_mode),
                    conversation_id,
                    queue,
                ))
            })
        }

        #[cfg(feature = "mens-dei")]
        PopuliAction::Review {
            targets,
            model,
            format,
            severity,
            free_only,
            diff,
            ci,
            pr_comment,
            diff_base,
            mode,
        } => {
            if let Some(ref m) = mode {
                // SAFETY: main-thread env set before spawning review; no concurrent readers
                #[allow(unsafe_code)]
                unsafe {
                    std::env::set_var("VOX_DEI_MODE_PROFILE", m);
                }
            }
            let mut cmd = tokio::process::Command::new("vox");
            cmd.arg("review");
            for t in &targets {
                cmd.arg(t);
            }
            if let Some(m) = model {
                cmd.arg("--model").arg(m);
            }
            if let Some(f) = format {
                cmd.arg("--format").arg(f);
            }
            if let Some(s) = severity {
                cmd.arg("--severity").arg(s);
            }
            if free_only {
                cmd.arg("--free-only");
            }
            if diff {
                cmd.arg("--diff");
            }
            if ci {
                cmd.arg("--ci");
            }
            if pr_comment {
                cmd.arg("--pr-comment");
            }
            if let Some(db) = diff_base {
                cmd.arg("--diff-base").arg(db);
            }

            let status = cmd.status().await?;
            if !status.success() {
                anyhow::bail!("vox review via subprocess failed");
            }
            Ok(())
        }

        #[cfg(feature = "mens-dei")]
        PopuliAction::Workflow(action) => crate::commands::ai::workflow::run(action).await,

        #[cfg(feature = "mens-dei")]
        PopuliAction::Check { file } => {
            crate::dei_daemon::call(
                crate::dei_daemon::method::AI_CHECK,
                serde_json::json!({
                    "file": file,
                }),
                false,
            )
            .await?;
            Ok(())
        }

        #[cfg(feature = "mens-dei")]
        PopuliAction::Fix { file, errors } => {
            let code = vox_bounded_fs::read_utf8_path_capped(&file)?;
            let errors_val = if let Some(e) = errors {
                e
            } else {
                "".to_string()
            };
            crate::dei_daemon::call(
                crate::dei_daemon::method::AI_FIX,
                serde_json::json!({
                    "code": code,
                    "errors": errors_val,
                }),
                false,
            )
            .await?;
            Ok(())
        }

        #[cfg(feature = "gpu")]
        PopuliAction::EvalLocal {
            model,
            base,
            bench,
            max_tokens,
            temperature,
            samples,
            seed_base,
            output,
        } => eval_local::run_eval_local(
            model,
            base,
            bench,
            max_tokens,
            temperature,
            samples,
            seed_base,
            output,
        ),

        PopuliAction::Hub(hub_action) => crate::commands::mens::hub::run_hub(hub_action).await,

        PopuliAction::MensTail(tail) => match tail {
            super::mens_tail_subcommands::PopuliMensTail::EvalGate { run_dir, policy } => {
                let code = eval_gate::run_eval_gate(run_dir, policy)?;
                std::process::exit(code);
            }

            super::mens_tail_subcommands::PopuliMensTail::Baseline {
                spoke,
                base_eval_dir,
                out,
            } => {
                let created = chrono::Utc::now().to_rfc3339();
                let report = eval_gate::capture_baseline(&base_eval_dir, &spoke, &created, None)?;
                let out_path = out.unwrap_or_else(|| base_eval_dir.join("baseline_report.json"));
                eval_gate::save_baseline(&out_path, &report)?;
                let entry = &report.entries[0];
                println!("Captured baseline → {}", out_path.display());
                println!(
                    "  spoke={} metric={} value={:.4} ci=[{:.4},{:.4}] sample_size={}",
                    entry.spoke,
                    entry.metric_name,
                    entry.value,
                    entry.ci_low,
                    entry.ci_high,
                    entry.sample_size,
                );
                Ok(())
            }

            super::mens_tail_subcommands::PopuliMensTail::EvalCollateralDamage {
                pre_score,
                post_adapter,
            } => {
                #[cfg(not(feature = "gpu"))]
                let _ = (pre_score, post_adapter);

                #[cfg(feature = "gpu")]
                crate::commands::mens::eval_collateral::run_collateral_damage(
                    pre_score,
                    post_adapter,
                )?;
                Ok(())
            }

            super::mens_tail_subcommands::PopuliMensTail::BenchCompletion {
                url,
                count,
                warmup,
            } => bench_completion::run_bench(&url, count, warmup).await,

            super::mens_tail_subcommands::PopuliMensTail::SystemPromptTemplate {
                output,
                format,
            } => crate::commands::mens::system_prompt_template::run(output, &format).await,

            #[cfg(feature = "cloud")]
            super::mens_tail_subcommands::PopuliMensTail::CloudEstimate {
                model_dir,
                target,
                max_budget,
                batch_size,
                seq_len,
                num_samples,
                epochs,
            } => {
                crate::commands::mens::cloud_estimate::run(
                    model_dir,
                    target,
                    max_budget,
                    batch_size,
                    seq_len,
                    num_samples,
                    epochs,
                )
                .await
            }
        },
    }
}

/// `export-gguf` is not a separate step; this message tells the caller which
/// of the two `merge-qlora` routes to use instead. Extracted from the
/// `ExportGguf` dispatch arm so `the_export_gguf_error_only_names_flags_that_exist`
/// below can check it against `merge-qlora`'s real clap definition without
/// invoking the CLI.
fn export_gguf_not_implemented_message(
    input: &std::path::Path,
    output: &std::path::Path,
) -> String {
    format!(
        "`vox mens export-gguf` is not a separate step. Two routes, by base architecture:\n  \
         llama / gemma2 base — let Ollama convert:\n    \
         vox mens merge-qlora --base-shard <base> --adapter <adapter> \\\n      \
           --meta <meta.json> --output <merged.safetensors> \\\n      \
           --quantize q4_k_m --keep-merged\n    \
         cd <merged-parent>/recombined_full && ollama create -q q4_K_M <name> -f Modelfile\n  \
         Qwen3 base (MENS default) — ollama create rejects the architecture; use llama.cpp:\n    \
         vox mens merge-qlora --base-shard <base> --adapter <adapter> \\\n      \
           --meta <meta.json> --output <merged.safetensors> \\\n      \
           --keep-merged --gguf-out <out.gguf> --llama-cpp <llama.cpp checkout>\n\
         Requested: input={} output={}",
        input.display(),
        output.display()
    )
}

/// Refuse to serve a model whose `gate_receipt.json` says `overall_passed: true`
/// while every individual gate inside it actually verified nothing (all
/// "not applicable" / "skipped" / "missing" messages). `eval_gate` has
/// branches that turn a missing artifact into a passing gate; this is the
/// serve-boundary backstop that catches every present and future silent-skip
/// without having to touch each of those branches individually.
#[cfg(feature = "gpu")]
fn assert_serve_preconditions(run_dir: &std::path::Path) -> Result<()> {
    use anyhow::Context;

    let receipt_path = run_dir.join("gate_receipt.json");
    let receipt: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        &receipt_path,
    )
    .with_context(|| {
        format!(
            "no gate_receipt.json in {} — run `vox mens eval-gate --run-dir {} --policy <p>` first",
            run_dir.display(),
            run_dir.display()
        )
    })?)?;
    anyhow::ensure!(
        receipt.get("overall_passed") == Some(&serde_json::Value::Bool(true)),
        "eval-gate did not pass for this run; refusing to serve"
    );
    // A receipt whose gates all said "not applicable"/"skipped"/"missing" is not evidence.
    let substantive = receipt["gates"].as_array().map_or(0, |g| {
        g.iter()
            .filter(|r| {
                let m = r["message"].as_str().unwrap_or("");
                !m.contains("not applicable") && !m.contains("skipped") && !m.contains("missing")
            })
            .count()
    });
    anyhow::ensure!(
        substantive >= 2,
        "gate_receipt.json has {substantive} substantive gates — every check was skipped for a \
         missing artifact. Produce eval_results.json and baseline_report.json, then re-gate."
    );
    Ok(())
}

#[cfg(all(test, feature = "gpu"))]
mod serve_preconditions_tests {
    use super::*;

    #[test]
    fn a_receipt_whose_every_gate_skipped_is_not_evidence() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("gate_receipt.json"),
            r#"{
          "overall_passed": true,
          "gates": [
            {"name":"rust_compile_rate","passed":true,"message":"not applicable (no rust_authoring rows)"},
            {"name":"pass_at_k","passed":true,"message":"baseline file missing (skipped regression check)"}
          ]}"#,
        )
        .unwrap();
        let err = assert_serve_preconditions(dir.path())
            .unwrap_err()
            .to_string();
        assert!(err.contains("0 substantive gates"), "got: {err}");
    }

    #[test]
    fn a_receipt_with_two_real_gates_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("gate_receipt.json"),
            r#"{
          "overall_passed": true,
          "gates": [
            {"name":"throughput","passed":true,"message":"142 tok/s >= 100"},
            {"name":"supervised_ratio","passed":true,"message":"0.71 >= 0.60"}
          ]}"#,
        )
        .unwrap();
        assert!(assert_serve_preconditions(dir.path()).is_ok());
    }

    /// Z1 fixed a ~100x throughput regression (proven floor ~22-25 tok/s on Metal);
    /// this locks in that a run which measured below the real, shipped throughput
    /// floor (`mens/config/eval-gates.yaml`) is refused at the serve boundary —
    /// not just theoretically possible to refuse, but actually refused by the
    /// policy this repo ships. Runs the real `eval-gate` pipeline (not a
    /// hand-authored receipt) so a future change that silently makes the
    /// throughput gate non-blocking again is caught here.
    ///
    /// Every other blocking gate in that policy (pass_at_k, anti_stub,
    /// review_recurrence) is given a fixture that satisfies it, so the only
    /// gate deciding the outcome is throughput.
    #[test]
    fn a_below_floor_throughput_run_is_refused_by_the_real_policy() {
        let dir = tempfile::tempdir().unwrap();

        // Far below the `cpu` floor (1.0 tok/s) in mens/config/eval-gates.yaml —
        // no manifest is written, so device_profile defaults to "cpu".
        std::fs::write(
            dir.path().join("telemetry.jsonl"),
            "{\"tokens_per_sec\": 0.2}\n",
        )
        .unwrap();

        // Satisfy pass_at_k (block: true, needs metrics + baseline file).
        std::fs::write(
            dir.path().join("benchmark_passatk.json"),
            r#"{"pass_rate_at_1":0.30,"pass_rate_at_k":0.60}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("baseline_passatk.json"),
            r#"{"pass_rate_at_1":0.30,"pass_rate_at_k":0.60}"#,
        )
        .unwrap();

        // Satisfy anti_stub (block: true).
        std::fs::write(
            dir.path().join("eval_local_report.json"),
            r#"{"anti_stub_task_success":0.95,"placeholder_event_rate":0.05,
                "trivial_placeholder_event_rate":0.05,"construct_richness_mean":0.25}"#,
        )
        .unwrap();

        // Satisfy review_recurrence (block: true).
        std::fs::write(
            dir.path().join("review_metrics.json"),
            r#"{"repeated_finding_rate":0.10,"post_training_regression_rate":0.05,
                "recurrence_delta":0.02}"#,
        )
        .unwrap();

        let workspace_root = vox_corpus::training::contract::find_workspace_root()
            .expect("test runs inside the vox cargo workspace");
        let policy_path = workspace_root.join("mens/config/eval-gates.yaml");
        assert!(
            policy_path.exists(),
            "expected the shipped policy at {}",
            policy_path.display()
        );

        let exit_code = crate::commands::mens::eval_gate::run_eval_gate(
            dir.path().to_path_buf(),
            Some(policy_path),
        )
        .unwrap();
        assert_eq!(
            exit_code, 1,
            "eval-gate must report failure for a below-floor throughput run"
        );

        let err = assert_serve_preconditions(dir.path())
            .unwrap_err()
            .to_string();
        assert!(err.contains("eval-gate did not pass"), "got: {err}");
    }
}

#[cfg(all(test, feature = "gpu"))]
mod export_gguf_message_tests {
    use super::*;
    use clap::Subcommand;

    /// Guards against the exact drift class Task 2 of this plan already
    /// fixed elsewhere: Task 6's `export-gguf` error message names
    /// `--gguf-out` and `--llama-cpp` as the way forward, and Task 12 added
    /// those flags to `merge-qlora`. If a later change deletes or renames
    /// those flags while leaving this message untouched, the message starts
    /// lying about what `merge-qlora` actually accepts. This test fails the
    /// moment that happens, and passes today.
    #[test]
    fn the_export_gguf_error_only_names_flags_that_exist() {
        let msg = export_gguf_not_implemented_message(
            std::path::Path::new("/tmp/in.safetensors"),
            std::path::Path::new("/tmp/out.gguf"),
        );

        let merge_qlora_cmd = PopuliAction::augment_subcommands(clap::Command::new("populi"))
            .find_subcommand("merge-qlora")
            .expect("merge-qlora subcommand must exist behind the gpu feature")
            .clone();

        for flag in ["--gguf-out", "--llama-cpp"] {
            if msg.contains(flag) {
                let long = &flag[2..];
                assert!(
                    merge_qlora_cmd
                        .get_arguments()
                        .any(|a| a.get_long() == Some(long)),
                    "the error message advertises {flag}, which no longer exists on \
                     `merge-qlora` — Task 12 was cut or renamed its flags, and this \
                     message is now the same kind of lie Task 2 fixed"
                );
            }
        }
    }
}
