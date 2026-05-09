//! `vox build` — full compile pipeline and artifact layout.
//!
//! Writes **TypeScript** into `out_dir` and **Rust** under `target/generated/` (Axum-style backend).
//! Optional **`--scaffold`** (or `VOX_WEB_EMIT_SCAFFOLD=1`) writes user-owned Vite/app files via
//! [`vox_codegen::codegen_ts::scaffold`]. `@v0` uses `V0_API_KEY` when set — see `crate::v0::generate_component`.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use vox_bounded_fs::read_utf8_path_capped;

/// Run the build pipeline for `file`, writing TS to `out_dir` and Rust to `target/generated`.
///
/// `emit_scaffold`: write [`vox_codegen::codegen_ts::scaffold`] files when missing (or set `VOX_WEB_EMIT_SCAFFOLD=1`).
pub async fn run(
    file: &Path,
    out_dir: &Path,
    target: Option<String>,
    emit_scaffold: bool,
    emit_ir: bool,
    mode: crate::cli_args::BuildMode,
) -> Result<()> {
    let frontend = crate::pipeline::run_frontend(file, false).await?;
    crate::pipeline::print_diagnostics(&frontend, file, false);
    if frontend.has_errors() {
        anyhow::bail!(
            "Build failed with {} error(s) and {} warning(s)",
            frontend.error_count(),
            frontend.warning_count()
        );
    }
    tracing::info!(
        "Frontend passed with {} warning(s)",
        frontend.warning_count()
    );
    let crate::pipeline::FrontendResult { module, hir, .. } = frontend;

    // 5. Generate TypeScript (Frontend)
    let ts_opts = vox_codegen::codegen_ts::CodegenOptions {
        tanstack_start: vox_config::VoxConfig::load().web_tanstack_start,
        target: target.clone(),
        mode: match mode {
            crate::cli_args::BuildMode::App => vox_codegen::codegen_ts::emitter::BuildMode::App,
            crate::cli_args::BuildMode::Library => {
                vox_codegen::codegen_ts::emitter::BuildMode::Library
            }
        },
    };
    let ts_output = vox_codegen::codegen_ts::generate_with_options(&hir, ts_opts)
        .map_err(|e| anyhow::anyhow!("TypeScript codegen error: {}", e))?;

    // 6. Generate Rust (Backend)
    let rust_output = vox_codegen::codegen_rust::generate(&hir, "vox_generated_app")
        .map_err(|e| anyhow::anyhow!("Rust code generation failed: {e}"))?;

    // 7. Write output files
    fs::create_dir_all(out_dir)
        .with_context(|| format!("Failed to create output directory: {}", out_dir.display()))?;

    // Write generated TS files
    for (filename, content) in &ts_output.files {
        let path = out_dir.join(filename);
        fs::write(&path, content)
            .with_context(|| format!("Failed to write output file: {}", path.display()))?;
        println!("  wrote {}", path.display());
    }

    let emitted_manifest = ts_output
        .files
        .iter()
        .any(|(n, _)| n == "routes.manifest.ts" || n == "routes.manifest.json");
    if emitted_manifest {
        let written_names: std::collections::HashSet<&str> =
            ts_output.files.iter().map(|(n, _)| n.as_str()).collect();
        let mut to_remove = vec!["App.tsx", "VoxTanStackRouter.tsx", "serverFns.ts"];
        if mode == crate::cli_args::BuildMode::Library {
            to_remove.push("routes.manifest.ts");
        }
        for stale_name in to_remove {
            // Do not delete artifacts emitted in this same build — stale cleanup targets only
            // leftover files from prior scaffold/toolchain versions (see routes.manifest import graph).
            if written_names.contains(stale_name) {
                continue;
            }
            let stale = out_dir.join(stale_name);
            if stale.is_file() {
                fs::remove_file(&stale)
                    .with_context(|| format!("Failed to remove stale {}", stale.display()))?;
                println!("  removed stale {}", stale.display());
            }
        }
    }

    let scaffold_env = std::env::var("VOX_WEB_EMIT_SCAFFOLD")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if emit_scaffold || scaffold_env {
        let project_root = out_dir.parent().unwrap_or(out_dir);
        vox_codegen::codegen_ts::scaffold::write_scaffold_if_missing(project_root, "vox-app")
            .with_context(|| "Failed to write web scaffold files")?;
    }

    // 8. Handle @v0 components
    // We iterate over the parsed declarations to find V0Components
    for decl in &module.declarations {
        if let vox_compiler::ast::decl::Decl::V0Component(comp) = decl {
            if comp.image_path.is_some() {
                // Asset-hint form (`@v0 from "…"`) has no v0 chat id; placeholder TSX comes from codegen only.
                continue;
            }
            if comp.v0_id.is_empty() {
                continue;
            }
            let component_name = &comp.name;
            let filename = format!("{}.tsx", component_name);
            let target_path = out_dir.join(&filename);

            // Only generate if file doesn't exist to avoid overwriting edits
            if !target_path.exists() {
                println!("Generating v0 component '{}'...", component_name);

                println!(
                    "Downloading v0 component '{}' via npx v0 add...",
                    component_name
                );
                let status = tokio::process::Command::new("npx")
                    .arg("v0")
                    .arg("add")
                    .arg(&comp.v0_id)
                    .arg("--name")
                    .arg(component_name)
                    .arg("--path")
                    .arg(target_path.to_string_lossy().as_ref())
                    .arg("--yes")
                    .current_dir(file.parent().unwrap_or(Path::new(".")))
                    .status()
                    .await;

                match status {
                    Ok(s) if s.success() => {
                        println!("  generated v0 component: {}", target_path.display())
                    }
                    Ok(s) => eprintln!(
                        "  failed to download v0 component '{}': exited with {}",
                        component_name, s
                    ),
                    Err(e) => eprintln!(
                        "  failed to execute npx v0 add for '{}': {}",
                        component_name, e
                    ),
                }
            } else {
                println!("  skipping v0 component '{}' (file exists)", component_name);
            }
        }
    }

    for decl in &module.declarations {
        if let vox_compiler::ast::decl::Decl::V0Component(comp) = decl {
            let target_path = out_dir.join(format!("{}.tsx", comp.name));
            if target_path.is_file() {
                let tsx = read_utf8_path_capped(&target_path)
                    .with_context(|| format!("read @v0 component {}", target_path.display()))?;
                if let Some(msg) =
                    crate::v0_tsx_normalize::v0_named_export_violation(&tsx, &comp.name)
                {
                    anyhow::bail!("@v0 named export contract: {msg}");
                }
            }
        }
    }

    verify_app_tsx_route_imports(out_dir)
        .context("generated TS import graph (routes.manifest / App)")?;

    // Write API client for server functions (if any)
    if !rust_output.api_client_ts.is_empty() {
        let api_path = out_dir.join("api.ts");
        fs::write(&api_path, &rust_output.api_client_ts)
            .with_context(|| format!("Failed to write API client: {}", api_path.display()))?;
        println!("  wrote {}", api_path.display());
    }

    // Rust goes to target/generated
    let generated_dir = std::path::Path::new("target").join("generated");
    fs::create_dir_all(generated_dir.join("src"))
        .context("Failed to create generated src directory")?;

    for (filename, content) in &rust_output.files {
        let path = generated_dir.join(filename);
        // Ensure parent dir exists (e.g. src/)
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)
            .with_context(|| format!("Failed to write output file: {}", path.display()))?;
        println!("  wrote {}", path.display());
    }

    if emit_ir {
        let web_ir = vox_codegen::web_ir::lower::lower_hir_to_web_ir(&hir);
        let ir_json =
            serde_json::to_string_pretty(&web_ir).context("Failed to serialize WebIR to JSON")?;
        let ir_path = out_dir.join("web-ir.v1.json");
        fs::write(&ir_path, ir_json)
            .with_context(|| format!("Failed to write IR file: {}", ir_path.display()))?;
        println!("  wrote {}", ir_path.display());
    }

    let public_dir = generated_dir.join("public").join("ssg-shells");
    fs::create_dir_all(&public_dir).context("Failed to create public/ssg-shells")?;
    for (rel_path, html) in vox_ssg::generate_static_site(&module) {
        let out = public_dir.join(&rel_path);
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&out, html).with_context(|| {
            format!(
                "Failed to write SSG shell {} (from {})",
                out.display(),
                rel_path
            )
        })?;
        println!("  wrote {}", out.display());
    }

    if let Some(t) = target {
        if t == "ios" || t == "android" {
            println!("Synchronizing Capacitor project for {}...", t);
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let status = tokio::process::Command::new("npx")
                .arg("cap")
                .arg("sync")
                .arg(&t)
                .current_dir(&cwd)
                .status()
                .await;
            match status {
                Ok(s) if s.success() => println!("  Capacitor sync complete."),
                Ok(s) => eprintln!("  Capacitor sync exited with {s}"),
                Err(e) => eprintln!("  Failed to execute npx cap sync: {e}"),
            }

            if t == "android" {
                let res_dir = cwd.join("android/app/src/main/res/xml");
                if std::fs::create_dir_all(&res_dir).is_ok() {
                    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<network-security-config>
    <domain-config cleartextTrafficPermitted="true">
        <domain includeSubdomains="true">127.0.0.1</domain>
        <domain includeSubdomains="true">localhost</domain>
    </domain-config>
</network-security-config>"#;
                    let _ = std::fs::write(res_dir.join("network_security_config.xml"), xml);
                }

                // WAKE_LOCK injection
                let manifest_path = cwd.join("android/app/src/main/AndroidManifest.xml");
                if manifest_path.is_file() {
                    let mut m = std::fs::read_to_string(&manifest_path).unwrap_or_default();
                    if !m.contains("android.permission.WAKE_LOCK") {
                        m = m.replace("<application", "<uses-permission android:name=\"android.permission.WAKE_LOCK\" />\n    <application");
                        let _ = std::fs::write(&manifest_path, m);
                    }
                }
            }
        }
    }

    println!(
        "Build complete: {} TS file(s), {} Rust file(s) generated",
        ts_output.files.len(),
        rust_output.files.len()
    );
    Ok(())
}

/// After `scaffold_react_app`, ensure `main.tsx` / `routes/index.tsx` only import existing files.
///
/// Resolves `./…` and `../…` paths relative to each file's directory (Vite TS style).
pub fn verify_app_src_generated_imports(app_src_dir: &Path) -> Result<()> {
    for rel in ["main.tsx", "routes/index.tsx"] {
        let p = app_src_dir.join(rel);
        if !p.is_file() {
            continue;
        }
        verify_ts_relative_imports_from_file(&p)?;
    }
    Ok(())
}

fn resolve_ts_import_path(from_dir: &Path, import: &str) -> PathBuf {
    let import = import.replace('\\', "/");
    let mut out = from_dir.to_path_buf();
    for part in import.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            let _ = out.pop();
        } else {
            out.push(part);
        }
    }
    out
}

fn ts_import_target_exists(path: &Path) -> bool {
    if path.is_file() {
        return true;
    }
    let tsx = path.with_extension("tsx");
    if tsx.is_file() {
        return true;
    }
    path.with_extension("ts").is_file()
}

fn verify_ts_relative_imports_from_file(ts_file: &Path) -> Result<()> {
    let from_dir = ts_file
        .parent()
        .with_context(|| format!("No parent for {}", ts_file.display()))?;
    let content = read_utf8_path_capped(ts_file)
        .with_context(|| format!("Failed to read {}", ts_file.display()))?;
    let re = regex::Regex::new(r#"(?sm)^\s*(?:import|export)\s+.*?\s+from\s+["']([^"']+)["']"#)
        .with_context(|| format!("compile TS import regex ({})", ts_file.display()))?;
    for cap in re.captures_iter(&content) {
        let raw = cap
            .get(1)
            .with_context(|| format!("TS import regex missing capture ({})", ts_file.display()))?
            .as_str();
        if !(raw.starts_with("./") || raw.starts_with("../")) {
            continue;
        }
        let target = resolve_ts_import_path(from_dir, raw);
        if !ts_import_target_exists(&target) {
            anyhow::bail!(
                "{} imports `{raw}` but `{}` was not found (expected .tsx/.ts next to the scaffold).",
                ts_file.display(),
                target.display()
            );
        }
    }
    Ok(())
}

/// Fail fast when generated `routes.manifest.ts` or `App.tsx` references missing `./` modules.
fn verify_app_tsx_route_imports(out_dir: &Path) -> Result<()> {
    let manifest = out_dir.join("routes.manifest.ts");
    if manifest.is_file() {
        verify_ts_relative_imports_from_file(&manifest)?;
    }
    let app_path = out_dir.join("App.tsx");
    if app_path.is_file() {
        verify_ts_relative_imports_from_file(&app_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod route_import_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn verify_app_tsx_route_imports_ok_when_all_exist() -> Result<()> {
        let dir = tempdir().context("tempdir for route import test")?;
        let root = dir.path();
        fs::write(
            root.join("App.tsx"),
            "import { Chat } from \"./Chat.tsx\";\n",
        )
        .with_context(|| format!("write {}", root.join("App.tsx").display()))?;
        fs::write(root.join("Chat.tsx"), "export function Chat() {}\n")
            .with_context(|| format!("write {}", root.join("Chat.tsx").display()))?;
        verify_app_tsx_route_imports(root)?;
        Ok(())
    }

    #[test]
    fn verify_app_tsx_route_imports_errors_on_missing() -> Result<()> {
        let dir = tempdir().context("tempdir for missing import test")?;
        let root = dir.path();
        fs::write(
            root.join("App.tsx"),
            "import { Missing } from \"./Missing.tsx\";\n",
        )
        .with_context(|| format!("write {}", root.join("App.tsx").display()))?;
        let err = verify_app_tsx_route_imports(root)
            .err()
            .context("expected verify_app_tsx_route_imports to fail on missing file")?;
        assert!(
            err.to_string().contains("Missing.tsx"),
            "expected missing file in error: {err}"
        );
        Ok(())
    }

    #[test]
    fn verify_app_src_generated_imports_main_tsx_ok() -> Result<()> {
        let dir = tempdir().context("tempdir for main.tsx import test")?;
        let src = dir.path().join("src");
        fs::create_dir_all(src.join("generated"))
            .with_context(|| format!("create {}", src.join("generated").display()))?;
        fs::write(
            src.join("generated/Home.tsx"),
            "export function Home() {}\n",
        )
        .with_context(|| format!("write {}", src.join("generated/Home.tsx").display()))?;
        fs::write(
            src.join("main.tsx"),
            "import { Home } from \"./generated/Home\";\n",
        )
        .with_context(|| format!("write {}", src.join("main.tsx").display()))?;
        super::verify_app_src_generated_imports(&src)?;
        Ok(())
    }

    #[test]
    fn verify_app_src_generated_imports_routes_index_parent_generated_ok() -> Result<()> {
        let dir = tempdir().context("tempdir for routes/index import test")?;
        let src = dir.path().join("src");
        fs::create_dir_all(src.join("routes"))
            .with_context(|| format!("create {}", src.join("routes").display()))?;
        fs::create_dir_all(src.join("generated"))
            .with_context(|| format!("create {}", src.join("generated").display()))?;
        fs::write(src.join("generated/App.tsx"), "export function App() {}\n")
            .with_context(|| format!("write {}", src.join("generated/App.tsx").display()))?;
        fs::write(
            src.join("routes/index.tsx"),
            "import App from \"../generated/App\";\n",
        )
        .with_context(|| format!("write {}", src.join("routes/index.tsx").display()))?;
        super::verify_app_src_generated_imports(&src)?;
        Ok(())
    }
}
