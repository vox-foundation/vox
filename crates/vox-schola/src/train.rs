//! `vox-schola train` — dispatches QLoRA training.

use anyhow::{Context as _, Result};
use std::path::PathBuf;

use vox_populi::mens::tensor::training_config::{ChatmlConfig, ContextFilter};

use crate::cli::{Args, Cmd};

pub async fn run(args: Args) -> Result<()> {
    let Cmd::Train {
        model,
        device,
        data_dir,
        output_dir,
        preset,
        rank,
        alpha,
        seq_len,
        checkpoint_every,
        batch_size,
        grad_accum,
        epochs,
        lr,
        warmup,
        seed,
        min_rating,
        resume,
        resume_checkpoint,
        force_restart,
        adapter_tag,
        context_filter,
        vram_limit_fraction,
        background,
        log_dir,
        skip_corpus_mix,
        qlora_no_double_quant,
        qlora_require_full_proxy_stack,
        qlora_allow_partial_proxy_stack,
        qlora_lm_head_only,
        qlora_max_skip_rate,
        qlora_proxy_max_layers,
        qlora_ce_last_k,
        base_model_family,
        upstream_model_id,
        license_class,
        attribution_required,
        trajectory_weighting_enabled,
        trajectory_tool_trace_boost,
        trajectory_failure_category_boost,
        trajectory_quality_floor,
        trajectory_quality_boost,
    } = args.cmd
    else {
        unreachable!()
    };

    if let Some(status) = crate::forward::maybe_forward_to_vox(
        &model,
        &device,
        &data_dir,
        &output_dir,
        &preset,
        rank,
        alpha,
        seq_len,
        checkpoint_every,
        batch_size,
        grad_accum,
        epochs,
        lr,
        warmup,
        seed,
        min_rating,
        &resume,
        resume_checkpoint,
        force_restart,
        &adapter_tag,
        &context_filter,
        vram_limit_fraction,
        background,
        &log_dir,
        skip_corpus_mix,
        qlora_no_double_quant,
        qlora_require_full_proxy_stack,
        qlora_allow_partial_proxy_stack,
        qlora_lm_head_only,
        qlora_max_skip_rate,
        qlora_proxy_max_layers,
        qlora_ce_last_k,
        &base_model_family,
        &upstream_model_id,
        &license_class,
        attribution_required,
        trajectory_weighting_enabled,
        trajectory_tool_trace_boost,
        trajectory_failure_category_boost,
        trajectory_quality_floor,
        trajectory_quality_boost,
    )? {
        std::process::exit(status.code().unwrap_or(1));
    }

    // ── Env setup (suppress GPU/Vulkan noise) ─────────────────────────────────
    #[allow(unsafe_code)]
    // SAFETY: CLI entrypoint environment mutation runs synchronously before multithreaded runtimes start.
    unsafe {
        if let Some(ref m) = model {
            std::env::set_var("VOX_BASE_MODEL", m);
        }
        std::env::set_var("VK_LOADER_DEBUG", "none");
        std::env::set_var("VK_LOADER_LOG_LEVEL", "none");
        std::env::set_var("WGPU_LOG_LEVEL", "error");
        std::env::set_var("WGPU_VALIDATION", "0");
        if skip_corpus_mix {
            std::env::set_var("VOX_TRAIN_SKIP_CORPUS_MIX", "1");
        }
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "error");
        }
    }

    // ── Background mode ───────────────────────────────────────────────────────
    if background || log_dir.is_some() {
        return spawn_background(log_dir);
    }

    // ── Device ────────────────────────────────────────────────────────────────
    let device_kind =
        vox_populi::mens::normalize_device(&device).map_err(|e| anyhow::anyhow!("{}", e))?;
    vox_populi::mens::apply_backend_env(device_kind);

    if qlora_allow_partial_proxy_stack && qlora_require_full_proxy_stack {
        anyhow::bail!(
            "`--qlora-allow-partial-proxy-stack` cannot be combined with `--qlora-require-full-proxy-stack`."
        );
    }

    let mut model = model;
    if model.is_none() {
        model = Some(vox_populi::mens::DEFAULT_MODEL_ID.to_string());
        tracing::info!(
            model = %vox_populi::mens::DEFAULT_MODEL_ID,
            "Using default HF model (`--model` omitted; matches `vox mens train` SSOT)."
        );
    }
    #[allow(unsafe_code)]
    // SAFETY: Single-threaded sync mutation prior to background execution or parallel pipelines.
    unsafe {
        if let Some(ref m) = model {
            std::env::set_var("VOX_BASE_MODEL", m);
        }
    }

    let effective_qlora_require_full_proxy_stack = !qlora_allow_partial_proxy_stack
        && (qlora_require_full_proxy_stack
            || matches!(device_kind, vox_populi::mens::DeviceKind::Cuda));

    // ── Validation ────────────────────────────────────────────────────────────
    if let Some(r) = qlora_max_skip_rate
        && (!r.is_finite() || !(0.0..=1.0).contains(&r))
    {
        anyhow::bail!("--qlora-max-skip-rate must be 0.0–1.0 (got {r})");
    }
    if effective_qlora_require_full_proxy_stack && qlora_lm_head_only {
        anyhow::bail!(
            "Full proxy stack (CUDA default or `--qlora-require-full-proxy-stack`) conflicts with `--qlora-lm-head-only`"
        );
    }
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

    // ── Device / VRAM diagnostics ─────────────────────────────────────────────
    let gpu_info = vox_populi::mens::probe_gpu();
    let device_profile =
        vox_populi::mens::DeviceProfile::from_gpu_info(&gpu_info.model_name, gpu_info.vram_mb);
    let cli_overrides = vox_populi::mens::CliOverrides {
        rank,
        alpha,
        seq_len,
        batch_size,
        grad_accum,
        epochs,
        warmup,
        lr,
    };

    // ── Corpus preflight (shared mix / train-input prep with `vox mens train`) ──
    let workspace_root = vox_corpus::training::contract::find_workspace_root();
    let data_dir = vox_corpus::training::contract::normalize_workspace_relative_path(
        data_dir,
        workspace_root.as_deref(),
    );
    let output_dir = vox_corpus::training::contract::normalize_workspace_relative_path(
        output_dir,
        workspace_root.as_deref(),
    );
    let resume = resume.map(|r| {
        vox_corpus::training::contract::normalize_training_resume_path(r, workspace_root.as_deref())
    });
    let skip_mix = vox_corpus::training::mix_prepare::corpus_mix_skip_from_env();
    let explicit_mix_path = if let Some(tag) = adapter_tag.as_deref() {
        if let Ok(domain) = vox_populi::mens::tensor::domain_profiles::EffectiveDomainProfile::load_domain_profile(tag, workspace_root.as_deref()) {
            domain.mix_config.clone()
        } else {
            None
        }
    } else {
        None
    };

    let mix_path = explicit_mix_path.unwrap_or_else(|| {
        vox_corpus::training::mix_prepare::resolve_mix_config_path(workspace_root.as_deref())
    });
    if !skip_mix && mix_path.is_file() {
        eprintln!("  🔄 Running corpus mix to refresh training data...");
    }
    let contract_override =
        vox_corpus::training::mix_prepare::refresh_train_contract_override_from_mix(
            workspace_root.as_deref(),
            &data_dir,
            skip_mix,
            true,
            Some(&mix_path),
        )?;

    // We must validate train preflight here directly
    let resolved = vox_corpus::training::preflight::validate_train_preflight(
        &data_dir,
        contract_override.as_deref(),
        workspace_root.as_deref(),
    )?;

    // Re-resolve profile now that we have sample_count
    let profile = vox_populi::mens::resolve_effective_profile(
        preset.as_deref(),
        device_profile.clone(),
        resolved.sample_count,
        cli_overrides,
    );

    if device.eq_ignore_ascii_case("cuda") {
        eprintln!(
            "  ⚙ {}",
            vox_populi::mens::tensor::vram_autodetect::vram_summary(true)
        );
    }

    // ── HF model download ─────────────────────────────────────────────────────
    let mut base_model_paths = None::<(Vec<std::path::PathBuf>, std::path::PathBuf)>;
    let mut tokenizer_path = None::<std::path::PathBuf>;

    if let Some(ref repo_id) = model {
        eprintln!("  📥 Downloading from HuggingFace: {}", repo_id);
        let files = vox_populi::mens::hub::download_model_blocking(repo_id).map_err(|e| {
            anyhow::anyhow!(
                "HF download failed for `{repo_id}` ({e}). Set HF token env vars and retry."
            )
        })?;
        if !files.is_safetensors() {
            anyhow::bail!(
                "HF model `{repo_id}` has no safetensors; QLoRA requires safetensors base weights."
            );
        }
        eprintln!("  ✓ Cached at {}", files.cache_dir.display());
        base_model_paths = Some((files.weights.clone(), files.config.clone()));
        tokenizer_path = files.tokenizer.clone();
        if let Ok(arch) = vox_populi::mens::tensor::hf_load::detect_hf_architecture(&files.config) {
            let cfg = vox_populi::mens::tensor::hf_load::config_dims_for_architecture(
                &files.config,
                arch,
            )
            .map_err(|e| anyhow::anyhow!("HF config: {}", e))?;
            let est_mb = vox_populi::mens::estimate_training_vram_mb_qlora(
                cfg.n_embd,
                cfg.n_head,
                cfg.n_layer,
                cfg.vocab_size,
                profile.batch_size,
                profile.seq_len,
            );
            if gpu_info.vram_mb > 0 && est_mb as f64 > gpu_info.vram_mb as f64 * 0.85 {
                eprintln!(
                    "  ⚠ VRAM risk: est. {est_mb} MB > 85% of {} MB — try --preset safe",
                    gpu_info.vram_mb
                );
            } else if gpu_info.vram_mb > 0 {
                eprintln!(
                    "  ✓ VRAM: est. ~{est_mb} MB / {} MB available",
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

    // ── Banner ────────────────────────────────────────────────────────────────
    let rank = profile.rank;
    let alpha = profile.alpha;
    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║   Vox Train — Candle QLoRA (NF4)        ║");
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!(
        "  Model:       {}",
        model.as_deref().unwrap_or("(none — scratch)")
    );
    eprintln!("  Device:      {device}");
    eprintln!("  Data:        {}", data_dir.display());
    eprintln!("  Output:      {}", output_dir.display());
    eprintln!("  Rank/Alpha:  {rank}/{alpha}");
    eprintln!(
        "  Batch/Accum: {}/{} (eff={})",
        profile.batch_size,
        profile.grad_accum,
        profile.batch_size * profile.grad_accum
    );
    eprintln!("  Seq len:     {}", profile.seq_len);
    if let Some(ref r) = resume {
        eprintln!("  Resume:      {}", r.display());
    }
    eprintln!();

    // ── Assemble LoraTrainingConfig and dispatch ───────────────────────────────
    let effective_force_restart = if resume.is_some() {
        force_restart
    } else {
        force_restart || !resume_checkpoint
    };

    let run_id = vox_corpus::training::timestamp_string();
    let git_sha = option_env!("VOX_GIT_HASH").unwrap_or("unknown").to_string();
    let device_profile_str = if device_kind == vox_populi::mens::DeviceKind::Cpu {
        "cpu".to_string()
    } else {
        gpu_info.model_name.clone()
    };

    let parsed_context_filter: Option<ContextFilter> = match context_filter.as_deref() {
        None => None,
        Some(raw) => {
            let raw = raw.trim();
            if raw.is_empty() {
                None
            } else {
                Some(serde_json::from_str(raw).context("invalid --context-filter JSON")?)
            }
        }
    };

    let parsed_reward_hook = if let Some(tag) = adapter_tag.as_deref() {
        if let Ok(domain) = vox_populi::mens::tensor::domain_profiles::EffectiveDomainProfile::load_domain_profile(tag, workspace_root.as_deref()) {
            domain.reward_hook.clone()
        } else {
            None
        }
    } else {
        None
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
        seq_len: profile.seq_len,
        batch_size: profile.batch_size,
        grad_accum: profile.grad_accum,
        resume_from: resume,
        epochs: profile.epochs,
        learning_rate: profile.lr,
        warmup_steps: profile.warmup,
        seed,
        min_rating: min_rating.unwrap_or(3),
        run_id: Some(run_id),
        git_sha: Some(git_sha),
        device_profile: Some(device_profile_str),
        max_vram_fraction: vram_limit_fraction,
        adapter_tag,
        context_filter: parsed_context_filter,
        tokenizer_mode: vox_populi::mens::MensTokenizerMode::Hf,
        qlora_double_quant: !qlora_no_double_quant,
        finetune_contract_digest: None,
        qlora_require_full_proxy_stack: effective_qlora_require_full_proxy_stack,
        qlora_max_skip_rate,
        qlora_lm_head_only,
        qlora_proxy_max_layers,
        qlora_ce_last_k: qlora_ce_last_k.max(1),
        checkpoint_every,
        force_restart: effective_force_restart,
        deployment_target: vox_populi::mens::TrainingDeploymentTarget::Workstation,
        validation_split_ratio: Some(0.05),
        curriculum: false,
        curriculum_schedule: None,
        optimizer_experiment_mode:
            vox_populi::mens::tensor::training_config::OptimizerExperimentMode::Off,
        trajectory_weighting_enabled,
        trajectory_tool_trace_boost,
        trajectory_failure_category_boost,
        trajectory_quality_floor,
        trajectory_quality_boost,
        require_gpu: false,
        allow_cpu_fallback: true,
        chatml: ChatmlConfig::default(),
        reward_hook: parsed_reward_hook,
    };

    let system_prompt = vox_corpus::training::generate_training_system_prompt();
    vox_populi::mens::run_mens_training(
        vox_populi::mens::PopuliTrainBackend::CandleQlora,
        &data_dir,
        Some(&output_dir),
        &config,
        device_kind,
        &system_prompt,
    )?;
    Ok(())
}

fn spawn_background(log_dir: Option<PathBuf>) -> Result<()> {
    let log_dir = log_dir.unwrap_or_else(|| PathBuf::from("mens/runs/logs"));
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| anyhow::anyhow!("create log dir {}: {e}", log_dir.display()))?;
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let log_path = log_dir.join(format!("train_{ts}.log"));
    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| anyhow::anyhow!("create log file {}: {e}", log_path.display()))?;

    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let mut args = Vec::new();
    let mut skip_next = false;
    for arg in &raw_args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--background" {
            continue;
        }
        if arg.starts_with("--log-dir=") {
            continue;
        }
        if arg == "--log-dir" {
            skip_next = true;
            continue;
        }
        args.push(arg.clone());
    }
    let exe = std::env::current_exe()?;
    use std::process::Stdio;
    let mut cmd = std::process::Command::new(&exe);
    for a in &args {
        cmd.arg(a);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file));

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        cmd.creation_flags(CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB);
    }

    let child = cmd.spawn()?;
    eprintln!(
        "✓ Training started in background. PID: {}. Log: {}",
        child.id(),
        log_path.display()
    );
    eprintln!("  Tail with: Get-Content {} -Wait", log_path.display());
    Ok(())
}
