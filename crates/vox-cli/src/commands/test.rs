//! `vox test` — runs `cargo test` in the generated Rust crate under `target/generated`.

use crate::commands::build;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build `file` into `dist/` / `target/generated`, then execute `cargo test` in the backend workspace.
pub async fn run(args: &crate::cli_args::TestArgs) -> Result<()> {
    // 1. Build
    let out_dir = PathBuf::from("dist");
    let file = &args.file;

    println!("Building for tests: {}...", file.display());
    build::run(file, &out_dir, None, false).await?;

    // 2. Run Tests
    let generated_dir = Path::new("target").join("generated");

    println!("Running tests in {}...", generated_dir.display());

    let mut cmd = Command::new("cargo");
    cmd.arg("test").current_dir(&generated_dir);
    if let Some(f) = &args.filter {
        cmd.arg(f);
    }
    if args.coverage {
        // Instrument for branch/line coverage via llvm-cov source-based instrumentation.
        cmd.env(
            "RUSTFLAGS",
            "-C instrument-coverage -C llvm-args=--instrprof-output-path=coverage.profraw",
        );
        cmd.env("LLVM_PROFILE_FILE", "coverage-%p-%m.profraw");
    }
    if args.update_snapshots {
        // Signal snapshot crates (e.g. insta) to update golden files.
        cmd.env("UPDATE_EXPECT", "1");
        cmd.env("INSTA_UPDATE", "always");
    }
    // Forward forall_iterations as an env var; runtime harnesses can read it.
    if let Some(iters) = args.forall_iterations {
        cmd.env("VOX_FORALL_ITERATIONS", iters.to_string());
    }

    let status = cmd.status().context("Failed to execute cargo test")?;

    if !status.success() {
        anyhow::bail!("Tests failed with exit code: {:?}", status.code());
    }

    Ok(())
}
