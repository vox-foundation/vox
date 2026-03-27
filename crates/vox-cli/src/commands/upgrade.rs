//! `vox upgrade` — Vox toolchain refresh: release binary lane or local repo lane (never touches Vox.toml or vox.lock).

use crate::cli_args::UpgradeToolchainArgs;
use anyhow::Result;

/// Check-only by default; `--apply` runs the selected `--source` lane (release binary or repo + `cargo install`).
pub async fn run(args: &UpgradeToolchainArgs, json_output: bool) -> Result<()> {
    let args = args.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        crate::commands::toolchain_upgrade::run_toolchain_upgrade(&args, json_output)
    })
    .await
    .map_err(|e| anyhow::anyhow!("upgrade worker: {e}"))?
}
