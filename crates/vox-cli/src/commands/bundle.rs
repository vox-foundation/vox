//! `vox bundle` — production-style packaging: codegen, React/Vite app, npm build, embed static files, ship one binary.

use crate::commands::build;
use crate::commands::runtime::run::script;
use crate::frontend;
use crate::cli_args::BundleMode;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;

/// Bundle a Vox source file into a complete, runnable web application or script binary.
///
/// 1. For `App` mode: Runs full web scaffolding, npm build, and embeds assets.
/// 2. For `Script` mode: Compiles to a single standalone binary (native or WASI).
pub async fn run(file: &Path, out_dir: &Path, target: Option<&str>, release: bool, mode: BundleMode) -> Result<()> {
    match mode {
        BundleMode::App => run_app_bundle(file, out_dir, target, release).await,
        BundleMode::Script => run_script_bundle(file, out_dir, target).await,
    }
}

async fn run_script_bundle(file: &Path, out_dir: &Path, target: Option<&str>) -> Result<()> {
    println!("=== Bundling Script: {} ===", file.display());
    
    // Setup script options for bundling
    let opts = script::ScriptOpts {
        sandbox: false,
        allow_mcp: false,
        no_cache: false,
        isolation: None, // Default to native for now, or could check env
        trust_class: None,
        wasi_dirs: Vec::new(),
        target_triple: target.map(|s| s.to_string()),
    };

    let (artifact_path, backend) = script::compile(file, &opts).await?;
    
    fs::create_dir_all(out_dir).await?;
    let app_name = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "script".into());
    let bin_name = if backend.cache_label().contains("wasi") {
        format!("{}.wasm", app_name)
    } else if cfg!(windows) {
        format!("{}.exe", app_name)
    } else {
        app_name
    };
    
    let dest = out_dir.join(bin_name);
    fs::copy(&artifact_path, &dest).await.context("Failed to copy script binary to output")?;
    
    println!("\n✓ Script bundle complete!");
    println!("  Binary: {}", dest.display());
    println!("  Size: {:.2} MB", fs::metadata(&dest).await?.len() as f64 / 1_048_576.0);
    
    Ok(())
}

async fn run_app_bundle(file: &Path, out_dir: &Path, target: Option<&str>, release: bool) -> Result<()> {
    // Step 1: Run the standard build pipeline
    println!("=== Step 1/5: Compiling Vox source ===");
    build::run(file, out_dir).await?;

    // Check if we have any frontend components
    let chat_tsx = out_dir.join("Chat.tsx");
    let has_chat = fs::try_exists(&chat_tsx).await.unwrap_or(false);
    let mut has_other_tsx = false;
    if !has_chat {
        if let Ok(mut rd) = fs::read_dir(out_dir).await {
            while let Ok(Some(e)) = rd.next_entry().await {
                if e.path().extension().is_some_and(|ext| ext == "tsx") {
                    has_other_tsx = true;
                    break;
                }
            }
        }
    }
    let has_frontend = has_chat || has_other_tsx;

    if !has_frontend {
        println!("No frontend components found. Backend-only build complete.");
        return Ok(());
    }

    // Step 2: Scaffold the React/Vite project
    println!("=== Step 2/5: Scaffolding React application ===");
    let app_dir = out_dir.join("app");
    let app_dir_for_scaffold = app_dir.clone();
    let out_for_scaffold = out_dir.to_path_buf();
    let tanstack_start = vox_config::VoxConfig::load().web_tanstack_start;
    tokio::task::spawn_blocking(move || {
        frontend::scaffold_react_app(&app_dir_for_scaffold, &out_for_scaffold, tanstack_start)
    })
    .await
    .context("scaffold join")?
    .context("Failed to scaffold Vite + React app")?;

    build::verify_app_src_generated_imports(&app_dir.join("src"))
        .context("Scaffold entry import graph (main.tsx / routes/index.tsx)")?;

    // Step 3: Install deps and build
    println!("=== Step 3/5: Installing dependencies & building ===");
    npm_install_and_build(&app_dir).await?;

    // Step 4: Copy built assets to backend public dir
    println!("=== Step 4/5: Packaging static assets ===");
    let generated_dir = PathBuf::from("target").join("generated");
    let public_dir = generated_dir.join("public");
    copy_built_assets(&app_dir.join("dist"), &public_dir).await?;
    crate::frontend::build_islands_if_present(&generated_dir, "public")?;

    // Step 5: Build the single binary
    println!("=== Step 5/5: Building single binary ===");
    let binary_path = build_single_binary(&generated_dir, target, release).await?;

    // Copy binary to dist/
    let dist_dir = PathBuf::from("dist");
    fs::create_dir_all(&dist_dir).await?;
    let app_name = file
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "app".to_string());
    let ext =
        if target.is_some_and(|t| t.contains("windows")) || (cfg!(windows) && target.is_none()) {
            ".exe"
        } else {
            ""
        };
    let dest = dist_dir.join(format!("{app_name}{ext}"));
    fs::copy(&binary_path, &dest)
        .await
        .with_context(|| format!("Failed to copy binary to {}", dest.display()))?;

    println!("\n✓ Bundle complete!");
    println!("  Single binary: {}", dest.display());
    if let Some(t) = target {
        println!("  Target: {}", t);
    }
    println!(
        "  Size: {:.1} MB",
        fs::metadata(&dest).await?.len() as f64 / 1_048_576.0
    );
    println!("\n  Run with: ./{}", dest.display());
    println!("  Then open: http://localhost:3000");

    Ok(())
}

/// Run npm install and build in the scaffolded project.
async fn npm_install_and_build(app_dir: &Path) -> Result<()> {
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    println!("  Running npm install...");
    let install_status = Command::new(npm)
        .arg("install")
        .arg("--prefer-offline")
        .current_dir(app_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .context("Failed to run npm install. Is Node.js/npm installed?")?;

    if !install_status.success() {
        anyhow::bail!("npm install failed");
    }

    println!("  Running npm run build...");
    let build_status = Command::new(npm)
        .arg("run")
        .arg("build")
        .current_dir(app_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .context("Failed to run npm run build")?;

    if !build_status.success() {
        anyhow::bail!("npm run build failed");
    }

    Ok(())
}

fn copy_dir_recursive_sync(from: &Path, to: &Path) -> Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let from_path = entry.path();
        let to_path = to.join(entry.file_name());

        if from_path.is_dir() {
            std::fs::create_dir_all(&to_path)?;
            copy_dir_recursive_sync(&from_path, &to_path)?;
        } else {
            std::fs::copy(&from_path, &to_path)?;
        }
    }
    Ok(())
}

/// Copy built static assets from Vite output to the backend's public directory.
async fn copy_built_assets(from: &Path, to: &Path) -> Result<()> {
    let from = from.to_path_buf();
    let to = to.to_path_buf();
    tokio::task::spawn_blocking(move || {
        if !from.exists() {
            anyhow::bail!("Built assets not found at {}", from.display());
        }
        if to.exists() {
            std::fs::remove_dir_all(&to).ok();
        }
        std::fs::create_dir_all(&to)?;
        copy_dir_recursive_sync(&from, &to).with_context(|| {
            format!(
                "Failed to copy assets from {} to {}",
                from.display(),
                to.display()
            )
        })
    })
    .await
    .map_err(|e| anyhow::anyhow!("copy task join: {e}"))?
}

/// Build the generated Rust backend into a single binary.
/// Optionally cross-compiles for a specific target triple.
async fn build_single_binary(
    generated_dir: &Path,
    target: Option<&str>,
    release: bool,
) -> Result<PathBuf> {
    if let Some(target_triple) = target {
        println!("  Installing target: {}", target_triple);
        let rustup = if cfg!(windows) {
            "rustup.exe"
        } else {
            "rustup"
        };
        let _ = Command::new(rustup)
            .args(["target", "add", target_triple])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .await;
    }

    let cargo = if cfg!(windows) { "cargo.exe" } else { "cargo" };
    let mut cmd = Command::new(cargo);
    cmd.arg("build");

    if release {
        cmd.arg("--release");
    }

    if let Some(target_triple) = target {
        cmd.args(["--target", target_triple]);
    }

    cmd.current_dir(generated_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    println!("  Running cargo build in {}", generated_dir.display());
    let status = cmd
        .status()
        .await
        .context("Failed to run cargo build on generated backend")?;

    if !status.success() {
        anyhow::bail!("cargo build failed");
    }

    let profile = if release { "release" } else { "debug" };
    let binary_name =
        if target.is_some_and(|t| t.contains("windows")) || (cfg!(windows) && target.is_none()) {
            "vox_generated_app.exe"
        } else {
            "vox_generated_app"
        };

    let binary_path = if let Some(target_triple) = target {
        generated_dir
            .join("target")
            .join(target_triple)
            .join(profile)
            .join(binary_name)
    } else {
        generated_dir.join("target").join(profile).join(binary_name)
    };

    if !fs::try_exists(&binary_path).await.unwrap_or(false) {
        anyhow::bail!("Binary not found at {}", binary_path.display());
    }

    Ok(binary_path)
}
