//! Shared frontend scaffold, install, and build logic for run and bundle commands.
//!
//! **`pnpm`** invocations (`vox run` scaffold, **`islands/`** Vite build) use [`pnpm_executable`] as the
//! single source of truth for the OS-specific binary name.

use anyhow::{Context, Result};
use std::path::Path;
use std::time::Duration;

use crate::config;
use crate::fs_utils;
use crate::templates;

/// Basename of the **pnpm** CLI for this OS (`pnpm.cmd` on Windows, `pnpm` elsewhere).
#[must_use]
pub fn pnpm_executable() -> &'static str {
    if cfg!(windows) { "pnpm.cmd" } else { "pnpm" }
}

/// When **`islands/package.json`** exists and the repo has no `pnpm-workspace.yaml` yet, write one
/// listing the scaffolded main app package and **`islands/`** so pnpm treats them as one workspace.
pub fn maybe_write_root_pnpm_workspace(app_dir: &Path) -> Result<()> {
    let cwd = std::env::current_dir().context("current_dir for pnpm-workspace")?;
    let ctx = vox_repository::discover_repository_or_fallback(&cwd);
    let islands_pkg = ctx.root.join("islands").join("package.json");
    if !islands_pkg.is_file() {
        return Ok(());
    }
    let workspace_path = ctx.root.join("pnpm-workspace.yaml");
    if workspace_path.exists() {
        return Ok(());
    }
    let app_abs = if app_dir.is_absolute() {
        app_dir.to_path_buf()
    } else {
        cwd.join(app_dir)
    };
    let app_abs = std::fs::canonicalize(&app_abs).unwrap_or(app_abs);
    let repo_root = std::fs::canonicalize(&ctx.root).unwrap_or_else(|_| ctx.root.clone());
    let app_pkg_rel = app_abs
        .strip_prefix(&repo_root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dist/app".to_string());
    let yaml = format!("packages:\n  - '{app_pkg_rel}'\n  - 'islands'\n");
    std::fs::write(&workspace_path, yaml)
        .with_context(|| format!("write {}", workspace_path.display()))?;
    println!(
        "  Wrote {} (pnpm workspace: main app + islands)",
        workspace_path.display()
    );
    Ok(())
}

/// Spawns **`pnpm run dev:ssr-upstream`** when **`VOX_ORCHESTRATE_VITE=1`** so Axum can proxy HTML to Vite (see **`VOX_SSR_DEV_URL`**).
pub struct OrchestratedViteGuard(Option<std::process::Child>);

impl OrchestratedViteGuard {
    /// No child process (default when frontend is skipped or orchestration is off).
    #[must_use]
    pub fn disabled() -> Self {
        Self(None)
    }

    /// If **`VOX_ORCHESTRATE_VITE=1`**, start Vite on port **3001**.
    ///
    /// Returns an optional **`VOX_SSR_DEV_URL`** pair for the **`cargo run`** child when unset
    /// (Rust 2024 avoids mutating process environment via `set_var` here).
    pub fn maybe_spawn(app_dir: &Path) -> Result<(Self, Option<(String, String)>)> {
        if std::env::var("VOX_ORCHESTRATE_VITE").ok().as_deref() != Some("1") {
            return Ok((Self(None), None));
        }
        let pnpm = pnpm_executable();
        println!(
            "  VOX_ORCHESTRATE_VITE=1: spawning pnpm run dev:ssr-upstream in {}...",
            app_dir.display()
        );
        let child = std::process::Command::new(pnpm)
            .args(["run", "dev:ssr-upstream"])
            .current_dir(app_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .with_context(|| {
                format!(
                    "failed to spawn pnpm dev:ssr-upstream in {}",
                    app_dir.display()
                )
            })?;
        std::thread::sleep(Duration::from_secs(2));
        let url = "http://127.0.0.1:3001";
        let inject = if std::env::var("VOX_SSR_DEV_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .is_some()
        {
            None
        } else {
            println!("  Passing VOX_SSR_DEV_URL={url} to generated Axum (GET proxy for non-/api)");
            Some(("VOX_SSR_DEV_URL".to_string(), url.to_string()))
        };
        Ok((Self(Some(child)), inject))
    }
}

impl Drop for OrchestratedViteGuard {
    fn drop(&mut self) {
        if let Some(mut c) = self.0.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// Scaffold a Vite + React project around generated TS components.
///
/// When `tanstack_start` is true, writes a TanStack Start layout (`src/routes/*`, `router.tsx`,
/// `routeTree.gen.ts`) instead of `index.html` + `main.tsx`. Controlled by `Vox.toml` `[web] tanstack_start`
/// or **`VOX_WEB_TANSTACK_START=1`**.
pub fn scaffold_react_app(
    app_dir: &Path,
    generated_ts_dir: &Path,
    tanstack_start: bool,
) -> Result<()> {
    let src_dir = app_dir.join("src");
    let generated_dir = src_dir.join("generated");

    std::fs::create_dir_all(&generated_dir)
        .context("Failed to create app/src/generated directory")?;

    maybe_write_root_pnpm_workspace(app_dir)?;

    let port = config::default_port();
    let pkg = templates::package_json(tanstack_start);
    let vite = templates::vite_config(port, tanstack_start);

    std::fs::write(app_dir.join("package.json"), pkg).context("Failed to write package.json")?;
    std::fs::write(app_dir.join("vite.config.ts"), vite)
        .context("Failed to write vite.config.ts")?;
    std::fs::write(app_dir.join("tsconfig.json"), templates::tsconfig_json())
        .context("Failed to write tsconfig.json")?;
    std::fs::write(src_dir.join("index.css"), templates::index_css())
        .context("Failed to write index.css")?;

    if tanstack_start {
        std::fs::create_dir_all(src_dir.join("routes")).context("Failed to create src/routes")?;
        std::fs::write(
            src_dir.join("routes/__root.tsx"),
            templates::tanstack_start_root_tsx(),
        )
        .context("Failed to write routes/__root.tsx")?;
        let has_vox_programmatic_router = generated_ts_dir.join("VoxTanStackRouter.tsx").is_file();
        if has_vox_programmatic_router {
            // `routes:` + TanStack Start: single router from codegen (`voxRouteTree`); no file-route index.
            std::fs::write(
                src_dir.join("routeTree.gen.ts"),
                templates::tanstack_start_route_tree_gen_reexport(),
            )
            .context("Failed to write routeTree.gen.ts (re-export)")?;
        } else {
            let has_app = generated_ts_dir.join("App.tsx").is_file();
            let component_name = fs_utils::find_component_name(generated_ts_dir)?;
            let index_tsx = if has_app {
                templates::tanstack_start_index_for_app().to_string()
            } else {
                templates::tanstack_start_index_for_component(&component_name)
            };
            std::fs::write(src_dir.join("routes/index.tsx"), index_tsx)
                .context("Failed to write routes/index.tsx")?;
            std::fs::write(
                src_dir.join("routeTree.gen.ts"),
                templates::tanstack_start_route_tree_gen(),
            )
            .context("Failed to write routeTree.gen.ts")?;
        }
        std::fs::write(
            src_dir.join("router.tsx"),
            templates::tanstack_start_router_tsx(),
        )
        .context("Failed to write router.tsx")?;
    } else {
        std::fs::write(app_dir.join("index.html"), templates::index_html())
            .context("Failed to write index.html")?;
        let component_name = fs_utils::find_component_name(generated_ts_dir)?;
        std::fs::write(
            src_dir.join("main.tsx"),
            templates::main_tsx(&component_name),
        )
        .context("Failed to write main.tsx")?;
    }

    for entry in
        std::fs::read_dir(generated_ts_dir).context("Failed to read generated TS directory")?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "tsx" || e == "ts") {
            let dest = generated_dir.join(path.file_name().unwrap());
            std::fs::copy(&path, &dest)
                .with_context(|| format!("Failed to copy {} to generated/", path.display()))?;
        }
    }

    Ok(())
}

/// Run pnpm install and build in the scaffolded project.
pub fn npm_install_and_build(app_dir: &Path) -> Result<()> {
    let pnpm = pnpm_executable();

    if !app_dir.join("node_modules").exists() {
        println!("  Installing pnpm dependencies...");
        let status = std::process::Command::new(pnpm)
            .args(["install", "--prefer-offline"])
            .current_dir(app_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit())
            .status()
            .context("Failed to run pnpm install. Is Node.js and pnpm installed?")?;
        if !status.success() {
            anyhow::bail!("pnpm install failed");
        }
    }

    println!("  Building frontend assets...");
    let build_status = std::process::Command::new(pnpm)
        .args(["run", "build"])
        .current_dir(app_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("Failed to build frontend")?;
    if !build_status.success() {
        anyhow::bail!("Frontend build failed");
    }

    Ok(())
}

/// Build islands (Vite) when `islands/package.json` exists.
///
/// Runs `pnpm install` and `pnpm build` in `islands/`, then copies
/// `islands/dist/*` into `target/generated/<static_dir>/islands/` for `rust_embed`
/// (alongside the main Vite app under `public/`).
pub fn build_islands_if_present(generated_dir: &Path, static_dir: &str) -> Result<()> {
    let islands_dir = Path::new("islands");
    let package_json = islands_dir.join("package.json");
    if !package_json.exists() {
        return Ok(());
    }

    let island_src = islands_dir.join("src");
    std::fs::create_dir_all(&island_src).context("Failed to create islands/src")?;
    std::fs::write(
        island_src.join("island-mount.tsx"),
        templates::islands_island_mount_tsx(),
    )
    .context("Failed to write islands/src/island-mount.tsx")?;

    let vite_path = islands_dir.join("vite.config.ts");
    if let Ok(existing) = std::fs::read_to_string(&vite_path) {
        if !existing.contains("island-mount") {
            std::fs::write(&vite_path, templates::islands_vite_config())
                .context("Failed to upgrade islands/vite.config.ts for island-mount entry")?;
        }
    }

    let pnpm = pnpm_executable();

    if !islands_dir.join("node_modules").exists() {
        println!("  Installing island dependencies...");
        let status = std::process::Command::new(pnpm)
            .args(["install", "--prefer-offline"])
            .current_dir(islands_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit())
            .status()
            .context("Failed to run pnpm install in islands/. Is Node.js and pnpm installed?")?;
        if !status.success() {
            anyhow::bail!("pnpm install failed in islands/");
        }
    }

    println!("  Building islands...");
    let build_status = std::process::Command::new(pnpm)
        .args(["run", "build"])
        .current_dir(islands_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("Failed to build islands")?;
    if !build_status.success() {
        anyhow::bail!("Island build failed");
    }

    let source = islands_dir.join("dist");
    let dest = generated_dir.join(static_dir).join("islands");
    if source.exists() {
        std::fs::create_dir_all(&dest).context("Failed to create islands output dir")?;
        fs_utils::copy_dir_recursive(&source, &dest).with_context(|| {
            format!("Failed to copy {} to {}", source.display(), dest.display())
        })?;
        println!("  Island bundles copied to {}", dest.display());

        let index_html = generated_dir.join(static_dir).join("index.html");
        if index_html.is_file() {
            inject_island_mount_script(&index_html).with_context(|| {
                format!(
                    "Failed to inject island-mount script into {}",
                    index_html.display()
                )
            })?;
        }
    }

    Ok(())
}

fn inject_island_mount_script(index_path: &Path) -> Result<()> {
    let mut html = std::fs::read_to_string(index_path)
        .with_context(|| format!("read {}", index_path.display()))?;
    if html.contains("island-mount.js") {
        return Ok(());
    }
    let inject = "  <script type=\"module\" src=\"/islands/island-mount.js\"></script>\n";
    if let Some(pos) = html.rfind("</body>") {
        html.insert_str(pos, inject);
        std::fs::write(index_path, html)?;
    }
    Ok(())
}

/// Copy built static assets from Vite output to the backend's public directory.
pub fn copy_built_assets(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        anyhow::bail!("Built assets not found at {}", from.display());
    }
    if to.exists() {
        std::fs::remove_dir_all(to).ok();
    }
    std::fs::create_dir_all(to)?;
    fs_utils::copy_dir_recursive(from, to).with_context(|| {
        format!(
            "Failed to copy assets from {} to {}",
            from.display(),
            to.display()
        )
    })?;
    Ok(())
}
