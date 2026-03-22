//! Clap subcommands for [`super::db`] (`vox db …`).

use clap::Subcommand;
use std::path::PathBuf;

/// Subcommands for `vox db`.
#[derive(Subcommand)]
pub enum DbCli {
    /// Print schema version and data directory
    Status,
    /// Drop user tables and re-run migrations from a `.vox` module
    Reset {
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Print schema digest for LLM context from a `.vox` file
    Schema {
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Print sample rows from a table
    Sample {
        #[arg(long)]
        table: String,
        #[arg(long, default_value_t = 10)]
        limit: i64,
    },
    /// Apply schema migrations from declarations in a `.vox` file
    Migrate {
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Export preferences and memory for a user to JSON
    Export {
        #[arg(long)]
        user_id: String,
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Import from JSON produced by `export`
    Import {
        #[arg(required = true)]
        path: PathBuf,
    },
    /// Run VACUUM on the local database
    Vacuum,
    /// Delete old agent_memory rows for a user
    Prune {
        #[arg(long)]
        user_id: String,
        #[arg(long, default_value_t = 30)]
        days: u32,
    },
    /// Get one preference key
    #[command(name = "pref-get")]
    PrefGet {
        #[arg(long)]
        user_id: String,
        #[arg(long)]
        key: String,
    },
    /// Set one preference key
    #[command(name = "pref-set")]
    PrefSet {
        #[arg(long)]
        user_id: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        value: String,
    },
    /// List preferences (optional key prefix)
    #[command(name = "pref-list")]
    PrefList {
        #[arg(long)]
        user_id: String,
        #[arg(long)]
        prefix: Option<String>,
    },
    /// List Codex MCP invocable bindings
    #[command(name = "capability-list")]
    CapabilityList,
    /// Sync invocables from a JSON file into Codex
    #[command(name = "sync-invocables")]
    SyncInvocables {
        #[arg(required = true)]
        path: PathBuf,
    },
    /// Show retrieval / embedding diagnostics
    #[command(name = "retrieval-status")]
    RetrievalStatus,
    /// Ingest research from a URL
    #[command(name = "research-ingest-url")]
    ResearchIngestUrl {
        #[arg(long)]
        vendor: String,
        #[arg(long)]
        topic: String,
        #[arg(long)]
        url: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long, default_value = "web")]
        source_type: String,
        #[arg(long)]
        area: Option<String>,
        #[arg(long)]
        kb_id: Option<String>,
        #[arg(long)]
        tags: Option<String>,
        #[arg(long, default_value_t = 0.85)]
        confidence: f64,
    },
    /// Ingest a local markdown file as research
    #[command(name = "research-ingest-file")]
    ResearchIngestFile {
        #[arg(long)]
        vendor: String,
        #[arg(long)]
        topic: String,
        #[arg(required = true)]
        path: PathBuf,
        #[arg(long)]
        area: Option<String>,
        #[arg(long)]
        kb_id: Option<String>,
        #[arg(long)]
        tags: Option<String>,
        #[arg(long, default_value_t = 0.85)]
        confidence: f64,
    },
    /// Refresh bundled research sources (e.g. openclaw, context_engineering)
    #[command(name = "research-refresh")]
    ResearchRefresh {
        #[arg(long)]
        vendor: String,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// List stored research packets
    #[command(name = "research-list")]
    ResearchList {
        #[arg(long)]
        vendor: Option<String>,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    /// Add one capability-map row
    #[command(name = "research-map-add")]
    ResearchMapAdd {
        #[arg(long)]
        vendor: String,
        #[arg(long)]
        topic: String,
        #[arg(long)]
        area: String,
        #[arg(long)]
        openclaw_capability: String,
        #[arg(long)]
        vox_evidence: String,
        #[arg(long)]
        status: String,
        #[arg(long)]
        advantage_direction: String,
        #[arg(long)]
        recommended_action: String,
        #[arg(long)]
        linked_paths: Option<String>,
    },
    /// List capability-map rows
    #[command(name = "research-map-list")]
    ResearchMapList {
        #[arg(long)]
        vendor: Option<String>,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    /// List research metrics for a session id
    #[command(name = "research-metrics")]
    ResearchMetrics {
        #[arg(long)]
        session_id: i64,
        #[arg(long)]
        metric_type: Option<String>,
    },
}

/// Dispatch `vox db` subcommands to `commands::db` implementations.
pub async fn run(cmd: DbCli) -> anyhow::Result<()> {
    use super::db;
    match cmd {
        DbCli::Status => db::status().await,
        DbCli::Reset { file } => db::reset(file.as_ref()).await,
        DbCli::Schema { file } => db::schema(file.as_ref()).await,
        DbCli::Sample { table, limit } => db::sample(&table, limit).await,
        DbCli::Migrate { file } => db::migrate(file.as_ref()).await,
        DbCli::Export { user_id, output } => db::export(&user_id, output.as_ref()).await,
        DbCli::Import { path } => db::import(&path).await,
        DbCli::Vacuum => db::vacuum().await,
        DbCli::Prune { user_id, days } => db::prune(&user_id, days).await,
        DbCli::PrefGet { user_id, key } => db::pref_get(&user_id, &key).await,
        DbCli::PrefSet {
            user_id,
            key,
            value,
        } => db::pref_set(&user_id, &key, &value).await,
        DbCli::PrefList { user_id, prefix } => db::pref_list(&user_id, prefix.as_deref()).await,
        DbCli::CapabilityList => db::capability_list().await,
        DbCli::SyncInvocables { path } => db::sync_invocables(&path).await,
        DbCli::RetrievalStatus => db::retrieval_status().await,
        DbCli::ResearchIngestUrl {
            vendor,
            topic,
            url,
            title,
            summary,
            source_type,
            area,
            kb_id,
            tags,
            confidence,
        } => {
            db::research_ingest_url(
                &vendor,
                &topic,
                &url,
                title.as_deref(),
                summary.as_deref(),
                &source_type,
                area.as_deref(),
                kb_id.as_deref(),
                tags.as_deref(),
                confidence,
            )
            .await
        }
        DbCli::ResearchIngestFile {
            vendor,
            topic,
            path,
            area,
            kb_id,
            tags,
            confidence,
        } => {
            db::research_ingest_file(
                &vendor,
                &topic,
                &path,
                area.as_deref(),
                kb_id.as_deref(),
                tags.as_deref(),
                confidence,
            )
            .await
        }
        DbCli::ResearchRefresh { vendor, dry_run } => db::research_refresh(&vendor, dry_run).await,
        DbCli::ResearchList {
            vendor,
            topic,
            limit,
        } => db::research_list(vendor.as_deref(), topic.as_deref(), limit).await,
        DbCli::ResearchMapAdd {
            vendor,
            topic,
            area,
            openclaw_capability,
            vox_evidence,
            status,
            advantage_direction,
            recommended_action,
            linked_paths,
        } => {
            db::research_map_add(
                &vendor,
                &topic,
                &area,
                &openclaw_capability,
                &vox_evidence,
                &status,
                &advantage_direction,
                &recommended_action,
                linked_paths.as_deref(),
            )
            .await
        }
        DbCli::ResearchMapList {
            vendor,
            topic,
            limit,
        } => db::research_map_list(vendor.as_deref(), topic.as_deref(), limit).await,
        DbCli::ResearchMetrics {
            session_id,
            metric_type,
        } => db::research_metrics(session_id, metric_type.as_deref()).await,
    }
}
