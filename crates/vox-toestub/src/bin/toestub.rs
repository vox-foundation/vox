//! TOEStub CLI — run architecture / TOE compliance checks from the terminal.
//!
//! Usage: `toestub [ROOT_DIR]` — defaults to the current directory. Prefer passing a crate path
//! in CI (e.g. `toestub crates/vox-toestub`) for a fast, focused gate.

use std::path::PathBuf;
use vox_toestub::rules::Severity;
use vox_toestub::{OutputFormat, ToestubConfig, ToestubEngine};

fn main() -> anyhow::Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let config = ToestubConfig {
        roots: vec![root],
        min_severity: Severity::Warning,
        format: OutputFormat::Terminal,
        suggest_fixes: true,
        ..Default::default()
    };

    let engine = ToestubEngine::new(config);
    let (result, output) = engine.run_and_report();

    println!("{}", output);

    if result.has_errors() {
        std::process::exit(1);
    }

    Ok(())
}
