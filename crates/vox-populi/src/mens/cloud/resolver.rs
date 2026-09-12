//! Cloud GPU offer resolver — parallel query, budget gate, cost-ranked dispatch.
//!
//! Queries all configured cloud providers in parallel, applies filters, estimates
//! cost per job, gates on the budget ledger, and returns a ranked list of offers.

use std::path::PathBuf;
use std::sync::Arc;

use super::{
    BudgetLedger, CloudJobSpec, CloudProvider, CloudProviderConfig, CloudTarget, GpuOffer,
    JobHandle, JobKind, ProviderKind,
    estimator::{EstimateSource, TimeEstimator},
    offer_filter::{UnsuitableReason, offer_is_suitable},
    runpod_provider::RunPodClient,
    vast::VastClient,
    watchdog::CloudWatchdog,
};

/// A ranked cloud GPU offer with cost estimation.
pub struct ResolvedOffer {
    /// The raw offer from the provider.
    pub offer: GpuOffer,
    /// Estimated total time in seconds (including overhead).
    pub estimated_secs: f64,
    /// Estimated total cost in USD.
    pub estimated_cost_usd: f64,
    /// Which estimation tier produced this result.
    pub estimate_source: EstimateSource,
    /// Canonical preset name for this GPU's VRAM tier.
    ///
    /// Always `"auto"` — the VRAM-tiered `preset_for_vram` lookup that used
    /// to compute this per-offer was deleted (Task 8: it duplicated, and
    /// disagreed with, the same VRAM-to-preset ladders removed from
    /// `preset_schema.rs`/`vram_autodetect.rs`, and this field has no reader
    /// besides its own construction — nothing downstream branches on it).
    /// Cloud dispatch resolves the real preset via `CloudJobSpec.preset`
    /// (see `build_train_spec`/`build_serve_spec`), independent of this field.
    pub effective_preset: &'static str,
}

/// Request parameters passed to [`CloudResolver::resolve`].
pub struct ResolveRequest {
    /// Minimum VRAM needed for this job.
    pub min_vram_mb: u64,
    /// Training sequence length (for time estimation).
    pub seq_len: usize,
    /// Micro-batch size (for time estimation).
    pub batch_size: usize,
    /// Total number of training pairs.
    pub num_samples: usize,
    /// Number of training epochs.
    pub epochs: usize,
    /// Maximum acceptable estimated cost in USD.
    pub max_acceptable_cost: f64,
    /// Which providers to query.
    pub target: CloudTarget,
}

/// The old, pre-memory-SSOT default: a flat 24GB assumption, used only where
/// [`min_vram_mb_for_dispatch`] cannot size the real request (inference/agent
/// jobs, which `plan_for`'s training-shaped `Request` does not model; or a Hub
/// lookup failure for a training job — see that function's doc comment).
const LEGACY_DEFAULT_MIN_VRAM_MB: u64 = 24_000;

/// Derive `min_vram_mb` for a real training dispatch the same way `vox mens
/// cloud-estimate` derives it: fetch the model's shape from the Hub API (no
/// download — see [`crate::mens::hub::model_shape_from_hub`]) and run it through
/// [`min_vram_mb_for_cuda`], the same `plan_for` threshold cloud-estimate uses.
/// This is what closes the disagreement between the read-only estimate and the
/// real dispatch decision — see the mens-cloud-training final-review finding
/// this exists to fix.
///
/// `pub` (not just used by [`min_vram_mb_for_dispatch`] below) because
/// `vox-ml-cli`'s `train_arm.rs` cloud path drives `resolve` + `dispatch_top`
/// directly instead of [`CloudResolver::dispatch`] and needs the identical
/// sizing rather than a re-implementation that could drift from this one.
///
/// Fails closed on a Hub lookup error (gated repo, no network, or an
/// uncalibrated lane) — dispatch should stop rather than silently under-size
/// the rental.
pub async fn min_vram_mb_for_training(
    model_id: &str,
    batch_size: usize,
    seq_len: usize,
) -> anyhow::Result<u64> {
    use crate::mens::hub::model_shape_from_hub;
    use crate::mens::tensor::memory_model::{Request, min_vram_mb_for_cuda};

    let shape = model_shape_from_hub(model_id).await.map_err(|e| {
        anyhow::anyhow!(
            "cannot size cloud dispatch for `{model_id}`: {e}. Run `vox mens cloud-estimate \
             --model-dir <dir>` against a local copy to diagnose, or measure the lane with \
             `vox mens probe --measure`."
        )
    })?;
    let request = Request {
        batch_size: batch_size as u64,
        seq_len: seq_len as u64,
    };
    min_vram_mb_for_cuda(&shape, &request)
}

/// Derive `min_vram_mb` for [`CloudResolver::dispatch`]. Scoped to
/// [`JobKind::Train`]: `plan_for`'s `Request` (batch_size × seq_len activation
/// memory) models a *training* step, not inference serving (weights + KV
/// cache, no activations/gradients) — reusing [`min_vram_mb_for_training`] for
/// `Infer`/`Agent` would produce a confidently wrong number, not a merely
/// approximate one. Those job kinds keep [`LEGACY_DEFAULT_MIN_VRAM_MB`] until a
/// real inference memory model exists.
async fn min_vram_mb_for_dispatch(spec: &CloudJobSpec) -> anyhow::Result<u64> {
    if spec.job_kind != JobKind::Train {
        return Ok(LEGACY_DEFAULT_MIN_VRAM_MB);
    }
    min_vram_mb_for_training(&spec.model_id, spec.batch_size, spec.seq_len).await
}

/// Overhead factor per provider (accounts for launch + teardown time in cost estimate).
///
/// Vast.ai: fire-and-forget termination → 10% overhead.
/// RunPod: watchdog-polled termination → 20% overhead.
const OVERHEAD_AUTO_TERMINATE: f64 = 1.10;
const OVERHEAD_POLL_TERMINATE: f64 = 1.20;

/// Queries cloud providers, estimates costs, and returns ranked offers.
///
/// Owns the provider clients so `dispatch_top` can reuse them without
/// creating new clients (which would discard config).
pub struct CloudResolver {
    vast: Option<Arc<VastClient>>,
    runpod: Option<Arc<RunPodClient>>,
    local: Option<Arc<super::local_provider::LocalProvider>>,
    estimator: TimeEstimator,
    pub budget: Arc<BudgetLedger>,
    pub config: Arc<CloudProviderConfig>,
}

impl CloudResolver {
    /// Construct a resolver.
    ///
    /// - `gpu_specs_path`: path to `mens/config/gpu-specs.yaml`
    /// - `profiles`: output of `VoxDb::cloud_load_throughput_profiles()`
    /// - `budget`: Arca-backed spend ledger
    /// - `config`: shared provider config (reused by watchdog and dispatch)
    pub fn new(
        gpu_specs_path: PathBuf,
        profiles: Vec<(String, usize, usize, f64)>,
        budget: Arc<BudgetLedger>,
        config: Arc<CloudProviderConfig>,
        local: Option<Arc<super::local_provider::LocalProvider>>,
    ) -> anyhow::Result<Self> {
        let estimator = TimeEstimator::new(&gpu_specs_path, profiles)?;
        let vast = VastClient::from_env(Arc::clone(&config)).ok().map(Arc::new);
        let runpod = RunPodClient::from_env(Arc::clone(&config))
            .ok()
            .map(Arc::new);
        if vast.is_none() && runpod.is_none() && local.is_none() {
            anyhow::bail!(
                "No GPU providers available. Either:\n\
                 - Set VOX_VAST_API_KEY (Vast.ai) or VOX_RUNPOD_API_KEY (RunPod)\n\
                 - Ensure a GPU is detected locally (vox mens probe)"
            );
        }
        Ok(Self {
            vast,
            runpod,
            local,
            estimator,
            budget,
            config,
        })
    }

    /// Convenience: build a resolver from environment and any attached Arca store.
    pub async fn new_from_env() -> anyhow::Result<Self> {
        let root = vox_corpus::training::contract::find_workspace_root().ok_or_else(|| {
            anyhow::anyhow!("Could not find workspace root (required for gpu-specs.yaml)")
        })?;
        let specs_path = root.join("mens/config/gpu-specs.yaml");
        let config = Arc::new(CloudProviderConfig::default());

        let db = vox_db::connect_canonical_optional(
            vox_db::DbConnectSurface::PopuliCloudResolver,
            false,
        )
        .await
        .map(Arc::new);
        let budget = Arc::new(BudgetLedger::new(db.clone(), &config));

        let profiles = if let Some(ref voxdb) = db {
            voxdb.cloud_load_throughput_profiles().await?
        } else {
            vec![]
        };

        // Initialize local provider if hardware supports it
        let hw = crate::mens::hardware::HardwareRegistry::probe().await;
        let local = if hw.vram_mb > 0 {
            Some(Arc::new(super::local_provider::LocalProvider::new(
                hw,
                config.clone(),
            )))
        } else {
            None
        };

        Self::new(specs_path, profiles, budget, config, local)
    }

    /// Query all configured providers and return offers ranked by estimated cost.
    ///
    /// Providers are queried in parallel; individual failures are logged and skipped.
    pub async fn resolve(
        &self,
        req: &ResolveRequest,
    ) -> anyhow::Result<(Vec<ResolvedOffer>, Vec<(String, UnsuitableReason)>)> {
        self.budget.check_capacity(req.max_acceptable_cost).await?;

        let use_vast =
            matches!(req.target, CloudTarget::Auto | CloudTarget::Vast) && self.vast.is_some();
        let use_runpod =
            matches!(req.target, CloudTarget::Auto | CloudTarget::RunPod) && self.runpod.is_some();
        let use_local =
            matches!(req.target, CloudTarget::Auto | CloudTarget::Local) && self.local.is_some();

        let (vast_r, runpod_r, local_r) = tokio::join!(
            async {
                if use_vast {
                    self.vast
                        .as_ref()
                        .unwrap()
                        .list_offers(req.min_vram_mb)
                        .await
                } else {
                    Ok(vec![])
                }
            },
            async {
                if use_runpod {
                    self.runpod
                        .as_ref()
                        .unwrap()
                        .list_offers(req.min_vram_mb)
                        .await
                } else {
                    Ok(vec![])
                }
            },
            async {
                if use_local {
                    self.local
                        .as_ref()
                        .unwrap()
                        .list_offers(req.min_vram_mb)
                        .await
                } else {
                    Ok(vec![])
                }
            },
        );

        let mut all: Vec<GpuOffer> = vec![];
        if let Ok(v) = local_r {
            all.extend(v);
        }
        match vast_r {
            Ok(v) => all.extend(v),
            Err(e) => tracing::warn!("Vast.ai query failed (skipping): {e}"),
        }
        match runpod_r {
            Ok(v) => all.extend(v),
            Err(e) => tracing::warn!("RunPod query failed (skipping): {e}"),
        }

        if all.is_empty() {
            anyhow::bail!(
                "No GPU offers found (min_vram={}MB). Check API keys and filters.",
                req.min_vram_mb
            );
        }

        let remaining = self.budget.remaining_usd().await;

        let (ranked, rejected) = rank_offers(all, req, &self.config, &self.estimator, remaining);
        for (id, reason) in &rejected {
            tracing::debug!(offer_id = %id, reason = %reason, "offer filtered out");
        }
        Ok((ranked, rejected))
    }

    /// Dispatch a job to the top-ranked offer, start watchdog, return handle.
    ///
    /// Reuses the resolver's owned provider clients — no config discarding.
    /// Resolve and dispatch the top offer in one call.
    pub async fn dispatch(&self, spec: CloudJobSpec, target_str: &str) -> anyhow::Result<()> {
        use std::str::FromStr;
        let target = CloudTarget::from_str(target_str)?;
        let min_vram_mb = min_vram_mb_for_dispatch(&spec).await?;
        let req = ResolveRequest {
            target,
            min_vram_mb,
            max_acceptable_cost: spec.max_budget_usd.unwrap_or(self.config.max_budget_usd),
            seq_len: spec.seq_len,
            batch_size: spec.batch_size,
            num_samples: spec.num_samples,
            epochs: spec.epochs,
        };
        let (ranked, rejected) = self.resolve(&req).await?;
        if ranked.is_empty() && !rejected.is_empty() {
            // `dispatch_top` below only ever says "no offers to dispatch" —
            // surface why here, at the one path a caller who is about to
            // spend money actually reaches, instead of dropping this into
            // `tracing::debug!` where nobody watching the CLI sees it.
            eprintln!("  ⚠ No suitable cloud offers — every candidate was rejected:");
            for (offer_id, reason) in &rejected {
                eprintln!("      {offer_id}: {reason}");
            }
        }
        let (_handle, join, _provider) = self.dispatch_top(&ranked, &spec).await?;

        // Wait for the watchdog if requested or just return handle
        // For the CLI, we usually want to wait until completion or detach.
        // run_train in mod.rs seems to expect a Result<()> and might background it itself.

        join.await
            .map_err(|e| anyhow::anyhow!("Watchdog task failed: {e}"))
    }

    /// Resolve and dispatch the top offer in one call.
    pub async fn dispatch_top(
        &self,
        ranked: &[ResolvedOffer],
        spec: &CloudJobSpec,
    ) -> anyhow::Result<(
        JobHandle,
        tokio::task::JoinHandle<()>,
        Arc<dyn CloudProvider>,
    )> {
        let top = ranked.first().ok_or_else(|| {
            anyhow::anyhow!("No offers to dispatch — resolve returned empty list")
        })?;

        // Validate runtime for serve/agent jobs
        if spec.job_kind.requires_explicit_runtime() && spec.max_runtime_secs.is_none() {
            anyhow::bail!(
                "JobKind::{:?} requires --max-runtime to prevent unbounded billing.",
                spec.job_kind
            );
        }

        // Reuse the owned client — does NOT create new clients or discard config
        let provider: Arc<dyn CloudProvider> = match top.offer.provider {
            ProviderKind::Vast => self
                .vast
                .as_ref()
                .map(|c| Arc::clone(c) as Arc<dyn CloudProvider>)
                .ok_or_else(|| anyhow::anyhow!("Vast.ai client not available"))?,
            ProviderKind::RunPod => self
                .runpod
                .as_ref()
                .map(|c| Arc::clone(c) as Arc<dyn CloudProvider>)
                .ok_or_else(|| anyhow::anyhow!("RunPod client not available"))?,
            ProviderKind::Local => self
                .local
                .as_ref()
                .map(|c| Arc::clone(c) as Arc<dyn CloudProvider>)
                .ok_or_else(|| anyhow::anyhow!("Local provider not available"))?,
        };

        let mut handle = provider.dispatch(&top.offer, spec).await?;
        handle.estimated_seconds = top.estimated_secs;

        // Record in Arca
        self.budget
            .open_job(
                &handle,
                &top.offer.offer_id,
                &top.offer.gpu_name,
                top.offer.vram_mb,
                top.estimated_cost_usd,
                spec.job_kind.as_str(),
            )
            .await?;

        // Spawn watchdog with THE SAME config (not a new default)
        let watchdog = CloudWatchdog {
            provider: Arc::clone(&provider),
            handle: handle.clone(),
            budget: Arc::clone(&self.budget),
            config: Arc::clone(&self.config),
        };
        let wh = watchdog.spawn();

        Ok((handle, wh, provider))
    }

    /// Print a ranked offer table to stdout (CLI display).
    pub fn print_offer_table(ranked: &[ResolvedOffer]) {
        if ranked.is_empty() {
            println!("No offers available within budget.");
            return;
        }
        println!(
            "\n{:<8} {:<20} {:<7} {:<9} {:<10} {:<10} {:<5} {:<20}",
            "Provider", "GPU", "VRAM", "$/hr", "Est Time", "Est Cost", "Auto", "Est. Source"
        );
        println!("{}", "-".repeat(95));
        for r in ranked {
            let mins = r.estimated_secs as u64 / 60;
            let time_str = if mins >= 60 {
                format!("{}h{:02}m", mins / 60, mins % 60)
            } else {
                let secs = r.estimated_secs as u64 % 60;
                format!("{mins}m{secs:02}s")
            };
            // Safe UTF-8 truncation — no panic on multi-byte chars
            let gpu_display: String = r.offer.gpu_name.chars().take(18).collect();
            println!(
                "{:<8} {:<20} {:<7} ${:<8.3} {:<10} ${:<9.2} {:<5} {}",
                r.offer.provider.display_name(),
                gpu_display,
                format!(
                    "{} GB",
                    r.offer.vram_mb / crate::mens::hardware::types::MB_PER_GB
                ),
                r.offer.price_per_hour_usd,
                time_str,
                r.estimated_cost_usd,
                if r.offer.auto_terminate { "✓" } else { "" },
                r.estimate_source,
            );
        }
        println!();
    }
}

/// Cost, filter, and rank offers. Pure: no I/O, no clock.
///
/// Returns `(ranked, rejected)`. Rejections carry their reason so the caller can tell
/// the operator *why* the board is empty instead of "no offers found".
pub(crate) fn rank_offers(
    offers: Vec<GpuOffer>,
    req: &ResolveRequest,
    config: &CloudProviderConfig,
    estimator: &TimeEstimator,
    remaining_usd: f64,
) -> (Vec<ResolvedOffer>, Vec<(String, UnsuitableReason)>) {
    let mut rejected: Vec<(String, UnsuitableReason)> = vec![];
    let mut ranked: Vec<ResolvedOffer> = vec![];

    for offer in offers {
        if let Err(reason) = offer_is_suitable(&offer, config, req.min_vram_mb) {
            rejected.push((offer.offer_id.clone(), reason));
            continue;
        }

        let overhead = if offer.auto_terminate {
            OVERHEAD_AUTO_TERMINATE
        } else {
            OVERHEAD_POLL_TERMINATE
        };
        let (est_secs, source) = estimator.estimate(
            &offer.gpu_name,
            req.seq_len,
            req.batch_size,
            req.num_samples,
            req.epochs,
        );
        let total_secs = est_secs * overhead;
        let cost = (total_secs / 3600.0) * offer.price_per_hour_usd;

        let budget_usd = remaining_usd.min(req.max_acceptable_cost);
        if cost > budget_usd {
            rejected.push((
                offer.offer_id.clone(),
                UnsuitableReason::OverBudget {
                    estimated_cost_usd: cost,
                    budget_usd,
                },
            ));
            continue;
        }

        ranked.push(ResolvedOffer {
            effective_preset: "auto",
            estimated_secs: total_secs,
            estimated_cost_usd: cost,
            estimate_source: source,
            offer,
        });
    }

    // Sort: cheapest → prefer auto_terminate → higher reliability.
    ranked.sort_by(|a, b| {
        a.estimated_cost_usd
            .partial_cmp(&b.estimated_cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.offer.auto_terminate.cmp(&a.offer.auto_terminate))
            .then(
                b.offer
                    .reliability_pct
                    .partial_cmp(&a.offer.reliability_pct)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    (ranked, rejected)
}

/// Convenience constructor for a standard training job spec.
pub fn build_train_spec(
    config: &CloudProviderConfig,
    model_id: Option<String>,
    train_data_hf: Option<String>,
    adapter_upload_hf: Option<String>,
    extra_env: Vec<(String, String)>,
) -> CloudJobSpec {
    CloudJobSpec {
        model_id: model_id.unwrap_or_else(crate::mens::default_model_id),
        preset: "auto".to_string(),
        train_data_hf,
        adapter_upload_hf,
        image_tag: config.image_tag.clone(),
        extra_env,
        job_kind: JobKind::Train,
        checkpoint_volume: None,
        max_runtime_secs: None,
        max_budget_usd: None,
        seq_len: 256,
        num_samples: 5000,
        epochs: 3,
        batch_size: 4,
        serve_port: 8080,
        persistent: false,
    }
}

/// Convenience constructor for a serve/inference job spec.
pub fn build_serve_spec(
    config: &CloudProviderConfig,
    model_id: String,
    max_runtime_secs: u64,
    serve_port: u16,
) -> CloudJobSpec {
    CloudJobSpec {
        model_id,
        preset: "auto".to_string(),
        train_data_hf: None,
        adapter_upload_hf: None,
        image_tag: config.image_tag.clone(),
        extra_env: vec![],
        job_kind: JobKind::Infer,
        checkpoint_volume: None,
        max_runtime_secs: Some(max_runtime_secs),
        max_budget_usd: None,
        seq_len: 256,
        num_samples: 0,
        epochs: 0,
        batch_size: 1,
        serve_port,
        persistent: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_train_spec_and_build_serve_spec_both_default_to_auto_preset() {
        // The VRAM-tiered `preset_for_vram` lookup that used to compute
        // `ResolvedOffer.effective_preset` was deleted (Task 8): it had no
        // 27B rung and disagreed with the equivalent ladders removed from
        // preset_schema.rs/vram_autodetect.rs. Cloud dispatch's own preset
        // resolution is independent of that field and unaffected — both job
        // spec constructors still request the "auto" preset by default.
        let config = CloudProviderConfig::default();
        let train = build_train_spec(&config, None, None, None, vec![]);
        assert_eq!(train.preset, "auto");
        let serve = build_serve_spec(&config, "some-model".to_string(), 60, 8080);
        assert_eq!(serve.preset, "auto");
    }

    /// MUTATION CAUGHT: routing `Infer`/`Agent` jobs through the Hub-based
    /// `min_vram_mb_for_cuda` sizing (which models training-step activation
    /// memory, not inference serving). `min_vram_mb_for_dispatch` must return
    /// the legacy constant for these job kinds without making a network call —
    /// a `#[tokio::test]` with no network available still passes only if the
    /// Train-only branch is never taken for `Infer`.
    #[tokio::test]
    async fn infer_jobs_keep_the_legacy_default_without_a_hub_lookup() {
        let config = CloudProviderConfig::default();
        let spec = build_serve_spec(&config, "some-model".to_string(), 60, 8080);
        assert_eq!(spec.job_kind, JobKind::Infer);
        let got = min_vram_mb_for_dispatch(&spec)
            .await
            .expect("infer sizing must not require network");
        assert_eq!(got, LEGACY_DEFAULT_MIN_VRAM_MB);
    }
}

#[cfg(test)]
mod rank_tests {
    use super::*;
    use crate::mens::cloud::{ProviderKind, test_offer};

    fn offer(id: &str, gpu_count: u32, vram_mb: u64, usd: f64) -> GpuOffer {
        GpuOffer {
            provider: ProviderKind::Vast,
            offer_id: id.into(),
            gpu_name: "h100 sxm".into(),
            gpu_count,
            vram_mb,
            price_per_hour_usd: usd,
            reliability_pct: 97.0,
            auto_terminate: true,
            ..test_offer()
        }
    }

    /// Same shape as `pipeline_dispatch::tests::zero_estimator`: an empty specs file
    /// keeps the estimator on its conservative fallback tier, which is deterministic.
    fn test_estimator() -> TimeEstimator {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gpu-specs.yaml");
        std::fs::write(&path, "gpus: {}\npresets: {}\n").unwrap();
        TimeEstimator::new(&path, vec![]).unwrap()
    }

    fn req(min_vram_mb: u64) -> ResolveRequest {
        ResolveRequest {
            min_vram_mb,
            seq_len: 512,
            batch_size: 1,
            num_samples: 8,
            epochs: 1,
            // Budget must never be the thing that drops an offer in this test --
            // otherwise a deleted filter still yields a "correct-looking" ranking.
            max_acceptable_cost: f64::MAX,
            target: CloudTarget::Auto,
        }
    }

    /// MUTATION CAUGHT: deleting the `offer_is_suitable` call from `rank_offers`.
    /// Without it the 8-GPU node survives -- and because it is *not* the cheapest
    /// row, a test that only asserted "the H100 ranks first" would still pass. This
    /// asserts the node is absent, which is the only assertion the mutant fails.
    #[test]
    fn ranking_drops_a_paid_multi_gpu_node_and_keeps_the_single_gpu_offer() {
        let offers = vec![
            offer("node-8x", 8, 655_360, 16.72),
            offer("single-h100", 1, 81_920, 2.09),
        ];
        let (ranked, rejected) = rank_offers(
            offers,
            &req(81_920),
            &CloudProviderConfig::default(),
            &test_estimator(),
            f64::MAX,
        );

        let ids: Vec<&str> = ranked.iter().map(|r| r.offer.offer_id.as_str()).collect();
        assert_eq!(ids, ["single-h100"], "the 8-GPU node must not be rankable");
        assert!(
            rejected.iter().any(
                |(id, r)| id == "node-8x" && matches!(r, UnsuitableReason::MultiGpuNode { .. })
            ),
            "the rejection must be reported with its reason, got {rejected:?}"
        );
    }

    /// MUTATION CAUGHT: swallowing rejections silently (returning only the ranked
    /// vec). An operator whose whole board was filtered out needs to see why, or
    /// the failure reads as "the provider has no capacity" and they go buy the
    /// wrong thing somewhere else.
    #[test]
    fn every_rejected_offer_is_reported_with_a_reason() {
        let offers = vec![offer("too-small", 1, 24_576, 0.44)];
        let (ranked, rejected) = rank_offers(
            offers,
            &req(81_920),
            &CloudProviderConfig::default(),
            &test_estimator(),
            f64::MAX,
        );
        assert!(ranked.is_empty());
        assert_eq!(rejected.len(), 1);
        assert!(matches!(
            rejected[0].1,
            UnsuitableReason::InsufficientVram {
                per_gpu_mb: 24_576,
                required_mb: 81_920
            }
        ));
    }

    /// MUTATION CAUGHT: reverting the over-budget branch to a bare `continue`.
    /// A board that is entirely priced above budget must not look identical to
    /// a board with no suitable offers at all — the operator needs "raise
    /// --max-budget", not "no capacity exists".
    #[test]
    fn an_over_budget_offer_is_reported_with_a_reason_not_silently_dropped() {
        let offers = vec![offer("single-h100", 1, 81_920, 50.0)];
        // A budget of $0 is exceeded by any offer with a positive estimated
        // cost, regardless of what the synthetic estimator's tiny fixture
        // sample count works out to in wall-clock seconds.
        let (ranked, rejected) = rank_offers(
            offers,
            &req(81_920),
            &CloudProviderConfig::default(),
            &test_estimator(),
            0.0,
        );
        assert_eq!(ranked.len(), 0, "expected no ranked offers");
        assert_eq!(rejected.len(), 1, "got {rejected:?}");
        assert!(
            matches!(rejected[0].1, UnsuitableReason::OverBudget { budget_usd, .. } if budget_usd == 0.0),
            "got {:?}",
            rejected[0].1
        );
    }
}
