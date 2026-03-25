//! `vox ci build-timings` — rich per-crate build telemetry persisted to Arca.
//!
//! Data collected per run:
//!   - Wall-clock windows between `compiler-artifact` events → elapsed_ms per crate
//!   - `compiler-message` warnings (code, message, crate)
//!   - `cargo metadata` dep-graph fingerprint (SHA-256 of sorted `name@version` strings)
//!   - Total run duration
//!
//! All records land in the V34 `build_run / build_crate_sample / build_warning` tables.
//! Falls back to the old `research_metrics` row if the DB is unavailable.

use std::path::Path;
use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ── Internal structs ─────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize)]
struct BuildSummary {
    pub total_ms: i64,
    pub run_name: Option<String>,
    pub rustc_version: Option<String>,
    pub dep_fingerprint: Option<String>,
    pub profile: String,
    pub crates: Vec<CrateRecord>,
    pub warnings: Vec<WarningRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CrateRecord {
    pub name: String,
    pub version: Option<String>,
    pub elapsed_ms: Option<i64>,
    pub fresh: bool,
    pub features: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
struct WarningRecord {
    pub crate_name: String,
    pub level: String,
    pub code: Option<String>,
    pub message: String,
}

// ── Dep-graph fingerprint ────────────────────────────────────────────────────

fn dep_graph_fingerprint() -> Option<String> {
    // Fix 7: remove --no-deps to catch third-party version changes
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let packages = json["packages"].as_array()?;
    let mut entries: Vec<String> = packages
        .iter()
        .filter_map(|p| {
            let name = p["name"].as_str()?;
            let ver = p["version"].as_str()?;
            Some(format!("{name}@{ver}"))
        })
        .collect();
    entries.sort();

    // Fix 2: Deterministic FNV-1a hash instead of unstable DefaultHasher
    let mut h: u64 = 0xcbf29ce484222325;
    for e in &entries {
        for b in e.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x00000100000001b3);
        }
        h ^= b'\n' as u64;
        h = h.wrapping_mul(0x00000100000001b3);
    }
    Some(format!("{h:016x}"))
}

// ── Rustc version ────────────────────────────────────────────────────────────

fn rustc_version() -> Option<String> {
    let out = Command::new("rustc").args(["--version"]).output().ok()?;
    String::from_utf8(out.stdout)
        .ok()
        .map(|s| s.trim().to_string())
}

// ── Main collector ───────────────────────────────────────────────────────────

/// Run `cargo build --message-format=json` on the given package (or the whole workspace),
/// collect per-crate timings and warnings, persist to Arca, and print a concise summary.
pub async fn bench_build_run(
    persist: bool,
    run_name: Option<String>,
    profile: Option<String>,
) -> Result<()> {
    let target_pkg = "vox-corpus"; // never self — avoids Access Denied on Windows
    let profile = profile.unwrap_or_else(|| "dev".to_string());

    let mut args = vec!["build", "--message-format=json", "-p", target_pkg];
    if profile == "release" {
        args.push("--release");
    }

    println!(
        "vox ci build-timings → cargo build {} -p {target_pkg} --message-format=json",
        if profile == "release" {
            "--release"
        } else {
            ""
        }
    );

    let dep_fp = dep_graph_fingerprint();
    let rustc_ver = rustc_version();
    let start = Instant::now();

    let output = Command::new("cargo")
        .args(&args)
        .output()
        .context("failed to spawn cargo build")?;

    let total_ms = start.elapsed().as_millis() as i64;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("cargo build failed:\n{stderr}");
    }

    let mut summary = BuildSummary {
        total_ms,
        run_name: run_name.clone(),
        rustc_version: rustc_ver.clone(),
        dep_fingerprint: dep_fp.clone(),
        profile: profile.clone(),
        ..Default::default()
    };

    // Track artifact timestamps for elapsed_ms windows
    let mut last_ts = start;
    let mut raw_warnings = Vec::new();

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        match val["reason"].as_str() {
            Some("compiler-artifact") => {
                let pkg_id = val["package_id"].as_str().unwrap_or("");
                let name = pkg_id
                    .split_whitespace()
                    .next()
                    .unwrap_or(pkg_id)
                    .to_string();
                let version = pkg_id.split_whitespace().nth(1).map(|v| v.to_string());
                let fresh = val["fresh"].as_bool().unwrap_or(false);
                let features = val["features"].as_array().map(|a| {
                    a.iter()
                        .filter_map(|f| f.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                        .join(",")
                });
                let now = Instant::now();
                let elapsed_ms = if fresh {
                    None
                } else {
                    Some(now.duration_since(last_ts).as_millis() as i64)
                };
                last_ts = now;
                summary.crates.push(CrateRecord {
                    name,
                    version,
                    elapsed_ms,
                    fresh,
                    features,
                });
            }
            Some("compiler-message") => {
                let msg = &val["message"];
                if let Some(level) = msg["level"].as_str() {
                    if level == "warning" || level == "error" {
                        let code = msg["code"]
                            .as_object()
                            .and_then(|c| c.get("code"))
                            .and_then(|c| c.as_str())
                            .map(str::to_string);
                        let message = msg["message"].as_str().unwrap_or("").to_string();

                        // Fix 13: robust crate extraction using target name if path guess fails
                        let mut crate_name = val["target"]["name"].as_str().map(|s| s.to_string());

                        if crate_name.is_none() {
                            crate_name = msg["spans"]
                                .as_array()
                                .and_then(|s| s.first())
                                .and_then(|s| s["file_name"].as_str())
                                .map(|f| {
                                    let p = Path::new(f);
                                    p.components()
                                        .find_map(|c| {
                                            c.as_os_str().to_str().filter(|s| s.starts_with("vox-"))
                                        })
                                        .unwrap_or("unknown")
                                        .to_string()
                                });
                        }

                        raw_warnings.push(WarningRecord {
                            crate_name: crate_name.unwrap_or_else(|| "unknown".into()),
                            level: level.to_string(),
                            code,
                            message,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    // Fix 11: Deduplicate warnings within the same run
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for w in raw_warnings {
        if seen.insert(w.clone()) {
            summary.warnings.push(w);
        }
    }

    let crate_count = summary.crates.len() as i64;
    let fresh_count = summary.crates.iter().filter(|c| c.fresh).count() as i64;

    // ── Print concise summary ───────────────────────────────────────────────
    println!(
        "✓ Build complete ({}) — {:.2}s  ({} compiled, {} cached, {} warnings)",
        profile,
        total_ms as f64 / 1000.0,
        crate_count - fresh_count,
        fresh_count,
        summary.warnings.len()
    );

    // Slowest 5 non-fresh crates
    let mut compiled: Vec<_> = summary
        .crates
        .iter()
        .filter(|c| !c.fresh && c.elapsed_ms.is_some())
        .collect();
    compiled.sort_by_key(|c| std::cmp::Reverse(c.elapsed_ms.unwrap_or(0)));
    if !compiled.is_empty() {
        println!("Slowest crates:");
        for c in compiled.iter().take(5) {
            println!("  {:40} {:>6}ms", c.name, c.elapsed_ms.unwrap_or(0));
        }
    }

    // ── Persist ─────────────────────────────────────────────────────────────
    if persist {
        // Fix 4: Resolve real repository_id instead of hardcoded "local"
        let repo =
            vox_repository::discover_repository(&std::env::current_dir().unwrap_or_default());
        let repo_id = repo
            .as_ref()
            .map(|r| r.repository_id.clone())
            .unwrap_or_else(|_| "local".into());

        // Try new Arca tables first
        let persisted = try_persist_to_arca(
            &repo_id,
            &summary,
            crate_count,
            fresh_count,
            &rustc_ver,
            &dep_fp,
            &run_name,
        )
        .await;

        if persisted {
            println!("Recorded metrics to VoxDB (V34 build tables).");
        } else {
            // Fix 6: Use correct record_benchmark_event API
            use vox_db::{DbConfig, VoxDb};
            if let Ok(config) = DbConfig::from_env() {
                if let Ok(db) = VoxDb::connect(config).await {
                    let details = serde_json::to_value(&summary).unwrap_or_default();
                    let _ = db
                        .record_benchmark_event(
                            &repo_id,
                            "cargo_build_metrics",
                            Some(total_ms as f64 / 1000.0),
                            Some(details),
                        )
                        .await;
                    println!("Recorded fallback metrics to research_metrics.");
                }
            }
        }
    }

    Ok(())
}

async fn try_persist_to_arca(
    repo_id: &str,
    summary: &BuildSummary,
    crate_count: i64,
    fresh_count: i64,
    rustc_ver: &Option<String>,
    dep_fp: &Option<String>,
    run_name: &Option<String>,
) -> bool {
    use vox_db::{DbConfig, VoxDb};

    let config = match DbConfig::from_env() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let Ok(db) = VoxDb::connect(config).await else {
        return false;
    };

    let run_id = match db
        .insert_build_run(
            repo_id,
            run_name.as_deref(),
            rustc_ver.as_deref(),
            &summary.profile,
            summary.total_ms,
            crate_count,
            fresh_count,
            dep_fp.as_deref(),
        )
        .await
    {
        Ok(id) => id,
        Err(_) => return false,
    };

    // Samples
    let samples: Vec<_> = summary
        .crates
        .iter()
        .map(|c| {
            (
                c.name.as_str(),
                c.version.as_deref(),
                c.elapsed_ms,
                c.fresh,
                c.features.as_deref(),
            )
        })
        .collect();
    let _ = db.insert_crate_samples(run_id, &samples).await;

    // Warnings
    let warns: Vec<_> = summary
        .warnings
        .iter()
        .map(|w| {
            (
                w.crate_name.as_str(),
                w.level.as_str(),
                w.code.as_deref(),
                w.message.as_str(),
            )
        })
        .collect();
    let _ = db.insert_build_warnings(run_id, &warns).await;

    true
}
