//! `PopuliAction::Train` implementation (corpus preflight + `schola::train`).

use std::path::{Path, PathBuf};

use super::action::{
    MensTokenizerCli, PopuliTrainBackendCli, TrainDataModeCli, TrainingDeploymentTargetCli,
};
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
    domain: Option<String>,
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
    qlora_allow_partial_proxy_stack: bool,
    qlora_lm_head_only: bool,
    qlora_max_skip_rate: Option<f32>,
    qlora_proxy_max_layers: Option<usize>,
    qlora_ce_last_k: usize,
    checkpoint_every: Option<usize>,
    force_restart: bool,
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
    cloud: String,
    _max_budget: Option<f64>,
    _train_data_hf: Option<String>,
    _adapter_upload_hf: Option<String>,
    _max_runtime_secs: Option<u64>,
    validation_split_ratio: f64,
    curriculum: bool,
    optimizer_experiment_mode: vox_populi::mens::OptimizerExperimentMode,
    data_mode: TrainDataModeCli,
    fast_corpus: bool,
    #[cfg_attr(not(feature = "cloud"), allow(unused_variables))] persistent: bool,
) -> anyhow::Result<()> {
    if cloud != "local" {
        #[cfg(feature = "cloud")]
        {
            use vox_populi::mens::cloud::{
                CloudJobSpec, CloudOrchestrationOutcome, CloudResolver, TrainingManifest,
                check_spend_gate, post_training_flow,
            };
            use vox_populi::mens::tensor::domain_router::DomainRouter;

            // B5.4: dry-run / ALLOW_SPEND gate (must come before any network call)
            if let Some(gate_outcome) = check_spend_gate(false) {
                match gate_outcome {
                    CloudOrchestrationOutcome::DryRunPrinted => {
                        println!("Cloud training dry-run: plan printed. No provisioning.");
                        return Ok(());
                    }
                    CloudOrchestrationOutcome::SpendNotAllowed => {
                        println!(
                            "Cloud training requires VOX_MENS_ALLOW_SPEND=1 to provision.\n\
                             Set the variable and re-run to start billing."
                        );
                        return Ok(());
                    }
                    _ => {}
                }
            }

            let effective_domain = domain.as_deref().unwrap_or("vox");
            let workspace_root = vox_corpus::training::contract::find_workspace_root();

            // BLOCKER 4 + DEFAULT BASE: resolve the domain spoke base via the REAL
            // resolver instead of falling back to the legacy default_model_id().
            // An unresolved/bare default resolves to a Qwen3 agentic_default rung
            // (DEFAULT_MODEL_ID is now Qwen3-8B).
            let (resolved_hf_id, resolved_rung, resolved_quant) = resolve_cloud_spoke_base(
                workspace_root.as_deref(),
                Some(effective_domain),
                model.as_deref(),
                preset.as_deref(),
            )?;

            // BLOCKER 3: fail-closed placeholder guard on the real (--apply) path.
            // This fires BEFORE any provisioning/dispatch. A dry-run would have
            // already returned above via check_spend_gate.
            vox_populi::mens::tensor::spoke_base_resolver::ensure_not_placeholder(&resolved_hf_id)?;

            // BLOCKER 2: derive base_revision from the resolved @sha and rung from
            // the resolved preset/VRAM rung (no fabricated "main"/"cloud").
            let base_revision = resolved_hf_id
                .split_once('@')
                .map(|(_, rev)| rev.to_string())
                .unwrap_or_else(|| "main".to_string());

            let config = vox_populi::mens::cloud::CloudProviderConfig::default();
            let mut spec = CloudJobSpec::new_train(&config);
            spec.model_id = resolved_hf_id.clone();
            spec.train_data_hf = _train_data_hf;
            spec.adapter_upload_hf = _adapter_upload_hf.clone();
            spec.max_budget_usd = _max_budget;
            spec.max_runtime_secs = _max_runtime_secs;
            spec.preset = preset.clone().unwrap_or_else(|| resolved_rung.clone());
            spec.seq_len = seq_len.unwrap_or(512);
            spec.batch_size = batch_size.unwrap_or(4);
            spec.epochs = epochs.unwrap_or(3);
            // Sizes the runtime estimate and the 1.5x watchdog kill; a fixed 5000
            // under-counted the real 9,268-row corpus.
            spec.num_samples =
                jsonl_row_count(&data_dir.join("train.jsonl")).unwrap_or_else(|| {
                    eprintln!(
                        "  [cloud] no local train.jsonl to count; assuming 5000 rows for sizing"
                    );
                    5000
                });
            spec.persistent = persistent;

            // Corpus hash for the idempotency key (stable per input corpus).
            let corpus_hash = workspace_root
                .as_deref()
                .map(vox_corpus::corpus::preflight::compute_corpus_fingerprint)
                .unwrap_or_default();

            let resolver = CloudResolver::new_from_env().await?;

            // B5 WIRING: route the cloud --apply path through idempotency +
            // TerminateOnDrop so a spot preemption / mid-flight error cleans up the
            // pod and a re-run with the same spoke+corpus does not double-provision.
            // We wire idempotency + TerminateOnDrop here over the resolver's public
            // surface (resolve + dispatch_top). Full checkpoint-resume is deferred.
            // TODO(b5-resume): wire sync_checkpoint_down / read_checkpoint_uri so a
            // preempted job resumes from its last checkpoint instead of restarting.
            let idem_key =
                vox_populi::mens::cloud::make_idempotency_key(effective_domain, &corpus_hash);
            let state_dir = workspace_root
                .as_deref()
                .map(|r| r.join("mens/runs/cloud-state"))
                .unwrap_or_else(|| PathBuf::from("mens/runs/cloud-state"));
            std::fs::create_dir_all(&state_dir).ok();
            let key_path = state_dir.join(format!("{idem_key}.active_job"));

            let _guard: Option<vox_populi::mens::cloud::TerminateOnDrop> = if key_path.exists() {
                // Idempotency: a job for this spoke+corpus is already in flight.
                // Do NOT provision a second pod and do NOT arm a guard for a job
                // this caller does not own (F7 semantics at the call site).
                eprintln!(
                    "  Reusing in-flight cloud job for '{effective_domain}' (idempotency key present); not re-provisioning."
                );
                None
            } else {
                // Same threshold logic `vox mens cloud-estimate` uses (Task 3) and
                // `CloudResolver::dispatch` uses for Train jobs, so the read-only
                // estimate and every real dispatch path agree on whether an offer
                // is big enough. No local model directory exists yet on a
                // from-scratch cloud run, so this sizes from Hub API metadata
                // (one small request, no weight bytes) rather than
                // `ModelShape::from_model_dir`.
                let min_vram_mb = vox_populi::mens::cloud::min_vram_mb_for_training(
                    &spec.model_id,
                    spec.batch_size,
                    spec.seq_len,
                )
                .await?;
                let (ranked, rejected) = resolver
                    .resolve(&vox_populi::mens::cloud::ResolveRequest {
                        target: std::str::FromStr::from_str(&cloud)?,
                        min_vram_mb,
                        max_acceptable_cost: spec
                            .max_budget_usd
                            .unwrap_or(resolver.config.max_budget_usd),
                        seq_len: spec.seq_len,
                        batch_size: spec.batch_size,
                        num_samples: spec.num_samples,
                        epochs: spec.epochs,
                    })
                    .await?;
                if ranked.is_empty() && !rejected.is_empty() {
                    // Every real dispatch path used to lose these reasons in
                    // a `tracing::debug!` the operator never sees — surface
                    // them here, at the one point they're about to be told
                    // only "no offers to dispatch" while spending money.
                    eprintln!("  ⚠ No suitable cloud offers — every candidate was rejected:");
                    for (offer_id, reason) in &rejected {
                        eprintln!("      {offer_id}: {reason}");
                    }
                }
                let (handle, watchdog, provider) = resolver.dispatch_top(&ranked, &spec).await?;
                // Record the idempotency key so a concurrent / retried invocation
                // detects the in-flight job and reuses it.
                std::fs::write(&key_path, &handle.job_id).ok();
                // Arm TerminateOnDrop for this fresh provision.
                let guard = vox_populi::mens::cloud::TerminateOnDrop::new(
                    provider,
                    handle,
                    std::sync::Arc::clone(&resolver.budget),
                );
                // Wait for the watchdog (job completion / kill); on error the guard
                // drops armed and terminates the orphaned pod.
                watchdog
                    .await
                    .map_err(|e| anyhow::anyhow!("Cloud watchdog task failed: {e}"))?;
                Some(guard)
            };

            let adapter_dir = output_dir.clone();
            let repo_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let git_sha = vox_git::read_only(&repo_root, &["rev-parse", "--short", "HEAD"])
                .ok()
                .unwrap_or_default()
                .trim()
                .to_string();

            let manifest = TrainingManifest {
                base_hf_id: resolved_hf_id.clone(),
                base_revision,
                rung: resolved_rung.clone(),
                quantization: resolved_quant.clone(),
                preset: spec.preset.clone(),
                rank: rank.unwrap_or(16) as u32,
                alpha: alpha.unwrap_or(32.0),
                seed,
                corpus_hash,
                metrics: serde_json::json!({}),
                cost_usd: _max_budget.unwrap_or(0.0),
                provider: cloud.clone(),
                git_sha,
                created_at: chrono::Utc::now().to_rfc3339(),
            };

            // BLOCKER 1: run the REAL eval gate on the trained/downloaded adapter
            // and map the result to PassedBase / BelowBase / EvalError. Fail-closed:
            // any eval error → EvalError → adapter is NOT registered.
            let eval_outcome = run_cloud_eval_gate(&adapter_dir, workspace_root.as_deref());
            let mut router = DomainRouter::new();
            let outcome = post_training_flow(
                eval_outcome,
                &mut router,
                effective_domain,
                &adapter_dir,
                &manifest,
            )?;

            // Training reached a terminal, gated outcome; idempotency key may be
            // cleared so a deliberate re-train can provision again.
            let _ = std::fs::remove_file(&key_path);

            match outcome {
                CloudOrchestrationOutcome::RegisteredChallenger { ref domain, .. } => {
                    use owo_colors::OwoColorize;
                    eprintln!(
                        "  {} Cloud adapter registered as challenger for domain: {}",
                        "✓".green(),
                        domain.cyan()
                    );
                }
                CloudOrchestrationOutcome::EvalGateFailed { ref domain } => {
                    use owo_colors::OwoColorize;
                    eprintln!(
                        "  {} Eval gate: adapter did not beat base for domain '{}'. Not registered.",
                        "⚠".yellow(),
                        domain.yellow()
                    );
                }
                _ => {}
            }

            return Ok(());
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

    // Never clear a user-set `VOX_TRAIN_SKIP_CORPUS_MIX`: the flag only adds the opt-out.
    let skip_mix = vox_corpus::training::mix_prepare::corpus_mix_skipped(fast_corpus);
    if fast_corpus {
        // SAFETY: single-threaded CLI setup; read later by the trainer's mix step.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("VOX_TRAIN_SKIP_CORPUS_MIX", "1");
        }
    }
    // The stale-corpus refresh (pairs → data_dir/train.jsonl, then mix) only applies to
    // the canonical data dir; an explicit --data-dir is the user's data.
    let canonical_data_dir = vox_corpus::training::mix_prepare::is_canonical_data_dir(
        workspace_root.as_deref(),
        &data_dir,
    );

    // Preflight: stale corpus fingerprint → same refresh path for both data modes (synthetic + pipeline w/o train + mix).
    // `strict`: refresh failures abort. `auto-refresh`: log warnings and continue (legacy).
    if let Some(ref root) = workspace_root {
        use owo_colors::OwoColorize;
        let current_fp = vox_corpus::corpus::preflight::compute_corpus_fingerprint(root);

        let fingerprint_fresh = if let Ok(db) = vox_db::VoxDb::connect_default().await {
            db.is_corpus_fresh(&current_fp).await.unwrap_or(false)
        } else {
            let fp_file = vox_corpus::corpus::preflight::fingerprint_cache_path(root);
            vox_corpus::corpus::preflight::corpus_is_fresh(root, &fp_file)
        };

        // Version gate: even when the input fingerprint is unchanged, a corpus
        // produced by a different compiler version may use a stale pair-encoding
        // schema. Force a refresh when `metadata.json`'s `compiler_version` does
        // not match this build, so a version bump alone triggers regeneration.
        let version_mismatch = corpus_compiler_version_mismatch(&data_dir);
        let is_fresh = fingerprint_fresh && version_mismatch.is_none();

        if !is_fresh && !skip_mix && canonical_data_dir {
            let strict = matches!(data_mode, TrainDataModeCli::Strict);
            let reason = match &version_mismatch {
                Some((found, current)) => format!(
                    "compiler version changed ({found} → {current}); fingerprint: {current_fp}"
                ),
                None => format!("fingerprint: {current_fp}"),
            };
            eprintln!(
                "  {} Stale corpus detected ({}). {}",
                "🔄".cyan(),
                reason,
                if strict {
                    "Running blocking refresh before train..."
                } else {
                    "Regenerating..."
                }
            );
            let res =
                refresh_stale_training_corpus(root, &data_dir, &output_dir, &current_fp, strict)
                    .await;
            if strict {
                res?;
            } else {
                let _ = res;
            }
        }
    }

    let mut effective_min_rating = min_rating;
    let mut effective_ce_last_k = qlora_ce_last_k;
    // Resolution order for memory-sizing knobs: explicit CLI > domain profile >
    // VRAM-aware budget > preset fallback (applied in gpu.rs). `None` means
    // "not yet set"; each stage fills only what an earlier stage left unset.
    let mut effective_seq_len: Option<usize> = seq_len;
    let effective_batch_size: Option<usize> = batch_size;
    let effective_grad_accum: Option<usize> = grad_accum;
    // Never silently swapped for a smaller model by the VRAM budget below —
    // see `never_retreat_the_named_model`.
    let mut effective_model = model;
    // Resolved alongside `effective_model` below when `--domain` is given, via
    // the same `resolve_training_selection` the cloud path already uses (see
    // `resolve_cloud_spoke_base`) — otherwise stays the raw CLI `--preset`.
    let mut effective_preset = preset.clone();
    let mut effective_validation_split_ratio = validation_split_ratio;
    let mut _effective_max_grad_norm = None; // pass down if needed
    let mut effective_curriculum = curriculum;
    let mut effective_trajectory_weighting_enabled = trajectory_weighting_enabled;
    let mut effective_trajectory_tool_trace_boost = trajectory_tool_trace_boost;
    let mut effective_context_filter = None;
    let mut effective_adapter_tag = adapter_tag.clone();
    let mut effective_curriculum_schedule = None;
    let mut effective_chatml = vox_populi::mens::tensor::training_config::ChatmlConfig::default();
    let mut effective_mix_config = None;

    if let Some(domain_name) = &domain {
        match vox_populi::mens::tensor::domain_profiles::EffectiveDomainProfile::load_domain_profile(
            domain_name,
            workspace_root.as_deref(),
        ) {
            Ok(profile) => {
                use owo_colors::OwoColorize;
                eprintln!(
                    "  {} Applied domain profile: {}",
                    "✓".green(),
                    domain_name.cyan()
                );

                if let Some(desc) = &profile.description {
                    eprintln!("    Description: {}", desc.dimmed());
                }

                effective_min_rating = profile.min_rating.or(min_rating);
                effective_ce_last_k = profile.ce_last_k.unwrap_or(qlora_ce_last_k);
                // Domain seq_len fills the slot only when the user did not pass --seq-len.
                if effective_seq_len.is_none() {
                    effective_seq_len = profile.seq_len;
                }
                effective_validation_split_ratio = profile
                    .validation_split_ratio
                    .unwrap_or(validation_split_ratio);
                _effective_max_grad_norm = profile.max_grad_norm;
                effective_curriculum = profile.curriculum.unwrap_or(curriculum);
                effective_trajectory_weighting_enabled = profile
                    .trajectory_weighting
                    .unwrap_or(trajectory_weighting_enabled);
                if let Some(boost) = profile.trajectory_tool_trace_boost {
                    effective_trajectory_tool_trace_boost = boost;
                }
                effective_context_filter = profile.context_filter.clone();
                if effective_adapter_tag.is_none() {
                    effective_adapter_tag = Some(domain_name.clone());
                }
                effective_curriculum_schedule = profile.curriculum_schedule.clone();
                effective_chatml = profile.chatml.clone();

                if let Some(ref mix_path) = profile.mix_config {
                    effective_mix_config = Some(mix_path.clone());
                    // Update env var to point mix to this one if `vox mens corpus mix` called?
                    // Actually simply inform.
                    eprintln!("    Mix config: {}", mix_path.display());
                }

                // BLOCKER (D1): resolve the domain spoke's base model + preset via
                // the same real resolver the cloud path already uses
                // (`resolve_cloud_spoke_base` → `resolve_training_selection`).
                // Previously `--domain rust --cloud local` left `effective_model`
                // as the raw (often `None`) `--model` and `preset` as the raw
                // (often `None`) `--preset`, silently ignoring the domain's
                // pinned base — CLI-supplied values still win.
                let (resolved_model, resolved_preset) = resolve_local_spoke_base(
                    workspace_root.as_deref(),
                    domain_name,
                    effective_model.as_deref(),
                    effective_preset.as_deref(),
                )?;
                effective_model = resolved_model;
                effective_preset = Some(resolved_preset);
            }
            Err(e) => {
                anyhow::bail!("Failed to load domain profile '{}': {}", domain_name, e);
            }
        }
    }

    // The old CUDA pre-download budget estimate (params_b-only ladder: Qwen
    // family classifiers + `get_resident_per_b` + `plan_with_resident*`) was
    // deleted — no `ModelShape` (real on-disk layer/hidden/artifact-bytes
    // facts) exists this early, so it was never more than a guess. The real
    // per-host fit now happens post-download via the measured
    // `memory_model::sweep`/`plan_for` entry point once the model is on disk
    // (see `crate::commands::schola::train::gpu::run_gpu_training`); until
    // then these stay unset and `resolve_effective_profile` uses the plain
    // preset defaults.
    let budget_seq_len = None;
    let budget_batch_size = None;
    let budget_grad_accum = None;

    let parsed_filter = if let Some(cf) = effective_context_filter {
        Some(cf)
    } else {
        context_filter
            .or_else(|| effective_adapter_tag.clone())
            .map(
                |s| vox_populi::mens::tensor::training_config::ContextFilter {
                    categories: Some(vec![s]),
                    ..Default::default()
                },
            )
    };

    let spawn_log_dir = if background {
        Some(log_dir.clone().unwrap_or_else(|| {
            workspace_root
                .as_ref()
                .map(|r| r.join("mens/runs/logs"))
                .unwrap_or_else(|| PathBuf::from("mens/runs/logs"))
        }))
    } else {
        log_dir.clone()
    };
    if let Some(ref log_dir) = spawn_log_dir {
        return crate::commands::schola::train::spawn_train_with_log(log_dir.clone());
    }
    let deployment_target = if effective_preset.as_deref() == Some("mobile_edge") {
        vox_populi::mens::TrainingDeploymentTarget::MobileEdge
    } else {
        deployment_target.into()
    };
    let train_res = train::run_train(
        backend.into(),
        effective_model,
        device,
        data_dir,
        output_dir,
        rank,
        alpha,
        effective_seq_len,
        effective_batch_size,
        effective_grad_accum,
        budget_seq_len,
        budget_batch_size,
        budget_grad_accum,
        resume,
        epochs,
        lr,
        warmup,
        seed,
        effective_min_rating,
        effective_preset,
        deployment_target,
        process_priority,
        vram_limit_fraction,
        effective_adapter_tag,
        parsed_filter,
        Some(effective_validation_split_ratio),
        tokenizer.into(),
        qlora_no_double_quant,
        qlora_require_full_proxy_stack,
        qlora_allow_partial_proxy_stack,
        qlora_max_skip_rate,
        qlora_lm_head_only,
        qlora_proxy_max_layers,
        effective_ce_last_k,
        checkpoint_every,
        force_restart,
        effective_curriculum,
        optimizer_experiment_mode,
        require_gpu,
        allow_cpu_fallback,
        base_model_family,
        upstream_model_id,
        license_class,
        attribution_required,
        effective_trajectory_weighting_enabled,
        effective_trajectory_tool_trace_boost,
        trajectory_failure_category_boost,
        trajectory_quality_floor,
        trajectory_quality_boost,
        effective_curriculum_schedule,
        effective_chatml,
        effective_mix_config,
    )
    .await;

    if train_res.is_ok()
        && let Some(ref r) = workspace_root
    {
        let mixed = r.join("target/dogfood/train_mixed.jsonl");
        let backup = r.join("mens/data/train_full_backup.jsonl");
        if mixed.exists() {
            if let Some(parent) = backup.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::copy(&mixed, &backup) {
                use owo_colors::OwoColorize;
                eprintln!(
                    "  {} Failed to copy train_mixed.jsonl to backup: {e}",
                    "⚠️".yellow()
                );
            } else {
                use owo_colors::OwoColorize;
                eprintln!(
                    "  {} Backed up running corpus to {}",
                    "✓".green(),
                    backup.display()
                );
            }
        }
    }

    train_res
}

/// Resolve the local spoke base for `--domain <name> --cloud local`: returns
/// `(model, preset)`.
///
/// Mirrors `resolve_cloud_spoke_base` below — same real resolver
/// (`resolve_training_selection`), same "CLI wins" precedence — but sizes
/// with VRAM autodetect (`None`) rather than a fixed cloud tier, matching how
/// `pipeline.rs`'s `PipelineStage::Train` already calls this resolver for a
/// local run. Not `cfg(feature = "cloud")`: the local path must resolve
/// regardless of whether the cloud feature is compiled in.
fn resolve_local_spoke_base(
    workspace_root: Option<&Path>,
    domain: &str,
    cli_model: Option<&str>,
    cli_preset: Option<&str>,
) -> anyhow::Result<(Option<String>, String)> {
    use crate::commands::mens::training_selection::{
        TrainingSelection, resolve_training_selection,
    };

    let root = workspace_root
        .map(Path::to_path_buf)
        .or_else(vox_corpus::training::contract::find_workspace_root)
        .ok_or_else(|| {
            anyhow::anyhow!("could not find workspace root for spoke base resolution")
        })?;

    let selection = resolve_training_selection(&root, Some(domain), cli_model, cli_preset, None)?;

    match selection {
        TrainingSelection::Train { model, preset, .. } => Ok((model, preset)),
        TrainingSelection::Skip { reason } => {
            anyhow::bail!("spoke '{domain}' is {reason} — nothing to train locally")
        }
    }
}

/// Resolve the cloud spoke base: returns `(hf_id@revision, rung, quantization)`.
///
/// BLOCKER 4 + DEFAULT BASE: routes through the real `resolve_training_selection`
/// (domain spoke base.model → VRAM-fit rung) instead of the legacy
/// `default_model_id()` fallback. The rung is the resolved preset; quantization is
/// derived from the resolved `TrainBase.method` (qlora vs lora) so the un-quantized
/// 48GB LoRA rung is labelled correctly (F9 at the producer side).
///
/// `vram_mb_override` for cloud is fixed to the 24GB tier (matching the resolver's
/// default `min_vram_mb`) so the rung selection is deterministic and GPU-free.
#[cfg(feature = "cloud")]
fn resolve_cloud_spoke_base(
    workspace_root: Option<&Path>,
    domain: Option<&str>,
    cli_model: Option<&str>,
    cli_preset: Option<&str>,
) -> anyhow::Result<(String, String, String)> {
    use crate::commands::mens::training_selection::{
        TrainingSelection, resolve_training_selection,
    };

    let root = workspace_root
        .map(Path::to_path_buf)
        .or_else(vox_corpus::training::contract::find_workspace_root)
        .ok_or_else(|| {
            anyhow::anyhow!("could not find workspace root for spoke base resolution")
        })?;

    // Cloud sizing tier: 24GB-class GPU (matches CloudResolver default min_vram_mb).
    const CLOUD_VRAM_MB: u32 = 24_000;

    let selection =
        resolve_training_selection(&root, domain, cli_model, cli_preset, Some(CLOUD_VRAM_MB))?;

    let (model, rung) = match selection {
        TrainingSelection::Train { model, preset, .. } => (model, preset),
        TrainingSelection::Skip { reason } => {
            anyhow::bail!("spoke '{domain:?}' is {reason} — nothing to train in the cloud")
        }
    };

    // hf_id: resolved spoke base; fall back to the Qwen3 default (DEFAULT_MODEL_ID)
    // when the spoke did not pin a base. This is the bare/unresolved default and is
    // a Qwen3 agentic_default rung per USER DECISION.
    let hf_id = model.unwrap_or_else(vox_populi::mens::default_model_id);

    // Quantization: derive from the resolved TrainBase.method when the spoke base
    // is a capability tag; concrete ids default to qlora.
    let quant = derive_quantization_for_base(&root, domain, &hf_id, CLOUD_VRAM_MB);

    Ok((hf_id, rung, quant))
}

/// Derive the quantization label ("qlora" / "lora") for the resolved base by
/// re-inspecting the spoke's base tag in the overlay. Defaults to "qlora".
#[cfg(feature = "cloud")]
fn derive_quantization_for_base(
    root: &Path,
    domain: Option<&str>,
    resolved_hf_id: &str,
    vram_mb: u32,
) -> String {
    use vox_populi::mens::tensor::domain_profiles::EffectiveDomainProfile;
    use vox_populi::mens::tensor::spoke_base_resolver::{load_overlay, pick_base};

    let tag = domain
        .and_then(|d| EffectiveDomainProfile::load_domain_profile(d, Some(root)).ok())
        .and_then(|e| e.base.as_ref().map(|b| b.model.clone()));

    // The resolved id must match the picked base for the method to apply; an
    // un-quantized rung advertises "lora" / "full_lora", otherwise qlora.
    if let Some(tag) = tag
        && let Ok(overlay) = load_overlay(root)
        && let Ok(base) = pick_base(&overlay, &tag, vram_mb)
        && base.hf_id == resolved_hf_id
        && base.methods.iter().any(|m| m == "lora" || m == "full_lora")
    {
        return "lora".to_string();
    }
    "qlora".to_string()
}

/// Run the REAL eval gate on a cloud-trained adapter (BLOCKER 1).
///
/// Maps the gate to [`EvalGateOutcome`]:
/// - gate passes (exit 0) → `PassedBase`
/// - gate fails  (exit 1) → `BelowBase`
/// - gate errors (could not run) → `EvalError` (fail-closed: NOT registered)
///
/// Fail-closed: any error running the gate returns `EvalError`, never a default
/// `PassedBase`. A failed/absent gate must NOT register an adapter.
#[cfg(feature = "cloud")]
fn run_cloud_eval_gate(
    adapter_dir: &Path,
    workspace_root: Option<&Path>,
) -> vox_populi::mens::cloud::EvalGateOutcome {
    use vox_populi::mens::cloud::EvalGateOutcome;

    let policy_path = workspace_root.map(|r| r.join("mens/config/eval-gates.yaml"));
    match crate::commands::mens::eval_gate::run_eval_gate(adapter_dir.to_path_buf(), policy_path) {
        Ok(0) => EvalGateOutcome::PassedBase,
        Ok(_) => EvalGateOutcome::BelowBase,
        Err(e) => EvalGateOutcome::EvalError(format!("eval gate could not run: {e}")),
    }
}

/// The version this build stamps into freshly generated corpora.
fn current_corpus_compiler_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Inspect `<data_dir>/metadata.json` and return `Some((found, current))` when its
/// recorded `compiler_version` differs from this build's version (i.e. the corpus
/// was generated by a different compiler and should be regenerated). Returns `None`
/// when the versions match, when there is no metadata yet (a fresh build will create
/// it), or when the field is absent/unparseable (don't force a refresh on noise).
fn corpus_compiler_version_mismatch(data_dir: &Path) -> Option<(String, String)> {
    let meta_path = data_dir.join("metadata.json");
    let raw = std::fs::read_to_string(&meta_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let found = value.get("compiler_version")?.as_str()?.trim().to_string();
    if found.is_empty() {
        return None;
    }
    let current = current_corpus_compiler_version();
    if found != current {
        Some((found, current.to_string()))
    } else {
        None
    }
}

/// Regenerate synthetic data, run `vox mens pipeline` with `skip_train` (pairs → `train.jsonl`,
/// mix → the mix config's `output:`), then record fingerprint. The trainer's own mix step
/// then resolves the mix output (incremental skip when unchanged). See [`TrainDataModeCli`](super::action::TrainDataModeCli).
async fn refresh_stale_training_corpus(
    root: &Path,
    data_dir: &Path,
    output_dir: &Path,
    current_fp: &str,
    strict: bool,
) -> anyhow::Result<()> {
    use anyhow::Context;
    use owo_colors::OwoColorize;

    if strict {
        vox_corpus::corpus::preflight::clean_corpus_targets(root)
            .map_err(|e| anyhow::anyhow!("clean_corpus_targets: {e}"))?;
    } else {
        let _ = vox_corpus::corpus::preflight::clean_corpus_targets(root);
    }

    let cfg = vox_corpus::synthetic_gen::SyntheticGenConfig::default();
    let out_path = root.join("mens/data/synthetic.jsonl");
    let mut pairs: i64 = 0;
    match vox_corpus::synthetic_gen::generate_all(&cfg, &out_path) {
        Ok(count) => {
            eprintln!("  {} Regenerated {} synthetic pairs", "✓".green(), count);
            pairs = count as i64;
        }
        Err(e) => {
            if strict {
                return Err(anyhow::anyhow!("synthetic corpus regen: {e}"));
            }
            eprintln!("  {} Synthetic regen failed: {}", "⚠️".yellow(), e);
        }
    }

    eprintln!("  {} Running corpus extraction pipeline...", "🔄".cyan());
    match crate::commands::mens::pipeline::run(
        data_dir.to_path_buf(),
        output_dir.to_path_buf(),
        true,
        false,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
    )
    .await
    {
        Ok(()) => eprintln!("  {} Corpus extraction pipeline completed.", "✓".green()),
        Err(e) => {
            if strict {
                return Err(e).context("corpus extraction pipeline (stale-fingerprint refresh)");
            }
            eprintln!("  {} Pipeline error: {}", "⚠️".yellow(), e);
        }
    }

    if let Ok(db) = vox_db::VoxDb::connect_default().await {
        if strict {
            db.record_corpus_snapshot(current_fp, env!("CARGO_PKG_VERSION"), pairs, None)
                .await
                .map_err(|e| anyhow::anyhow!("record_corpus_snapshot: {e}"))?;
        } else {
            let _ = db
                .record_corpus_snapshot(current_fp, env!("CARGO_PKG_VERSION"), pairs, None)
                .await;
        }
    } else {
        let fp_file = vox_corpus::corpus::preflight::fingerprint_cache_path(root);
        if strict {
            vox_corpus::corpus::preflight::write_fingerprint_snapshot(root, &fp_file)
                .map_err(|e| anyhow::anyhow!("write fingerprint snapshot: {e}"))?;
        } else {
            let _ = vox_corpus::corpus::preflight::write_fingerprint_snapshot(root, &fp_file);
        }
    }

    Ok(())
}

#[cfg(all(test, feature = "cloud"))]
mod cloud_eval_gate_tests {
    use super::run_cloud_eval_gate;
    use vox_populi::mens::cloud::{EvalGateOutcome, TrainingManifest, post_training_flow};
    use vox_populi::mens::tensor::domain_router::DomainRouter;

    /// BLOCKER 1: an absent / failing eval gate must NOT map to PassedBase.
    /// A run dir with no eval-gates.yaml → the gate fails (Ok(1)) → BelowBase,
    /// never the old hardcoded PassedBase.
    #[test]
    fn absent_gate_does_not_pass_base() {
        let tmp = tempfile::tempdir().unwrap();
        // workspace_root with no mens/config/eval-gates.yaml → policy not found.
        let outcome = run_cloud_eval_gate(tmp.path(), Some(tmp.path()));
        assert!(
            !matches!(outcome, EvalGateOutcome::PassedBase),
            "absent gate must NOT be PassedBase (got {outcome:?})"
        );
    }

    /// BLOCKER 1: a below-base / errored gate must NOT register an adapter.
    /// Exercises the real outcome→register decision via post_training_flow.
    #[test]
    fn below_base_gate_does_not_register() {
        let tmp = tempfile::tempdir().unwrap();
        let outcome = run_cloud_eval_gate(tmp.path(), Some(tmp.path()));
        let mut router = DomainRouter::new();
        let manifest = TrainingManifest {
            base_hf_id: "Qwen/Qwen3-8B@abc123".into(),
            base_revision: "abc123".into(),
            rung: "qwen3_16g".into(),
            quantization: "qlora".into(),
            preset: "qwen3_16g".into(),
            rank: 16,
            alpha: 32.0,
            seed: 42,
            corpus_hash: "deadbeef".into(),
            metrics: serde_json::json!({}),
            cost_usd: 0.0,
            provider: "runpod".into(),
            git_sha: "abc".into(),
            created_at: "2026-06-21T00:00:00Z".into(),
        };
        // EvalError must fail-closed (Err); BelowBase returns EvalGateFailed.
        match outcome {
            EvalGateOutcome::EvalError(_) => {
                let res = post_training_flow(outcome, &mut router, "vox", tmp.path(), &manifest);
                assert!(res.is_err(), "EvalError must fail-closed (no register)");
                assert!(router.route("vox").is_none(), "must not register on error");
            }
            other => {
                let res = post_training_flow(other, &mut router, "vox", tmp.path(), &manifest)
                    .expect("BelowBase returns Ok(EvalGateFailed)");
                assert!(
                    !matches!(
                        res,
                        vox_populi::mens::cloud::CloudOrchestrationOutcome::RegisteredChallenger { .. }
                    ),
                    "below-base gate must NOT register"
                );
                assert!(
                    router.route("vox").is_none(),
                    "must not register below base"
                );
            }
        }
    }
}

/// D1: `--domain rust --cloud local` wiring. Proves `resolve_local_spoke_base`
/// (the function `run_train`'s local branch now calls) actually resolves the
/// spoke's real base + preset — not the bug this replaces, where the local
/// branch was `let effective_model = model;` and silently kept the raw
/// (`None`) CLI `--model`/`--preset` regardless of `--domain`.
///
/// This is NOT a re-test of `training_selection.rs::rust_resolves_qwen_qlora`
/// (which already covers the resolver itself) — it exercises the train_arm.rs
/// wiring on top of it: that the local path's own helper, called with no CLI
/// overrides, still surfaces the "rust" spoke's Qwen base instead of leaving
/// `model` as `None` / `preset` unset.
/// Non-empty lines in a JSONL file, or `None` when it cannot be read.
#[cfg_attr(not(feature = "cloud"), allow(dead_code))]
fn jsonl_row_count(path: &std::path::Path) -> Option<usize> {
    let text = vox_bounded_fs::read_utf8_path_capped(path).ok()?;
    Some(text.lines().filter(|l| !l.trim().is_empty()).count())
}

#[cfg(test)]
#[test]
fn jsonl_row_count_counts_non_empty_lines() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("train.jsonl");
    std::fs::write(&p, "{\"a\":1}\n\n{\"a\":2}\n").unwrap();
    assert_eq!(jsonl_row_count(&p), Some(2));
    assert_eq!(jsonl_row_count(&d.path().join("missing.jsonl")), None);
}

#[cfg(test)]
mod local_spoke_base_wiring_tests {
    use super::resolve_local_spoke_base;

    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn domain_rust_resolves_spoke_base_and_preset_not_defaults() {
        let (model, preset) = resolve_local_spoke_base(Some(&root()), "rust", None, None).unwrap();
        let model = model.expect("rust spoke must resolve a base model, not None");
        assert!(
            model.contains("Qwen"),
            "expected the rust spoke's Qwen base, got {model:?}"
        );
        assert!(!preset.is_empty(), "resolved preset must not be empty");
    }

    #[test]
    fn cli_model_still_wins_over_domain_resolution() {
        let (model, _preset) =
            resolve_local_spoke_base(Some(&root()), "rust", Some("org/Manual"), None).unwrap();
        assert_eq!(model.as_deref(), Some("org/Manual"));
    }
}
