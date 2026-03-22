//! `vox island` — generate, upgrade, list, and cache v0.dev React islands.
//!
//! Entry point: [`run`]. Dispatches to the four action handlers:
//! * [`generate`] — call v0 API, write TSX, emit Vox stub, optionally build.
//! * [`upgrade`] — re-generate with existing code as context.
//! * [`list_islands`] — scan `islands/src/` and `Vox.toml`.
//! * [`handle_cache`] — list / clear / remove cache entries.

use anyhow::{Context, Result, anyhow};
use std::path::Path;

use crate::cli_actions::{IslandCacheAction, IslandCli};
use crate::frontend;
use crate::island_paths::{island_root, island_src_dir, resolve_island_main_tsx};
use crate::templates;
use crate::v0::{self, IslandCache};

/// Dispatch `vox island <subcommand>`.
pub async fn run(cmd: IslandCli) -> Result<()> {
    let project_root = std::env::current_dir()
        .context("Cannot determine project root — is the current directory accessible?")?;

    match cmd {
        IslandCli::Generate {
            name,
            prompt,
            target,
            force,
            no_build,
            image,
        } => {
            generate(
                &name,
                &prompt,
                &project_root,
                target.as_deref(),
                force,
                no_build,
                image.as_deref(),
            )
            .await
        }
        IslandCli::Upgrade {
            name,
            prompt,
            no_build,
        } => upgrade(&name, &prompt, &project_root, no_build).await,
        IslandCli::List { json } => list_islands(&project_root, json),
        IslandCli::Add { component, from } => {
            add_shadcn(&component, &project_root, from.as_deref()).await
        }
        IslandCli::Cache { action } => handle_cache(action),
    }
}

// ── generate ─────────────────────────────────────────────────────────────────

/// Generate a new island from a v0.dev prompt.
///
/// Pipeline:
/// 1. Validate `name` is CamelCase.
/// 2. Call v0 API (or restore from cache).
/// 3. Infer Vox prop types from generated TSX.
/// 4. Inject `@island` stub into `target` .vox file, or print to stdout.
/// 5. Run **`pnpm run build`** in **`islands/`** (unless `--no-build`).
async fn generate(
    name: &str,
    prompt: &str,
    root: &Path,
    target: Option<&Path>,
    force: bool,
    no_build: bool,
    image: Option<&Path>,
) -> Result<()> {
    // Guard: name must start with uppercase (CamelCase)
    if !name
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        anyhow::bail!("Island name must be CamelCase (e.g. AgentStatusBadge). Got: '{name}'");
    }

    bootstrap_islands_if_needed(root)?;

    // 1. Generate TSX (cache-aware)
    let tsx_path = v0::generate_island_tsx(prompt, name, root, image, force).await?;

    // 2. Emit @island stub from inferred prop types
    let tsx = std::fs::read_to_string(&tsx_path)
        .with_context(|| format!("Cannot read generated TSX: {}", tsx_path.display()))?;
    let stub = v0::emit_island_stub(&tsx, name, target);

    // 3. Write stub to target .vox file or print for manual integration
    if let Some(vox_file) = target {
        inject_or_update_island_stub(vox_file, name, &stub)?;
        println!("📝 Updated {}", vox_file.display());
    } else {
        println!("\n── @island stub ─────────────────────────────────────────");
        println!("{stub}");
        println!("─────────────────────────────────────────────────────────");
        println!("💡 Paste the stub above into your .vox file, or use:");
        println!("   vox island generate {name} -p '...' --target <file.vox>");
    }

    // 4. Optional pnpm build
    if !no_build {
        build_islands(root).await?;
    }

    println!("\n✅  Island '{name}' ready. Mount it in Vox with:");
    println!("    <{name}[island] ...props... />");

    Ok(())
}

// ── upgrade ───────────────────────────────────────────────────────────────────

/// Upgrade an existing island by providing its current TSX as context alongside new instructions.
///
/// Always bypasses the cache so the upgraded version is always a fresh API call.
async fn upgrade(name: &str, prompt: &str, root: &Path, no_build: bool) -> Result<()> {
    bootstrap_islands_if_needed(root)?;
    let tsx_path = resolve_island_main_tsx(root, name)?;

    let existing_tsx = std::fs::read_to_string(&tsx_path)
        .with_context(|| format!("Cannot read existing island: {}", tsx_path.display()))?;

    // Build a prompt that includes the existing code as context
    let upgrade_prompt = format!(
        "Upgrade the following React island component while preserving all existing prop types.\n\
        Upgrade instructions: {prompt}\n\n\
        EXISTING CODE TO UPGRADE:\n\
        ```tsx\n\
        {existing_tsx}\n\
        ```"
    );

    // Force-regenerate (bypass cache for upgrades by definition)
    v0::generate_island_tsx(&upgrade_prompt, name, root, None, true).await?;

    if !no_build {
        build_islands(root).await?;
    }
    println!("✅  Island '{name}' upgraded.");
    Ok(())
}

// ── list_islands ──────────────────────────────────────────────────────────────

/// List all islands: scans `islands/src/` for `.tsx` files and directories with `index.tsx`,
/// then merges with any entries from `Vox.toml [islands]`.
fn list_islands(root: &Path, json: bool) -> Result<()> {
    let src_dir = island_src_dir(root);
    let mut found: Vec<(String, String)> = vec![];

    // Scan islands/src/
    if src_dir.exists() {
        for entry in std::fs::read_dir(&src_dir)
            .with_context(|| format!("Cannot read {}", src_dir.display()))?
        {
            let e = entry?;
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) == Some("tsx") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    found.push((stem.to_string(), format!("islands/src/{stem}.tsx")));
                }
            } else if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if path.join(format!("{name}.component.tsx")).exists()
                        || path.join("index.tsx").exists()
                    {
                        found.push((name.to_string(), format!("islands/src/{name}/")));
                    }
                }
            }
        }
    }

    // Merge Vox.toml [islands] (adds any registered names not found in src/)
    let vox_toml = root.join("Vox.toml");
    if vox_toml.exists() {
        let content = std::fs::read_to_string(&vox_toml).context("Failed to read Vox.toml")?;
        let mut in_section = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "[islands]" {
                in_section = true;
                continue;
            }
            if in_section {
                if trimmed.starts_with('[') {
                    break;
                }
                if !trimmed.is_empty()
                    && !trimmed.starts_with('#')
                    && let Some((name, _)) = trimmed.split_once('=')
                {
                    let name = name.trim().to_string();
                    if !found.iter().any(|(n, _)| n == &name) {
                        found.push((name, "Vox.toml".to_string()));
                    }
                }
            }
        }
    }

    // Deduplicate and sort by name
    found.sort_by(|a, b| a.0.cmp(&b.0));

    if json {
        let names: Vec<&str> = found.iter().map(|(n, _)| n.as_str()).collect();
        println!("{}", serde_json::json!({ "islands": names }));
    } else if found.is_empty() {
        crate::diagnostics::print_info(
            "No islands found. Generate one with:\n  \
            vox island generate MyComponent --prompt 'A dark card showing…'",
        );
    } else {
        let mut table = crate::table::OutputTable::new(&["Island", "Source"]);
        for (name, src) in &found {
            table.add_row(vec![name.clone(), src.clone()]);
        }
        table.print();
    }
    Ok(())
}

// ── handle_cache ──────────────────────────────────────────────────────────────

fn handle_cache(action: IslandCacheAction) -> Result<()> {
    let cache = IslandCache::new()?;
    match action {
        IslandCacheAction::List => {
            let entries = cache.list()?;
            if entries.is_empty() {
                println!("Island cache is empty (~/.vox/island-cache/).");
            } else {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                println!("{:<24} {:<58} {}", "Island", "Prompt (preview)", "Age");
                for m in entries {
                    let age_hrs = (now.saturating_sub(m.generated_at)) / 3600;
                    let prompt_preview: String = m.prompt.chars().take(55).collect();
                    let prompt_display = if m.prompt.len() > 55 {
                        format!("{prompt_preview}…")
                    } else {
                        prompt_preview
                    };
                    println!("{:<24} {:<58} {}h ago", m.name, prompt_display, age_hrs);
                }
            }
        }
        IslandCacheAction::Clear => {
            let n = cache.clear()?;
            println!("Cleared {n} cached island(s).");
        }
        IslandCacheAction::Remove { name } => {
            let n = cache.remove(&name)?;
            if n == 0 {
                println!("No cache entry found for '{name}'.");
            } else {
                println!("Removed cache entry for '{name}'.");
            }
        }
    }
    Ok(())
}

// ── bootstrap ─────────────────────────────────────────────────────────────────

/// Ensure a minimal **`islands/`** Vite + React tree exists (first `vox island generate` / `add`).
///
/// Skips when **`islands/package.json`** is already present so user customizations are preserved.
fn bootstrap_islands_if_needed(root: &Path) -> Result<()> {
    let islands_dir = island_root(root);
    let pkg = islands_dir.join("package.json");
    if pkg.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(island_src_dir(root))?;
    std::fs::write(&pkg, templates::islands_package_json())?;
    std::fs::write(
        islands_dir.join("vite.config.ts"),
        templates::islands_vite_config(),
    )?;
    std::fs::write(
        islands_dir.join("index.html"),
        templates::islands_index_html(),
    )?;
    std::fs::write(
        island_src_dir(root).join("main.tsx"),
        templates::islands_main_tsx(),
    )?;
    std::fs::write(
        island_src_dir(root).join("island-mount.tsx"),
        templates::islands_island_mount_tsx(),
    )?;
    std::fs::write(
        islands_dir.join("tsconfig.json"),
        templates::tsconfig_json(),
    )?;
    crate::diagnostics::print_info(
        "Bootstrapped minimal islands/ (Vite + React, pnpm). Dependencies install on first build.",
    );
    Ok(())
}

// ── build_islands ─────────────────────────────────────────────────────────────

/// Run **`pnpm run build`** in **`islands/`**, with fingerprint-based skip for warm builds.
///
/// Installs dependencies via **`pnpm install --prefer-offline`** when `node_modules` is absent
/// or stale relative to **`package.json`** / **`pnpm-lock.yaml`**.
pub async fn build_islands(root: &Path) -> Result<()> {
    bootstrap_islands_if_needed(root)?;

    let islands_dir = island_root(root);
    which::which(frontend::pnpm_executable()).map_err(|_| {
        anyhow!(
            "pnpm not found in PATH. Install pnpm (https://pnpm.io/) to build islands; Node.js required."
        )
    })?;

    let nm = islands_dir.join("node_modules");
    let lock = islands_dir.join("pnpm-lock.yaml");
    let pkg = islands_dir.join("package.json");
    let nm_mtime = nm.metadata().and_then(|m| m.modified()).ok();
    let needs_install = !nm.exists()
        || (lock.exists() && lock.metadata().and_then(|m| m.modified()).ok() > nm_mtime)
        || (pkg.exists() && pkg.metadata().and_then(|m| m.modified()).ok() > nm_mtime);

    if needs_install {
        println!("📦 Installing island dependencies (pnpm)…");
        let status = tokio::process::Command::new(frontend::pnpm_executable())
            .args(["install", "--prefer-offline"])
            .current_dir(&islands_dir)
            .status()
            .await
            .context("Failed to run pnpm install in islands/")?;
        if !status.success() {
            anyhow::bail!("pnpm install failed in {}", islands_dir.display());
        }
    }

    // Fingerprint-based skip: skip build if no island source is newer than the last build
    if !needs_island_rebuild(root)? {
        println!("⚡ Islands are up-to-date, skipping build.");
        return Ok(());
    }

    println!("🔨 Building islands with Vite (pnpm)…");

    let status = tokio::process::Command::new(frontend::pnpm_executable())
        .args(["run", "build"])
        .current_dir(&islands_dir)
        .status()
        .await
        .context("Failed to run pnpm run build in islands/")?;
    if !status.success() {
        anyhow::bail!("pnpm run build failed in {}", islands_dir.display());
    }

    write_island_fingerprint(root)?;
    println!("✅ Islands built.");
    Ok(())
}

/// Returns `true` if any file under `islands/src/` is newer than the fingerprint marker.
fn needs_island_rebuild(root: &Path) -> Result<bool> {
    let fp = root.join(".vox-build-cache").join("islands.fingerprint");
    if !fp.exists() {
        return Ok(true);
    }
    let fp_mtime = fp.metadata()?.modified()?;
    let islands_dir = island_root(root);
    for marker in [
        islands_dir.join("package.json"),
        islands_dir.join("vite.config.ts"),
        islands_dir.join("tsconfig.json"),
        islands_dir.join("pnpm-lock.yaml"),
    ] {
        if marker.exists() && marker.metadata()?.modified()? > fp_mtime {
            return Ok(true);
        }
    }
    let src_dir = island_src_dir(root);
    if !src_dir.exists() {
        return Ok(false);
    }
    for entry in walkdir::WalkDir::new(&src_dir) {
        let e = entry?;
        if e.file_type().is_file() && e.metadata()?.modified()? > fp_mtime {
            return Ok(true);
        }
    }
    Ok(false)
}

fn write_island_fingerprint(root: &Path) -> Result<()> {
    let cache_dir = root.join(".vox-build-cache");
    std::fs::create_dir_all(&cache_dir)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    std::fs::write(cache_dir.join("islands.fingerprint"), ts.to_string())?;
    Ok(())
}

// ── inject_or_update_island_stub ──────────────────────────────────────────────

/// Inject or replace an `@island <Name>:` block in an existing `.vox` file.
///
/// Strategy:
/// - If `@island <Name>:` already exists in the file: replace the entire block.
/// - Otherwise: append the stub to the end of the file.
///
/// Zero-destruction: reads the file before writing. Never truncates other content.
fn inject_or_update_island_stub(vox_file: &Path, name: &str, stub: &str) -> Result<()> {
    // ZERO DESTRUCTION: read before write
    let existing = if vox_file.exists() {
        std::fs::read_to_string(vox_file)
            .with_context(|| format!("Cannot read {}", vox_file.display()))?
    } else {
        String::new()
    };

    let marker = format!("@island {name}:");
    let new_content = if let Some(start_idx) = existing.find(&marker) {
        let before = &existing[..start_idx];
        let after_block = &existing[start_idx..];
        // Find where this block ends: the next top-level declaration or EOF
        let block_end = find_block_end(after_block);
        let after = &existing[start_idx + block_end..];
        format!("{before}{stub}\n{after}")
    } else {
        // Append to file
        let trimmed = existing.trim_end();
        if trimmed.is_empty() {
            format!("{stub}\n")
        } else {
            format!("{trimmed}\n\n{stub}\n")
        }
    };

    std::fs::write(vox_file, new_content)
        .with_context(|| format!("Cannot write {}", vox_file.display()))?;
    Ok(())
}

/// Find the byte offset within `text` where the topmost `@island` block ends.
///
/// A block ends at the start of the next top-level Vox declaration keyword
/// (`@island`, `@page`, `@layout`, `@theme`, `@keyframes`)
/// or at the end of the string.
fn find_block_end(text: &str) -> usize {
    let terminators = ["@island ", "@page ", "@layout ", "@theme ", "@keyframes "];
    // Skip the first line (the `@island Name:` line itself)
    let after_first_newline = text.find('\n').map(|i| i + 1).unwrap_or(text.len());
    let rest = &text[after_first_newline..];
    for term in &terminators {
        if let Some(pos) = rest.find(term) {
            return after_first_newline + pos;
        }
    }
    text.len()
}

/// ShadCN registry names are often kebab-case; Vox `@shadcn` aliases should be PascalCase.
fn shadcn_import_alias(component: &str) -> String {
    component
        .split('-')
        .map(|part| {
            let mut ch = part.chars();
            match ch.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + ch.as_str(),
            }
        })
        .collect()
}

async fn add_shadcn(component: &str, root: &Path, from_file: Option<&str>) -> Result<()> {
    bootstrap_islands_if_needed(root)?;
    let islands_dir = island_root(root);

    // 1. Ensure components.json exists (init if not)
    let components_json = islands_dir.join("components.json");
    if !components_json.exists() {
        println!("🚀 Initializing ShadCN in islands/...");
        let content = r#"{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "new-york",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "tailwind.config.ts",
    "css": "src/globals.css",
    "baseColor": "zinc",
    "cssVariables": true,
    "prefix": ""
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils"
  }
}
"#;
        std::fs::write(&components_json, content)?;

        // Ensure globals.css exists
        let globals_css = islands_dir.join("src").join("globals.css");
        if !globals_css.exists() {
            std::fs::create_dir_all(globals_css.parent().unwrap())?;
            let _ = std::fs::write(
                &globals_css,
                "@tailwind base;\n@tailwind components;\n@tailwind utilities;\n",
            );
        }

        // Ensure lib/utils.ts exists
        let utils_ts = islands_dir.join("src").join("lib").join("utils.ts");
        if !utils_ts.exists() {
            std::fs::create_dir_all(utils_ts.parent().unwrap())?;
            let _ = std::fs::write(
                &utils_ts,
                r#"import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
"#,
            );
        }
    }

    // 2. Add component
    println!("📦 Adding ShadCN component: {}...", component);
    // Use shell for npx on Windows
    let mut cmd = if cfg!(windows) {
        let mut c = tokio::process::Command::new("cmd");
        c.args(["/C", "npx", "shadcn@latest", "add", component, "-y"]);
        c
    } else {
        let mut c = tokio::process::Command::new("npx");
        c.args(["shadcn@latest", "add", component, "-y"]);
        c
    };

    let status = cmd
        .current_dir(&islands_dir)
        .status()
        .await
        .context("Failed to run npx shadcn")?;

    if !status.success() {
        anyhow::bail!("Failed to add ShadCN component '{}'.", component);
    }

    // 3. Optional: inject @shadcn import into .vox file
    if let Some(vox_path) = from_file {
        let path = Path::new(vox_path);
        if path.exists() {
            let alias = shadcn_import_alias(component);
            let import_line = format!("@shadcn \"{component}\" as {alias}");
            let existing = std::fs::read_to_string(path)?;
            if !existing.contains(&import_line) {
                let mut new_content = existing.clone();
                if !new_content.is_empty() && !new_content.ends_with('\n') {
                    new_content.push('\n');
                }
                new_content.push_str(&import_line);
                new_content.push('\n');
                std::fs::write(path, new_content)?;
                println!("📝 Injected `{}` into {}", import_line, vox_path);
            }
        }
    }

    println!("✅ Added {} to islands/src/components/ui/", component);
    Ok(())
}

#[cfg(test)]
mod shadcn_alias_tests {
    use super::shadcn_import_alias;

    #[test]
    fn kebab_case_maps_to_pascal_case() {
        assert_eq!(shadcn_import_alias("dropdown-menu"), "DropdownMenu");
        assert_eq!(shadcn_import_alias("button"), "Button");
    }
}
