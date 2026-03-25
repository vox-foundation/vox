use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use regex::Regex;
use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

pub fn run(repo_root: &Path) -> Result<()> {
    let docs_src = repo_root.join("docs").join("src");
    if !docs_src.exists() {
        println!(
            "{} Docs source not found at {:?}",
            "SKIP".yellow(),
            docs_src
        );
        return Ok(());
    }

    println!(
        "{} Checking internal links in {:?}...",
        "INIT".bright_blue(),
        docs_src.strip_prefix(repo_root).unwrap_or(&docs_src)
    );

    let mut total_links = 0;
    let mut broken_links = Vec::new();

    // Match [text](target) where target is not external or anchor-only
    let link_re = Regex::new(r"\[[^\]]+\]\(([^)]+)\)").unwrap();

    use walkdir::WalkDir;

    for entry in WalkDir::new(&docs_src).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "md") {
            let path = entry.path();
            let content =
                read_utf8_path_capped(path).with_context(|| format!("Failed to read {:?}", path))?;

            let parent_dir = path.parent().unwrap();

            for cap in link_re.captures_iter(&content) {
                let target_full = &cap[1];

                // Skip external links and local anchors
                if target_full.starts_with("http")
                    || target_full.starts_with("#")
                    || target_full.starts_with("mailto:")
                {
                    continue;
                }

                total_links += 1;

                // Remove anchor part
                let target_path_str = target_full.split('#').next().unwrap_or(target_full);
                if target_path_str.is_empty() {
                    continue;
                }

                // Resolve path
                let target_path = if target_path_str.starts_with("/") {
                    // Treat as root-relative (relative to repo root)
                    repo_root.join(target_path_str.trim_start_matches('/'))
                } else {
                    parent_dir.join(target_path_str)
                };

                // Normalize and check existence
                if !target_path.exists() {
                    broken_links.push((path.to_path_buf(), target_full.to_string(), target_path));
                }
            }
        }
    }

    println!("{} Checked {} internal links.", "DONE".green(), total_links);

    if broken_links.is_empty() {
        println!("{} All internal links are valid!", "PASS".green().bold());
        Ok(())
    } else {
        println!(
            "{} Found {} broken links:",
            "FAIL".red().bold(),
            broken_links.len()
        );
        for (source, target_str, resolved) in &broken_links {
            let rel_source = source.strip_prefix(repo_root).unwrap_or(source);
            println!(
                "  {} -> {} (resolved: {:?})",
                rel_source.display().cyan(),
                target_str.yellow(),
                resolved.display().dimmed()
            );
        }
        Err(anyhow::anyhow!("Documentation link check failed"))
    }
}
