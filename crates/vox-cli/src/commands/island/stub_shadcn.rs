use anyhow::{Context, Result};

use crate::commands::ci::bounded_read::read_utf8_path_capped_async;
use std::path::Path;

use crate::island_paths::island_root;

use super::build::bootstrap_islands_if_needed;

pub(super) async fn inject_or_update_island_stub(
    vox_file: &Path,
    name: &str,
    island_block: &str,
) -> Result<()> {
    // ZERO DESTRUCTION: read before write
    let existing = if tokio::fs::metadata(vox_file).await.is_ok() {
        read_utf8_path_capped_async(vox_file)
            .await
            .with_context(|| format!("Cannot read {}", vox_file.display()))?
    } else {
        String::new()
    };

    let marker = format!("@island {name}:");
    let new_content = if let Some(start_idx) = existing.find(&marker) {
        let before = &existing[..start_idx];
        let after_block = &existing[start_idx..];
        let block_end = find_block_end(after_block);
        let after = &existing[start_idx + block_end..];
        format!("{before}{island_block}\n{after}")
    } else {
        let trimmed = existing.trim_end();
        if trimmed.is_empty() {
            format!("{island_block}\n")
        } else {
            format!("{trimmed}\n\n{island_block}\n")
        }
    };

    tokio::fs::write(vox_file, new_content)
        .await
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

pub(super) async fn add_shadcn(
    component: &str,
    root: &Path,
    from_file: Option<&str>,
) -> Result<()> {
    bootstrap_islands_if_needed(root)?;
    let islands_dir = island_root(root);

    let components_json = islands_dir.join("components.json");
    if tokio::fs::metadata(&components_json).await.is_err() {
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
        tokio::fs::write(&components_json, content).await?;

        let globals_css = islands_dir.join("src").join("globals.css");
        if tokio::fs::metadata(&globals_css).await.is_err() {
            if let Some(parent) = globals_css.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let _ = tokio::fs::write(
                &globals_css,
                "@tailwind base;\n@tailwind components;\n@tailwind utilities;\n",
            )
            .await;
        }

        let utils_ts = islands_dir.join("src").join("lib").join("utils.ts");
        if tokio::fs::metadata(&utils_ts).await.is_err() {
            if let Some(parent) = utils_ts.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let _ = tokio::fs::write(
                &utils_ts,
                r#"import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
"#,
            )
            .await;
        }
    }

    println!("📦 Adding ShadCN component: {}...", component);
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

    if let Some(vox_path) = from_file {
        let path = Path::new(vox_path);
        if tokio::fs::metadata(path).await.is_ok() {
            let alias = shadcn_import_alias(component);
            let import_line = format!("@shadcn \"{component}\" as {alias}");
            let existing = read_utf8_path_capped_async(path).await?;
            if !existing.contains(&import_line) {
                let mut new_content = existing.clone();
                if !new_content.is_empty() && !new_content.ends_with('\n') {
                    new_content.push('\n');
                }
                new_content.push_str(&import_line);
                new_content.push('\n');
                tokio::fs::write(path, new_content).await?;
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
