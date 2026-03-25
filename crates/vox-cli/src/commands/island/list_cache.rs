use anyhow::{Context, Result};
use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped;
use crate::cli_actions::IslandCacheAction;
use crate::island_paths::island_src_dir;
use crate::v0::IslandCache;

/// List all islands: scans `islands/src/` for `.tsx` files and directories with `index.tsx`,
/// then merges with any entries from `Vox.toml [islands]`.
pub(super) fn list_islands(root: &Path, json: bool) -> Result<()> {
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
        let content = read_utf8_path_capped(&vox_toml).context("Failed to read Vox.toml")?;
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

pub(super) fn handle_cache(action: IslandCacheAction) -> Result<()> {
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
