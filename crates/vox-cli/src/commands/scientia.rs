//! `vox scientia` — Vox Scientia facade over Codex research and capability-map tools.
//!
//! Delegates to [`super::db_cli::DbCli`] so `vox db …` remains the implementation SSOT.

use clap::Subcommand;

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
    };
    db_cli::run(db_cmd).await
}
