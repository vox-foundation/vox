//! Shared [`clap::Args`] structs for top-level `vox` commands and Latin namespace groups.

use clap::Args;
use std::path::PathBuf;

/// `vox build` / `vox fabrica build`
#[derive(Args, Clone, Debug)]
pub struct BuildArgs {
    /// Path to the `.vox` file
    #[arg(required = true)]
    pub file: PathBuf,
    /// Output directory for generated TypeScript
    #[arg(short, long, default_value = "dist")]
    pub out_dir: PathBuf,
}

/// `vox check` / `vox fabrica check`
#[derive(Args, Clone, Debug)]
pub struct CheckArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    /// Append successful check output as a training JSONL record
    #[arg(long, value_name = "PATH")]
    pub emit_training_jsonl: Option<PathBuf>,
}

/// `vox test` / `vox fabrica test`
#[derive(Args, Clone, Debug)]
pub struct TestArgs {
    #[arg(required = true)]
    pub file: PathBuf,
}

/// `vox run` / `vox fabrica run`
#[derive(Args, Clone, Debug)]
pub struct RunArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    /// Backend listen port (sets `VOX_PORT` for generated Axum and Vite proxy)
    #[arg(long)]
    pub port: Option<u16>,
    /// `app` = generated server; `script` = `fn main()` script lane; `auto` = heuristic.
    #[arg(long, value_enum, default_value_t = crate::commands::run::RunMode::Auto)]
    pub mode: crate::commands::run::RunMode,
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

/// `vox script` / `vox fabrica script`
#[cfg(feature = "script-execution")]
#[derive(Args, Clone, Debug)]
pub struct ScriptArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    #[arg(long, default_value_t = false)]
    pub sandbox: bool,
    #[arg(long, default_value_t = false)]
    pub no_cache: bool,
    #[arg(long)]
    pub isolation: Option<String>,
    #[arg(long)]
    pub trust_class: Option<String>,
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

/// `vox dev` / `vox fabrica dev`
#[derive(Args, Clone, Debug)]
pub struct DevArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    #[arg(short, long, default_value = "dist")]
    pub out_dir: PathBuf,
    #[arg(long)]
    pub port: Option<u16>,
    #[arg(long, default_value = "false")]
    pub open: bool,
}

/// `vox bundle` / `vox fabrica bundle`
#[derive(Args, Clone, Debug)]
pub struct BundleArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    #[arg(short, long, default_value = "dist")]
    pub out_dir: PathBuf,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long, default_value = "true")]
    pub release: bool,
}

/// `vox fmt` / `vox fabrica fmt`
#[derive(Args, Clone, Debug)]
pub struct FmtArgs {
    #[arg(required = true)]
    pub file: PathBuf,
}

/// `vox doctor` / `vox mens doctor`
#[derive(Args, Clone, Debug)]
pub struct DoctorArgs {
    #[arg(long, default_value_t = false)]
    pub auto_heal: bool,
    #[arg(long, default_value_t = false)]
    pub test_health: bool,
    #[arg(long, default_value_t = false)]
    pub build_perf: bool,
    #[arg(long, default_value_t = false)]
    pub scope: bool,
    #[arg(long, default_value_t = false)]
    pub json: bool,
    /// OCI / automation: run default doctor checks and exit with non-zero status if any fail (no banner; stable for HEALTHCHECK).
    #[arg(long, default_value_t = false)]
    pub probe: bool,
}

/// `vox train` (legacy; canonical: `vox schola train`)
#[cfg(all(feature = "gpu", feature = "mens-dei"))]
#[derive(Args, Clone, Debug)]
pub struct TrainLegacyArgs {
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
    #[arg(long)]
    pub output_dir: Option<PathBuf>,
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long, default_value = "false")]
    pub native: bool,
}

/// `vox stub-check` / `vox mens stub-check`
#[cfg(feature = "stub-check")]
#[derive(Args, Clone, Debug)]
pub struct StubCheckArgs {
    #[arg(long, short = 'p', value_name = "PATH", conflicts_with = "scan_pos")]
    pub path: Option<PathBuf>,
    #[arg(value_name = "PATH", conflicts_with = "path")]
    pub scan_pos: Option<PathBuf>,
    #[arg(short = 'f', long)]
    pub format: Option<String>,
    #[arg(short = 's', long)]
    pub severity: Option<String>,
    #[arg(long, default_value = "true")]
    pub suggest_fixes: bool,
    #[arg(long)]
    pub rules: Option<String>,
    #[arg(long)]
    pub excludes: Vec<String>,
    #[arg(long)]
    pub langs: Option<String>,
    #[arg(long)]
    pub baseline: Option<String>,
    #[arg(long)]
    pub save_baseline: Option<String>,
    #[arg(long)]
    pub task_list: bool,
    #[arg(long)]
    pub import_suppressions: bool,
    #[arg(long)]
    pub ingest_findings: Option<PathBuf>,
    #[arg(long)]
    pub fix_pipeline: bool,
    #[arg(long)]
    pub fix_pipeline_apply: bool,
    #[arg(long, value_name = "MODE")]
    pub gate: Option<String>,
    #[arg(long, value_name = "PATH")]
    pub gate_budget_path: Option<PathBuf>,
    #[arg(long)]
    pub verify_impacted: bool,
    #[arg(long, default_value = "1", value_name = "N")]
    pub max_escalation: u8,
    #[arg(long)]
    pub self_heal_safe_mode: bool,
}
