//! Shared CLI action enums.

use clap::Subcommand;

#[derive(Subcommand, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ArchitectAction {
    /// Validate workspace architecture.
    Check,
    /// Automatically move crates.
    FixSprawl {
        #[arg(long)]
        apply: bool,
    },
    /// Analyze God Objects.
    Analyze {
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
    },
}

#[derive(Subcommand, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum WorkflowAction {
    /// List all workflow and activity definitions.
    List {
        #[arg(required = true)]
        file: std::path::PathBuf,
    },
    /// Show detailed info about a specific workflow.
    Inspect {
        #[arg(required = true)]
        file: std::path::PathBuf,
        #[arg(required = true)]
        name: String,
    },
    /// Type-check a workflow file.
    Check {
        #[arg(required = true)]
        file: std::path::PathBuf,
    },
    /// Run a workflow.
    Run {
        #[arg(required = true)]
        file: std::path::PathBuf,
        #[arg(required = true)]
        name: String,
        #[arg(long)]
        args: Option<String>,
        #[arg(long)]
        run_id: Option<String>,
        #[arg(long, default_value_t = false)]
        mesh: bool,
    },
}
