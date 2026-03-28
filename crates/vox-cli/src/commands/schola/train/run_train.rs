use anyhow::Result;
use std::path::PathBuf;

#[allow(
    clippy::too_many_arguments,
    unused_variables,
    unused_assignments,
    unsafe_code
)]
pub async fn run_train(
    train_backend: vox_populi::mens::PopuliTrainBackend,
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
    deployment_target: vox_populi::mens::TrainingDeploymentTarget,
    process_priority: String,
    vram_limit_fraction: Option<f32>,
    adapter_tag: Option<String>,
    context_filter: Option<String>,
    validation_split_ratio: Option<f64>,
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
) -> Result<()> {
    use owo_colors::OwoColorize;

    super::super::process_priority::apply(&process_priority);

    let device_kind =
        vox_populi::mens::normalize_device(&device).map_err(|e| anyhow::anyhow!("{}", e))?;
    vox_populi::mens::apply_backend_env(device_kind);

    if matches!(
        deployment_target,
        vox_populi::mens::TrainingDeploymentTarget::MobileEdge
    ) && !matches!(device_kind, vox_populi::mens::DeviceKind::Cpu)
    {
        anyhow::bail!(vox_populi::mens::operator_messages::MOBILE_EDGE_REQUIRES_CPU_DEVICE);
    }

    if matches!(
        train_backend,
        vox_populi::mens::PopuliTrainBackend::CandleQlora
    ) && tokenizer_mode != vox_populi::mens::MensTokenizerMode::Hf
    {
        anyhow::bail!(vox_populi::mens::operator_messages::QLORA_REQUIRES_HF_TOKENIZER);
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

    if matches!(
        train_backend,
        vox_populi::mens::PopuliTrainBackend::CandleQlora
    ) {
        if matches!(device_kind, vox_populi::mens::DeviceKind::Cuda) {
            #[cfg(not(feature = "mens-candle-cuda"))]
            anyhow::bail!(
                "`--device cuda` for Candle QLoRA requires a CUDA-enabled build.\n\
                 Rebuild: `cargo build -p vox-cli --features gpu,mens-candle-cuda` (or `cargo vox-cuda-release`).\n\
                 On Windows use a VS Developer shell so `nvcc` can find MSVC."
            );
        }
        #[cfg(target_os = "macos")]
        if matches!(device_kind, vox_populi::mens::DeviceKind::Metal) {
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
        "vox mens train entry (backend + tokenizer SSOT)"
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
        ?optimizer_experiment_mode,
        require_gpu,
        allow_cpu_fallback,
        ?base_model_family,
        ?upstream_model_id,
        ?license_class,
        attribution_required,
        trajectory_weighting_enabled,
        trajectory_tool_trace_boost,
        trajectory_failure_category_boost,
        ?trajectory_quality_floor,
        trajectory_quality_boost,
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
    let preview_profile = vox_populi::mens::resolve_effective_profile(
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
        return super::gpu::run_gpu_training(
            train_backend,
            model,
            device,
            data_dir,
            output_dir,
            resume,
            preset,
            device_profile,
            cli_overrides,
            gpu_info,
            device_kind,
            min_rating,
            deployment_target,
            tokenizer_mode,
            qlora_no_double_quant,
            qlora_require_full_proxy_stack,
            qlora_max_skip_rate,
            qlora_lm_head_only,
            qlora_proxy_max_layers,
            qlora_ce_last_k,
            checkpoint_every,
            force_restart,
            curriculum,
            optimizer_experiment_mode,
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
            vram_limit_fraction,
            adapter_tag,
            context_filter,
            validation_split_ratio,
            seed,
        )
        .await;
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
            curriculum,
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
            optimizer_experiment_mode,
        );
        eprintln!(
            "  {} LoRA training requires the 'gpu' feature.",
            "⚠".yellow()
        );
        eprintln!("  Rebuild with: cargo build --features gpu");
        eprintln!();
        eprintln!("  Canonical QLoRA (when `gpu` is enabled): `vox mens train --backend qlora …`");
        eprintln!("  See docs/src/reference/mens-training.md");
        Ok(())
    }
}
