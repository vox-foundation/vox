//! Shared resolution helpers for `vox add` / `update` / `lock` / `sync`.

use anyhow::{Context, Result};
use std::path::PathBuf;
use vox_db::VoxDb;
use vox_package::lockfile::{Lockfile, PackageSource};
use vox_package::manifest::{DependencySpec, VoxManifest};
use vox_package::{SemVer, VersionReq, content_hash};

/// Default static registry base URL (GitHub raw tree; overridable via `--registry` on PM commands).
pub const DEFAULT_REGISTRY_BASE: &str =
    "https://raw.githubusercontent.com/vox-foundation/vox/main/registry";

fn vox_modules_root() -> PathBuf {
    PathBuf::from(".vox_modules")
}

/// Open the project-local PM index at `.vox_modules/local_store.db`.
pub async fn open_local_pm_store() -> Result<VoxDb> {
    let store_dir = vox_modules_root();
    std::fs::create_dir_all(&store_dir).with_context(|| "Failed to create .vox_modules/")?;
    let db_path = store_dir.join("local_store.db");
    VoxDb::open(db_path.to_str().expect("utf-8 path"))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// Resolve `[dependencies]` into a fresh lockfile using only the local PM index (no HTTP).
///
/// When `strict` is false, registry dependencies missing from the local index are skipped
/// (warning only), preserving prior `vox update` behavior.
pub async fn resolve_lockfile_from_manifest_local(
    manifest: &VoxManifest,
    store: &VoxDb,
    strict: bool,
) -> Result<Lockfile> {
    let mut lockfile = Lockfile::new();

    for (name, spec) in &manifest.dependencies {
        if let Some(p) = spec.path() {
            let p_norm = p.replace('\\', "/");
            let v = SemVer::parse("0.1.0").expect("semver");
            let skills = match spec {
                DependencySpec::Detailed(d) => d.skills.clone(),
                _ => vec![],
            };
            lockfile.add(
                name,
                &v,
                &content_hash(p_norm.as_bytes()),
                PackageSource::Path(p_norm),
                vec![],
                skills,
            );
            continue;
        }

        if let DependencySpec::Detailed(d) = spec {
            if d.git.is_some() {
                let url = d.git.as_deref().unwrap_or_default().to_string();
                let rev = d.branch.clone();
                let marker = format!("{url}\0{rev:?}");
                let v = SemVer::parse(
                    d.version
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or("0.1.0"),
                )
                .unwrap_or_else(|_| SemVer::parse("0.1.0").expect("semver"));
                lockfile.add(
                    name,
                    &v,
                    &content_hash(marker.as_bytes()),
                    PackageSource::Git { url, rev },
                    vec![],
                    d.skills.clone(),
                );
                continue;
            }
        }

        let ver_req_str = spec.version_req().unwrap_or("*");
        let versions = store.get_package_versions(name).await.unwrap_or_default();

        if versions.is_empty() {
            if strict {
                anyhow::bail!(
                    "dependency `{name}` is not present in the local PM index (.vox_modules/local_store.db); \
                     run `vox sync` after publishing or mirroring packages, or use a path/git dependency"
                );
            }
            eprintln!("  ⚠ {name}: not in local registry, skipping (non-strict update mode)");
            continue;
        }

        let req = VersionReq::parse(ver_req_str)
            .with_context(|| format!("invalid version requirement for `{name}`"))?;
        let mut candidates: Vec<SemVer> = versions
            .iter()
            .filter_map(|(v, _)| SemVer::parse(v).ok())
            .filter(|v| req.matches(v))
            .collect();
        candidates.sort();

        let Some(best) = candidates.last() else {
            if strict {
                anyhow::bail!(
                    "no version of `{name}` satisfies `{ver_req_str}` in the local index"
                );
            }
            eprintln!("  ⚠ {name}: no matching version for `{ver_req_str}`, skipping");
            continue;
        };

        let hash = versions
            .iter()
            .find(|(v, _)| SemVer::parse(v).ok().as_ref() == Some(best))
            .map(|(_, h)| h.clone())
            .unwrap_or_default();

        let skills = match spec {
            DependencySpec::Detailed(d) => d.skills.clone(),
            _ => vec![],
        };

        lockfile.add(name, best, &hash, PackageSource::Registry, vec![], skills);
    }

    Ok(lockfile)
}

/// Project-relative path for `vox.lock`.
pub fn lockfile_path() -> PathBuf {
    PathBuf::from("vox.lock")
}

/// Materialize one registry artifact under `.vox_modules/dl/<name>/<version>/`.
pub fn package_blob_dir(name: &str, version: &str) -> PathBuf {
    vox_modules_root().join("dl").join(name).join(version)
}
