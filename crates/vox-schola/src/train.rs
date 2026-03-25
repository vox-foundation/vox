//! `vox-schola train` — dispatches QLoRA training.

use anyhow::Result;
use std::path::PathBuf;

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
        qlora_lm_head_only,
        qlora_max_skip_rate,
        qlora_proxy_max_layers,
        qlora_ce_last_k,
    } = args.cmd
    else {
        unreachable!()
    };

    // ── Env setup (suppress GPU/Vulkan noise) ─────────────────────────────────
    #[allow(unsafe_code)]
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
    let device_kind = vox_populi::mens::normalize_device(&device).map_err(|e| anyhow::anyhow!("{}", e))?;
    vox_populi::mens::apply_backend_env(device_kind);

    // ── Validation ────────────────────────────────────────────────────────────
    if let Some(r) = qlora_max_skip_rate
        && (!r.is_finite() || !(0.0..=1.0).contains(&r))
    {
        anyhow::bail!("--qlora-max-skip-rate must be 0.0–1.0 (got {r})");
    }
    if qlora_require_full_proxy_stack && qlora_lm_head_only {
        anyhow::bail!("--qlora-require-full-proxy-stack conflicts with --qlora-lm-head-only");
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

    // ── Corpus preflight ──────────────────────────────────────────────────────
    let workspace_root = vox_corpus::training::contract::find_workspace_root();
    let mix_config = workspace_root
        .as_ref()
        .map(|r| r.join("mens/config/mix.yaml"));
    let mut contract_override: Option<PathBuf> = None;
    let skip_mix = std::env::var("VOX_TRAIN_SKIP_CORPUS_MIX")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !skip_mix
        && let Some(ref cfg_path) = mix_config
        && cfg_path.exists()
    {
        eprintln!("  🔄 Running corpus mix to refresh training data...");
        if let Err(e) = vox_corpus::corpus::run_mix(cfg_path) {
            eprintln!("  ⚠ Mix failed ({e}); continuing with existing corpus");
        } else if let Ok(mix_cfg) = vox_corpus::corpus::MixConfigSchema::load(cfg_path) {
            let cwd = std::env::current_dir().unwrap_or_else(|_| data_dir.clone());
            let mix_output = cwd.join(&mix_cfg.output);
            if mix_output.exists() {
                contract_override = Some(mix_output);
            }
        }
    }

    let resolved = vox_corpus::training::preflight::validate_train_preflight(
        &data_dir,
        contract_override.as_deref(),
        workspace_root.as_deref(),
    )?;

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
        let repo_id = repo_id.clone();
        let repo_id_for_download = repo_id.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(vox_populi::mens::hub::download_model(&repo_id_for_download));
            let _ = tx.send(result);
        });
        match rx
            .recv()
            .map_err(|_| anyhow::anyhow!("HF download thread exited"))?
        {
            Ok(files) if files.is_safetensors() => {
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
            Ok(_) => anyhow::bail!(
                "HF model `{repo_id}` has no safetensors; QLoRA requires safetensors base weights."
            ),
            Err(e) => anyhow::bail!(
                "HF download failed for `{repo_id}` ({e}). Set HF token env vars and retry."
            ),
        }
    }

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

    let config = vox_populi::mens::LoraTrainingConfig {
        base_model: model,
        base_model_paths,
        tokenizer_path,
        train_file: Some(resolved.path),
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
        context_filter,
        tokenizer_mode: vox_populi::mens::MensTokenizerMode::Hf,
        qlora_double_quant: !qlora_no_double_quant,
        finetune_contract_digest: None,
        qlora_require_full_proxy_stack,
        qlora_max_skip_rate,
        qlora_lm_head_only,
        qlora_proxy_max_layers,
        qlora_ce_last_k: qlora_ce_last_k.max(1),
        checkpoint_every,
        force_restart: effective_force_restart,
        deployment_target: vox_populi::mens::TrainingDeploymentTarget::Workstation,
        validation_split_ratio: Some(0.05),
        curriculum: false,
        require_gpu: false,
        allow_cpu_fallback: true,
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
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
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
