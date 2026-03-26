//! Legacy `vox codex` subcommand variants (schema verify, JSONL import/export, …).

use clap::Subcommand;
use std::path::PathBuf;

/// Subcommands for the legacy `vox codex` facade.
#[derive(Subcommand)]
pub enum CodexCmd {
    /// Print schema version and whether Codex reactivity (V8) tables exist
    Verify,
    /// Export configured legacy tables as JSONL (see `vox_db::codex_legacy::LEGACY_EXPORT_TABLES`)
    ExportLegacy {
        /// Output file path
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    /// Import JSONL produced by `export-legacy`
    ImportLegacy {
        /// Input file path
        #[arg(long, short = 'i')]
        input: PathBuf,
    },
    /// Import orchestrator-style `memory/*.md` snapshots into `memories`
    #[command(name = "import-orchestrator-memory")]
    ImportOrchestratorMemory {
        /// Directory containing `*.md` files (non-recursive)
        #[arg(long)]
        dir: PathBuf,
        /// `memories.agent_id` for inserted rows
        #[arg(long)]
        agent_id: String,
        /// `memories.session_id` for inserted rows
        #[arg(long, default_value = "orchestrator-import")]
        session_id: String,
    },
    /// Upsert `skill_manifests` from a JSON file `{ id, version, manifest_json, skill_md }`
    #[command(name = "import-skill-bundle")]
    ImportSkillBundle {
        #[arg(long)]
        file: PathBuf,
    },
    /// Guided legacy-chain → fresh baseline file (export JSONL, create target DB, import, write sidecar)
    Cutover {
        /// SQLite file to create and populate (must not exist unless `--force`)
        #[arg(long)]
        target_db: PathBuf,
        /// Legacy SQLite to read (defaults to `VOX_DB_PATH` / resolved local config)
        #[arg(long)]
        source_db: Option<PathBuf>,
        /// Directory for `codex-cutover-*.jsonl` + `.sidecar.json` (default: cwd)
        #[arg(long)]
        artifact_dir: Option<PathBuf>,
        /// Overwrite `target_db` if it already exists
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Aggregate MCP Socrates `research_metrics` rows and print JSON (`SocratesSurfaceAggregate`)
    #[command(name = "socrates-metrics")]
    SocratesMetrics {
        /// Restrict to sessions `mcp:<repository_id>` (omit to include all repos)
        #[arg(long)]
        repository_id: Option<String>,
        /// Max recent `socrates_surface` rows to scan
        #[arg(long, default_value_t = 500)]
        limit: i64,
    },
    /// Append one `eval_runs` summary from recent Socrates metrics (cron-friendly)
    #[command(name = "socrates-eval-snapshot")]
    SocratesEvalSnapshot {
        /// Stable id for this snapshot (e.g. `daily-2026-03-21` or CI build id)
        #[arg(long)]
        eval_id: String,
        /// Optional repository constraint.
        #[arg(long)]
        repository_id: Option<String>,
        /// Number of recent metrics to pull into the snapshot.
        #[arg(long, default_value_t = 500)]
        limit: i64,
    },
}
