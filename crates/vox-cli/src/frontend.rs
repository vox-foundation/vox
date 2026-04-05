//! Shared frontend scaffold, install, and build logic for run and bundle commands.
//!
//! **`pnpm`** invocations (`vox run` scaffold, **`islands/`** Vite build) use [`pnpm_executable`] as the
//! single source of truth for the OS-specific binary name.
//!
//! ## Islands → Axum static (`build_islands_if_present`)
//!
//! After **`pnpm run build`** in **`islands/`**, artifacts land under **`target/generated/<static>/islands/`**.
//! The app shell **`index.html`** must load **`/islands/island-mount.js`** (V1) so browser hydration can mount
//! **`data-vox-island`** nodes.
//!
//! **Injection helper (OP-S043):** [`apply_island_mount_script_to_index_html`] is the pure gate (counts
//! `island-mount.js`, rejects duplicates, inserts [`ISLAND_MOUNT_INDEX_SCRIPT_SNIPPET`] before `</body>`).
//! [`inject_island_mount_script_into_index_file`] wraps read/write for build; decode of emitted attrs stays
//! in [`crate::templates::islands::islands_props_from_element_ts`] (OP-S041).
//!
//! **Telemetry + build notes A/B/C (OP-S071 / S093 / S141 / S175 / S203):** islands copy + `index.html` injection
//! are observable in `IslandsBuildSummary`; extend logging only with gate tests in `full_stack_minimal_build.rs`.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::commands::ci::bounded_read::read_utf8_path_capped;
use crate::config;
use crate::fs_utils;
use crate::templates;

static LOGGED_VOX_ISLAND_MOUNT_V2_STUB: AtomicBool = AtomicBool::new(false);

/// One-shot `eprintln!` line when `VOX_ISLAND_MOUNT_V2=1` (OP-0252 / OP-0311 contract; grep-friendly in CI logs).
pub const VOX_ISLAND_MOUNT_V2_STUB_MESSAGE: &str = "vox frontend: VOX_ISLAND_MOUNT_V2=1 — V2 index injection is not implemented; using V1 /islands/island-mount.js";

fn maybe_log_vox_island_mount_v2_stub_once() {
    let on = std::env::var("VOX_ISLAND_MOUNT_V2")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !on || LOGGED_VOX_ISLAND_MOUNT_V2_STUB.swap(true, Ordering::Relaxed) {
        return;
    }
    eprintln!("{VOX_ISLAND_MOUNT_V2_STUB_MESSAGE}");
}

#[cfg(test)]
pub(crate) fn reset_island_mount_v2_stub_log_for_tests() {
    LOGGED_VOX_ISLAND_MOUNT_V2_STUB.store(false, Ordering::Relaxed);
}

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
    let islands_pkg = if ctx
        .root
        .join("packages")
        .join("islands")
        .join("package.json")
        .is_file()
    {
        ctx.root
            .join("packages")
            .join("islands")
            .join("package.json")
    } else {
        ctx.root.join("islands").join("package.json")
    };
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
    let islands_dir = islands_pkg.parent().expect("islands package.json parent");
    let islands_rel = islands_dir
        .strip_prefix(&repo_root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "islands".to_string());
    let yaml = format!("packages:\n  - '{app_pkg_rel}'\n  - '{islands_rel}'\n");
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
    let has_vox_programmatic_router = generated_ts_dir.join("VoxTanStackRouter.tsx").is_file();
    let file_route_tsr_pregen = tanstack_start && !has_vox_programmatic_router;
    let pkg = templates::package_json(tanstack_start, file_route_tsr_pregen);
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
        if path
            .extension()
            .is_some_and(|e| e == "tsx" || e == "ts" || e == "css")
        {
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

    try_pnpm_routes_gen(app_dir, pnpm)?;

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

/// When the app uses TanStack file routes (not programmatic `voxRouteTree` re-export), run
/// **`pnpm run routes:gen`** so `routeTree.gen.ts` matches `src/routes/**` (TanStack Router CLI).
fn try_pnpm_routes_gen(app_dir: &Path, pnpm: &str) -> Result<()> {
    let route_tree = app_dir.join("src").join("routeTree.gen.ts");
    if !route_tree.is_file() {
        return Ok(());
    }
    let rt = read_utf8_path_capped(&route_tree)
        .with_context(|| format!("read {}", route_tree.display()))?;
    if rt.contains("voxRouteTree") {
        return Ok(());
    }
    let pkg_path = app_dir.join("package.json");
    if !pkg_path.is_file() {
        return Ok(());
    }
    let pkg =
        read_utf8_path_capped(&pkg_path).with_context(|| format!("read {}", pkg_path.display()))?;
    if !pkg.contains("\"routes:gen\"") {
        return Ok(());
    }
    println!("  Regenerating TanStack route tree (pnpm run routes:gen)...");
    let status = std::process::Command::new(pnpm)
        .args(["run", "routes:gen"])
        .current_dir(app_dir)
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("Failed to run pnpm run routes:gen. Is @tanstack/router-cli installed?")?;
    if !status.success() {
        anyhow::bail!("pnpm run routes:gen failed");
    }
    Ok(())
}

/// V1 **`index.html`** injection line; keep in sync with `island-mount.js` substring counting in [`apply_island_mount_script_to_index_html`].
pub const ISLAND_MOUNT_INDEX_SCRIPT_SNIPPET: &str =
    r#"  <script type="module" src="/islands/island-mount.js"></script>"#;

/// Result of scanning / rewriting the Axum static **`index.html`** for island hydration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IslandMountScriptInjection {
    /// `false` when no **`index.html`** was examined (missing upstream).
    pub evaluated: bool,
    /// Occurrences of the substring **`island-mount.js`** before rewrite (**>1** is rejected).
    pub island_mount_js_refs: usize,
    pub injected: bool,
    pub skipped_already_present: bool,
    pub skipped_no_body: bool,
}

/// Per-run rollup from [`build_islands_if_present`] for logging and integration tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IslandsBuildSummary {
    pub islands_package_present: bool,
    pub vite_dist_copied: bool,
    pub index_injection: IslandMountScriptInjection,
}

fn count_island_mount_js_refs(html: &str) -> usize {
    html.matches("island-mount.js").count()
}

/// Pure helper: append the V1 mount script before **`</body>`** when **`island-mount.js`** is not already referenced.
///
/// Fails when the HTML already contains **more than one** `island-mount.js` reference (duplicate tags / comments).
pub fn apply_island_mount_script_to_index_html(
    html: &str,
) -> Result<(String, IslandMountScriptInjection)> {
    maybe_log_vox_island_mount_v2_stub_once();
    let island_mount_js_refs = count_island_mount_js_refs(html);
    if island_mount_js_refs > 1 {
        bail!(
            "index.html references island-mount.js {island_mount_js_refs} times (expected at most one)"
        );
    }
    let mut report = IslandMountScriptInjection {
        evaluated: true,
        island_mount_js_refs,
        ..Default::default()
    };
    if island_mount_js_refs == 1 {
        report.skipped_already_present = true;
        return Ok((html.to_owned(), report));
    }
    let inject = format!("{}\n", ISLAND_MOUNT_INDEX_SCRIPT_SNIPPET);
    if let Some(pos) = html.rfind("</body>") {
        let mut out = html.to_owned();
        out.insert_str(pos, &inject);
        report.injected = true;
        Ok((out, report))
    } else {
        report.skipped_no_body = true;
        Ok((html.to_owned(), report))
    }
}

pub fn inject_island_mount_script_into_index_file(
    index_path: &Path,
) -> Result<IslandMountScriptInjection> {
    let html = read_utf8_path_capped(index_path)
        .with_context(|| format!("read {}", index_path.display()))?;
    let (new_html, report) = apply_island_mount_script_to_index_html(&html).with_context(|| {
        format!(
            "island-mount script policy failed for {}",
            index_path.display()
        )
    })?;
    if report.injected && new_html != html {
        std::fs::write(index_path, &new_html)
            .with_context(|| format!("write {}", index_path.display()))?;
        println!(
            "  vox frontend compat: island-mount V1 script linked in {}",
            index_path.display()
        );
    } else if report.skipped_no_body {
        println!(
            "  note: {} has no </body>; island-mount.js not auto-injected",
            index_path.display()
        );
    }
    Ok(report)
}

/// Build islands (Vite) when `islands/package.json` exists.
///
/// Runs `pnpm install` and `pnpm build` in `islands/`, then copies
/// `islands/dist/*` into `target/generated/<static_dir>/islands/` for `rust_embed`
/// (alongside the main Vite app under `public/`).
pub fn build_islands_if_present(
    generated_dir: &Path,
    static_dir: &str,
) -> Result<IslandsBuildSummary> {
    let mut summary = IslandsBuildSummary::default();
    let cwd = std::env::current_dir().context("cwd for islands build")?;
    let ctx = vox_repository::discover_repository_or_fallback(&cwd);
    let islands_dir = crate::island_paths::island_package_root(&ctx.root);
    let package_json = islands_dir.join("package.json");
    if !package_json.exists() {
        return Ok(summary);
    }
    summary.islands_package_present = true;

    let island_src = islands_dir.join("src");
    std::fs::create_dir_all(&island_src).context("Failed to create islands/src")?;
    std::fs::write(
        island_src.join("island-mount.tsx"),
        templates::islands_island_mount_tsx(),
    )
    .context("Failed to write islands/src/island-mount.tsx")?;

    let vite_path = islands_dir.join("vite.config.ts");
    if let Ok(existing) = read_utf8_path_capped(&vite_path) {
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
            .current_dir(&islands_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit())
            .status()
            .context("Failed to run pnpm install in islands/. Is Node.js and pnpm installed?")?;
        if !status.success() {
            bail!("pnpm install failed in islands/");
        }
    }

    println!("  Building islands...");
    let build_status = std::process::Command::new(pnpm)
        .args(["run", "build"])
        .current_dir(&islands_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("Failed to build islands")?;
    if !build_status.success() {
        bail!("Island build failed");
    }

    let source = islands_dir.join("dist");
    let dest = generated_dir.join(static_dir).join("islands");
    if source.exists() {
        summary.vite_dist_copied = true;
        std::fs::create_dir_all(&dest).context("Failed to create islands output dir")?;
        fs_utils::copy_dir_recursive(&source, &dest).with_context(|| {
            format!("Failed to copy {} to {}", source.display(), dest.display())
        })?;
        println!("  Island bundles copied to {}", dest.display());

        let index_html = generated_dir.join(static_dir).join("index.html");
        if index_html.is_file() {
            summary.index_injection = inject_island_mount_script_into_index_file(&index_html)
                .with_context(|| {
                    format!(
                        "Failed to inject island-mount script into {}",
                        index_html.display()
                    )
                })?;
        }
    }

    Ok(summary)
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

#[cfg(test)]
mod island_mount_index_tests {
    #![allow(unsafe_code)]

    use super::*;

    #[test]
    fn apply_inserts_snippet_before_body() {
        let html = "<html><head></head><body></body></html>";
        let (out, r) = apply_island_mount_script_to_index_html(html).unwrap();
        assert!(r.evaluated);
        assert_eq!(r.island_mount_js_refs, 0);
        assert!(r.injected);
        assert!(!r.skipped_already_present);
        assert!(!r.skipped_no_body);
        assert!(out.contains(ISLAND_MOUNT_INDEX_SCRIPT_SNIPPET));
        assert!(out.contains("</body>"));
    }

    #[test]
    fn apply_skips_when_ref_present() {
        let html = concat!(
            "<body>",
            r#"<script type="module" src="/islands/island-mount.js"></script>"#,
            "</body>"
        );
        let (out, r) = apply_island_mount_script_to_index_html(html).unwrap();
        assert!(r.skipped_already_present);
        assert!(!r.injected);
        assert_eq!(out, html);
    }

    #[test]
    fn apply_errors_on_duplicate_refs() {
        let html = concat!(
            "<body>",
            r#"<script src="/islands/island-mount.js"></script>"#,
            r#"<script type="module" src="/islands/island-mount.js"></script>"#,
            "</body>"
        );
        assert!(apply_island_mount_script_to_index_html(html).is_err());
    }

    #[test]
    fn apply_no_body_is_skipped() {
        let html = "<html></html>";
        let (out, r) = apply_island_mount_script_to_index_html(html).unwrap();
        assert!(r.skipped_no_body);
        assert!(!r.injected);
        assert_eq!(out, html);
    }

    /// OP-0311: stderr line is a single SSOT string (CI log grep); env triggers `eprintln!` path without panicking.
    #[test]
    fn v2_stub_message_contract_and_apply_with_env_succeeds() {
        reset_island_mount_v2_stub_log_for_tests();
        assert!(VOX_ISLAND_MOUNT_V2_STUB_MESSAGE.contains("V2 index injection"));
        assert!(VOX_ISLAND_MOUNT_V2_STUB_MESSAGE.contains("/islands/island-mount.js"));

        struct Guard {
            prev: Option<std::ffi::OsString>,
        }
        impl Drop for Guard {
            fn drop(&mut self) {
                match &self.prev {
                    Some(v) => unsafe { std::env::set_var("VOX_ISLAND_MOUNT_V2", v) },
                    None => unsafe { std::env::remove_var("VOX_ISLAND_MOUNT_V2") },
                }
            }
        }
        let prev = std::env::var_os("VOX_ISLAND_MOUNT_V2");
        unsafe {
            std::env::set_var("VOX_ISLAND_MOUNT_V2", "1");
        }
        let _guard = Guard { prev };
        let (out, r) =
            apply_island_mount_script_to_index_html("<html><body></body></html>").unwrap();
        assert!(r.injected);
        assert!(out.contains(ISLAND_MOUNT_INDEX_SCRIPT_SNIPPET));
    }
}
