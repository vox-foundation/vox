//! `--build-perf` and `--scope` doctor paths (needs `vox-eval`, `walkdir`).

use anyhow::Result;
use tokio::process::Command;

use crate::commands::ci::bounded_read::{read_utf8_path_capped, read_utf8_path_capped_async};
use std::path::Path;

use super::common::Check;
use super::output;

/// Run dep-budget, TLS, sccache, baseline, linker checks; prints footer hints.
pub async fn run_build_perf(json: bool) -> Result<()> {
    println!("Running build performance analysis...");
    println!();

    let mut checks: Vec<Check> = Vec::new();

    let dep_budget = Command::new("cargo")
        .args(["xtask", "dep-budget"])
        .output()
        .await;
    checks.push(match dep_budget {
        Ok(o) if o.status.success() => Check::pass(
            "Dep Weight Ceilings (cargo xtask dep-budget)",
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .take(10)
                .collect::<Vec<_>>()
                .join(" | "),
        ),
        Ok(o) => Check::fail(
            "Dep Weight Ceilings (cargo xtask dep-budget)",
            format!("FAILED\n{}", String::from_utf8_lossy(&o.stdout).trim()),
        ),
        Err(e) => Check {
            name: "Dep Weight Ceilings (cargo xtask dep-budget)".to_string(),
            pass: false,
            detail: format!("Failed to run cargo xtask dep-budget: {e}"),
        },
    });

    let tls_pass = if tokio::fs::try_exists("Cargo.lock").await.unwrap_or(false) {
        let lock = read_utf8_path_capped_async(Path::new("Cargo.lock"))
            .await
            .unwrap_or_default();
        if lock.contains("name = \"native-tls\"") || lock.contains("name = \"openssl-sys\"") {
            let tree = Command::new("cargo")
                .args(["tree", "-p", "vox-cli", "-e", "normal"])
                .output()
                .await
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();
            !tree.contains("native-tls") && !tree.contains("openssl-sys")
        } else {
            true
        }
    } else {
        true
    };
    checks.push(Check {
        name: "TLS Backend (rustls-tls, no native-tls)".to_string(),
        pass: tls_pass,
        detail: if tls_pass {
            "✅ No native-tls or openssl-sys reachable from vox-cli".to_string()
        } else {
            "❌ native-tls or openssl-sys is reachable! Use rustls-tls. See docs/src/how-to/adding-a-dependency.md"
                .to_string()
        },
    });

    let sccache = Command::new("sccache").arg("--version").output().await;
    checks.push(match sccache {
        Ok(o) if o.status.success() => Check {
            name: "sccache (build cache)".to_string(),
            pass: true,
            detail: String::from_utf8_lossy(&o.stdout).trim().to_string(),
        },
        _ => Check {
            name: "sccache (build cache)".to_string(),
            pass: false,
            detail: "not installed — install: cargo install sccache (see docs/src/how-to/fast-builds.md)"
                .to_string(),
        },
    });

    let baseline_path = "docs/src/reference/ref-build-baseline.md";
    let baseline_status = if tokio::fs::try_exists(baseline_path).await.unwrap_or(false) {
        let content = read_utf8_path_capped_async(Path::new(baseline_path))
            .await
            .unwrap_or_default();
        if content.contains("## Timing Baseline") {
            "Found — run 'cargo xtask baseline-timings --update' after builds to refresh"
                .to_string()
        } else {
            "Missing '## Timing Baseline' section — run 'cargo xtask baseline-timings --update'"
                .to_string()
        }
    } else {
        format!("{baseline_path} not found (not in workspace root?)")
    };
    checks.push(Check::new(
        "Timing Baseline SSOT",
        baseline_status.contains("Found"),
        baseline_status,
    ));

    let cargo_config = {
        let mut dir = std::env::current_dir().ok();
        let mut found: Option<String> = None;
        while let Some(d) = dir {
            let candidate = d.join(".cargo/config.toml");
            if candidate.exists() {
                found = read_utf8_path_capped(&candidate).ok();
                break;
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
        found.unwrap_or_default()
    };
    let linker_pass = cargo_config.contains("rust-lld")
        || cargo_config.contains("ld64.lld")
        || cargo_config.contains("fuse-ld=mold");
    let linker_detail = if cargo_config.contains("rust-lld") {
        "✅ rust-lld configured (Windows/cross-platform fast linker)"
    } else if cargo_config.contains("fuse-ld=mold") {
        "✅ mold configured via clang + fuse-ld=mold (Linux fast linker)"
    } else if cargo_config.contains("ld64.lld") {
        "✅ ld64.lld configured (macOS fast linker)"
    } else if cargo_config.is_empty() {
        "❌ .cargo/config.toml not found in workspace tree. See docs/src/how-to/fast-builds.md"
    } else {
        "❌ No fast linker configured. See docs/src/how-to/fast-builds.md"
    };
    checks.push(Check {
        name: "Linker Optimization".to_string(),
        pass: linker_pass,
        detail: linker_detail.to_string(),
    });

    output::print_results(&checks, false, json);
    println!();
    println!("Run 'cargo xtask dep-budget' for full ceiling details.");
    println!("Run 'cargo xtask baseline-timings --update' to refresh the CI timing baseline.");
    Ok(())
}

/// Domain scope audit over `examples/` and `docs/src`.
pub async fn run_scope(json: bool) -> Result<()> {
    println!("Running domain scope compliance audit...");
    println!();

    let mut checks: Vec<Check> = Vec::new();
    let mut total_files = 0;
    let mut total_violations = 0;

    let mut scan_paths: Vec<std::path::PathBuf> = vec![];
    for entry in walkdir::WalkDir::new("examples")
        .into_iter()
        .chain(walkdir::WalkDir::new("docs/src"))
        .filter_map(Result::ok)
    {
        let path = entry.into_path();
        if path.is_file() {
            let p = path.to_string_lossy();
            if p.ends_with(".vox") || p.ends_with(".md") {
                scan_paths.push(path);
            }
        }
    }

    for path in scan_paths {
        let content = read_utf8_path_capped_async(&path)
            .await
            .unwrap_or_default();

        let snippets: Vec<String> = if path.extension().is_some_and(|e| e == "md") {
            let mut blocks = vec![];
            let mut in_block = false;
            let mut current_block = String::new();
            for line in content.lines() {
                if line.starts_with("```vox") {
                    in_block = true;
                    current_block.clear();
                } else if line.starts_with("```") && in_block {
                    in_block = false;
                    blocks.push(current_block.clone());
                } else if in_block {
                    current_block.push_str(line);
                    current_block.push('\n');
                }
            }
            blocks
        } else {
            vec![content]
        };

        if snippets.is_empty() {
            continue;
        }

        total_files += 1;
        let mut file_violations = 0;

        for snippet in snippets {
            if vox_eval::scope_compliance_score(&snippet) < 1.0 {
                file_violations += 1;
            }
        }

        if file_violations > 0 {
            total_violations += file_violations;
            checks.push(Check {
                name: path.to_string_lossy().into_owned(),
                pass: false,
                detail: format!("{} violations detected", file_violations),
            });
        } else {
            checks.push(Check {
                name: path.to_string_lossy().into_owned(),
                pass: true,
                detail: "compliant".to_string(),
            });
        }
    }

    output::print_results(&checks, false, json);
    println!();
    if total_violations == 0 {
        println!(
            "✅ No violations. All {} scanned files are scope-compliant.",
            total_files
        );
    } else {
        println!(
            "⚠ Found {} scope violations across {} scanned files.",
            total_violations, total_files
        );
        println!(
            "  Files using domain-specific syntax must include the corresponding 'import vox.*'."
        );
    }
    Ok(())
}
