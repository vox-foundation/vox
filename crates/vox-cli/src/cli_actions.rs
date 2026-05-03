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

