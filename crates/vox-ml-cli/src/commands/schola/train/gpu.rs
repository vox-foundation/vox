//! GPU-enabled training path (`vox mens train` with `gpu` feature).

use anyhow::Result;
use std::path::PathBuf;

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_gpu_training(
    train_backend: vox_populi::mens::PopuliTrainBackend,
    model: Option<String>,
    device: String,
    data_dir: PathBuf,
    output_dir: PathBuf,
    resume: Option<PathBuf>,
    preset: Option<String>,
    device_profile: vox_populi::mens::DeviceProfile,
    cli_overrides: vox_populi::mens::CliOverrides,
    gpu_info: vox_populi::mens::GpuInfo,
    device_kind: vox_populi::mens::DeviceKind,
    min_rating: Option<u8>,
    deployment_target: vox_populi::mens::TrainingDeploymentTarget,
    tokenizer_mode: vox_populi::mens::MensTokenizerMode,
    qlora_no_double_quant: bool,
    qlora_require_full_proxy_stack: bool,
    qlora_max_skip_rate: Option<f32>,
    qlora_lm_head_only: bool,
    qlora_proxy_max_layers: Option<usize>,
    qlora_ce_last_k: usize,
    checkpoint_every: Option<usize>,
    force_restart: bool,
    curriculum: bool,
    optimizer_experiment_mode: vox_populi::mens::OptimizerExperimentMode,
    require_gpu: bool,
    allow_cpu_fallback: bool,
    base_model_family: Option<String>,
    upstream_model_id: Option<String>,
    license_class: Option<String>,
    attribution_required: bool,
    trajectory_weighting_enabled: bool,
    trajectory_tool_trace_boost: f32,
    trajectory_failure_category_boost: f32,
    trajectory_quality_floor: Option<u8>,
    trajectory_quality_boost: f32,
    vram_limit_fraction: Option<f32>,
    adapter_tag: Option<String>,
    context_filter: Option<vox_populi::mens::tensor::training_config::ContextFilter>,
    validation_split_ratio: Option<f64>,
    seed: u64,
    curriculum_schedule: Option<vox_populi::mens::tensor::training_config::CurriculumSchedule>,
    chatml: vox_populi::mens::tensor::training_config::ChatmlConfig,
    mix_config: Option<PathBuf>,
) -> Result<()> {
    use owo_colors::OwoColorize;

    let workspace_root = vox_corpus::training::contract::find_workspace_root();
    let skip_mix = vox_corpus::training::mix_prepare::corpus_mix_skip_from_env();
    if skip_mix {
        eprintln!(
            "  {} Skipping corpus mix (`VOX_TRAIN_SKIP_CORPUS_MIX`); using train file under data-dir",
            "⏭".cyan()
        );
    }
    let mix_path = mix_config.unwrap_or_else(|| {
        vox_corpus::training::mix_prepare::resolve_mix_config_path(workspace_root.as_deref())
    });
    if !skip_mix && mix_path.is_file() {
        eprintln!(
            "  {} Running corpus mix to refresh training data...",
            "🔄".cyan()
        );
    }
    let contract_override =
        vox_corpus::training::mix_prepare::refresh_train_contract_override_from_mix(
            workspace_root.as_deref(),
            &data_dir,
            skip_mix,
            true,
            Some(&mix_path),
        )?;

    let resolved = vox_corpus::training::preflight::validate_train_preflight(
        &data_dir,
        contract_override.as_deref(),
        workspace_root.as_deref(),
    )?;
    tracing::debug!(path = %resolved.path.display(), source = ?resolved.source, "Preflight resolved train input");

    // The old VRAM-tiered `auto_preset` lookup that used to pick `final_preset`
    // here was deleted (Task 8: it had no 27B rung and silently handed a 27B
    // model the 14B preset). Leaving `final_preset` unset for CUDA with no
    // explicit `--preset` is not a behavior change: `resolve_effective_profile`
    // already resolves an omitted CUDA preset to `DEFAULT_PRESET` ("4080")
    // regardless, so this was only ever a cosmetic early log line naming a
    // preset resolve_effective_profile would then independently reproduce.
    let final_preset = preset.clone();
    if final_preset.is_none() && device.to_lowercase() == "cuda" {
        eprintln!(
            "  {} {}",
            "⚙".cyan(),
            vox_populi::mens::tensor::vram_autodetect::vram_summary(true)
        );
    }

    // Captured before `cli_overrides` is moved into `resolve_effective_profile`
    // below: Step 4's "no sizing flags" case for the real `sweep`-based
    // auto-sizing default (see `auto_size_from_model_dir` further down) —
    // neither `--batch-size` nor `--seq-len` (nor a domain profile pin, which
    // also fills these fields) was supplied.
    let no_explicit_sizing = cli_overrides.seq_len.is_none() && cli_overrides.batch_size.is_none();

    let profile = vox_populi::mens::resolve_effective_profile(
        final_preset.as_deref(),
        device_profile,
        resolved.sample_count,
        cli_overrides,
    )?;
    let rank = profile.rank;
    let alpha = profile.alpha;
    let mut seq_len = profile.seq_len;
    if matches!(
        train_backend,
        vox_populi::mens::PopuliTrainBackend::CandleQlora
    ) {
        let k = qlora_ce_last_k;
        if k > 0 && k > 64 {
            anyhow::bail!("--qlora-ce-last-k must be at most 64 (got {k})");
        }
        if k > seq_len {
            anyhow::bail!(
                "--qlora-ce-last-k ({k}) cannot exceed effective sequence length ({seq_len})"
            );
        }
    }
    let mut batch_size = profile.batch_size;
    let grad_accum = profile.grad_accum;
    let epochs = profile.epochs;
    let warmup = profile.warmup;
    let lr = profile.lr;
    if !trajectory_tool_trace_boost.is_finite() || trajectory_tool_trace_boost < 0.0 {
        anyhow::bail!(
            "--trajectory-tool-trace-boost must be finite and non-negative (got {trajectory_tool_trace_boost})"
        );
    }
    if !trajectory_failure_category_boost.is_finite() || trajectory_failure_category_boost < 0.0 {
        anyhow::bail!(
            "--trajectory-failure-category-boost must be finite and non-negative (got {trajectory_failure_category_boost})"
        );
    }
    if !trajectory_quality_boost.is_finite() || trajectory_quality_boost < 0.0 {
        anyhow::bail!(
            "--trajectory-quality-boost must be finite and non-negative (got {trajectory_quality_boost})"
        );
    }
    if let Some(q) = trajectory_quality_floor
        && !(1..=5).contains(&q)
    {
        anyhow::bail!("--trajectory-quality-floor must be between 1 and 5 (got {q})");
    }

    // Activation/gradient checkpointing default policy:
    //   * explicit `--gradient-checkpointing` (→ env var) always wins;
    //   * otherwise auto-enable for ~3B models, which OOM the single-backward peak
    //     on a 16GB GPU without it (1.5B fits fine without checkpointing).
    // Computed here (rather than after download, where it used to live) so the
    // auto-sizing sweep below — which needs a `CalKey` including this flag —
    // can use the same value the final `LoraTrainingConfig` uses.
    let gc_explicit = std::env::var("VOX_MENS_GRADIENT_CHECKPOINTING")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let gc_auto_large = model
        .as_deref()
        .and_then(|m| vox_populi::mens::tensor::memory_budget::params_b_from_model_hint(m))
        .map(|b| b >= 2.9)
        .unwrap_or(false);
    let gradient_checkpointing = gc_explicit || gc_auto_large;
    if gradient_checkpointing {
        tracing::info!(
            explicit = gc_explicit,
            auto_large = gc_auto_large,
            "activation/gradient checkpointing ENABLED for this run"
        );
    }

    let mut base_model_paths = None::<(Vec<std::path::PathBuf>, std::path::PathBuf)>;
    let mut tokenizer_path = None::<std::path::PathBuf>;
    if let Some(ref repo_id) = model {
        eprintln!(
            "  {} Downloading base model from Hugging Face: {}",
            "📥".cyan(),
            repo_id
        );
        let files = vox_populi::mens::hub::download_model_blocking(repo_id).map_err(|e| {
            anyhow::anyhow!(
                "HF download failed for `{repo_id}` ({e}). \
                 Set HF token env vars if this is a gated repo and retry."
            )
        })?;
        if !files.is_safetensors() {
            anyhow::bail!(
                "HF model `{repo_id}` has no safetensors; QLoRA requires safetensors base weights."
            );
        }
        base_model_paths = Some((files.weights.clone(), files.config.clone()));
        tokenizer_path = files.tokenizer.clone();
        eprintln!("  {} Cached at {}", "✓".green(), files.cache_dir.display());

        // Step 4's "no sizing flags" default: now that the model is on disk,
        // a real `ModelShape` exists, so the live `sweep`/`plan_for` entry
        // point (the same one `vox mens probe --sweep` calls) can pick the
        // largest batch size that actually fits this host — not the
        // Qwen-family/generic ladder's params_b-based estimate. Explicit
        // `--batch-size`/`--seq-len` are gated, never adjusted, so this only
        // runs when neither was supplied.
        if no_explicit_sizing {
            match auto_size_from_model_dir(&files.cache_dir, seq_len as u64, gradient_checkpointing)
            {
                Ok(AutoSizeOutcome::Sized(picked)) => {
                    eprintln!(
                        "  {} Auto-sized via sweep: batch_size {} → {} at seq_len {} \
                         (largest shape that fits this host's real memory budget).",
                        "📐".cyan(),
                        batch_size,
                        picked.batch_size,
                        picked.seq_len
                    );
                    batch_size = picked.batch_size as usize;
                    seq_len = picked.seq_len as usize;
                }
                Ok(AutoSizeOutcome::NoMeasurement(reason)) => {
                    // No calibration row for this lane (e.g. candle-metal has
                    // none yet) — there is no measured basis to accept or
                    // refuse this config, so this is NOT the same as a
                    // measured refusal below. Warn and fall back to the
                    // preset's own seq_len/batch_size.
                    eprintln!(
                        "  {} Auto-sizing via sweep unavailable ({reason}); using preset \
                         sizing instead (batch_size={batch_size}, seq_len={seq_len}). This lane \
                         is uncalibrated, so nothing here has verified this config fits — \
                         measure it with `vox mens probe --measure`.",
                        "⚠".yellow()
                    );
                }
                Ok(AutoSizeOutcome::Refused(reason)) => {
                    // The lane IS calibrated and plan_for measured that even
                    // batch_size=1 does not fit. This is a REAL refusal, not
                    // an absence of data — never silently proceed into an
                    // OOM (this is the exact failure class this program
                    // exists to fix). Mirrors the old train_arm.rs
                    // `budget_gate`'s VOX_MENS_FORCE_TRAIN escape hatch.
                    if force_train_env() {
                        eprintln!(
                            "  {} VOX_MENS_FORCE_TRAIN=1 — proceeding despite a measured VRAM \
                             refusal: {reason}",
                            "⚠".yellow()
                        );
                    } else {
                        anyhow::bail!(
                            "model does not fit in the detected VRAM at batch_size={batch_size}, \
                             seq_len={seq_len}: {reason}\n\
                             reduce --seq-len/--batch-size, pick a smaller model, or set \
                             VOX_MENS_FORCE_TRAIN=1 to proceed anyway."
                        );
                    }
                }
                Err(e) => {
                    // Could not even attempt a measurement (accelerator probe
                    // failed, ModelShape unreadable, ...) — same "no basis to
                    // refuse" bucket as NoMeasurement above, not a measured
                    // refusal.
                    eprintln!(
                        "  {} Auto-sizing via sweep unavailable ({e}); using preset/VRAM-budget \
                         sizing instead (batch_size={batch_size}, seq_len={seq_len}).",
                        "⚠".yellow()
                    );
                }
            }
        }

        if let Ok(arch) = vox_populi::mens::tensor::hf_load::detect_hf_architecture(&files.config) {
            eprintln!("  {} Architecture: {:?}", "📐".cyan(), arch);
            let cfg = vox_populi::mens::tensor::hf_load::config_dims_for_architecture(
                &files.config,
                arch,
            )
            .map_err(|e| anyhow::anyhow!("HF config: {}", e))?;
            let tokenizer_src = tokenizer_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Vox (built-in)".to_string());
            eprintln!("  {} Tokenizer: {}", "🔤".cyan(), tokenizer_src);
            let est_mb = if matches!(
                train_backend,
                vox_populi::mens::PopuliTrainBackend::CandleQlora
            ) {
                // `batch_size`/`seq_len` here (not `profile.*`) so this estimate
                // reflects the post-sweep sizing when auto-sizing ran above.
                vox_populi::mens::estimate_training_vram_mb_qlora(
                    cfg.n_embd,
                    cfg.n_head,
                    cfg.n_layer,
                    cfg.vocab_size,
                    batch_size,
                    seq_len,
                )
            } else {
                vox_populi::mens::estimate_training_vram_mb(
                    cfg.n_embd,
                    cfg.n_head,
                    cfg.n_layer,
                    cfg.vocab_size,
                    batch_size,
                    seq_len,
                )
            };
            if gpu_info.vram_mb > 0 && est_mb as f64 > gpu_info.vram_mb as f64 * 0.85 {
                eprintln!(
                    "  {} VRAM risk: est. {} MB > 85% of {} MB. Try --batch-size 2 --seq-len 256 or VOX_TRAIN_PROFILE=safe",
                    "⚠".yellow(),
                    est_mb,
                    gpu_info.vram_mb
                );
            } else if gpu_info.vram_mb > 0 {
                eprintln!(
                    "  {} VRAM: est. ~{} MB / {} MB available",
                    "✓".green(),
                    est_mb,
                    gpu_info.vram_mb
                );
            }
        }
    }

    let train_file_path =
        vox_corpus::training::mix_prepare::recover_train_input_path_after_prefetch(
            workspace_root.as_deref(),
            &data_dir,
            &mix_path,
            skip_mix,
            &resolved.path,
        )?;

    let run_id = vox_corpus::training::timestamp_string();
    let git_sha = option_env!("VOX_GIT_HASH").unwrap_or("unknown").to_string();
    let device_profile_str = if device_kind == vox_populi::mens::DeviceKind::Cpu {
        "cpu".to_string()
    } else {
        gpu_info.model_name.clone()
    };

    let config = vox_populi::mens::LoraTrainingConfig {
        base_model: model,
        base_model_family,
        upstream_model_id,
        license_class,
        attribution_required,
        base_model_paths,
        tokenizer_path,
        train_file: Some(train_file_path),
        rank,
        alpha,
        seq_len,
        batch_size,
        grad_accum,
        resume_from: resume,
        epochs,
        learning_rate: lr,
        warmup_steps: warmup,
        seed,
        min_rating: min_rating.unwrap_or(3),
        run_id: Some(run_id),
        git_sha: Some(git_sha),
        device_profile: Some(device_profile_str.clone()),
        max_vram_fraction: vram_limit_fraction,
        adapter_tag,
        context_filter,
        validation_split_ratio,
        tokenizer_mode,
        qlora_double_quant: !qlora_no_double_quant,
        finetune_contract_digest: None,
        qlora_require_full_proxy_stack,
        qlora_max_skip_rate,
        qlora_lm_head_only,
        qlora_proxy_max_layers,
        qlora_ce_last_k,
        checkpoint_every,
        force_restart,
        deployment_target,
        curriculum,
        optimizer_experiment_mode,
        trajectory_weighting_enabled,
        trajectory_tool_trace_boost,
        trajectory_failure_category_boost,
        trajectory_quality_floor,
        trajectory_quality_boost,
        require_gpu,
        allow_cpu_fallback,
        curriculum_schedule,
        chatml,
        reward_hook: None,
        launch_argv: std::env::args().collect(),
        gradient_checkpointing,
    };
    let model_name_for_stats = config
        .base_model
        .clone()
        .unwrap_or_else(|| "scratch".to_string());
    let preset_for_stats = preset.clone().unwrap_or_else(|| "unknown".to_string());

    let system_prompt = vox_corpus::training::generate_training_system_prompt();

    let summary = vox_populi::mens::run_mens_training(
        train_backend,
        &data_dir,
        Some(&output_dir),
        &config,
        device_kind,
        &system_prompt,
    )?;

    if let Ok(db) = vox_db::VoxDb::connect_default().await {
        let _ = db
            .local_log_train_run(
                &device_profile_str,
                &model_name_for_stats,
                &preset_for_stats,
                summary.wall_secs,
                summary.total_steps as i64,
                summary.total_tokens as i64,
                Some(summary.ms_per_step),
            )
            .await;
    }

    Ok(())
}

/// Outcome of attempting to auto-size against the real per-host memory
/// model. Distinguishes two genuinely different cases the caller must NOT
/// treat the same way:
///
/// - [`Self::NoMeasurement`]: the lane has no calibration row (e.g.
///   `candle-metal` today) or the attempt could not even be made (no
///   accelerator budget, unreadable `ModelShape`, ...). There is no
///   measured basis to accept or refuse this config — warn and fall back.
/// - [`Self::Refused`]: the lane IS calibrated and `plan_for` measured that
///   this config does not fit, even at the smallest batch size. This is a
///   REAL, measured refusal, not an absence of data.
///
/// Flattening both into one warn-and-proceed branch was a real regression
/// (fix round 1 of Task 8): it silently dropped the old `train_arm.rs`
/// `budget_gate`'s refusal-with-`VOX_MENS_FORCE_TRAIN`-override contract for
/// the (now much more common) case of a calibrated lane whose measured
/// prediction genuinely exceeds the usable budget.
#[derive(Debug)]
enum AutoSizeOutcome {
    Sized(vox_populi::mens::tensor::memory_model::Request),
    NoMeasurement(String),
    Refused(String),
}

/// `vox mens train`'s no-sizing-flags default (Step 4): the largest batch
/// size that fits this host's REAL memory budget for the model now sitting
/// at `model_dir`, via the same `memory_model::sweep`/`plan_for` entry point
/// `vox mens probe --sweep` calls — not a second, parallel sizing algorithm.
/// Queries the real accelerator (`accel_budget::query_accel_budget`) rather
/// than trusting the CLI's `--device` intent, since that can be `Best`.
/// Returns `Ok(AutoSizeOutcome::NoMeasurement(_))` — never an `Err` — for an
/// uncalibrated lane or an unattemptable measurement, so the caller can fall
/// back to preset sizing without treating that the same as a measured
/// refusal (see [`AutoSizeOutcome`]). `Err` is reserved for genuinely
/// exceptional setup failures the caller should still surface.
fn auto_size_from_model_dir(
    model_dir: &std::path::Path,
    seq_len: u64,
    gradient_checkpointing: bool,
) -> Result<AutoSizeOutcome> {
    use vox_populi::mens::tensor::accel_budget::{BudgetSource, query_accel_budget};
    use vox_populi::mens::tensor::memory_model::{DeviceBudget, Lane, MemoryModels};

    let Some(accel) = query_accel_budget() else {
        return Ok(AutoSizeOutcome::NoMeasurement(
            "no accelerator budget available on this host".to_string(),
        ));
    };
    let lane = match accel.source {
        BudgetSource::Metal => Lane::CandleMetal,
        BudgetSource::Cuda => Lane::CandleCuda,
    };
    let budget = DeviceBudget {
        working_set_bytes: accel.working_set_bytes,
        operator_fraction: None,
    };
    let models = MemoryModels::load_default()?;
    auto_size_with_budget(
        model_dir,
        seq_len,
        gradient_checkpointing,
        &budget,
        lane,
        &models,
    )
}

/// The pure decision `auto_size_from_model_dir` delegates to, with the
/// hardware query and contract load already resolved by the caller — split
/// out so this can be exercised without live hardware, while still calling
/// the real `sweep`/`MemoryModels::get`, never a reimplementation of them.
///
/// Checks calibration presence itself (via `models.get`) BEFORE calling
/// `sweep`, specifically so an uncalibrated lane can be told apart from a
/// calibrated lane `sweep` refused — `plan_for`'s own `Verdict` type
/// flattens both into `Refused(reason)` internally (a differently-worded
/// string is the only difference), so that distinction has to be made here,
/// one layer up, not inside `sweep`/`plan_for` themselves.
fn auto_size_with_budget(
    model_dir: &std::path::Path,
    seq_len: u64,
    gradient_checkpointing: bool,
    budget: &vox_populi::mens::tensor::memory_model::DeviceBudget,
    lane: vox_populi::mens::tensor::memory_model::Lane,
    models: &vox_populi::mens::tensor::memory_model::MemoryModels,
) -> Result<AutoSizeOutcome> {
    use vox_populi::mens::tensor::memory_model::{CalKey, ModelShape, sweep};

    let key = CalKey::new(lane, gradient_checkpointing)?;
    let shape = ModelShape::from_model_dir(model_dir)?;

    if let Err(e) = models.get(&key) {
        return Ok(AutoSizeOutcome::NoMeasurement(e.to_string()));
    }

    Ok(match sweep(budget, models, &key, &shape, seq_len) {
        Ok(req) => AutoSizeOutcome::Sized(req),
        Err(e) => AutoSizeOutcome::Refused(e.to_string()),
    })
}

/// `VOX_MENS_FORCE_TRAIN=1`/`true` — the registered operator override
/// meaning "proceed past a failing gate", including the VRAM-fit refusal
/// above. Restores exactly the escape hatch `train_arm.rs`'s deleted
/// `budget_gate`/`force_train_env` used to provide for the params_b-only
/// ladder's `ModelPlan.over_budget`, now applied to the real measured
/// `plan_for` verdict instead.
fn force_train_env() -> bool {
    std::env::var("VOX_MENS_FORCE_TRAIN")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[cfg(test)]
mod auto_size_tests {
    use super::{AutoSizeOutcome, auto_size_with_budget, force_train_env};
    use vox_populi::mens::tensor::memory_model::{DeviceBudget, Lane, MemoryModels};

    // Synthetic fixture — not a real candle-metal measurement, matches the
    // convention in memory_model.rs's own tests.
    const SEED_YAML: &str = r#"
schema: vox.mens.memory-model.v1
lanes:
  - lane: candle-metal
    gradient_checkpointing: false
    act_bytes_per_lht: 100.0
    source: measured
"#;

    fn model_dir_fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"num_hidden_layers":64,"hidden_size":5120}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("model.safetensors"),
            vec![0u8; 1], // artifact_bytes doesn't affect this test; kept tiny
        )
        .unwrap();
        dir
    }

    /// This is the test the review round asked for: confirm the real
    /// `vox mens train`-path auto-sizing decision actually calls `sweep`
    /// (not a parallel reimplementation) by checking its answer against
    /// `sweep`'s own documented property — the largest batch size that
    /// fits, not the first.
    #[test]
    fn auto_size_with_budget_matches_sweeps_largest_fitting_answer() {
        let dir = model_dir_fixture();
        let budget = DeviceBudget {
            working_set_bytes: 128 * 1024 * 1024 * 1024,
            operator_fraction: None,
        };
        let models = MemoryModels::load_from_str(SEED_YAML).unwrap();

        let outcome =
            auto_size_with_budget(dir.path(), 512, false, &budget, Lane::CandleMetal, &models)
                .expect("no setup error for a calibrated lane and a readable model dir");
        let AutoSizeOutcome::Sized(via_wrapper) = outcome else {
            panic!("expected Sized for a calibrated lane and a 128 GiB budget, got {outcome:?}");
        };

        // Directly reconstruct what `sweep` alone would answer for the same
        // inputs and assert byte-for-byte equality — proving the wrapper is
        // a pass-through, not a second algorithm.
        use vox_populi::mens::tensor::memory_model::{CalKey, ModelShape, sweep};
        let key = CalKey::new(Lane::CandleMetal, false).unwrap();
        let shape = ModelShape::from_model_dir(dir.path()).unwrap();
        let direct = sweep(&budget, &models, &key, &shape, 512).unwrap();

        assert_eq!(via_wrapper.batch_size, direct.batch_size);
        assert_eq!(via_wrapper.seq_len, direct.seq_len);
    }

    /// An uncalibrated lane (no `candle-metal` row) must report
    /// `NoMeasurement` naming why, rather than fabricating a batch size OR
    /// being conflated with a measured refusal — the caller (`gpu.rs`'s
    /// `run_gpu_training`) treats these as genuinely different outcomes.
    #[test]
    fn auto_size_with_budget_reports_no_measurement_for_an_uncalibrated_lane() {
        let dir = model_dir_fixture();
        let budget = DeviceBudget {
            working_set_bytes: 128 * 1024 * 1024 * 1024,
            operator_fraction: None,
        };
        let empty =
            MemoryModels::load_from_str("schema: vox.mens.memory-model.v1\nlanes: []\n").unwrap();
        let outcome =
            auto_size_with_budget(dir.path(), 512, false, &budget, Lane::CandleMetal, &empty)
                .expect("no setup error just because the lane is uncalibrated");
        match outcome {
            AutoSizeOutcome::NoMeasurement(reason) => {
                assert!(reason.contains("candle-metal"));
            }
            other => panic!("expected NoMeasurement for an uncalibrated lane, got {other:?}"),
        }
    }

    /// **Fix-round regression test (the critical safety gap this round
    /// exists to close):** a CALIBRATED lane whose measured `plan_for`
    /// verdict refuses even `batch_size=1` must report `Refused`, distinct
    /// from `NoMeasurement` above — never silently treated as "no data,
    /// proceed anyway". The old `train_arm.rs::budget_gate` used to `bail!`
    /// on exactly this case (a real ModelPlan.over_budget); this is its
    /// replacement's unit-level equivalent. `run_gpu_training`'s call site
    /// is what turns `Refused` into a `bail!` unless
    /// `VOX_MENS_FORCE_TRAIN=1` — this test pins the type-level distinction
    /// that decision depends on.
    #[test]
    fn auto_size_with_budget_refuses_for_a_calibrated_lane_that_does_not_fit() {
        let dir = model_dir_fixture();
        // 1 KiB: nothing fits at any batch size for this shape, and the
        // lane below IS calibrated — so this must be a measured refusal,
        // not an absence of data.
        let budget = DeviceBudget {
            working_set_bytes: 1024,
            operator_fraction: None,
        };
        let models = MemoryModels::load_from_str(SEED_YAML).unwrap();
        let outcome =
            auto_size_with_budget(dir.path(), 512, false, &budget, Lane::CandleMetal, &models)
                .expect("no setup error for a calibrated lane, even when it refuses");
        match outcome {
            AutoSizeOutcome::Refused(reason) => {
                assert!(!reason.is_empty());
            }
            other => panic!(
                "expected a measured Refused for a 1 KiB budget on a calibrated lane, got {other:?}"
            ),
        }
    }

    #[test]
    #[allow(unsafe_code)]
    fn force_train_env_parses_truthy_and_falsy_values() {
        // Guards concurrent mutation of the process-global env var — no
        // other test in this binary reads or writes VOX_MENS_FORCE_TRAIN.
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prior = std::env::var("VOX_MENS_FORCE_TRAIN").ok();

        // SAFETY: single-threaded section guarded by LOCK above.
        unsafe { std::env::remove_var("VOX_MENS_FORCE_TRAIN") };
        assert!(!force_train_env(), "unset -> false");
        for on in ["1", "true", "TRUE"] {
            unsafe { std::env::set_var("VOX_MENS_FORCE_TRAIN", on) };
            assert!(force_train_env(), "{on} -> true");
        }
        unsafe { std::env::set_var("VOX_MENS_FORCE_TRAIN", "0") };
        assert!(!force_train_env(), "0 -> false");

        match prior {
            Some(v) => unsafe { std::env::set_var("VOX_MENS_FORCE_TRAIN", v) },
            None => unsafe { std::env::remove_var("VOX_MENS_FORCE_TRAIN") },
        }
    }
}
