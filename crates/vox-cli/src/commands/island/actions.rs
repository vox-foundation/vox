use anyhow::{Context, Result};
use std::path::Path;

use crate::island_paths::resolve_island_main_tsx;
use crate::v0;

use super::build::{bootstrap_islands_if_needed, build_islands};
use super::stub_shadcn::inject_or_update_island_stub;
/// Generate a new island from a v0.dev prompt.
///
/// Pipeline:
/// 1. Validate `name` is CamelCase.
/// 2. Call v0 API (or restore from cache).
/// 3. Infer Vox prop types from generated TSX.
/// 4. Inject `@island` stub into `target` .vox file, or print to stdout.
/// 5. Run **`pnpm run build`** in **`islands/`** (unless `--no-build`).
pub(super) async fn generate(
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

/// Upgrade an existing island by providing its current TSX as context alongside new instructions.
///
/// Always bypasses the cache so the upgraded version is always a fresh API call.
pub(super) async fn upgrade(name: &str, prompt: &str, root: &Path, no_build: bool) -> Result<()> {
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
