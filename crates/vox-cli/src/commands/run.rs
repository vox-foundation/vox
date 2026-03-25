// toestub-ignore(arch/sprawl)
use crate::commands::build;
use crate::config;
use crate::frontend;
use anyhow::{Context, Result};
use clap::ValueEnum;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// How `vox run` chooses between app (compilerd / generated server) and script execution.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum RunMode {
    /// If the file has no `@page` (first 8 KiB scan), run as a script when `script-execution` is enabled; else app path. Override with `Vox.toml` `[web] run_mode` or `VOX_WEB_RUN_MODE`.
    #[default]
    Auto,
    /// Always use the app / dev-server path (build + `target/generated` server).
    App,
    /// Always use the script runner (`fn main()`), requires `--features script-execution`.
    Script,
}

/// Parse run mode strings from CLI / `vox-compilerd` JSON (`auto`, `app`, `script`).
///
/// Unknown values map to [`RunMode::Auto`] so older clients stay compatible.
pub fn parse_run_mode_from_str(s: &str) -> RunMode {
    match s.trim().to_ascii_lowercase().as_str() {
        "app" => RunMode::App,
        "script" => RunMode::Script,
        _ => RunMode::Auto,
    }
}

/// When **`VOX_MESH_ENABLED`** is set and this binary was built with **`populi`** (`vox-populi`),
/// publish this process to the local mens registry once (covers app and script `vox run` paths,
/// including `vox-compilerd` `run`).
async fn mesh_publish_best_effort_for_run() {
    #[cfg(feature = "populi")]
    {
        if vox_populi::populi_enabled_from_env() {
            let node_id = vox_populi::populi_env().node_id.clone();
            let path = vox_populi::local_registry_path();
            match vox_populi::publish_local_registry_best_effort() {
                Ok(()) => {
                    tracing::info!(
                        target: "vox.mens",
                        path = %path.display(),
                        node_id = node_id.as_deref().unwrap_or("(generated)"),
                        "mens registry publish (vox run)"
                    );
                    crate::populi_codex_telemetry::record_local_registry_publish_opt(
                        &path,
                        node_id.as_deref(),
                    )
                    .await;
                }
                Err(e) => {
                    tracing::debug!(
                        target: "vox.mens",
                        error = %e,
                        "mens registry publish failed (best-effort)"
                    );
                }
            }
            let _ = vox_populi::http_lifecycle::populi_http_join_best_effort(
                vox_populi::populi_registration_record_for_process(),
                "vox run",
            )
            .await;
        }
    }
}

/// Execute the `vox run` command (dispatch to App or Script mode).
pub async fn run(file: &Path, args: &[String], mode: RunMode) -> Result<()> {
    mesh_publish_best_effort_for_run().await;

    let use_script = match mode {
        RunMode::App => false,
        RunMode::Script => true,
        RunMode::Auto => match vox_config::VoxConfig::load().web_run_mode {
            vox_config::WebRunMode::App => false,
            vox_config::WebRunMode::Script => true,
            vox_config::WebRunMode::Auto => {
                crate::commands::runtime::run::run::is_script_file_by_page_heuristic(file)
            }
        },
    };

    #[cfg(feature = "script-execution")]
    if use_script {
        tracing::info!(
            target: "vox.script",
            path = %file.display(),
            ?mode,
            "dispatch native script execution lane"
        );
        let opts = crate::commands::runtime::run::script::ScriptOpts {
            sandbox: false,
            allow_mcp: false,
            no_cache: false,
            isolation: None,
            trust_class: Some("trusted_dev".into()),
            wasi_dirs: Vec::new(),
        };
        return crate::commands::runtime::run::script::run(file, args, &opts).await;
    }

    #[cfg(not(feature = "script-execution"))]
    if use_script {
        anyhow::bail!(
            "script run mode requires `vox` built with `--features script-execution` (file: {})",
            file.display()
        );
    }

    // 1. Build using existing build command logic
    let out_dir = PathBuf::from("dist");

    println!("Building {}...", file.display());
    build::run(file, &out_dir).await?;

    // 2. Check if we have frontend components to bundle
    let has_frontend = fs::read_dir(&out_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .any(|e| e.path().extension().is_some_and(|ext| ext == "tsx"))
        })
        .unwrap_or(false);

    if has_frontend {
        println!("\nBundling frontend...");
        build_frontend(&out_dir)?;
    }

    // 3. Run backend (Rust)
    let generated_dir = Path::new("target").join("generated");

    let (_orchestrated_vite, ssr_env) = if has_frontend {
        frontend::OrchestratedViteGuard::maybe_spawn(&out_dir.join("app"))?
    } else {
        (frontend::OrchestratedViteGuard::disabled(), None)
    };

    let port = config::default_port();
    println!("\nStarting server...");
    if has_frontend {
        println!("  Frontend + Backend at http://127.0.0.1:{port}");
    } else {
        println!("  Backend at http://127.0.0.1:{port}");
    }
    println!("  Press Ctrl+C to stop\n");

    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd
        .arg("run")
        .arg("--")
        .args(args)
        .current_dir(&generated_dir);
    if let Some((k, v)) = ssr_env {
        cargo_cmd.env(k, v);
    }
    let status = cargo_cmd
        .status()
        .context("Failed to execute cargo run in generated directory")?;

    if !status.success() {
        anyhow::bail!("Application exited with error code: {:?}", status.code());
    }

    Ok(())
}

/// Build the frontend React application and copy assets to backend public dir.
fn build_frontend(generated_ts_dir: &Path) -> Result<()> {
    let app_dir = generated_ts_dir.join("app");
    let tanstack_start = vox_config::VoxConfig::load().web_tanstack_start;
    frontend::scaffold_react_app(&app_dir, generated_ts_dir, tanstack_start)
        .context("Failed to scaffold Vite + React app")?;
    crate::commands::build::verify_app_src_generated_imports(&app_dir.join("src"))
        .context("Scaffold entry import graph (main.tsx / routes/index.tsx)")?;

    // pnpm install (skip if node_modules exists and is fresh)
    let node_modules = app_dir.join("node_modules");
    let pnpm = frontend::pnpm_executable();
    if !node_modules.exists() {
        println!("  Installing frontend dependencies (pnpm)...");
        let status = Command::new(pnpm)
            .args(["install", "--prefer-offline"])
            .current_dir(&app_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit())
            .status()
            .context("Failed to run pnpm install. Is pnpm on PATH?")?;

        if !status.success() {
            anyhow::bail!("pnpm install failed");
        }
    }

    // pnpm run build
    println!("  Building frontend assets...");
    let status = Command::new(pnpm)
        .args(["run", "build"])
        .current_dir(&app_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("Failed to build frontend")?;

    if !status.success() {
        anyhow::bail!("Frontend build failed");
    }

    // Copy built assets to target/generated/public/
    let public_dir = Path::new("target").join("generated").join("public");
    let built_dir = app_dir.join("dist");

    if built_dir.exists() {
        if public_dir.exists() {
            fs::remove_dir_all(&public_dir).ok();
        }
        fs::create_dir_all(&public_dir)?;
        copy_dir_recursive(&built_dir, &public_dir)?;
        println!("  Frontend assets copied to {}", public_dir.display());
    }

    let generated_root = Path::new("target").join("generated");
    frontend::build_islands_if_present(&generated_root, "public")?;

    Ok(())
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let from_path = entry.path();
        let to_path = to.join(entry.file_name());
        if from_path.is_dir() {
            fs::create_dir_all(&to_path)?;
            copy_dir_recursive(&from_path, &to_path)?;
        } else {
            fs::copy(&from_path, &to_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod parse_mode_tests {
    use super::{RunMode, parse_run_mode_from_str};

    #[test]
    fn parse_run_mode_from_str_maps_variants() {
        assert_eq!(parse_run_mode_from_str("SCRIPT"), RunMode::Script);
        assert_eq!(parse_run_mode_from_str("App "), RunMode::App);
        assert_eq!(parse_run_mode_from_str("auto"), RunMode::Auto);
        assert_eq!(parse_run_mode_from_str("unknown"), RunMode::Auto);
    }
}
