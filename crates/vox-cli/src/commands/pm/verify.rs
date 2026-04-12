//! Lockfile vs on-disk artifact verification.

use crate::commands::pm_lifecycle::{self, DEFAULT_REGISTRY_BASE};
use anyhow::{Context, Result};
use std::fs;
use vox_pm::lockfile::PackageSource;
use vox_pm::{Lockfile, RegistryClient, content_hash};

fn registry_client_for_verify(registry_url: Option<&str>) -> RegistryClient {
    let base = registry_url.unwrap_or(DEFAULT_REGISTRY_BASE);
    let token_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxRegistryToken);
    let token = token_resolved.expose()
        .filter(|s| !s.is_empty());
    if let Some(t) = token {
        RegistryClient::with_auth(base, &t)
    } else {
        RegistryClient::new(base)
    }
}

pub async fn verify_dl_against_lock(registry_url: Option<&str>) -> Result<()> {
    let lock_path = pm_lifecycle::lockfile_path();
    if !lock_path.exists() {
        anyhow::bail!("missing `{}`", lock_path.display());
    }
    let lock = Lockfile::load(&lock_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let client = registry_client_for_verify(registry_url);
    let mut ok = 0usize;
    let mut problems = Vec::new();

    for (name, pkg) in &lock.packages {
        match &pkg.source {
            PackageSource::Registry => {
                let blob = pm_lifecycle::package_blob_dir(name, &pkg.version).join("artifact.bin");
                if !blob.exists() {
                    // Try one-shot download for verify (still validates hash).
                    match client.download(name, &pkg.version).await {
                        Ok(dl) => {
                            let h = content_hash(&dl.data);
                            if !pkg.content_hash.is_empty() && h != pkg.content_hash {
                                problems.push(format!(
                                    "{name}@{}: downloaded hash mismatch",
                                    pkg.version
                                ));
                                continue;
                            }
                            ok += 1;
                        }
                        Err(e) => problems.push(format!("{name}@{}: {e}", pkg.version)),
                    }
                    continue;
                }
                let bytes = fs::read(&blob).with_context(|| format!("read {}", blob.display()))?;
                let h = content_hash(&bytes);
                if !pkg.content_hash.is_empty() && h != pkg.content_hash {
                    problems.push(format!(
                        "{name}@{}: on-disk hash does not match lockfile",
                        pkg.version
                    ));
                    continue;
                }
                ok += 1;
            }
            PackageSource::Path(p) => {
                if !std::path::Path::new(p).exists() {
                    problems.push(format!("{name}: path `{p}` missing"));
                } else {
                    ok += 1;
                }
            }
            PackageSource::Git { url, .. } => {
                let allow_unverified_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxPmAllowGitUnverified);
                if allow_unverified_resolved.expose().is_some() {
                    eprintln!(
                        "  ⚠ {name}: skipping git source verification for `{url}` (VOX_PM_ALLOW_GIT_UNVERIFIED=1)"
                    );
                    ok += 1;
                } else {
                    problems.push(format!(
                        "{name}: git source `{url}` cannot be verified in-repo yet (set VOX_PM_ALLOW_GIT_UNVERIFIED=1 to bypass)"
                    ));
                }
            }
        }
    }

    if !problems.is_empty() {
        anyhow::bail!(
            "verify failed ({} ok, {} issue(s)):\n{}",
            ok,
            problems.len(),
            problems.join("\n")
        );
    }

    println!("✓ Verified {} locked package(s) against policy.", ok);
    Ok(())
}
