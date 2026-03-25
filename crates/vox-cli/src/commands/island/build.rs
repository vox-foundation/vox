use anyhow::{Context, Result, anyhow};
use std::path::Path;

use crate::frontend;
use crate::island_paths::{island_root, island_src_dir};
use crate::templates;
pub(super) fn bootstrap_islands_if_needed(root: &Path) -> Result<()> {
    let islands_dir = island_root(root);
    let pkg = islands_dir.join("package.json");
    if pkg.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(island_src_dir(root))?;
    std::fs::write(&pkg, templates::islands_package_json())?;
    std::fs::write(
        islands_dir.join("vite.config.ts"),
        templates::islands_vite_config(),
    )?;
    std::fs::write(
        islands_dir.join("index.html"),
        templates::islands_index_html(),
    )?;
    std::fs::write(
        island_src_dir(root).join("main.tsx"),
        templates::islands_main_tsx(),
    )?;
    std::fs::write(
        island_src_dir(root).join("island-mount.tsx"),
        templates::islands_island_mount_tsx(),
    )?;
    std::fs::write(
        islands_dir.join("tsconfig.json"),
        templates::tsconfig_json(),
    )?;
    crate::diagnostics::print_info(
        "Bootstrapped minimal islands/ (Vite + React, pnpm). Dependencies install on first build.",
    );
    Ok(())
}

// ── build_islands ─────────────────────────────────────────────────────────────

/// Run **`pnpm run build`** in **`islands/`**, with fingerprint-based skip for warm builds.
///
/// Installs dependencies via **`pnpm install --prefer-offline`** when `node_modules` is absent
/// or stale relative to **`package.json`** / **`pnpm-lock.yaml`**.
pub async fn build_islands(root: &Path) -> Result<()> {
    bootstrap_islands_if_needed(root)?;

    let islands_dir = island_root(root);
    which::which(frontend::pnpm_executable()).map_err(|_| {
        anyhow!(
            "pnpm not found in PATH. Install pnpm (https://pnpm.io/) to build islands; Node.js required."
        )
    })?;

    let nm = islands_dir.join("node_modules");
    let lock = islands_dir.join("pnpm-lock.yaml");
    let pkg = islands_dir.join("package.json");
    let nm_mtime = nm.metadata().and_then(|m| m.modified()).ok();
    let needs_install = !nm.exists()
        || (lock.exists() && lock.metadata().and_then(|m| m.modified()).ok() > nm_mtime)
        || (pkg.exists() && pkg.metadata().and_then(|m| m.modified()).ok() > nm_mtime);

    if needs_install {
        println!("📦 Installing island dependencies (pnpm)…");
        let status = tokio::process::Command::new(frontend::pnpm_executable())
            .args(["install", "--prefer-offline"])
            .current_dir(&islands_dir)
            .status()
            .await
            .context("Failed to run pnpm install in islands/")?;
        if !status.success() {
            anyhow::bail!("pnpm install failed in {}", islands_dir.display());
        }
    }

    // Fingerprint-based skip: skip build if no island source is newer than the last build
    if !needs_island_rebuild(root)? {
        println!("⚡ Islands are up-to-date, skipping build.");
        return Ok(());
    }

    println!("🔨 Building islands with Vite (pnpm)…");

    let status = tokio::process::Command::new(frontend::pnpm_executable())
        .args(["run", "build"])
        .current_dir(&islands_dir)
        .status()
        .await
        .context("Failed to run pnpm run build in islands/")?;
    if !status.success() {
        anyhow::bail!("pnpm run build failed in {}", islands_dir.display());
    }

    write_island_fingerprint(root)?;
    println!("✅ Islands built.");
    Ok(())
}

/// Returns `true` if any file under `islands/src/` is newer than the fingerprint marker.
fn needs_island_rebuild(root: &Path) -> Result<bool> {
    let fp = root.join(".vox-build-cache").join("islands.fingerprint");
    if !fp.exists() {
        return Ok(true);
    }
    let fp_mtime = fp.metadata()?.modified()?;
    let islands_dir = island_root(root);
    for marker in [
        islands_dir.join("package.json"),
        islands_dir.join("vite.config.ts"),
        islands_dir.join("tsconfig.json"),
        islands_dir.join("pnpm-lock.yaml"),
    ] {
        if marker.exists() && marker.metadata()?.modified()? > fp_mtime {
            return Ok(true);
        }
    }
    let src_dir = island_src_dir(root);
    if !src_dir.exists() {
        return Ok(false);
    }
    for entry in walkdir::WalkDir::new(&src_dir) {
        let e = entry?;
        if e.file_type().is_file() && e.metadata()?.modified()? > fp_mtime {
            return Ok(true);
        }
    }
    Ok(false)
}

fn write_island_fingerprint(root: &Path) -> Result<()> {
    let cache_dir = root.join(".vox-build-cache");
    std::fs::create_dir_all(&cache_dir)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    std::fs::write(cache_dir.join("islands.fingerprint"), ts.to_string())?;
    Ok(())
}
