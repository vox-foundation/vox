//! `DbCli` clap enum — kept separate to keep `mod.rs` under the god-object line budget.

use clap::Subcommand;
use std::path::PathBuf;

use super::types::{DbPreflightProfileCli, PublicationPrepareBodyCli};

/// Subcommands for `vox db`.
#[derive(Subcommand)]
pub enum DbCli {
    /// Print schema version and data directory
    Status,
    /// Drop user tables and re-run migrations from a `.vox` module
    Reset {
        /// Optional specific schema source file.
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Print schema digest for LLM context from a `.vox` file
    Schema {
        /// Source file defining compiler entities.
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Print sample rows from a table
    Sample {
        /// Target table name.
        #[arg(long)]
        table: String,
        /// Maximum number of rows to print.
        #[arg(long, default_value_t = 10)]
        limit: i64,
    },
    /// Apply schema migrations from declarations in a `.vox` file
    Migrate {
        /// Source file containing schema states.
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Export preferences and memory for a user to JSON
    Export {
        /// User identifier to query.
        #[arg(long)]
        user_id: String,
        /// File to write JSON to.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Import from JSON produced by `export`
    Import {
        /// Source JSON file.
        #[arg(required = true)]
        path: PathBuf,
    },
    /// Run VACUUM on the local database
    Vacuum,
    /// Delete old agent_memory rows for a user
    Prune {
        /// User identifier.
        #[arg(long)]
        user_id: String,
        /// Retain rows from the last N days.
        #[arg(long, default_value_t = 30)]
        days: u32,
    },
    /// Get one preference key
    #[command(name = "pref-get")]
    PrefGet {
        /// User identifier.
        #[arg(long)]
        user_id: String,
        /// Target preference key string.
        #[arg(long)]
        key: String,
    },
    /// Set one preference key
    #[command(name = "pref-set")]
    PrefSet {
        /// User identifier.
        #[arg(long)]
        user_id: String,
        /// String key to override.
        #[arg(long)]
        key: String,
        /// JSON-compatible string value.
        #[arg(long)]
        value: String,
    },
    /// List preferences (optional key prefix)
    #[command(name = "pref-list")]
    PrefList {
        /// User identifier.
        #[arg(long)]
        user_id: String,
        /// Optional substring prefix to match against keys.
        #[arg(long)]
        prefix: Option<String>,
    },
    /// List Codex MCP invocable bindings
    #[command(name = "capability-list")]
    CapabilityList,
    /// Sync invocables from a JSON file into Codex
    #[command(name = "sync-invocables")]
    SyncInvocables {
        /// Input JSON file path.
        #[arg(required = true)]
        path: PathBuf,
    },
    /// Show retrieval / embedding diagnostics
    #[command(name = "retrieval-status")]
    RetrievalStatus,
    /// Ingest research from a URL
    #[command(name = "research-ingest-url")]
    ResearchIngestUrl {
        /// Namespace / vendor name.
        #[arg(long)]
        vendor: String,
        /// Logical research topic grouping.
        #[arg(long)]
        topic: String,
        /// Canonical HTTP URL.
        #[arg(long)]
        url: String,
        /// Optional display title.
        #[arg(long)]
        title: Option<String>,
        /// Short excerpt or AI summary.
        #[arg(long)]
        summary: Option<String>,
        /// Classification (e.g. `web`, `pdf`).
        #[arg(long, default_value = "web")]
        source_type: String,
        /// Agentic mapping sub-area.
        #[arg(long)]
        area: Option<String>,
        /// Upstream knowledge-base ID for diffing.
        #[arg(long)]
        kb_id: Option<String>,
        /// Search indexes and comma-separated context values.
        #[arg(long)]
        tags: Option<String>,
        /// Trust baseline rating (0.0 to 1.0).
        #[arg(long, default_value_t = 0.85)]
        confidence: f64,
    },
    /// Ingest a local markdown file as research
    #[command(name = "research-ingest-file")]
    ResearchIngestFile {
        /// Namespace / vendor name.
        #[arg(long)]
        vendor: String,
        /// Logical research topic grouping.
        #[arg(long)]
        topic: String,
        /// Path to local ingestible content.
        #[arg(required = true)]
        path: PathBuf,
        /// Agentic mapping sub-area.
        #[arg(long)]
        area: Option<String>,
        /// Upstream knowledge base ID.
        #[arg(long)]
        kb_id: Option<String>,
        /// Optional tagging dimensions.
        #[arg(long)]
        tags: Option<String>,
        /// Trust baseline (0.0 to 1.0).
        #[arg(long, default_value_t = 0.85)]
        confidence: f64,
    },
    /// Refresh bundled research sources (e.g. openclaw, context_engineering)
    #[command(name = "research-refresh")]
    ResearchRefresh {
        /// Specific vendor path to refresh.
        #[arg(long)]
        vendor: String,
        /// Avoid writing to DB if true.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// List stored research packets
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
    /// Add one capability-map row
    #[command(name = "research-map-add")]
    ResearchMapAdd {
        /// Provider name (e.g. `openai`).
        #[arg(long)]
        vendor: String,
        /// Logical domain (e.g. `vision`).
        #[arg(long)]
        topic: String,
        /// Specialization area.
        #[arg(long)]
        area: String,
        /// OpenClaw capability identifier.
        #[arg(long)]
        openclaw_capability: String,
        /// Local test ID that demonstrates this feature.
        #[arg(long)]
        vox_evidence: String,
        /// Support level: `stable`, `beta`, `unsupported`.
        #[arg(long)]
        status: String,
        /// Indicates if Vox or the vendor currently handles this better.
        #[arg(long)]
        advantage_direction: String,
        /// Suggested action for the planner based on this map.
        #[arg(long)]
        recommended_action: String,
        /// Comma-separated list of test paths.
        #[arg(long)]
        linked_paths: Option<String>,
    },
    /// List capability-map rows
    #[command(name = "research-map-list")]
    ResearchMapList {
        /// Filter by vendor name.
        #[arg(long)]
        vendor: Option<String>,
        /// Filter by capability topic.
        #[arg(long)]
        topic: Option<String>,
        /// Query length limit.
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    /// List research metrics for a session id
    #[command(name = "research-metrics")]
    ResearchMetrics {
        /// ID of the Socrates session run.
        #[arg(long)]
        session_id: i64,
        /// Optional string matched against metric_type column (e.g., `hallucination_score`).
        #[arg(long)]
        metric_type: Option<String>,
    },
    /// List reliability scores for LLM endpoints, skills, workflows, or repositories
    #[command(name = "reliability-list")]
    ReliabilityList {
        /// Target domain: endpoints, skills, workflows, repositories
        #[arg(long, default_value = "endpoints")]
        domain: String,
        /// Row limit for listing.
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    /// List reliability scores and stats for execution agents
    #[command(name = "reliability-agents")]
    ReliabilityAgents {
        /// Row limit for listing.
        #[arg(long, default_value_t = 50)]
        limit: i64,
        /// Minimum reliability score (0.0 to 1.0)
        #[arg(long)]
        min_score: Option<f64>,
    },
    /// Upsert a publication manifest from markdown for scientia/journal flows.
    #[command(name = "publication-prepare")]
    PublicationPrepare {
        /// Content type (`scientia`, `news`, `paper`, ...).
        #[arg(long, default_value = "scientia")]
        content_type: String,
        #[command(flatten)]
        body: PublicationPrepareBodyCli,
        /// Run [`vox_publisher::publication_preflight`] before upsert; fail on error-level findings.
        #[arg(long, default_value_t = false)]
        preflight: bool,
        #[arg(long, value_enum, default_value_t = DbPreflightProfileCli::Default)]
        preflight_profile: DbPreflightProfileCli,
    },
    /// Like `publication-prepare` but always runs preflight (same as `--preflight`).
    #[command(name = "publication-prepare-validated")]
    PublicationPrepareValidated {
        /// Content type (`scientia`, `news`, `paper`, ...).
        #[arg(long, default_value = "scientia")]
        content_type: String,
        #[command(flatten)]
        body: PublicationPrepareBodyCli,
        #[arg(long, value_enum, default_value_t = DbPreflightProfileCli::Default)]
        preflight_profile: DbPreflightProfileCli,
    },
    /// Print publication preflight report JSON for an existing manifest (no writes).
    #[command(name = "publication-preflight")]
    PublicationPreflight {
        #[arg(long)]
        publication_id: String,
        #[arg(long, value_enum, default_value_t = DbPreflightProfileCli::Default)]
        profile: DbPreflightProfileCli,
    },
    /// Print Zenodo deposit `metadata` JSON for an existing manifest (stdout only; no HTTP).
    #[command(name = "publication-zenodo-metadata")]
    PublicationZenodoMetadata {
        #[arg(long)]
        publication_id: String,
    },
    /// Record digest-bound approval for a prepared publication.
    #[command(name = "publication-approve")]
    PublicationApprove {
        /// Stable publication id.
        #[arg(long)]
        publication_id: String,
        /// Approver identity (must be distinct across at least two values).
        #[arg(long)]
        approver: String,
    },
    /// Submit a prepared publication using the local scholarly adapter.
    #[command(name = "publication-submit-local")]
    PublicationSubmitLocal {
        /// Stable publication id.
        #[arg(long)]
        publication_id: String,
    },
    /// Print publication manifest, approvals, and scholarly submission rows.
    #[command(name = "publication-status")]
    PublicationStatus {
        /// Stable publication id.
        #[arg(long)]
        publication_id: String,
    },
}
