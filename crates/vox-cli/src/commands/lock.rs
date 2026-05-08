//! `vox lock` — resolve `Vox.toml` and write `vox.lock` (no artifact download).

use crate::commands::pm_lifecycle::{
    lockfile_path, open_local_pm_store, resolve_lockfile_from_manifest_local,
};
use anyhow::{Context, Result};
use std::path::PathBuf;
use vox_package::Lockfile;
use vox_package::VoxManifest;

/// `vox lock [--locked]` — refresh lockfile; `--locked` verifies existing lock matches manifest.
pub async fn run(verify_only: bool) -> Result<()> {
    let manifest_path = PathBuf::from("Vox.toml");
    let manifest = VoxManifest::load(&manifest_path)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| "No Vox.toml found. Run `vox init` first.")?;

    let store = open_local_pm_store()
        .await
        .context("open .vox_modules/local_store.db")?;
    let computed = resolve_lockfile_from_manifest_local(&manifest, &store, true).await?;
    let lock_path = lockfile_path();

    if verify_only {
        let on_disk = Lockfile::load(&lock_path).map_err(|e| anyhow::anyhow!("{e}"))?;
        if on_disk != computed {
            anyhow::bail!(
                "`vox.lock` is out of date for the current Vox.toml (run `vox lock` without `--locked` to refresh)"
            );
        }
        println!("✓ Lockfile is up to date.");
        return Ok(());
    }

    computed
        .save(&lock_path)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| "Failed to write vox.lock")?;
    println!(
        "✓ Wrote `{}` ({} package(s))",
        lock_path.display(),
        computed.packages.len()
    );
    println!(
        "• Cargo.lock remains build-generated from emitted Cargo.toml (source of truth: vox.lock inputs)."
    );
    Ok(())
}
