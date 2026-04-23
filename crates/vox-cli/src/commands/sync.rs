//! `vox sync` — materialize locked registry dependencies under `.vox_modules/dl/`.

use crate::commands::pm_lifecycle::{self, DEFAULT_REGISTRY_BASE};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use vox_pm::RegistryClient;
use vox_pm::lockfile::PackageSource;
use vox_pm::{Lockfile, VoxManifest, content_hash};

fn registry_client_for_sync(registry_url: Option<&str>) -> RegistryClient {
    let base = registry_url.unwrap_or(DEFAULT_REGISTRY_BASE);
    // Optional bearer token for private registries (same env many flows use).
    let token_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxRegistryToken);
    let token = token_resolved.expose().filter(|s| !s.is_empty());
    if let Some(t) = token {
        RegistryClient::with_auth(base, &t)
    } else {
        RegistryClient::new(base)
    }
}

/// `vox sync [--registry URL] [--frozen]` — download packages from `vox.lock`.
///
/// `--frozen` fails if `vox.lock` is missing or does not match a strict resolution from `Vox.toml`.
pub async fn run(registry_url: Option<&str>, frozen: bool) -> Result<()> {
    let manifest_path = PathBuf::from("Vox.toml");
    let manifest = VoxManifest::load(&manifest_path)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| "No Vox.toml found. Run `vox init` first.")?;

    let lock_path = pm_lifecycle::lockfile_path();
    if !lock_path.exists() {
        anyhow::bail!("missing `{}` — run `vox lock` first", lock_path.display());
    }

    let lock = Lockfile::load(&lock_path).map_err(|e| anyhow::anyhow!("{e}"))?;

    if frozen {
        let store = pm_lifecycle::open_local_pm_store()
            .await
            .context("open .vox_modules/local_store.db")?;
        let expected =
            pm_lifecycle::resolve_lockfile_from_manifest_local(&manifest, &store, true).await?;
        if expected != lock {
            anyhow::bail!(
                "`--frozen`: vox.lock does not match Vox.toml resolution (refresh with `vox lock`)"
            );
        }
    }

    let client = registry_client_for_sync(registry_url);
    let mut fetched = 0usize;

    for (name, pkg) in &lock.packages {
        match &pkg.source {
            PackageSource::Registry => {
                let dir = pm_lifecycle::package_blob_dir(name, &pkg.version);
                let blob_path = dir.join("artifact.bin");
                if blob_path.exists() {
                    let bytes = fs::read(&blob_path)
                        .with_context(|| format!("read {}", blob_path.display()))?;
                    let h = content_hash(&bytes);
                    if h == pkg.content_hash {
                        continue;
                    }
                }

                let dl = client.download(name, &pkg.version).await.map_err(|e| {
                    anyhow::anyhow!("registry download `{name}@{}`: {e}", pkg.version)
                })?;
                let h = content_hash(&dl.data);
                if !pkg.content_hash.is_empty() && h != pkg.content_hash {
                    anyhow::bail!(
                        "integrity mismatch for `{name}@{}`: lock has hash prefix `{}…`, downloaded `{}…`",
                        pkg.version,
                        &pkg.content_hash[..8.min(pkg.content_hash.len())],
                        &h[..8.min(h.len())],
                    );
                }
                fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
                fs::write(&blob_path, &dl.data)
                    .with_context(|| format!("write {}", blob_path.display()))?;
                fetched += 1;
            }
            PackageSource::Path(p) => {
                let path = PathBuf::from(p);
                if !path.exists() {
                    anyhow::bail!(
                        "path dependency `{name}` points to missing `{}`",
                        path.display()
                    );
                }
            }
            PackageSource::Git { .. } => {
                let allow_unverified_resolved =
                    vox_clavis::resolve_secret(vox_clavis::SecretId::VoxPmAllowGitUnverified);
                if allow_unverified_resolved.expose().is_some() {
                    eprintln!(
                        "  ⚠ {name}: skipping git dependency fetch (VOX_PM_ALLOW_GIT_UNVERIFIED=1)"
                    );
                    continue;
                }
                anyhow::bail!(
                    "`vox sync` does not fetch git dependencies yet (`{name}`); vendor manually or use a registry release, or set VOX_PM_ALLOW_GIT_UNVERIFIED=1 to skip"
                );
            }
        }
    }

    if fetched == 0 {
        println!("• Sync complete (all registry artifacts already present).");
    } else {
        println!("✓ Synced {fetched} registry package(s) into .vox_modules/dl/");
    }
    Ok(())
}
