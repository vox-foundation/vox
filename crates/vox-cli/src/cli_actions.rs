use clap::{Parser, Subcommand};

/// Subcommands for `vox architect` (requires `--features codex` or `stub-check`).
#[derive(Parser)]
pub enum ArchitectAction {
    /// Validate workspace architecture against vox-schema.json
    Check,
    /// Automatically move crates to their schema-correct locations
    FixSprawl {
        /// Actually move files (default: false, dry-run only)
        #[arg(long)]
        apply: bool,
    },
    /// Analyze God Objects and suggest trait decomposition
    Analyze {
        /// Path to source file or directory
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
    },
}

#[derive(Parser)]
pub enum WorkflowAction {
    /// List all workflow and activity definitions in a .vox file
    List {
        /// Path to the .vox source file
        #[arg(required = true)]
        file: std::path::PathBuf,
    },
    /// Show detailed info about a specific workflow
    Inspect {
        /// Path to the .vox source file
        #[arg(required = true)]
        file: std::path::PathBuf,
        /// Workflow name to inspect
        #[arg(required = true)]
        name: String,
    },
    /// Type-check a workflow file
    Check {
        /// Path to the .vox source file
        #[arg(required = true)]
        file: std::path::PathBuf,
    },
    /// Run a workflow (stub for future durable execution runtime)
    Run {
        /// Path to the .vox source file
        #[arg(required = true)]
        file: std::path::PathBuf,
        /// Workflow name to run
        #[arg(required = true)]
        name: String,
        /// JSON array of workflow arguments (e.g. `["a",42]`)
        #[arg(long)]
        args: Option<String>,
        /// Resume a specific interpreted workflow run by durable run id.
        /// If omitted, a new run id is generated.
        #[arg(long)]
        run_id: Option<String>,
        /// Automatically join the local node to the mesh before starting execution.
        #[arg(long, default_value_t = false)]
        mesh: bool,
    },
}

/// `vox island …` — v0.dev React islands under `islands/src/` (requires `--features island`).
#[derive(clap::Subcommand)]
pub enum IslandCli {
    /// Generate TSX under `islands/src/<Name>/` and print or inject an `@island` stub (`V0_API_KEY`)
    Generate {
        /// PascalCase component name (e.g. `AgentCard`)
        name: String,
        #[arg(short, long)]
        prompt: String,
        /// Optional `.vox` file to inject / update `@island` block
        #[arg(short, long)]
        target: Option<std::path::PathBuf>,
        /// Bypass cache and call the API even if a matching cache entry exists
        #[arg(long, default_value_t = false)]
        force: bool,
        /// Skip `npm run build` in `islands/`
        #[arg(long, default_value_t = false)]
        no_build: bool,
        /// Optional reference image for v0
        #[arg(short, long)]
        image: Option<std::path::PathBuf>,
    },
    /// Re-generate from existing TSX + new instructions (always calls API; bypasses cache)
    Upgrade {
        name: String,
        #[arg(short, long)]
        prompt: String,
        #[arg(long, default_value_t = false)]
        no_build: bool,
    },
    /// List islands from `islands/src/` and `Vox.toml [islands]`
    List {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Run `npx shadcn@latest add` inside `islands/`
    Add {
        component: String,
        #[arg(short, long)]
        from: Option<String>,
    },
    /// Manage `~/.vox/island-cache/` entries
    Cache {
        #[command(subcommand)]
        action: IslandCacheAction,
    },
}

/// Subcommands for `vox island cache`.
#[derive(Subcommand)]
pub enum IslandCacheAction {
    /// List cached generations
    List,
    /// Remove all cache entries
    Clear,
    /// Remove one cache entry by island name
    Remove {
        /// Same PascalCase name as `vox island generate`
        name: String,
    },
}
