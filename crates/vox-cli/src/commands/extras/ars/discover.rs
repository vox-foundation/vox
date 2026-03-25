use anyhow::Result;
use std::collections::HashSet;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

use super::registry::make_registry;

pub async fn discover() -> Result<()> {
    use owo_colors::OwoColorize;

    let registry = make_registry().await;
    let installed: HashSet<String> = registry.list(None).into_iter().map(|s| s.id).collect();

    let workspace_root = std::env::current_dir().unwrap_or_default();
    let mut found: Vec<(std::path::PathBuf, String)> = Vec::new();

    walk_for_skills(&workspace_root, 0, 6, &mut found);

    if found.is_empty() {
        println!("{}", "No .skill.md files found in the workspace.".dimmed());
        println!("  Create one with: {}", "vox skill create <name>".cyan());
        return Ok(());
    }

    let new_count = found
        .iter()
        .filter(|(_, id)| !installed.contains(id))
        .count();
    println!(
        "\n{} Found {} skill file(s) ({} not yet installed)\n",
        "🔍".bold(),
        found.len(),
        new_count
    );

    for (path, id) in &found {
        let is_installed = installed.contains(id);
        let rel = path.strip_prefix(&workspace_root).unwrap_or(path);
        if is_installed {
            println!(
                "  {} {:<32} [{}]",
                "✅".green(),
                id.dimmed(),
                rel.display().to_string().dimmed()
            );
        } else {
            println!(
                "  {} {:<32}  {} {}",
                "📦".yellow(),
                id.yellow(),
                rel.display().to_string().dimmed(),
                "← not installed".bright_yellow()
            );
            println!(
                "     {} vox skill install {}",
                "→".dimmed(),
                rel.display().to_string().cyan()
            );
        }
    }

    if new_count > 0 {
        println!(
            "\nInstall all: {}",
            "for f in $(find . -name '*.skill.md'); do vox skill install $f; done".cyan()
        );
    }

    Ok(())
}

fn walk_for_skills(
    dir: &std::path::Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<(std::path::PathBuf, String)>,
) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && (name.starts_with('.') || name == "target" || name == "node_modules")
        {
            continue;
        }
        if path.is_dir() {
            walk_for_skills(&path, depth + 1, max_depth, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if fname.ends_with(".skill.md") {
                if let Ok(content) = read_utf8_path_capped(&path) {
                    let id = extract_skill_id(&content)
                        .unwrap_or_else(|| fname.trim_end_matches(".skill.md").to_string());
                    out.push((path, id));
                }
            }
        }
    }
}

fn extract_skill_id(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("id") {
            if let Some(rest) = trimmed.strip_prefix("id").map(|s| s.trim())
                && let Some(rest) = rest.strip_prefix('=')
            {
                let val = rest.trim().trim_matches('"').trim_matches('\'').to_string();
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
    }
    None
}
