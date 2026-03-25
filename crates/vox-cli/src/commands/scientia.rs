//! `vox scientia` — Vox Scientia facade over Codex research and capability-map tools.
//!
//! Delegates to [`super::db_cli::DbCli`] so `vox db …` remains the implementation SSOT.

use clap::Subcommand;
use std::path::PathBuf;

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
        /// Stable publication id.
        #[arg(long)]
        publication_id: String,
        /// Author identity.
        #[arg(long)]
        author: String,
        /// Human title.
        #[arg(long)]
        title: String,
        /// Path to markdown body.
        #[arg(required = true)]
        path: PathBuf,
        /// Optional abstract text.
        #[arg(long)]
        abstract_text: Option<String>,
        /// Optional citations JSON file path.
        #[arg(long)]
        citations_json: Option<PathBuf>,
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
            publication_id,
            author,
            title,
            path,
            abstract_text,
            citations_json,
        } => DbCli::PublicationPrepare {
            publication_id,
            content_type: "scientia".to_string(),
            author,
            title,
            path,
            abstract_text,
            citations_json,
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
