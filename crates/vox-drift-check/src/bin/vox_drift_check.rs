use clap::Parser;
use std::path::PathBuf;
use vox_drift_check::{engine::DriftEngine, report};
use vox_code_audit::rules::Severity;

#[derive(Parser)]
#[command(name = "vox-drift-check", about = "Workspace-wide drift & repetition linter")]
struct Cli {
    /// Workspace root (defaults to current directory)
    #[arg(default_value = ".")]
    root: PathBuf,
    /// Emit JSON output
    #[arg(long)]
    json: bool,
    /// Minimum severity to show (info/warning/error)
    #[arg(long, default_value = "info")]
    severity: String,
    /// Exit non-zero if any findings at this level (info/warning/error)
    #[arg(long, default_value = "error")]
    fail_on: String,
}

fn parse_sev(s: &str) -> Severity {
    match s {
        "info" => Severity::Info,
        "error" | "critical" => Severity::Error,
        _ => Severity::Warning,
    }
}

fn main() {
    let cli = Cli::parse();
    let engine = DriftEngine::new(&cli.root);
    let findings = match engine.run_all() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    };
    if cli.json {
        report::print_json(&findings);
    } else {
        report::print_terminal(&findings, parse_sev(&cli.severity));
    }
    std::process::exit(report::exit_code(&findings, parse_sev(&cli.fail_on)));
}
