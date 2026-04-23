//! Scientia subcommand definitions.

use crate::constants::*;
use crate::db_types::*;
use clap::Subcommand;

/// Subcommands for `vox scientia`.
#[derive(Subcommand, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ScientiaCmd {
    /// Validate a finding-candidate JSON document.
    #[command(name = "finding-candidate-validate")]
    FindingCandidateValidate {
        /// Path to JSON instance.
        #[arg(long)]
        json: std::path::PathBuf,
    },
    /// Validate a novelty-evidence-bundle JSON document.
    #[command(name = "novelty-evidence-bundle-validate")]
    NoveltyEvidenceBundleValidate {
        /// Path to JSON instance.
        #[arg(long)]
        json: std::path::PathBuf,
    },
    /// List Codex MCP invocable bindings.
    #[command(name = "capability-list")]
    CapabilityList,
    /// List stored research packets.
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
    /// List capability-map rows.
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
    /// Retrieval / embedding diagnostics.
    #[command(name = "retrieval-status")]
    RetrievalStatus,
    /// Mirror markdown into Codex search corpus.
    #[command(name = "mirror-search-corpus")]
    MirrorSearchCorpus {
        /// Root directory to scan recursively for `*.md` files.
        #[arg(long)]
        root: std::path::PathBuf,
        /// Prefix for `search_documents.source_uri`.
        #[arg(long, default_value = "vox-docs:")]
        source_uri_prefix: String,
    },
    /// Refresh bundled research sources.
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
    /// Merged OpenReview invitation/signature/readers.
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
    /// Worthiness rubric evaluation JSON.
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
    /// Submit through the scholarly adapter.
    #[command(name = "publication-submit-local")]
    PublicationSubmitLocal {
        /// Stable publication id.
        #[arg(long)]
        publication_id: String,
        /// Override adapter.
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
    /// Rank SCIENTIA publication candidates.
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
    /// Prior-art fetch JSON.
    #[command(name = "publication-novelty-fetch")]
    PublicationNoveltyFetch {
        #[arg(long)]
        publication_id: String,
        #[arg(long, default_value_t = false)]
        offline: bool,
        #[arg(long, default_value_t = false)]
        persist_metadata: bool,
    },
    /// Decision snapshot JSON.
    #[command(name = "publication-decision-explain")]
    PublicationDecisionExplain {
        #[arg(long)]
        publication_id: String,
        #[arg(long, default_value_t = false)]
        live_prior_art: bool,
        #[arg(long, default_value_t = false)]
        offline: bool,
    },
    /// Happy-path bundle + candidate + worthiness JSON.
    #[command(name = "publication-novelty-happy-path")]
    PublicationNoveltyHappyPath {
        #[arg(long)]
        publication_id: String,
        #[arg(long, default_value_t = false)]
        offline: bool,
    },
    /// Poll remote scholarly repository status.
    #[command(name = "publication-scholarly-remote-status")]
    PublicationScholarlyRemoteStatus {
        #[arg(long)]
        publication_id: String,
        #[arg(long)]
        external_submission_id: Option<String>,
    },
    /// Poll remote status for every scholarly submission row.
    #[command(name = "publication-scholarly-remote-status-sync-all")]
    PublicationScholarlyRemoteStatusSyncAll {
        #[arg(long)]
        publication_id: String,
    },
    /// Batch remote status poll across publications.
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
    /// Record an arXiv-assist operator milestone.
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
    /// List scholarly outbound jobs in terminal `failed` state.
    #[command(name = "publication-external-jobs-dead-letter")]
    PublicationExternalJobsDeadLetter {
        #[arg(long, default_value_t = PUBLICATION_EXTERNAL_JOBS_DEFAULT_LIMIT)]
        limit: i64,
    },
    /// Requeue one dead-letter scholarly job.
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
    /// One-command scholarly path.
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
    /// JSON rollup of external scholarly pipeline metrics.
    #[command(name = "publication-external-pipeline-metrics")]
    PublicationExternalPipelineMetrics {
        #[arg(long, default_value_t = PUBLICATION_EXTERNAL_METRICS_DEFAULT_SINCE_HOURS)]
        since_hours: i64,
    },
    /// Run one batch of Scientist RSS/Atom crawling.
    #[command(name = "ingest-tick")]
    IngestTick {
        /// Optional specific feed id to tick.
        #[arg(long)]
        feed_id: Option<String>,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Register or update a feed source for inbound intelligence.
    #[command(name = "feed-source-add")]
    FeedSourceAdd {
        #[arg(long)]
        id: String,
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "rss")]
        kind: String,
        #[arg(long, default_value_t = 3600000)]
        interval_ms: i64,
    },
    /// List registered feed sources.
    #[command(name = "feed-source-list")]
    FeedSourceList,
    /// Diagnose syndication adapter health.
    #[command(name = "diagnose")]
    Diagnose {
        /// Force live heartbeat probes.
        #[arg(long, default_value_t = false)]
        live: bool,
    },
}
