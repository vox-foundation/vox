//! `vox schola train` — native LoRA training worker.

use anyhow::Result;
use std::path::PathBuf;
use std::process::Stdio;

/// Strip `--log-dir` and its value from argv so the child runs without log-dir (foreground training).
fn argv_without_log_dir(args: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--log-dir" {
            i += 1;
            if i < args.len() {
                i += 1; // skip value
            }
            continue;
        }
        if args[i].starts_with("--log-dir=") {
            i += 1;
            continue;
        }
        out.push(args[i].clone());
        i += 1;
    }
    out
}

/// Spawn `vox schola train` in a background process with stdout/stderr redirected to a log file.
/// Parent returns immediately so the IDE or agent tool does not hit wall-clock timeouts; tail the log
/// file to monitor progress. The child inherits the current environment (`VOX_*`, `RUST_LOG`, etc.).
pub fn spawn_train_with_log(log_dir: PathBuf) -> Result<()> {
    use owo_colors::OwoColorize;
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| anyhow::anyhow!("create log dir {}: {}", log_dir.display(), e))?;
    let timestamp = vox_corpus::training::timestamp_string();
    let log_path = log_dir.join(format!("train_{}.log", timestamp));
    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| anyhow::anyhow!("create log file {}: {}", log_path.display(), e))?;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let child_args = argv_without_log_dir(args);
    let exe = std::env::current_exe().map_err(|e| anyhow::anyhow!("current exe: {}", e))?;

    let mut cmd = std::process::Command::new(&exe);
    for a in &child_args {
        cmd.arg(a);
    }
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::from(log_file.try_clone()?));
    cmd.stderr(Stdio::from(log_file));

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("spawn training process: {}", e))?;
    let pid = child.id();

    eprintln!(
        "{} Training started in background. PID: {}. Log: {}",
        "✓".green(),
        pid,
        log_path.display()
    );
    eprintln!(
        "  Tail with: tail -f {}  (or Get-Content {} -Wait)",
        log_path.display(),
        log_path.display()
    );
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    unused_variables,
    unused_assignments,
    unsafe_code
)]
pub async fn run_train(
    train_backend: vox_mens::PopuliTrainBackend,
    model: Option<String>,
    device: String,
    data_dir: PathBuf,
    output_dir: PathBuf,
    rank: Option<usize>,
    alpha: Option<f32>,
    seq_len: Option<usize>,
    batch_size: Option<usize>,
    grad_accum: Option<usize>,
    resume: Option<PathBuf>,
    epochs: Option<usize>,
    lr: Option<f64>,
    warmup: Option<usize>,
    seed: u64,
    min_rating: Option<u8>,
    preset: Option<String>,
    deployment_target: vox_mens::TrainingDeploymentTarget,
    process_priority: String,
    vram_limit_fraction: Option<f32>,
    adapter_tag: Option<String>,
    context_filter: Option<String>,
    validation_split_ratio: Option<f64>,
    tokenizer_mode: vox_mens::MensTokenizerMode,
    qlora_no_double_quant: bool,
    qlora_require_full_proxy_stack: bool,
    qlora_max_skip_rate: Option<f32>,
    qlora_lm_head_only: bool,
    qlora_proxy_max_layers: Option<usize>,
    qlora_ce_last_k: usize,
    checkpoint_every: Option<usize>,
    force_restart: bool,
    curriculum: bool,
    require_gpu: bool,
    allow_cpu_fallback: bool,
) -> Result<()> {
    use owo_colors::OwoColorize;

    super::process_priority::apply(&process_priority);

    let device_kind = vox_mens::normalize_device(&device).map_err(|e| anyhow::anyhow!("{}", e))?;
    vox_mens::apply_backend_env(device_kind);

    if matches!(
        deployment_target,
        vox_mens::TrainingDeploymentTarget::MobileEdge
    ) && !matches!(device_kind, vox_mens::DeviceKind::Cpu)
    {
        anyhow::bail!(vox_mens::operator_messages::MOBILE_EDGE_REQUIRES_CPU_DEVICE);
    }

    if matches!(train_backend, vox_mens::PopuliTrainBackend::CandleQlora)
        && tokenizer_mode != vox_mens::MensTokenizerMode::Hf
    {
        anyhow::bail!(vox_mens::operator_messages::QLORA_REQUIRES_HF_TOKENIZER);
    }
    if let Some(r) = qlora_max_skip_rate {
        if !r.is_finite() || !(0.0..=1.0).contains(&r) {
            anyhow::bail!("--qlora-max-skip-rate must be between 0.0 and 1.0 (got {r})");
        }
    }
    if qlora_require_full_proxy_stack && qlora_lm_head_only {
        anyhow::bail!(
            "--qlora-require-full-proxy-stack conflicts with --qlora-lm-head-only; pick one."
        );
    }
    if qlora_lm_head_only && qlora_proxy_max_layers.is_some_and(|m| m > 0) {
        anyhow::bail!(
            "--qlora-lm-head-only conflicts with --qlora-proxy-max-layers > 0; omit the cap or use `--qlora-proxy-max-layers 0`."
        );
    }

    if matches!(train_backend, vox_mens::PopuliTrainBackend::CandleQlora) {
        if matches!(device_kind, vox_mens::DeviceKind::Cuda) {
            #[cfg(not(feature = "mens-candle-cuda"))]
            anyhow::bail!(
                "`--device cuda` for Candle QLoRA requires a CUDA-enabled build.\n\
                 Rebuild: `cargo build -p vox-cli --features gpu,mens-candle-cuda` (or `cargo vox-cuda-release`).\n\
                 On Windows use a VS Developer shell so `nvcc` can find MSVC."
            );
        }
        #[cfg(target_os = "macos")]
        if matches!(device_kind, vox_mens::DeviceKind::Metal) {
            #[cfg(not(feature = "mens-candle-metal"))]
            anyhow::bail!(
                "`--device metal` for Candle QLoRA requires `mens-candle-metal`.\n\
                 Rebuild: `cargo build -p vox-cli --features gpu,mens-candle-metal`."
            );
        }
    }

    tracing::debug!(
        ?train_backend,
        ?tokenizer_mode,
        model = ?model.as_deref(),
        "vox schola train entry (backend + tokenizer SSOT)"
    );
    tracing::debug!(
        model = ?model.as_deref(),
        device = %device,
        ?preset,
        ?rank,
        ?alpha,
        ?seq_len,
        ?batch_size,
        ?grad_accum,
        ?epochs,
        ?warmup,
        ?lr,
        seed,
        qlora_no_double_quant,
        qlora_require_full_proxy_stack,
        qlora_lm_head_only,
        ?qlora_max_skip_rate,
        ?qlora_proxy_max_layers,
        qlora_ce_last_k,
        checkpoint_every,
        force_restart,
        curriculum,
        require_gpu,
        allow_cpu_fallback,
        "Training parser payload"
    );

    unsafe {
        if let Some(ref m) = model {
            std::env::set_var("VOX_BASE_MODEL", m);
        }
        std::env::set_var("VK_LOADER_DEBUG", "none");
        std::env::set_var("VK_LOADER_LOG_LEVEL", "none");
        std::env::set_var("WGPU_LOG_LEVEL", "error");
        std::env::set_var("WGPU_VALIDATION", "0");
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "info");
        }
    }

    let gpu_info = vox_mens::probe_gpu();
    let device_profile =
        vox_mens::DeviceProfile::from_gpu_info(&gpu_info.model_name, gpu_info.vram_mb);
    let cli_overrides = vox_mens::CliOverrides {
        rank,
        alpha,
        seq_len,
        batch_size,
        grad_accum,
        epochs,
        warmup,
        lr,
    };
    let preview_profile = vox_mens::resolve_effective_profile(
        preset.as_deref(),
        device_profile.clone(),
        None,
        cli_overrides.clone(),
    );

    tracing::debug!(
        model = ?model,
        device = %device,
        rank = preview_profile.rank,
        alpha = preview_profile.alpha,
        "Dispatching training payload to native orchestra"
    );

    eprintln!("{}", "╔══════════════════════════════════════════╗".cyan());
    eprintln!("{}", "║   Vox Mens — Native LoRA Training     ║".cyan());
    eprintln!("{}", "╚══════════════════════════════════════════╝".cyan());
    eprintln!();
    eprintln!(
        "  Model:       {}",
        model.as_deref().unwrap_or("(scratch init)").cyan()
    );
    eprintln!("  Device:      {}", device.cyan());
    eprintln!("  Data:        {}", data_dir.display().cyan());
    eprintln!("  Output:      {}", output_dir.display().cyan());
    eprintln!(
        "  Rank/Alpha:  {}/{}",
        preview_profile.rank, preview_profile.alpha
    );
    eprintln!(
        "  Batch/Accum: {}/{} (effective={})",
        preview_profile.batch_size,
        preview_profile.grad_accum,
        preview_profile.batch_size * preview_profile.grad_accum
    );
    eprintln!("  Backend:     {}", train_backend);
    eprintln!("  Tokenizer:   {:?}", tokenizer_mode);
    eprintln!("  Seq len:     {}", preview_profile.seq_len);
    if let Some(ref r) = resume {
        eprintln!("  Resume:      {}", r.display().cyan());
    }
    eprintln!();

    #[cfg(feature = "gpu")]
    {
        // Run corpus mix before training to ensure latest corpus is used. Set
        // `VOX_TRAIN_SKIP_CORPUS_MIX=1` to skip (pinned `train.jsonl`, or long mix under IDE timeouts).
        let workspace_root = vox_corpus::training::contract::find_workspace_root();
        let mix_config_path: Option<std::path::PathBuf> = workspace_root
            .as_ref()
            .map(|r| r.join("mens/config/mix.yaml"));
        let skip_mix = std::env::var("VOX_TRAIN_SKIP_CORPUS_MIX")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if skip_mix {
            eprintln!(
                "  {} Skipping corpus mix (`VOX_TRAIN_SKIP_CORPUS_MIX`); using train file under data-dir",
                "⏭".cyan()
            );
        }
        let mut contract_override = None;
        if let Some(ref cfg_path) = mix_config_path {
            if !skip_mix && cfg_path.exists() {
                eprintln!(
                    "  {} Running corpus mix to refresh training data...",
                    "🔄".cyan()
                );
                if let Err(e) = vox_corpus::corpus::run_mix(cfg_path) {
                    eprintln!(
                        "  {} Mix failed ({}); continuing with existing corpus",
                        "⚠".yellow(),
                        e
                    );
                } else if let Ok(mix_cfg) = vox_corpus::corpus::MixConfigSchema::load(cfg_path) {
                    // Use mix output directly without corrupting primary train file
                    let cwd = std::env::current_dir().unwrap_or_else(|_| data_dir.clone());
                    let mix_output = cwd.join(&mix_cfg.output);
                    if mix_output.exists() {
                        contract_override = Some(mix_output);
                    }
                }
            }
        }

        // Preflight: resolve canonical train input, abort on ambiguity/stale/empty
        let resolved = vox_corpus::training::preflight::validate_train_preflight(
            &data_dir,
            contract_override.as_deref(),
            workspace_root.as_deref(),
        )?;
        tracing::debug!(path = %resolved.path.display(), source = ?resolved.source, "Preflight resolved train input");

        // VRAM Auto-Detect: Select 16g preset if CUDA is active and no preset is provided.
        let mut final_preset = preset.clone();
        if final_preset.is_none() && device.to_lowercase() == "cuda" {
            eprintln!(
                "  {} {}",
                "⚙".cyan(),
                vox_mens::tensor::vram_autodetect::vram_summary(true)
            );
            let auto_preset = vox_mens::tensor::vram_autodetect::auto_preset(
                true,
                vox_mens::tensor::vram_autodetect::get_system_vram_gb(),
            );
            if let Some(ap) = auto_preset {
                eprintln!(
                    "  {} Auto-detected 16 GB VRAM → using preset '{}'",
                    "⚙".cyan(),
                    ap
                );
                final_preset = Some(ap.to_string());
            }
        }

        let profile = vox_mens::resolve_effective_profile(
            final_preset.as_deref(),
            device_profile,
            resolved.sample_count,
            cli_overrides,
        );
        let rank = profile.rank;
        let alpha = profile.alpha;
        let seq_len = profile.seq_len;
        if matches!(train_backend, vox_mens::PopuliTrainBackend::CandleQlora) {
            let k = qlora_ce_last_k.max(1);
            if k > 64 {
                anyhow::bail!("--qlora-ce-last-k must be at most 64 (got {k})");
            }
            if k > seq_len {
                anyhow::bail!(
                    "--qlora-ce-last-k ({k}) cannot exceed effective sequence length ({seq_len})"
                );
            }
        }
        let batch_size = profile.batch_size;
        let grad_accum = profile.grad_accum;
        let epochs = profile.epochs;
        let warmup = profile.warmup;
        let lr = profile.lr;
        let mut base_model_paths = None::<(Vec<std::path::PathBuf>, std::path::PathBuf)>;
        let mut tokenizer_path = None::<std::path::PathBuf>;
        if let Some(ref repo_id) = model {
            eprintln!(
                "  {} Downloading base model from Hugging Face: {}",
                "📥".cyan(),
                repo_id
            );
            // Run download in a dedicated thread with its own runtime so we don't block_on inside the CLI's tokio runtime.
            let repo_id = repo_id.clone();
            let repo_id_for_download = repo_id.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let result = rt.block_on(vox_mens::hub::download_model(&repo_id_for_download));
                let _ = tx.send(result);
            });
            let download_result = rx
                .recv()
                .map_err(|_| anyhow::anyhow!("HF download thread exited without sending"))?;
            match download_result {
                Ok(files) if files.is_safetensors() => {
                    base_model_paths = Some((files.weights.clone(), files.config.clone()));
                    tokenizer_path = files.tokenizer.clone();
                    eprintln!("  {} Cached at {}", "✓".green(), files.cache_dir.display());

                    // Architecture + VRAM diagnostics
                    if let Ok(arch) =
                        vox_mens::tensor::hf_load::detect_hf_architecture(&files.config)
                    {
                        eprintln!("  {} Architecture: {:?}", "📐".cyan(), arch);
                        let cfg = vox_mens::tensor::hf_load::config_dims_for_architecture(
                            &files.config,
                            arch,
                        )
                        .map_err(|e| anyhow::anyhow!("HF config: {}", e))?;
                        let tokenizer_src = tokenizer_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "Vox (built-in)".to_string());
                        eprintln!("  {} Tokenizer: {}", "🔤".cyan(), tokenizer_src);
                        let est_mb =
                            if matches!(train_backend, vox_mens::PopuliTrainBackend::CandleQlora) {
                                vox_mens::estimate_training_vram_mb_qlora(
                                    cfg.n_embd,
                                    cfg.n_head,
                                    cfg.n_layer,
                                    cfg.vocab_size,
                                    profile.batch_size,
                                    profile.seq_len,
                                )
                            } else {
                                vox_mens::estimate_training_vram_mb(
                                    cfg.n_embd,
                                    cfg.n_head,
                                    cfg.n_layer,
                                    cfg.vocab_size,
                                    profile.batch_size,
                                    profile.seq_len,
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
                Ok(_) => anyhow::bail!(
                    "HF model `{repo_id}` has no safetensors; QLoRA requires safetensors base weights."
                ),
                Err(e) => {
                    anyhow::bail!(
                        "HF download failed for `{repo_id}` ({e}). \
                         Set HF token env vars if this is a gated repo and retry."
                    );
                }
            }
        }

        let run_id = vox_corpus::training::timestamp_string();
        let git_sha = option_env!("VOX_GIT_HASH").unwrap_or("unknown").to_string();
        let device_profile_str = if device_kind == vox_mens::DeviceKind::Cpu {
            "cpu".to_string()
        } else {
            gpu_info.model_name.clone()
        };
        let config = vox_mens::LoraTrainingConfig {
            base_model: model,
            base_model_paths,
            tokenizer_path,
            train_file: Some(resolved.path),
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
            qlora_ce_last_k: qlora_ce_last_k.max(1),
            checkpoint_every,
            force_restart,
            deployment_target,
            curriculum,
            require_gpu,
            allow_cpu_fallback,
        };
        let model_name_for_stats = config
            .base_model
            .clone()
            .unwrap_or_else(|| "scratch".to_string());
        let preset_for_stats = preset.clone().unwrap_or_else(|| "unknown".to_string());

        let system_prompt = vox_corpus::training::generate_training_system_prompt();

        // Note: `data_dir` is passed here as a fallback root.
        // The trainer relies on `config.train_file` (resolved during preflight)
        // as the single source of truth for the JSONL path.
        let summary = vox_mens::run_mens_training(
            train_backend,
            &data_dir,
            Some(&output_dir),
            &config,
            device_kind,
            &system_prompt,
        )?;

        // Wave 2: Local training telemetry write-back (4080 parity)
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

    #[cfg(not(feature = "gpu"))]
    {
        let _ = (
            train_backend,
            model,
            device,
            data_dir,
            output_dir,
            rank,
            alpha,
            seq_len,
            batch_size,
            grad_accum,
            resume,
            epochs,
            warmup,
            lr,
            seed,
            min_rating,
            preset,
            deployment_target,
            process_priority,
            vram_limit_fraction,
            adapter_tag,
            context_filter,
            tokenizer_mode,
            qlora_no_double_quant,
            qlora_require_full_proxy_stack,
            qlora_max_skip_rate,
            qlora_lm_head_only,
            qlora_proxy_max_layers,
            qlora_ce_last_k,
            checkpoint_every,
            force_restart,
        );
        eprintln!(
            "  {} LoRA training requires the 'gpu' feature.",
            "⚠".yellow()
        );
        eprintln!("  Rebuild with: cargo build --features gpu");
        eprintln!();
        eprintln!(
            "  Canonical QLoRA (when `gpu` is enabled): `vox schola train --backend qlora …`"
        );
        eprintln!("  See docs/src/architecture/mens-training-ssot.md");
        Ok(())
    }
}
