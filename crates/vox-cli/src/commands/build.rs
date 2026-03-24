//! `vox build` — full compile pipeline and artifact layout.
//!
//! Writes **TypeScript** into `out_dir` and **Rust** under `target/generated/` (Axum-style backend).
//! Emits `api.ts` when server functions produce a client. `@v0` declarations trigger optional
//! v0.dev generation when `V0_API_KEY` is set — see `crate::v0::generate_component`.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Run the build pipeline for `file`, writing TS to `out_dir` and Rust to `target/generated`.
pub async fn run(file: &Path, out_dir: &Path) -> Result<()> {
    let source = fs::read_to_string(file)
        .with_context(|| format!("Failed to read source file: {}", file.display()))?;

    // 1. Lex
    let tokens = vox_compiler::lexer::lex(&source);
    tracing::info!("Lexed {} tokens", tokens.len());

    // 2. Parse
    let module = vox_compiler::parser::parser::parse(tokens).map_err(|errors| {
        for e in &errors {
            eprintln!("Parse error: {} at {:?}", e.message, e.span);
        }
        anyhow::anyhow!("Parsing failed with {} error(s)", errors.len())
    })?;
    tracing::info!("Parsed {} declarations", module.declarations.len());

    // 3. Type check (HIR)
    let diagnostics = vox_compiler::typeck::typecheck_ast_module(&source, &module);
    let has_errors = diagnostics
        .iter()
        .any(|d| d.severity == vox_compiler::typeck::diagnostics::Severity::Error);
    for d in &diagnostics {
        match d.severity {
            vox_compiler::typeck::diagnostics::Severity::Error => {
                eprintln!("error: {} at {:?}", d.message, d.span)
            }
            vox_compiler::typeck::diagnostics::Severity::Warning => {
                eprintln!("warning: {} at {:?}", d.message, d.span)
            }
        }
    }
    if has_errors {
        anyhow::bail!("Type checking failed");
    }
    tracing::info!("Type checking passed");

    // 4. Lower to HIR (reuse for codegen)
    let hir = vox_compiler::hir::lower_module(&module);

    // 5. Generate TypeScript (Frontend)
    let ts_opts = vox_compiler::codegen_ts::CodegenOptions {
        tanstack_start: vox_config::VoxConfig::load().web_tanstack_start,
    };
    let ts_output = vox_compiler::codegen_ts::generate_with_options(&module, ts_opts)
        .map_err(|e| anyhow::anyhow!("TypeScript code generation failed: {e}"))?;

    // 6. Generate Rust (Backend)
    let rust_output = vox_compiler::codegen_rust::generate(&hir, "vox_generated_app")
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

    let emitted_vox_router = ts_output
        .files
        .iter()
        .any(|(n, _)| n == "VoxTanStackRouter.tsx");
    let emitted_app_tsx = ts_output
        .files
        .iter()
        .any(|(n, _)| n == "App.tsx");
    if emitted_vox_router {
        let stale = out_dir.join("App.tsx");
        if stale.is_file() {
            fs::remove_file(&stale)
                .with_context(|| format!("Failed to remove stale {}", stale.display()))?;
            println!("  removed stale {}", stale.display());
        }
    }
    if emitted_app_tsx {
        let stale = out_dir.join("VoxTanStackRouter.tsx");
        if stale.is_file() {
            fs::remove_file(&stale)
                .with_context(|| format!("Failed to remove stale {}", stale.display()))?;
            println!("  removed stale {}", stale.display());
        }
    }

    // 8. Handle @v0 components
    // We iterate over the parsed declarations to find V0Components
    for decl in &module.declarations {
        if let vox_compiler::ast::decl::Decl::V0Component(comp) = decl {
            let component_name = &comp.name;
            let filename = format!("{}.tsx", component_name);
            let target_path = out_dir.join(&filename);

            // Only generate if file doesn't exist to avoid overwriting edits
            if !target_path.exists() {
                println!("Generating v0 component '{}'...", component_name);

                // Determine prompt and optional image path
                let (prompt, image_path) = if !comp.prompt.is_empty() {
                    (comp.prompt.clone(), None)
                } else if let Some(img_str) = &comp.image_path {
                    let parent = file.parent().unwrap_or(Path::new("."));
                    let path = parent.join(img_str);
                    (
                        "Create a component based on the provided image.".to_string(),
                        Some(path),
                    )
                } else {
                    ("Create a React component".to_string(), None)
                };

                match crate::v0::generate_component(
                    &prompt,
                    component_name,
                    out_dir,
                    image_path.as_deref(),
                )
                .await
                {
                    Ok(path) => println!("  generated v0 component: {}", path.display()),
                    Err(e) => eprintln!(
                        "  failed to generate v0 component '{}': {}",
                        component_name, e
                    ),
                }
            } else {
                println!("  skipping v0 component '{}' (file exists)", component_name);
            }
        }
    }

    verify_app_tsx_route_imports(out_dir).context("App.tsx route import graph")?;

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
    let content = fs::read_to_string(ts_file)
        .with_context(|| format!("Failed to read {}", ts_file.display()))?;
    let re = regex::Regex::new(r#"from\s+["']([^"']+)["']"#)
        .expect("static regex for TS imports");
    for cap in re.captures_iter(&content) {
        let raw = cap.get(1).expect("capture group 1").as_str();
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

/// Fail fast when `App.tsx` references `./Component.tsx` (or `.ts`) that is missing from `out_dir`.
fn verify_app_tsx_route_imports(out_dir: &Path) -> Result<()> {
    let app_path = out_dir.join("App.tsx");
    if !app_path.is_file() {
        return Ok(());
    }
    let content = fs::read_to_string(&app_path)
        .with_context(|| format!("Failed to read {}", app_path.display()))?;
    let re = regex::Regex::new(r#"from\s+["']\./([^"']+)["']"#)
        .expect("static regex for TS relative imports");
    for cap in re.captures_iter(&content) {
        let rel = cap.get(1).expect("capture group 1").as_str();
        let target = out_dir.join(rel);
        if !target.is_file() {
            anyhow::bail!(
                "App.tsx imports `{rel}` but that file was not found under {} (fix routes: targets or emit the component).",
                out_dir.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod route_import_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn verify_app_tsx_route_imports_ok_when_all_exist() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(
            root.join("App.tsx"),
            "import { Chat } from \"./Chat.tsx\";\n",
        )
        .unwrap();
        fs::write(root.join("Chat.tsx"), "export function Chat() {}\n").unwrap();
        verify_app_tsx_route_imports(root).unwrap();
    }

    #[test]
    fn verify_app_tsx_route_imports_errors_on_missing() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(
            root.join("App.tsx"),
            "import { Missing } from \"./Missing.tsx\";\n",
        )
        .unwrap();
        let err = verify_app_tsx_route_imports(root).unwrap_err();
        assert!(
            err.to_string().contains("Missing.tsx"),
            "expected missing file in error: {err}"
        );
    }

    #[test]
    fn verify_app_src_generated_imports_main_tsx_ok() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(src.join("generated")).unwrap();
        fs::write(src.join("generated/Home.tsx"), "export function Home() {}\n").unwrap();
        fs::write(
            src.join("main.tsx"),
            "import { Home } from \"./generated/Home\";\n",
        )
        .unwrap();
        super::verify_app_src_generated_imports(&src).unwrap();
    }

    #[test]
    fn verify_app_src_generated_imports_routes_index_parent_generated_ok() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(src.join("routes")).unwrap();
        fs::create_dir_all(src.join("generated")).unwrap();
        fs::write(src.join("generated/App.tsx"), "export function App() {}\n").unwrap();
        fs::write(
            src.join("routes/index.tsx"),
            "import App from \"../generated/App\";\n",
        )
        .unwrap();
        super::verify_app_src_generated_imports(&src).unwrap();
    }
}
