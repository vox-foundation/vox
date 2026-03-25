//! TOEStub CLI — run architecture / TOE compliance checks from the terminal.
//!
//! Usage: `toestub [ROOT_DIR]` — defaults to the current directory. Prefer passing a crate path
//! in CI (e.g. `toestub crates/vox-repository`) for a fast, focused gate.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use vox_toestub::engine::ToestubRunMode;
use vox_toestub::rules::Severity;
use vox_toestub::{OutputFormat, ToestubConfig, ToestubEngine};

#[derive(Copy, Clone, Debug, ValueEnum)]
enum CliMode {
    Legacy,
    Audit,
    #[value(name = "enforce-warn")]
    EnforceWarn,
    #[value(name = "enforce-strict")]
    EnforceStrict,
}

impl From<CliMode> for ToestubRunMode {
    fn from(m: CliMode) -> Self {
        match m {
            CliMode::Legacy => ToestubRunMode::Legacy,
            CliMode::Audit => ToestubRunMode::Audit,
            CliMode::EnforceWarn => ToestubRunMode::EnforceWarn,
            CliMode::EnforceStrict => ToestubRunMode::EnforceStrict,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "toestub", about = "TOESTUB structural and policy checks")]
struct Opt {
    /// Root directory to scan (default `.`).
    #[arg(default_value = ".")]
    root: PathBuf,
    /// Exit-code policy: `legacy` (errors fail), `audit` (never fail), `enforce-warn` (critical only), `enforce-strict` (warnings+).
    #[arg(long, value_enum, default_value_t = CliMode::Legacy)]
    mode: CliMode,
    /// Minimum severity to report (audit mode forces `info`).
    #[arg(long, value_enum, default_value_t = SeverityArg::Warning)]
    min_severity: SeverityArg,
    /// Output format.
    #[arg(long, value_enum, default_value_t = FormatArg::Terminal)]
    format: FormatArg,
    /// Include fix suggestions in the task queue (Markdown / internal tooling).
    #[arg(long, default_value_t = false)]
    suggest_fixes: bool,
    /// Only run rules whose [`DetectionRule::id`] starts with one of these prefixes (repeatable; e.g. `--rules scaling`).
    #[arg(long = "rules", value_name = "PREFIX")]
    rule_prefix: Vec<String>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum SeverityArg {
    Info,
    Warning,
    Error,
    Critical,
}

impl From<SeverityArg> for Severity {
    fn from(s: SeverityArg) -> Self {
        match s {
            SeverityArg::Info => Severity::Info,
            SeverityArg::Warning => Severity::Warning,
            SeverityArg::Error => Severity::Error,
            SeverityArg::Critical => Severity::Critical,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum FormatArg {
    Terminal,
    Json,
    Markdown,
}

impl From<FormatArg> for OutputFormat {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Terminal => OutputFormat::Terminal,
            FormatArg::Json => OutputFormat::Json,
            FormatArg::Markdown => OutputFormat::Markdown,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();
    let run_mode: ToestubRunMode = opt.mode.into();
    let min_severity = match run_mode {
        ToestubRunMode::Audit => Severity::Info,
        _ => Severity::from(opt.min_severity),
    };

    let rule_filter = if opt.rule_prefix.is_empty() {
        None
    } else {
        Some(opt.rule_prefix)
    };
    let config = ToestubConfig {
        roots: vec![opt.root],
        min_severity,
        format: OutputFormat::from(opt.format),
        suggest_fixes: opt.suggest_fixes,
        rule_filter,
        run_mode,
        ..ToestubConfig::default()
    };
    let run_mode = config.run_mode;
    let engine = ToestubEngine::new(config);
    let (result, output) = engine.run_and_report();

    println!("{}", output);

    if result.should_fail_build(run_mode) {
        std::process::exit(1);
    }

    Ok(())
}
