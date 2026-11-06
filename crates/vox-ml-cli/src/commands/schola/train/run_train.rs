use anyhow::Result;
use std::path::{Path, PathBuf};

// ── P2-009: Corpus threshold gate (research: CL minimum corpus) ────────
// Refuse to start training if the corpus has fewer than the minimum viable
// pairs. Current corpus is ~340 (Gap G-11); research proves <500 pairs
// guarantees catastrophic overfitting.
// Reference: docs/src/architecture/research-cl-qlora-minimum-corpus-2026.md
const MIN_CORPUS_PAIRS: usize = 100;

/// Hard-fails if `train.jsonl` is missing (previously silently skipped the whole gate) or
/// has fewer than [`MIN_CORPUS_PAIRS`] non-blank lines.
pub(crate) fn check_corpus_threshold(data_dir: &Path) -> Result<()> {
    let train_jsonl = data_dir.join("train.jsonl");
    if !train_jsonl.exists() {
        // A missing train.jsonl is not "no corpus check needed" -- it means the corpus
        // was never built (or the data dir is wrong). Silently skipping here let
        // training start against zero pairs. Fail closed instead.
        anyhow::bail!(
            "Corpus file not found: {} (expected training pairs). \
             Generate the corpus with `vox mens corpus extract` + `vox mens corpus pairs` \
             before training.",
            train_jsonl.display()
        );
    }
    let pair_count = std::fs::read_to_string(&train_jsonl)
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);
    if pair_count < MIN_CORPUS_PAIRS {
        anyhow::bail!(
            "Corpus has {} validated pairs (minimum: {}). \
             Fine-tuning with fewer than {} pairs risks catastrophic overfitting \
             (see research-cl-qlora-minimum-corpus-2026.md).\n\
             Use `vox mens serve --rag` for in-context learning until the corpus \
             reaches the threshold, or generate more pairs with \
             `vox mens corpus extract` + `vox mens corpus pairs`.",
            pair_count,
            MIN_CORPUS_PAIRS,
            MIN_CORPUS_PAIRS
        );
    }
    Ok(())
}

#[cfg(test)]
mod corpus_threshold_tests {
    use super::{MIN_CORPUS_PAIRS, check_corpus_threshold};

    #[test]
    fn missing_train_jsonl_is_a_hard_error() {
        let dir = std::env::temp_dir().join(format!(
            "vox-corpus-threshold-test-missing-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // No train.jsonl written.
        let err = check_corpus_threshold(&dir).expect_err("missing train.jsonl must error");
        assert!(
            err.to_string().contains("Corpus file not found"),
            "unexpected error: {err}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn undersized_train_jsonl_is_a_hard_error() {
        let dir = std::env::temp_dir().join(format!(
            "vox-corpus-threshold-test-small-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("train.jsonl"), "{}\n{}\n").unwrap();
        let err = check_corpus_threshold(&dir).expect_err("under-threshold corpus must error");
        assert!(
            err.to_string().contains("validated pairs"),
            "unexpected error: {err}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sufficiently_sized_train_jsonl_passes() {
        let dir = std::env::temp_dir().join(format!(
            "vox-corpus-threshold-test-ok-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let lines = "{}\n".repeat(MIN_CORPUS_PAIRS);
        std::fs::write(dir.join("train.jsonl"), lines).unwrap();
        check_corpus_threshold(&dir).expect("corpus at threshold must pass");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[allow(
    clippy::too_many_arguments,
    unused_variables,
    unused_assignments,
    unsafe_code
)]

fn measured_qlora_row<'a>(
    overlay: &'a std::collections::HashMap<
        String,
        Vec<vox_populi::mens::tensor::spoke_base_resolver::TrainBase>,
    >,
    hf_id: &str,
) -> Option<&'a vox_populi::mens::tensor::spoke_base_resolver::TrainBase> {
    overlay.get("qwen3_code")?.iter().find(|base| {
        base.hf_id == hf_id
            && base.floor_measured
            && base.floor_mb.is_some()
            && base.methods.iter().any(|method| method == "qlora")
    })
}

/// A live free-memory reading is required only for an automatic Qwen3 pick,
/// an explicit size spelling, or a measured `qwen3_code` QLoRA row on this
/// exact Hugging Face id. A concrete id is not a reading just because its
/// name contains `Qwen3-`.
pub(crate) fn model_needs_accelerator_reading(
    model: &str,
    automatic_qwen3: bool,
    overlay: &std::collections::HashMap<
        String,
        Vec<vox_populi::mens::tensor::spoke_base_resolver::TrainBase>,
    >,
) -> bool {
    automatic_qwen3
        || vox_populi::mens::tensor::spoke_base_resolver::is_qwen3_size_spelling(model)
        || measured_qlora_row(overlay, model).is_some()
}

fn pick_from_measured_row(
    row: &vox_populi::mens::tensor::spoke_base_resolver::TrainBase,
    free_mb: u32,
) -> anyhow::Result<vox_populi::mens::tensor::spoke_base_resolver::Qwen3LivePick> {
    use vox_populi::mens::tensor::spoke_base_resolver::Qwen3LivePick;

    let repo = row.hf_id.split('@').next().unwrap_or(row.hf_id.as_str());
    let token = ["32B", "14B", "0.6B", "8B"]
        .into_iter()
        .find(|size| repo.ends_with(&format!("Qwen3-{size}")))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "measured qwen3_code QLoRA row '{}' is not a dense Qwen3 size",
                row.hf_id
            )
        })?;
    let preset = match token {
        "0.6B" => "qwen3_dev_cpu",
        "8B" => "qwen3_16g",
        "14B" => "qwen3_24g",
        "32B" => "qwen3_96g",
        other => anyhow::bail!("no qwen3_code QLoRA preset for size {other}"),
    };
    let floor_mb = row.floor_mb.ok_or_else(|| {
        anyhow::anyhow!(
            "measured qwen3_code QLoRA row '{}' has no floor_mb",
            row.hf_id
        )
    })?;
    Ok(Qwen3LivePick {
        hf_id: row.hf_id.clone(),
        size_token: token.to_string(),
        preset: preset.to_string(),
        floor_mb,
        free_mb,
    })
}

/// Re-check a resolved size against a free-memory reading the caller already took.
/// Does not read the accelerator. Automatic picks may step down. Explicit Qwen3
/// sizes and measured `qwen3_code` QLoRA pins are refused, never replaced.
pub(crate) fn apply_preload_free_mb(
    model: &str,
    preset: &str,
    automatic: bool,
    free_mb: u32,
) -> anyhow::Result<(
    String,
    String,
    Option<vox_populi::mens::tensor::spoke_base_resolver::PreloadRevision>,
)> {
    use vox_populi::mens::tensor::spoke_base_resolver::{
        SizeChoice, confirm_before_load, explicit_qwen3_size_for_free_mb, is_qwen3_size_spelling,
        load_overlay,
    };

    let root = vox_corpus::training::contract::find_workspace_root().ok_or_else(|| {
        anyhow::anyhow!("could not find workspace root for the qwen3 preload check")
    })?;
    let overlay = load_overlay(&root)?;
    if !automatic && !is_qwen3_size_spelling(model) && measured_qlora_row(&overlay, model).is_none()
    {
        return Ok((model.to_string(), preset.to_string(), None));
    }
    let pick = if automatic {
        let row = measured_qlora_row(&overlay, model).ok_or_else(|| {
            anyhow::anyhow!("automatic qwen3 pick '{model}' is not a measured qwen3_code QLoRA row")
        })?;
        pick_from_measured_row(row, free_mb)?
    } else if is_qwen3_size_spelling(model) {
        explicit_qwen3_size_for_free_mb(&overlay, model, free_mb)?
    } else {
        let row = measured_qlora_row(&overlay, model)
            .ok_or_else(|| anyhow::anyhow!("no measured qwen3_code QLoRA row for '{model}'"))?;
        pick_from_measured_row(row, free_mb)?
    };
    let choice = if automatic {
        SizeChoice::Automatic(pick)
    } else {
        SizeChoice::Explicit(pick)
    };
    let mut revision = confirm_before_load(&overlay, &choice, free_mb)?;
    // Same size: the caller's preset is the one they asked to train with.
    // A real step-down uses the smaller size's preset.
    if revision.loaded_size == revision.original_size && !preset.is_empty() {
        revision.loaded_preset = preset.to_string();
    }
    Ok((
        revision.loaded_hf_id.clone(),
        revision.loaded_preset.clone(),
        Some(revision),
    ))
}

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
    budget_seq_len: Option<usize>,
    budget_batch_size: Option<usize>,
    budget_grad_accum: Option<usize>,
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
    context_filter: Option<vox_populi::mens::tensor::training_config::ContextFilter>,
    validation_split_ratio: Option<f64>,
    tokenizer_mode: vox_populi::mens::MensTokenizerMode,
    qlora_no_double_quant: bool,
    qlora_require_full_proxy_stack: bool,
    qlora_allow_partial_proxy_stack: bool,
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
    curriculum_schedule: Option<vox_populi::mens::tensor::training_config::CurriculumSchedule>,
    chatml: vox_populi::mens::tensor::training_config::ChatmlConfig,
    mix_config: Option<PathBuf>,
    automatic_qwen3: bool,
) -> Result<Option<vox_populi::mens::tensor::spoke_base_resolver::PreloadRevision>> {
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
    if qlora_allow_partial_proxy_stack && qlora_require_full_proxy_stack {
        anyhow::bail!(
            "`--qlora-allow-partial-proxy-stack` cannot be combined with `--qlora-require-full-proxy-stack`."
        );
    }
    if let Some(r) = qlora_max_skip_rate
        && (!r.is_finite() || !(0.0..=1.0).contains(&r))
    {
        anyhow::bail!("--qlora-max-skip-rate must be between 0.0 and 1.0 (got {r})");
    }

    let gpu_info = vox_populi::mens::probe_gpu();
    let workspace_root = vox_corpus::training::contract::find_workspace_root();

    let mut model = model;
    let mut preset = preset;
    let mut automatic_qwen3 = automatic_qwen3;
    let env_default_model = std::env::var("VOX_MENS_DEFAULT_MODEL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if model.is_none()
        && let Some(pinned) = env_default_model.clone()
    {
        // A named pin is explicit: preload must refuse, not step down.
        model = Some(pinned);
        automatic_qwen3 = false;
    }
    if automatic_qwen3 && model.is_none() {
        let free_mb = vox_populi::mens::tensor::spoke_base_resolver::read_accelerator_free_mb()?;
        let pick =
            vox_populi::mens::tensor::spoke_base_resolver::select_qwen3_code_qlora_for_free_mb(
                free_mb,
            )?;
        tracing::info!(
            model = %pick.hf_id,
            size = %pick.size_token,
            free_mb,
            "Selected measured qwen3_code QLoRA size from free memory."
        );
        model = Some(pick.hf_id);
        preset = Some(pick.preset);
    } else {
        if let Some(resolved) =
            vox_populi::mens::tensor::spoke_base_resolver::maybe_resolve_metal_default_base(
                model.as_deref(),
                env_default_model.as_deref(),
                &gpu_info.vendor,
                matches!(
                    device_kind,
                    vox_populi::mens::DeviceKind::Best | vox_populi::mens::DeviceKind::Metal
                ),
                workspace_root.as_deref(),
                gpu_info.vram_mb,
            )?
        {
            tracing::info!(
                model = %resolved,
                vram_mb = gpu_info.vram_mb,
                "Using Metal-resolved HF model (`--model` omitted; agentic_default ladder)."
            );
            model = Some(resolved);
        }
        if matches!(
            train_backend,
            vox_populi::mens::PopuliTrainBackend::CandleQlora
        ) && model.is_none()
        {
            let resolved = vox_populi::mens::resolve_default_model_id(env_default_model.as_deref());
            tracing::info!(
                model = %resolved,
                "Using default HF model for Candle QLoRA (`--model` omitted; see contracts/mens/training-presets.v1.yaml)."
            );
            model = Some(resolved);
        }
    }
    let mut preload_revision = None;
    if let Some(current) = model.clone() {
        let root = vox_corpus::training::contract::find_workspace_root().ok_or_else(|| {
            anyhow::anyhow!("could not find workspace root for the qwen3 preload check")
        })?;
        let overlay = vox_populi::mens::tensor::spoke_base_resolver::load_overlay(&root)?;
        let needs_reading = model_needs_accelerator_reading(&current, automatic_qwen3, &overlay);
        let free_at_load = if needs_reading {
            vox_populi::mens::tensor::spoke_base_resolver::read_accelerator_free_mb()?
        } else {
            0
        };
        let (loaded_id, loaded_preset, revision) = apply_preload_free_mb(
            &current,
            preset.as_deref().unwrap_or(""),
            automatic_qwen3,
            free_at_load,
        )?;
        if let Some(revision) = revision {
            tracing::info!(
                original_size = %revision.original_size,
                loaded_size = %revision.loaded_size,
                free_mb = revision.free_mb,
                "pre-load free-memory revision"
            );
            model = Some(loaded_id);
            preset = Some(loaded_preset);
            preload_revision = Some(revision);
        }
    }

    let effective_qlora_require_full_proxy_stack = !qlora_allow_partial_proxy_stack
        && (qlora_require_full_proxy_stack
            || (matches!(
                train_backend,
                vox_populi::mens::PopuliTrainBackend::CandleQlora
            ) && matches!(device_kind, vox_populi::mens::DeviceKind::Cuda)));

    if effective_qlora_require_full_proxy_stack && qlora_lm_head_only {
        anyhow::bail!(
            "Full proxy stack (CUDA default or `--qlora-require-full-proxy-stack`) conflicts with `--qlora-lm-head-only`; pick one."
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
                 Rebuild: `cargo build -p vox-ml-cli --features gpu,mens-candle-cuda` (or `cargo vox-cuda-release`).\n\
                 On Windows use a VS Developer shell so `nvcc` can find MSVC."
            );
        }
        // Make sure the plugin the selector will dispatch to is installed and
        // loadable, self-healing a missing/stale one (opt out with
        // `--no-auto-heal` / VOX_MENS_NO_AUTO_HEAL). Same selector the training
        // dispatch in vox-populi uses, so they can't disagree. Skip the heal
        // where no release asset exists for this platform (it would try a
        // download that can't succeed); dispatch then fails at load with a
        // build-from-source hint.
        #[cfg(feature = "gpu")]
        {
            let plugin_id =
                vox_populi::mens::select_mens_backend(device_kind, &vox_plugin_host::probe());
            if vox_populi::mens::has_prebuilt_artifact(plugin_id) {
                crate::commands::mens::plugin_heal::ensure_plugin(plugin_id, true)?;
            }
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
        qlora_allow_partial_proxy_stack,
        effective_qlora_require_full_proxy_stack,
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
        ?trajectory_failure_category_boost,
        ?trajectory_quality_floor,
        trajectory_quality_boost,
        ?context_filter,
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

    let device_profile = vox_populi::mens::DeviceProfile::from_gpu_info(
        &gpu_info.model_name,
        gpu_info.vram_mb,
        &gpu_info.vendor,
    );
    let cli_overrides = vox_populi::mens::CliOverrides {
        rank,
        alpha,
        seq_len,
        batch_size,
        grad_accum,
        epochs,
        warmup,
        lr,
        budget_seq_len,
        budget_batch_size,
        budget_grad_accum,
        vram_limit_fraction,
    };
    let preview_profile = vox_populi::mens::resolve_effective_profile(
        preset.as_deref(),
        device_profile.clone(),
        None,
        cli_overrides.clone(),
    )?;

    tracing::debug!(
        model = ?model,
        device = %device,
        rank = preview_profile.rank,
        alpha = preview_profile.alpha,
        "Dispatching training payload to native orchestra"
    );

    check_corpus_threshold(&data_dir)?;

    eprintln!("{}", "╔══════════════════════════════════════════╗".cyan());
    eprintln!("{}", "║   VoxMens — native fine-tuning (QLoRA)  ║".cyan());
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

    // `return` is load-bearing in the non-gpu build, where a cfg(not) block follows.
    #[cfg(feature = "gpu")]
    #[allow(clippy::needless_return)]
    {
        super::gpu::run_gpu_training(
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
            effective_qlora_require_full_proxy_stack,
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
            curriculum_schedule,
            chatml,
            mix_config,
        )
        .await?;
        return Ok(preload_revision);
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
            qlora_allow_partial_proxy_stack,
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
        Ok(preload_revision)
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn metal_qlora_error_is_not_the_old_dead_gate_message() {
        let source = include_str!("run_train.rs");
        let old_gate = ["`--device metal` for Candle QLoRA", " is not supported yet"].concat();
        assert!(
            !source.contains(&old_gate),
            "Metal QLoRA must no longer use the old dead-gate error"
        );
    }
}

#[cfg(test)]
mod preload_tests {
    use super::apply_preload_free_mb;

    #[test]
    fn concrete_id_with_qwen3_in_the_name_does_not_require_a_live_reading() {
        let root = vox_corpus::training::contract::find_workspace_root().expect("workspace");
        let overlay =
            vox_populi::mens::tensor::spoke_base_resolver::load_overlay(&root).expect("overlay");
        let measured = "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218";
        assert!(!super::model_needs_accelerator_reading(
            "org/Qwen3-8B-finetune",
            false,
            &overlay
        ));
        assert!(!super::model_needs_accelerator_reading(
            "org/Manual",
            false,
            &overlay
        ));
        assert!(super::model_needs_accelerator_reading(
            "org/Manual",
            true,
            &overlay
        ));
        assert!(super::model_needs_accelerator_reading(
            "8B", false, &overlay
        ));
        assert!(super::model_needs_accelerator_reading(
            "Qwen3-8B", false, &overlay
        ));
        assert!(super::model_needs_accelerator_reading(
            measured, false, &overlay
        ));
        let needle = format!("repo.{}(\"Qwen3-\")", "contains");
        assert!(
            !include_str!("run_train.rs").contains(&needle),
            "needs_reading must not substring-match Qwen3-"
        );
    }

    use vox_populi::mens::tensor::spoke_base_resolver::select_qwen3_code_qlora_for_free_mb;

    #[test]
    fn apply_preload_free_mb_keeps_caller_preset_when_the_size_stays() {
        let eight = select_qwen3_code_qlora_for_free_mb(16_384).expect("8B");
        let (loaded_id, loaded_preset, revision) =
            apply_preload_free_mb(&eight.hf_id, "a100", true, 16_384).expect("kept");
        assert_eq!(loaded_id, eight.hf_id);
        assert_eq!(loaded_preset, "a100");
        let revision = revision.expect("revision");
        assert_eq!(revision.loaded_size, "8B");
        assert_eq!(revision.original_size, "8B");
        assert_eq!(revision.loaded_preset, "a100");
    }

    #[test]
    fn apply_preload_free_mb_steps_an_automatic_32b_pick_down() {
        let big = select_qwen3_code_qlora_for_free_mb(70_000).expect("32B fits 70000");
        assert_eq!(big.size_token, "32B");
        assert_eq!(big.preset, "qwen3_96g");
        let (loaded_id, loaded_preset, revision) =
            apply_preload_free_mb(&big.hf_id, "qwen3_96g", true, 25_000).expect("step down");
        let fourteen = select_qwen3_code_qlora_for_free_mb(25_000).expect("14B fits 25000");
        assert_eq!(loaded_id, fourteen.hf_id);
        assert_eq!(loaded_preset, "qwen3_24g");
        let revision = revision.expect("revision");
        assert_eq!(revision.original_size, "32B");
        assert_eq!(revision.loaded_size, "14B");
        assert_eq!(revision.free_mb, 25_000);
        assert_eq!(revision.loaded_hf_id, fourteen.hf_id);
    }

    #[test]
    fn apply_preload_free_mb_refuses_an_explicit_32b_pin() {
        let big = select_qwen3_code_qlora_for_free_mb(70_000).expect("32B");
        let fourteen = select_qwen3_code_qlora_for_free_mb(25_000).expect("14B");
        let err = apply_preload_free_mb(&big.hf_id, "qwen3_96g", false, 25_000)
            .expect_err("explicit 32B must not step down");
        let msg = format!("{err:#}");
        assert!(
            !msg.contains(&fourteen.hf_id),
            "explicit refusal must not substitute 14B: {msg}"
        );
    }

    #[test]
    fn apply_preload_free_mb_leaves_a_manual_id_unchanged() {
        let (id, preset, revision) =
            apply_preload_free_mb("org/Manual", "kept", false, 25_000).expect("passthrough");
        assert_eq!(id, "org/Manual");
        assert_eq!(preset, "kept");
        assert!(revision.is_none());
    }

    #[test]
    fn apply_preload_free_mb_errors_when_nothing_measured_fits() {
        let big = select_qwen3_code_qlora_for_free_mb(70_000).expect("32B");
        apply_preload_free_mb(&big.hf_id, "qwen3_96g", true, 1_500)
            .expect_err("1500 MB fits no measured QLoRA row");
    }

    #[test]
    fn run_train_checks_preload_before_select_mens_backend() {
        let src = include_str!("run_train.rs");
        let start = src.find("pub async fn run_train(").expect("run_train");
        let rest = &src[start..];
        let backend = rest
            .find("select_mens_backend")
            .expect("select_mens_backend");
        let window = &rest[..backend];
        assert!(
            window.contains("apply_preload_free_mb") || window.contains("confirm_before_load"),
            "run_train must re-check free memory before it selects a train backend"
        );

        let dispatch = include_str!("../../mens/populi/dispatch.rs");
        let dog = dispatch.find("PopuliAction::Dogfood").expect("dogfood");
        let dog_end = dispatch[dog..]
            .find("PopuliAction::Train")
            .expect("train arm");
        let dog_window = &dispatch[dog..dog + dog_end];
        assert!(
            dog_window.contains("None,"),
            "dogfood passes no model so the unnamed-size selector runs"
        );
        assert!(
            dog_window.contains("automatic_qwen3"),
            "dogfood must pass automatic true"
        );

        let pipeline = include_str!("../../mens/pipeline.rs");
        assert!(
            pipeline.contains("model.is_none() && env_default_model.is_none()")
                && pipeline.contains("VOX_MENS_DEFAULT_MODEL"),
            "pipeline passes automatic true only when the CLI model and the env pin are absent"
        );

        let train_arm = include_str!("../../mens/populi/train_arm.rs");
        assert!(
            train_arm.contains("VOX_MENS_DEFAULT_MODEL")
                && train_arm.contains("model.is_none() && env_default_model.is_none()"),
            "train_arm treats a non-empty env pin as explicit, not automatic"
        );
        let pin_at = window
            .find("VOX_MENS_DEFAULT_MODEL")
            .expect("run_train reads the env pin");
        let pick_at = window
            .find("select_qwen3_code_qlora_for_free_mb")
            .expect("live pick");
        assert!(
            pin_at < pick_at,
            "the env pin is applied before the live free-memory pick"
        );
        assert!(
            train_arm.contains("automatic_qwen3,"),
            "train_arm threads that flag into schola run_train"
        );
    }
}
