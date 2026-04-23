//! `vox pm …` — advanced package and registry operations.

mod verify;

use crate::commands::info;
use crate::commands::pm_lifecycle::{self, DEFAULT_REGISTRY_BASE};
use anyhow::{Context, Result};
use clap::Subcommand;
use std::fs;
use std::path::{Path, PathBuf};
use vox_pm::{PublishDependency, PublishRequest, RegistryClient, content_hash};

/// Advanced PM subcommands (registry, cache, verification).
#[derive(Subcommand)]
pub enum PmCli {
    /// Search the remote registry API (HTTP).
    Search {
        /// Query string.
        #[arg(required = true)]
        query: String,
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long)]
        registry: Option<String>,
    },
    /// Show metadata for one package (registry with local fallback).
    Info {
        #[arg(required = true)]
        package: String,
        #[arg(long)]
        registry: Option<String>,
    },
    /// Publish a package tarball to the registry (requires auth).
    Publish {
        #[arg(required = true)]
        name: String,
        #[arg(long)]
        version: String,
        #[arg(long, default_value = "library")]
        kind: String,
        /// Package archive bytes (single-file bundle).
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        registry: Option<String>,
    },
    /// Yank a version (requires registry support).
    Yank {
        #[arg(required = true)]
        name: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        registry: Option<String>,
    },
    /// Copy synced artifacts into `vendor/vox/` for offline builds.
    Vendor {
        #[arg(long, default_value = "vendor/vox")]
        dir: PathBuf,
    },
    /// Verify `vox.lock` against materialized `.vox_modules/dl/` blobs.
    Verify {
        #[arg(long)]
        registry: Option<String>,
    },
    /// Register a registry artifact in `.vox_modules/local_store.db` so `vox lock` / `vox update` can resolve it offline (file on disk or HTTP download).
    Mirror {
        #[arg(required = true)]
        name: String,
        #[arg(long)]
        version: String,
        /// Use exactly one of `--file` or `--from-registry`.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Registry base URL (same download API as `vox sync`); `VOX_REGISTRY_TOKEN` is used when set.
        #[arg(long = "from-registry")]
        from_registry: Option<String>,
    },
    /// Inspect or clear the local PM download cache.
    Cache {
        #[command(subcommand)]
        cmd: PmCacheCmd,
    },
}

#[derive(Subcommand)]
pub enum PmCacheCmd {
    /// Print aggregate cache size under `.vox_modules/dl`.
    Status,
    /// Delete `.vox_modules/dl` (re-download on next `vox sync`).
    Clear,
}

fn client(base: Option<&str>, token_from_env: bool) -> RegistryClient {
    let base = base.unwrap_or(DEFAULT_REGISTRY_BASE);
    let token_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxRegistryToken);
    let token = token_resolved.expose().filter(|s| !s.is_empty());
    match (token_from_env, token) {
        (true, Some(t)) => RegistryClient::with_auth(base, &t),
        _ => RegistryClient::new(base),
    }
}

/// Dispatch `vox pm …`.
pub async fn run(cmd: PmCli) -> Result<()> {
    match cmd {
        PmCli::Search {
            query,
            limit,
            registry,
        } => {
            let c = client(registry.as_deref(), false);
            match c.search(&query, limit, 0).await {
                Ok(res) => {
                    let n = res.packages.len();
                    if res.packages.is_empty() {
                        println!("No packages matched `{query}`.");
                    } else {
                        for p in &res.packages {
                            println!(
                                "{} v{} — {}",
                                p.name,
                                p.latest_version,
                                p.description.clone().unwrap_or_default()
                            );
                        }
                        println!("({n} result(s), total reported {})", res.total);
                    }
                }
                Err(e) => {
                    anyhow::bail!("registry search failed: {e}");
                }
            }
        }
        PmCli::Info { package, registry } => {
            info::run(&package, registry.as_deref()).await?;
        }
        PmCli::Publish {
            name,
            version,
            kind,
            file,
            registry,
        } => {
            let data = fs::read(&file).with_context(|| format!("read {}", file.display()))?;
            let content_hash_val = content_hash(&data);
            let manifest_path = PathBuf::from("Vox.toml");
            let deps = if manifest_path.exists() {
                let m = vox_pm::VoxManifest::load(&manifest_path)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                m.dependencies
                    .iter()
                    .map(|(n, s)| PublishDependency {
                        name: n.clone(),
                        version_req: s.version_req().unwrap_or("*").to_string(),
                    })
                    .collect()
            } else {
                vec![]
            };
            let req = PublishRequest {
                name: name.clone(),
                version: version.clone(),
                kind,
                description: None,
                license: None,
                content_hash: content_hash_val.clone(),
                data,
                dependencies: deps,
            };
            let registry_url = registry.as_deref().unwrap_or(DEFAULT_REGISTRY_BASE);
            let c = client(registry.as_deref(), true);
            c.publish(req).await.map_err(|e| anyhow::anyhow!("{e}"))?;
            let prov_dir = PathBuf::from(".vox_modules").join("provenance");
            fs::create_dir_all(&prov_dir)?;
            let built_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let prov = serde_json::json!({
                "schema": "vox.pm.provenance/1",
                "package": name,
                "version": version,
                "content_hash": content_hash_val,
                "built_at_epoch": built_at,
                "tool": env!("CARGO_PKG_NAME"),
                "registry": registry_url,
            });
            let prov_path = prov_dir.join(format!("{name}@{version}.json"));
            fs::write(
                &prov_path,
                serde_json::to_string_pretty(&prov).with_context(|| "serialize provenance")?,
            )
            .with_context(|| format!("write {}", prov_path.display()))?;
            println!(
                "✓ Published `{name}` {version} (provenance → {})",
                prov_path.display()
            );
        }
        PmCli::Yank {
            name,
            version,
            registry: _,
        } => {
            anyhow::bail!(
                "`vox pm yank` is not implemented for this registry client ({name}@{version}); use registry admin tools"
            );
        }
        PmCli::Vendor { dir } => {
            let src = PathBuf::from(".vox_modules").join("dl");
            if !src.exists() {
                anyhow::bail!("missing `.vox_modules/dl`; run `vox sync` first");
            }
            fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
            vendor_copy_tree(&src, &dir)?;
            println!("✓ Vendored `.vox_modules/dl` → {}", dir.display());
        }
        PmCli::Verify { registry } => {
            verify::verify_dl_against_lock(registry.as_deref()).await?;
        }
        PmCli::Mirror {
            name,
            version,
            file,
            from_registry,
        } => {
            let bytes = match (&file, &from_registry) {
                (Some(p), None) => fs::read(p).with_context(|| format!("read {}", p.display()))?,
                (None, Some(base)) => {
                    let c = client(Some(base.as_str()), true);
                    let dl = c.download(&name, &version).await.map_err(|e| {
                        anyhow::anyhow!("registry download `{name}@{version}`: {e}")
                    })?;
                    if !dl.content_hash.is_empty() {
                        let got = content_hash(&dl.data);
                        if got != dl.content_hash {
                            anyhow::bail!(
                                "registry content_hash mismatch for `{name}@{version}` (got `{}…`, declared `{}…`)",
                                &got[..8.min(got.len())],
                                &dl.content_hash[..8.min(dl.content_hash.len())],
                            );
                        }
                    }
                    dl.data
                }
                (Some(_), Some(_)) => {
                    anyhow::bail!("use only one of `--file` or `--from-registry`")
                }
                (None, None) => {
                    anyhow::bail!("specify `--file <path>` or `--from-registry <url>`")
                }
            };
            let db = pm_lifecycle::open_local_pm_store().await?;
            let h = db
                .record_pm_registry_mirror(&name, &version, &bytes)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!(
                "✓ Mirrored `{name}` {version} into local PM index (content_hash prefix `{}…`)",
                &h[..8.min(h.len())]
            );
        }
        PmCli::Cache { cmd } => match cmd {
            PmCacheCmd::Status => {
                let root = PathBuf::from(".vox_modules").join("dl");
                if !root.exists() {
                    println!("Cache empty (no `.vox_modules/dl`).");
                } else {
                    let bytes = dir_size(&root)?;
                    println!("`.vox_modules/dl` size: {} bytes", bytes);
                }
            }
            PmCacheCmd::Clear => {
                let root = PathBuf::from(".vox_modules").join("dl");
                if root.exists() {
                    fs::remove_dir_all(&root)
                        .with_context(|| format!("remove {}", root.display()))?;
                }
                println!("✓ Cleared `.vox_modules/dl`");
            }
        },
    }
    Ok(())
}

fn vendor_copy_tree(src: &Path, dst: &Path) -> Result<()> {
    for e in walkdir::WalkDir::new(src).min_depth(1) {
        let e = e?;
        let path = e.path();
        let rel = path.strip_prefix(src).expect("under src");
        let target = dst.join(rel);
        if e.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(p) = target.parent() {
                fs::create_dir_all(p)?;
            }
            fs::copy(path, &target)
                .with_context(|| format!("copy {} -> {}", path.display(), target.display()))?;
        }
    }
    Ok(())
}

fn dir_size(root: &PathBuf) -> Result<u64> {
    let mut total = 0u64;
    for e in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
    {
        if e.path().is_file() {
            total += e.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    Ok(total)
}
