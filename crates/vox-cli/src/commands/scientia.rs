//! `vox scientia` — Vox Scientia facade over Codex research and capability-map tools.
//!
//! Delegates to [`super::db_cli::DbCli`] so `vox db …` remains the implementation SSOT.

use clap::Subcommand;

use super::db_cli::{DbPreflightProfileCli, PublicationPrepareBodyCli};

/// Subcommands for `vox scientia`.
#[derive(Subcommand)]
pub enum ScientiaCmd {
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
    },
    /// Same as `publication-prepare` with mandatory preflight.
    #[command(name = "publication-prepare-validated")]
    PublicationPrepareValidated {
        #[command(flatten)]
        body: PublicationPrepareBodyCli,
        #[arg(long, value_enum, default_value_t = DbPreflightProfileCli::Default)]
        preflight_profile: DbPreflightProfileCli,
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
    /// Submit through the local scholarly adapter.
    #[command(name = "publication-submit-local")]
    PublicationSubmitLocal {
        /// Stable publication id.
        #[arg(long)]
        publication_id: String,
    },
    /// Show manifest + approval + scholarly status.
    #[command(name = "publication-status")]
    PublicationStatus {
        /// Stable publication id.
        #[arg(long)]
        publication_id: String,
    },
}

/// Dispatch `vox scientia` to the shared `vox db` handlers.
pub async fn run(cmd: ScientiaCmd) -> anyhow::Result<()> {
    use super::db_cli::{self, DbCli};
    let db_cmd = match cmd {
        ScientiaCmd::CapabilityList => DbCli::CapabilityList,
        ScientiaCmd::ResearchList {
            vendor,
            topic,
            limit,
        } => DbCli::ResearchList {
            vendor,
            topic,
            limit,
        },
        ScientiaCmd::ResearchMapList {
            vendor,
            topic,
            limit,
        } => DbCli::ResearchMapList {
            vendor,
            topic,
            limit,
        },
        ScientiaCmd::RetrievalStatus => DbCli::RetrievalStatus,
        ScientiaCmd::ResearchRefresh { vendor, dry_run } => {
            DbCli::ResearchRefresh { vendor, dry_run }
        }
        ScientiaCmd::PublicationPrepare {
            body,
            preflight,
            preflight_profile,
        } => DbCli::PublicationPrepare {
            content_type: "scientia".to_string(),
            body,
            preflight,
            preflight_profile,
        },
        ScientiaCmd::PublicationPrepareValidated {
            body,
            preflight_profile,
        } => DbCli::PublicationPrepareValidated {
            content_type: "scientia".to_string(),
            body,
            preflight_profile,
        },
        ScientiaCmd::PublicationPreflight {
            publication_id,
            profile,
            with_worthiness,
        } => DbCli::PublicationPreflight {
            publication_id,
            profile,
            with_worthiness,
        },
        ScientiaCmd::PublicationZenodoMetadata { publication_id } => {
            DbCli::PublicationZenodoMetadata { publication_id }
        }
        ScientiaCmd::PublicationWorthinessEvaluate {
            contract_yaml,
            metrics_json,
        } => DbCli::PublicationWorthinessEvaluate {
            contract_yaml,
            metrics_json,
        },
        ScientiaCmd::PublicationApprove {
            publication_id,
            approver,
        } => DbCli::PublicationApprove {
            publication_id,
            approver,
        },
        ScientiaCmd::PublicationSubmitLocal { publication_id } => {
            DbCli::PublicationSubmitLocal { publication_id }
        }
        ScientiaCmd::PublicationStatus { publication_id } => {
            DbCli::PublicationStatus { publication_id }
        }
    };
    db_cli::run(db_cmd).await
}
