//! `vox scientia` — Vox Scientia facade over Codex research and capability-map tools.
//!
//! Delegates to [`super::db_cli::DbCli`] so `vox db …` remains the implementation SSOT.

use clap::Subcommand;

use super::db_cli::{
    ArxivHandoffStageCli, DbPreflightProfileCli, DiscoveryIntakeGateCli,
    PUBLICATION_EXTERNAL_JOBS_DEFAULT_LIMIT, PUBLICATION_EXTERNAL_JOBS_TICK_DEFAULT_LIMIT,
    PUBLICATION_EXTERNAL_JOBS_TICK_DEFAULT_LOCK_TTL_MS,
    PUBLICATION_EXTERNAL_METRICS_DEFAULT_SINCE_HOURS, PUBLICATION_SYNC_BATCH_DEFAULT_LIMIT,
    PUBLICATION_WORKER_DEFAULT_INTERVAL_SECS, PUBLICATION_WORKER_DEFAULT_ITERATIONS,
    PUBLICATION_WORKER_DEFAULT_JITTER_SECS, PublicationPrepareBodyCli, ScholarlyVenueCli,
};

/// Subcommands for `vox scientia`.
#[derive(Subcommand)]
pub enum ScientiaCmd {
    /// Validate a finding-candidate JSON document against `contracts/scientia/finding-candidate.v1.schema.json`.
    #[command(name = "finding-candidate-validate")]
    FindingCandidateValidate {
        /// Path to JSON instance (absolute or repo-relative from cwd).
        #[arg(long)]
        json: std::path::PathBuf,
    },
    /// Validate a novelty-evidence-bundle JSON document against `contracts/scientia/novelty-evidence-bundle.v1.schema.json`.
    #[command(name = "novelty-evidence-bundle-validate")]
    NoveltyEvidenceBundleValidate {
        /// Path to JSON instance (absolute or repo-relative from cwd).
        #[arg(long)]
        json: std::path::PathBuf,
    },
    /// List Codex MCP invocable bindings (same as `vox db capability-list`).
    #[command(name = "capability-list")]
    CapabilityList,
    /// List stored research packets (`vox db research-list`).
    #[command(name = "research-list")]
    ResearchList {
        /// Optional namespace/vendor filter.
        #[arg(long)]
        vendor: Option<String>,
        /// Optional specific topic filter.
        #[arg(long)]
        topic: Option<String>,
        /// Row limit for listing.
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    /// List capability-map rows (`vox db research-map-list`).
    #[command(name = "research-map-list")]
    ResearchMapList {
        /// Optional namespace/vendor filter.
        #[arg(long)]
        vendor: Option<String>,
        /// Optional specific topic filter.
        #[arg(long)]
        topic: Option<String>,
        /// Row limit for listing.
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    /// Retrieval / embedding diagnostics (`vox db retrieval-status`).
    #[command(name = "retrieval-status")]
    RetrievalStatus,
    /// Mirror markdown into Codex search corpus (`vox db mirror-search-corpus`).
    #[command(name = "mirror-search-corpus")]
    MirrorSearchCorpus {
        /// Root directory to scan recursively for `*.md` files.
        #[arg(long)]
        root: std::path::PathBuf,
        /// Prefix for `search_documents.source_uri` (e.g. `vox-docs:`).
        #[arg(long, default_value = "vox-docs:")]
        source_uri_prefix: String,
    },
    /// Refresh bundled research sources (`vox db research-refresh`).
    #[command(name = "research-refresh")]
    ResearchRefresh {
        /// Specific vendor/provider path to refresh.
        #[arg(long)]
        vendor: String,
        /// Only check sync status without executing the refresh.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Prepare a scientific publication manifest from markdown.
    #[command(name = "publication-prepare")]
    PublicationPrepare {
        #[command(flatten)]
        body: PublicationPrepareBodyCli,
        #[arg(long, default_value_t = false)]
        preflight: bool,
        #[arg(long, value_enum, default_value_t = DbPreflightProfileCli::Default)]
        preflight_profile: DbPreflightProfileCli,
        #[arg(long, value_enum, default_value_t = DiscoveryIntakeGateCli::None)]
        discovery_intake_gate: DiscoveryIntakeGateCli,
    },
    /// Same as `publication-prepare` with mandatory preflight.
    #[command(name = "publication-prepare-validated")]
    PublicationPrepareValidated {
        #[command(flatten)]
        body: PublicationPrepareBodyCli,
        #[arg(long, value_enum, default_value_t = DbPreflightProfileCli::Default)]
        preflight_profile: DbPreflightProfileCli,
        #[arg(long, value_enum, default_value_t = DiscoveryIntakeGateCli::None)]
        discovery_intake_gate: DiscoveryIntakeGateCli,
    },
    /// JSON preflight report for an existing publication id.
    #[command(name = "publication-preflight")]
    PublicationPreflight {
        #[arg(long)]
        publication_id: String,
        #[arg(long, value_enum, default_value_t = DbPreflightProfileCli::Default)]
        profile: DbPreflightProfileCli,
        #[arg(long, default_value_t = false)]
        with_worthiness: bool,
    },
    #[command(name = "publication-zenodo-metadata")]
    PublicationZenodoMetadata {
        #[arg(long)]
        publication_id: String,
    },
    /// Merged OpenReview invitation/signature/readers + API base (stdout JSON; no HTTP).
    #[command(name = "publication-openreview-profile")]
    PublicationOpenreviewProfile {
        #[arg(long)]
        publication_id: String,
    },
    #[command(name = "publication-scholarly-staging-export")]
    PublicationScholarlyStagingExport {
        #[arg(long)]
        publication_id: String,
        #[arg(long)]
        output_dir: std::path::PathBuf,
        #[arg(long, value_enum)]
        venue: ScholarlyVenueCli,
    },
    /// Worthiness rubric evaluation JSON (`vox db publication-worthiness-evaluate`).
    #[command(name = "publication-worthiness-evaluate")]
    PublicationWorthinessEvaluate {
        #[arg(long)]
        contract_yaml: Option<std::path::PathBuf>,
        #[arg(long)]
        metrics_json: std::path::PathBuf,
    },
    /// Record digest-bound approval for a prepared publication.
    #[command(name = "publication-approve")]
    PublicationApprove {
        /// Stable publication id.
        #[arg(long)]
        publication_id: String,
        /// Approver identity.
        #[arg(long)]
        approver: String,
    },
    /// Submit through the scholarly adapter (`--adapter` or `VOX_SCHOLARLY_ADAPTER`).
    #[command(name = "publication-submit-local")]
    PublicationSubmitLocal {
        /// Stable publication id.
        #[arg(long)]
        publication_id: String,
        /// Override adapter: `local_ledger`, `echo_ledger`, `zenodo`, `openreview` (default: env).
        #[arg(long)]
        adapter: Option<String>,
    },
    /// Show manifest + approval + scholarly status.
    #[command(name = "publication-status")]
    PublicationStatus {
        /// Stable publication id.
        #[arg(long)]
        publication_id: String,
        #[arg(long, default_value_t = false)]
        with_worthiness: bool,
    },
    /// Rank SCIENTIA publication candidates (`content_type` fixed to `scientia` for this facade).
    #[command(name = "publication-discovery-scan")]
    PublicationDiscoveryScan {
        #[arg(long)]
        state: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    #[command(name = "publication-discovery-explain")]
    PublicationDiscoveryExplain {
        #[arg(long)]
        publication_id: String,
    },
    #[command(name = "publication-transform-preview")]
    PublicationTransformPreview {
        #[arg(long)]
        publication_id: String,
    },
    /// Prior-art fetch JSON (same as `vox db publication-novelty-fetch`; scientia `content_type` enforced on persist path in handler).
    #[command(name = "publication-novelty-fetch")]
    PublicationNoveltyFetch {
        #[arg(long)]
        publication_id: String,
        #[arg(long, default_value_t = false)]
        offline: bool,
        #[arg(long, default_value_t = false)]
        persist_metadata: bool,
    },
    /// Decision snapshot JSON (`vox db publication-decision-explain`).
    #[command(name = "publication-decision-explain")]
    PublicationDecisionExplain {
        #[arg(long)]
        publication_id: String,
        #[arg(long, default_value_t = false)]
        live_prior_art: bool,
        #[arg(long, default_value_t = false)]
        offline: bool,
    },
    /// Happy-path bundle + candidate + worthiness JSON (`vox db publication-novelty-happy-path`).
    #[command(name = "publication-novelty-happy-path")]
    PublicationNoveltyHappyPath {
        #[arg(long)]
        publication_id: String,
        #[arg(long, default_value_t = false)]
        offline: bool,
    },
    /// Poll remote scholarly repository status for a stored submission.
    #[command(name = "publication-scholarly-remote-status")]
    PublicationScholarlyRemoteStatus {
        #[arg(long)]
        publication_id: String,
        #[arg(long)]
        external_submission_id: Option<String>,
    },
    /// Poll remote status for every scholarly submission row on this publication (multi-deposit / multi-venue).
    #[command(name = "publication-scholarly-remote-status-sync-all")]
    PublicationScholarlyRemoteStatusSyncAll {
        #[arg(long)]
        publication_id: String,
    },
    /// Batch remote status poll across publications (for cron).
    #[command(name = "publication-scholarly-remote-status-sync-batch")]
    PublicationScholarlyRemoteStatusSyncBatch {
        #[arg(long, default_value_t = PUBLICATION_SYNC_BATCH_DEFAULT_LIMIT)]
        limit: i64,
        #[arg(long, default_value_t = PUBLICATION_WORKER_DEFAULT_ITERATIONS)]
        iterations: u32,
        #[arg(long, default_value_t = PUBLICATION_WORKER_DEFAULT_INTERVAL_SECS)]
        interval_secs: u64,
        #[arg(long)]
        max_runtime_secs: Option<u64>,
        #[arg(long, default_value_t = PUBLICATION_WORKER_DEFAULT_JITTER_SECS)]
        jitter_secs: u64,
    },
    /// Record an arXiv-assist operator milestone (append-only audit trail).
    #[command(name = "publication-arxiv-handoff-record")]
    PublicationArxivHandoffRecord {
        #[arg(long)]
        publication_id: String,
        #[arg(long)]
        stage: ArxivHandoffStageCli,
        #[arg(long)]
        operator: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        arxiv_id: Option<String>,
    },
    #[command(name = "publication-external-jobs-due")]
    PublicationExternalJobsDue {
        #[arg(long, default_value_t = PUBLICATION_EXTERNAL_JOBS_DEFAULT_LIMIT)]
        limit: i64,
    },
    /// List scholarly outbound jobs in terminal `failed` state (dead-letter review).
    #[command(name = "publication-external-jobs-dead-letter")]
    PublicationExternalJobsDeadLetter {
        #[arg(long, default_value_t = PUBLICATION_EXTERNAL_JOBS_DEFAULT_LIMIT)]
        limit: i64,
    },
    /// Requeue one dead-letter scholarly job (`failed` → `queued`) by job id.
    #[command(name = "publication-external-jobs-replay")]
    PublicationExternalJobsReplay {
        #[arg(long)]
        job_id: i64,
    },
    #[command(name = "publication-external-jobs-tick")]
    PublicationExternalJobsTick {
        #[arg(long, default_value_t = PUBLICATION_EXTERNAL_JOBS_TICK_DEFAULT_LIMIT)]
        limit: i64,
        #[arg(long, default_value_t = PUBLICATION_EXTERNAL_JOBS_TICK_DEFAULT_LOCK_TTL_MS)]
        lock_ttl_ms: i64,
        #[arg(long)]
        lock_owner: Option<String>,
        #[arg(long, default_value_t = PUBLICATION_WORKER_DEFAULT_ITERATIONS)]
        iterations: u32,
        #[arg(long, default_value_t = PUBLICATION_WORKER_DEFAULT_INTERVAL_SECS)]
        interval_secs: u64,
        #[arg(long)]
        max_runtime_secs: Option<u64>,
        #[arg(long, default_value_t = PUBLICATION_WORKER_DEFAULT_JITTER_SECS)]
        jitter_secs: u64,
    },
    /// One-command scholarly path: preflight, dual approval, optional staging, submit (same as `vox db publication-scholarly-pipeline-run`).
    #[command(name = "publication-scholarly-pipeline-run")]
    PublicationScholarlyPipelineRun {
        #[arg(long)]
        publication_id: String,
        #[arg(long, value_enum, default_value_t = DbPreflightProfileCli::Default)]
        preflight_profile: DbPreflightProfileCli,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long)]
        staging_output_dir: Option<std::path::PathBuf>,
        #[arg(long, value_enum)]
        venue: Option<ScholarlyVenueCli>,
        #[arg(long)]
        adapter: Option<String>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// JSON rollup of external scholarly pipeline metrics (see `vox db publication-external-pipeline-metrics`).
    #[command(name = "publication-external-pipeline-metrics")]
    PublicationExternalPipelineMetrics {
        #[arg(long, default_value_t = PUBLICATION_EXTERNAL_METRICS_DEFAULT_SINCE_HOURS)]
        since_hours: i64,
    },
}

/// Dispatch `vox scientia` to the shared `vox db` handlers.
pub async fn run(cmd: ScientiaCmd) -> anyhow::Result<()> {
    use super::ci::repo_root;
    use super::db_cli::{self, DbCli, DbCliCore, DbCliPublication};
    use super::scientia_ledger_contract;

    match cmd {
        ScientiaCmd::FindingCandidateValidate { json } => {
            let root = repo_root();
            let path = if json.is_absolute() {
                json
            } else {
                std::env::current_dir()?.join(json)
            };
            scientia_ledger_contract::validate_finding_candidate_file(&root, &path)?;
            return Ok(());
        }
        ScientiaCmd::NoveltyEvidenceBundleValidate { json } => {
            let root = repo_root();
            let path = if json.is_absolute() {
                json
            } else {
                std::env::current_dir()?.join(json)
            };
            scientia_ledger_contract::validate_novelty_bundle_file(&root, &path)?;
            return Ok(());
        }
        cmd => {
            let db_cmd = match cmd {
        ScientiaCmd::FindingCandidateValidate { .. }
        | ScientiaCmd::NoveltyEvidenceBundleValidate { .. } => unreachable!(
            "finding-candidate-validate and novelty-evidence-bundle-validate handled above"
        ),
        ScientiaCmd::CapabilityList => DbCli::Core(DbCliCore::CapabilityList),
        ScientiaCmd::ResearchList {
            vendor,
            topic,
            limit,
        } => DbCli::Core(DbCliCore::ResearchList {
            vendor,
            topic,
            limit,
        }),
        ScientiaCmd::ResearchMapList {
            vendor,
            topic,
            limit,
        } => DbCli::Core(DbCliCore::ResearchMapList {
            vendor,
            topic,
            limit,
        }),
        ScientiaCmd::RetrievalStatus => DbCli::Core(DbCliCore::RetrievalStatus),
        ScientiaCmd::MirrorSearchCorpus {
            root,
            source_uri_prefix,
        } => DbCli::Core(DbCliCore::MirrorSearchCorpus {
            root,
            source_uri_prefix,
        }),
        ScientiaCmd::ResearchRefresh { vendor, dry_run } => {
            DbCli::Core(DbCliCore::ResearchRefresh { vendor, dry_run })
        }
        ScientiaCmd::PublicationPrepare {
            body,
            preflight,
            preflight_profile,
            discovery_intake_gate,
        } => DbCli::Publication(DbCliPublication::PublicationPrepare {
            content_type: "scientia".to_string(),
            body,
            preflight,
            preflight_profile,
            discovery_intake_gate,
        }),
        ScientiaCmd::PublicationPrepareValidated {
            body,
            preflight_profile,
            discovery_intake_gate,
        } => DbCli::Publication(DbCliPublication::PublicationPrepareValidated {
            content_type: "scientia".to_string(),
            body,
            preflight_profile,
            discovery_intake_gate,
        }),
        ScientiaCmd::PublicationPreflight {
            publication_id,
            profile,
            with_worthiness,
        } => DbCli::Publication(DbCliPublication::PublicationPreflight {
            publication_id,
            profile,
            with_worthiness,
        }),
        ScientiaCmd::PublicationZenodoMetadata { publication_id } => {
            DbCli::Publication(DbCliPublication::PublicationZenodoMetadata { publication_id })
        }
        ScientiaCmd::PublicationOpenreviewProfile { publication_id } => {
            DbCli::Publication(DbCliPublication::PublicationOpenreviewProfile { publication_id })
        }
        ScientiaCmd::PublicationScholarlyStagingExport {
            publication_id,
            output_dir,
            venue,
        } => DbCli::Publication(DbCliPublication::PublicationScholarlyStagingExport {
            publication_id,
            output_dir,
            venue,
        }),
        ScientiaCmd::PublicationWorthinessEvaluate {
            contract_yaml,
            metrics_json,
        } => DbCli::Publication(DbCliPublication::PublicationWorthinessEvaluate {
            contract_yaml,
            metrics_json,
        }),
        ScientiaCmd::PublicationApprove {
            publication_id,
            approver,
        } => DbCli::Publication(DbCliPublication::PublicationApprove {
            publication_id,
            approver,
        }),
        ScientiaCmd::PublicationSubmitLocal {
            publication_id,
            adapter,
        } => DbCli::Publication(DbCliPublication::PublicationSubmitLocal {
            publication_id,
            adapter,
        }),
        ScientiaCmd::PublicationStatus {
            publication_id,
            with_worthiness,
        } => DbCli::Publication(DbCliPublication::PublicationStatus {
            publication_id,
            with_worthiness,
        }),
        ScientiaCmd::PublicationDiscoveryScan { state, limit } => {
            DbCli::Publication(DbCliPublication::PublicationDiscoveryScan {
                content_type: Some("scientia".to_string()),
                state,
                limit,
            })
        }
        ScientiaCmd::PublicationDiscoveryExplain { publication_id } => {
            DbCli::Publication(DbCliPublication::PublicationDiscoveryExplain { publication_id })
        }
        ScientiaCmd::PublicationTransformPreview { publication_id } => {
            DbCli::Publication(DbCliPublication::PublicationTransformPreview { publication_id })
        }
        ScientiaCmd::PublicationNoveltyFetch {
            publication_id,
            offline,
            persist_metadata,
        } => DbCli::Publication(DbCliPublication::PublicationNoveltyFetch {
            publication_id,
            offline,
            persist_metadata,
        }),
        ScientiaCmd::PublicationDecisionExplain {
            publication_id,
            live_prior_art,
            offline,
        } => DbCli::Publication(DbCliPublication::PublicationDecisionExplain {
            publication_id,
            live_prior_art,
            offline,
        }),
        ScientiaCmd::PublicationNoveltyHappyPath {
            publication_id,
            offline,
        } => DbCli::Publication(DbCliPublication::PublicationNoveltyHappyPath {
            publication_id,
            offline,
        }),
        ScientiaCmd::PublicationScholarlyRemoteStatus {
            publication_id,
            external_submission_id,
        } => DbCli::Publication(DbCliPublication::PublicationScholarlyRemoteStatus {
            publication_id,
            external_submission_id,
        }),
        ScientiaCmd::PublicationScholarlyRemoteStatusSyncAll { publication_id } => {
            DbCli::Publication(DbCliPublication::PublicationScholarlyRemoteStatusSyncAll {
                publication_id,
            })
        }
        ScientiaCmd::PublicationScholarlyRemoteStatusSyncBatch {
            limit,
            iterations,
            interval_secs,
            max_runtime_secs,
            jitter_secs,
        } => DbCli::Publication(
            DbCliPublication::PublicationScholarlyRemoteStatusSyncBatch {
                limit,
                iterations,
                interval_secs,
                max_runtime_secs,
                jitter_secs,
            },
        ),
        ScientiaCmd::PublicationArxivHandoffRecord {
            publication_id,
            stage,
            operator,
            note,
            arxiv_id,
        } => DbCli::Publication(DbCliPublication::PublicationArxivHandoffRecord {
            publication_id,
            stage,
            operator,
            note,
            arxiv_id,
        }),
        ScientiaCmd::PublicationExternalJobsDue { limit } => {
            DbCli::Publication(DbCliPublication::PublicationExternalJobsDue { limit })
        }
        ScientiaCmd::PublicationExternalJobsDeadLetter { limit } => {
            DbCli::Publication(DbCliPublication::PublicationExternalJobsDeadLetter { limit })
        }
        ScientiaCmd::PublicationExternalJobsReplay { job_id } => {
            DbCli::Publication(DbCliPublication::PublicationExternalJobsReplay { job_id })
        }
        ScientiaCmd::PublicationExternalJobsTick {
            limit,
            lock_ttl_ms,
            lock_owner,
            iterations,
            interval_secs,
            max_runtime_secs,
            jitter_secs,
        } => DbCli::Publication(DbCliPublication::PublicationExternalJobsTick {
            limit,
            lock_ttl_ms,
            lock_owner,
            iterations,
            interval_secs,
            max_runtime_secs,
            jitter_secs,
        }),
        ScientiaCmd::PublicationScholarlyPipelineRun {
            publication_id,
            preflight_profile,
            dry_run,
            staging_output_dir,
            venue,
            adapter,
            json,
        } => DbCli::Publication(DbCliPublication::PublicationScholarlyPipelineRun {
            publication_id,
            preflight_profile,
            dry_run,
            staging_output_dir,
            venue,
            adapter,
            json,
        }),
        ScientiaCmd::PublicationExternalPipelineMetrics { since_hours } => {
            DbCli::Publication(DbCliPublication::PublicationExternalPipelineMetrics { since_hours })
        }
            };
            db_cli::run(db_cmd).await
        }
    }
}
