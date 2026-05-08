use crate::commands::pm_lifecycle::{
    lockfile_path, open_local_pm_store, resolve_lockfile_from_manifest_local,
};
use anyhow::{Context, Result};
use std::path::PathBuf;

/// `vox update` — refresh `vox.lock` from the local PM index (project graph only; not toolchain).
pub async fn run() -> Result<()> {
    let manifest_path = PathBuf::from("Vox.toml");
    let manifest = vox_package::VoxManifest::load(&manifest_path)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| "No Vox.toml found. Run `vox init` first.")?;

    if manifest.dependencies.is_empty() {
        println!("No dependencies to update.");
        return Ok(());
    }

    println!(
        "Updating lockfile for {} v{}…",
        manifest.package.name, manifest.package.version
    );

    let store = open_local_pm_store()
        .await
        .context("open .vox_modules/local_store.db")?;

    let before = if lockfile_path().exists() {
        vox_package::Lockfile::load(&lockfile_path())
            .map_err(|e| anyhow::anyhow!("{e}"))
            .unwrap_or_else(|_| vox_package::Lockfile::new())
    } else {
        vox_package::Lockfile::new()
    };

    let after = resolve_lockfile_from_manifest_local(&manifest, &store, false).await?;

    let mut updated = 0usize;
    for (name, pkg) in &after.packages {
        let old = before.get_locked_version(name);
        let new_v = vox_package::SemVer::parse(&pkg.version).ok();
        if old.as_ref() != new_v.as_ref() {
            let old_s = old.map(|v| v.to_string()).unwrap_or_else(|| "none".into());
            println!("  ✓ {name}: {old_s} → {}", pkg.version);
            updated += 1;
        } else {
            println!("  • {name}: up to date ({})", pkg.version);
        }
    }

    after
        .save(&lockfile_path())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| "Failed to save vox.lock")?;

    if updated > 0 {
        println!("\n✓ Updated {updated} package(s). Lockfile saved.");
    } else {
        println!("\n• Dependency graph unchanged.");
    }

    Ok(())
}
