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

/// Live free accelerator memory for this run's device. A CPU run has no
/// accelerator reading (D-01).
pub(super) fn live_free_memory(
    device_kind: vox_populi::mens::DeviceKind,
) -> Option<vox_populi::mens::tensor::size_select::FreeMemory> {
    match device_kind {
        vox_populi::mens::DeviceKind::Cpu => None,
        _ => vox_populi::mens::tensor::size_select::read_free_memory(),
    }
}

/// The run's explicit size (D-04): `--model`, else a non-blank
/// `VOX_MENS_DEFAULT_MODEL`.
fn explicit_model(cli: Option<String>, env: Option<String>) -> Option<String> {
    cli.or_else(|| env.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
}

/// The run's one size pick (D-06): the largest Qwen3 size that fits live free
/// memory, or the operator's explicit size (D-04).
pub(super) fn pick_train_size(
    explicit: Option<&str>,
    workspace_root: Option<&Path>,
    device_kind: vox_populi::mens::DeviceKind,
) -> Result<vox_populi::mens::tensor::size_select::Selection> {
    use vox_populi::mens::tensor::size_select;
    let overlay = size_select::load_ladder_overlay(workspace_root)?;
    size_select::select_size(
        &overlay,
        live_free_memory(device_kind).as_ref(),
        explicit,
        force_train_env(),
    )
}

/// Re-read free memory once right before the download (D-05): an automatic
/// pick that no longer fits steps down; an explicit size is never stepped down.
/// Returns the reading it used, for `--plan-only`.
pub(super) fn recheck_size_pick(
    pick: vox_populi::mens::tensor::size_select::SizePick,
    workspace_root: Option<&Path>,
    device_kind: vox_populi::mens::DeviceKind,
) -> Result<(
    vox_populi::mens::tensor::size_select::SizePick,
    Option<vox_populi::mens::tensor::size_select::FreeMemory>,
)> {
    use owo_colors::OwoColorize;
    use vox_populi::mens::tensor::size_select;
    let overlay = size_select::load_ladder_overlay(workspace_root)?;
    let reading = live_free_memory(device_kind);
    let pick =
        size_select::recheck_before_load(pick, &overlay, reading.as_ref(), force_train_env())?;
    eprintln!("  {} size: {}", "ℹ".cyan(), pick.note);
    Ok((pick, reading))
}

/// `VOX_MENS_FORCE_TRAIN=1`/`true` — the registered operator override
/// meaning "proceed past a failing gate": the explicit-size fit refusal here
/// and the post-download `plan_for`/`sweep` VRAM-fit refusal in `gpu.rs`,
/// which runs for every CUDA run regardless of how sizing was chosen. `vox mens
/// serve` honors it for its fit check too.
pub(crate) fn force_train_env() -> bool {
    std::env::var("VOX_MENS_FORCE_TRAIN")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

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
    opts: super::TrainRunOpts,
) -> Result<Option<vox_populi::mens::tensor::size_select::SizePick>> {
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

    // One live size pick per run (D-01, D-04, D-06); no default size.
    let mut model = model;
    let mut size_pick = None;
    if matches!(
        train_backend,
        vox_populi::mens::PopuliTrainBackend::CandleQlora
    ) {
        let explicit = explicit_model(model.take(), std::env::var("VOX_MENS_DEFAULT_MODEL").ok());
        match pick_train_size(explicit.as_deref(), workspace_root.as_deref(), device_kind)? {
            vox_populi::mens::tensor::size_select::Selection::Size(pick) => {
                eprintln!("  {} size: {}", "ℹ".cyan(), pick.note);
                model = Some(pick.rung.hf_id.clone());
                size_pick = Some(pick);
            }
            vox_populi::mens::tensor::size_select::Selection::PassThrough(id) => model = Some(id),
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
    ) && matches!(device_kind, vox_populi::mens::DeviceKind::Cuda)
    {
        #[cfg(not(feature = "mens-candle-cuda"))]
        anyhow::bail!(
            "`--device cuda` for Candle QLoRA requires a CUDA-enabled build.\n\
             Rebuild: `cargo build -p vox-ml-cli --features gpu,mens-candle-cuda` (or `cargo vox-cuda-release`).\n\
             On Windows use a VS Developer shell so `nvcc` can find MSVC."
        );
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
        preset
            .as_deref()
            .or(size_pick.as_ref().map(|p| p.rung.preset)),
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

    // Make sure the plugin the selector will dispatch to is installed and
    // loadable, self-healing a missing/stale one (opt out with
    // `--no-auto-heal` / VOX_MENS_NO_AUTO_HEAL). Same selector the training
    // dispatch in vox-populi uses, so they can't disagree. Skip the heal
    // where no release asset exists for this platform (it would try a
    // download that can't succeed); dispatch then fails at load with a
    // build-from-source hint. Runs after the corpus check, so a missing corpus
    // is refused the same way with or without --plan-only, whatever state the
    // installed plugin is in. The heal can download a release asset; plan-only
    // never runs it (D-02).
    #[cfg(feature = "gpu")]
    if matches!(
        train_backend,
        vox_populi::mens::PopuliTrainBackend::CandleQlora
    ) && !opts.plan_only
    {
        let plugin_id =
            vox_populi::mens::select_mens_backend(device_kind, &vox_plugin_host::probe());
        if vox_populi::mens::has_prebuilt_artifact(plugin_id) {
            crate::commands::mens::plugin_heal::ensure_plugin(plugin_id, true)?;
        }
    }

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
            size_pick,
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
            opts,
        )
        .await
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
            opts,
        );
        eprintln!(
            "  {} LoRA training requires the 'gpu' feature.",
            "⚠".yellow()
        );
        eprintln!("  Rebuild with: cargo build --features gpu");
        eprintln!();
        eprintln!("  Canonical QLoRA (when `gpu` is enabled): `vox mens train --backend qlora …`");
        eprintln!("  See docs/src/reference/mens-training.md");
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        explicit_model, force_train_env, live_free_memory, pick_train_size, recheck_size_pick,
    };
    use std::path::{Path, PathBuf};
    use vox_populi::mens::DeviceKind;
    use vox_populi::mens::tensor::size_select::{self, FreeMemory, Selection};

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// WR-05: `--model` beats `VOX_MENS_DEFAULT_MODEL`; a blank env value is unset.
    #[test]
    fn cli_model_beats_the_default_model_env() {
        let s = |v: &str| Some(v.to_string());
        assert_eq!(explicit_model(s("Qwen3-4B"), s("Qwen3-14B")), s("Qwen3-4B"));
        assert_eq!(explicit_model(None, s(" Qwen3-14B ")), s("Qwen3-14B"));
        for blank in ["", "   "] {
            assert_eq!(explicit_model(None, s(blank)), None, "{blank:?}");
        }
        assert_eq!(explicit_model(None, None), None);
    }

    #[test]
    fn cpu_training_has_no_accelerator_reading() {
        assert_eq!(live_free_memory(DeviceKind::Cpu), None);
    }

    #[test]
    fn cpu_auto_pick_refuses_and_names_the_way_out() {
        let err = pick_train_size(None, Some(&root()), DeviceKind::Cpu)
            .expect_err("no reading on CPU must refuse")
            .to_string();
        assert!(err.contains("no live free-memory reading"), "{err}");
        assert!(err.contains("--model"), "{err}");
    }

    #[test]
    fn cpu_explicit_size_is_kept_unchecked() {
        let Selection::Size(p) =
            pick_train_size(Some("Qwen3-0.6B"), Some(&root()), DeviceKind::Cpu).expect("kept")
        else {
            panic!("a ladder size must not pass through");
        };
        assert!(
            p.rung.hf_id.starts_with("Qwen/Qwen3-0.6B@"),
            "{}",
            p.rung.hf_id
        );
        assert_eq!(p.rung.preset, "qwen3_dev_cpu");
        assert!(p.explicit);
        assert!(p.note.contains("fit not checked"), "{}", p.note);
    }

    #[test]
    fn cpu_concrete_id_passes_through() {
        let got = pick_train_size(Some("org/Manual"), Some(&root()), DeviceKind::Cpu).expect("ok");
        assert!(
            matches!(got, Selection::PassThrough(ref id) if id == "org/Manual"),
            "{got:?}"
        );
    }

    #[test]
    fn recheck_of_an_auto_pick_without_a_reading_refuses() {
        let overlay = size_select::load_ladder_overlay(Some(&root())).expect("overlay");
        let floor_8b = overlay
            .get("qwen3_code")
            .and_then(|rungs| {
                rungs.iter().find(|b| {
                    b.hf_id.starts_with("Qwen/Qwen3-8B@") && b.methods.iter().any(|m| m == "qlora")
                })
            })
            .map(|b| b.floor_mb)
            .expect("8B QLoRA rung");
        let free = FreeMemory {
            mb: u64::from(floor_8b) + 1_000,
            source: "test",
        };
        let Selection::Size(pick) =
            size_select::select_size(&overlay, Some(&free), None, false).expect("8B fits")
        else {
            panic!("automatic pick is a size");
        };
        assert!(!pick.explicit);
        let err = recheck_size_pick(pick, Some(&root()), DeviceKind::Cpu)
            .expect_err("no reading at re-check must refuse")
            .to_string();
        assert!(err.contains("no live free-memory reading"), "{err}");
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

    #[test]
    #[cfg(target_os = "macos")]
    fn metal_qlora_error_is_not_the_old_dead_gate_message() {
        let source = include_str!("run_train.rs");
        let old_gate = ["`--device metal` for Candle QLoRA", " is not supported yet"].concat();
        assert!(
            !source.contains(&old_gate),
            "Metal QLoRA must no longer use the old dead-gate error"
        );
    }
}
