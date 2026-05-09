use anyhow::Result;
use clap::Args;
use vox_drift_check::{engine::DriftEngine, report, Severity};

#[derive(Args, Debug)]
pub struct DriftCheckArgs {
    /// Workspace root (defaults to current directory)
    #[arg(default_value = ".")]
    pub root: std::path::PathBuf,
    /// Emit JSON output
    #[arg(long)]
    pub json: bool,
    /// Minimum severity to show (info/warning/error)
    #[arg(long, default_value = "info")]
    pub severity: String,
    /// Exit non-zero if any findings at this level
    #[arg(long, default_value = "error")]
    pub fail_on: String,
}

pub async fn run(args: DriftCheckArgs) -> Result<()> {
    let engine = DriftEngine::new(&args.root);
    let findings = engine.run_all()?;
    let min_sev = parse_sev(&args.severity);
    if args.json {
        report::print_json(&findings);
    } else {
        report::print_terminal(&findings, min_sev);
    }
    let exit = report::exit_code(&findings, parse_sev(&args.fail_on));
    if exit != 0 {
        anyhow::bail!(
            "drift-check found violations at {:?} level or above",
            parse_sev(&args.fail_on)
        );
    }
    Ok(())
}

fn parse_sev(s: &str) -> Severity {
    match s {
        "info" => Severity::Info,
        "error" | "critical" => Severity::Error,
        _ => Severity::Warning,
    }
}
