use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use regex::Regex;
use std::path::Path;
use walkdir::WalkDir;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

pub fn run(repo_root: &Path, target: Option<&Path>) -> Result<()> {
    let docs_src = repo_root.join("docs").join("src");
    let docs_dir = repo_root.join("docs");

    println!(
        "{} Checking internal links in docs and root guides...",
        "INIT".bright_blue(),
    );

    let mut total_links = 0;
    let mut broken_links = Vec::new();
    let mut nesting_errors = Vec::new();

    // Match [text](target) where target is not external or anchor-only
    let link_re = Regex::new(r"\[[^\]]+\]\(([^)]+)\)").unwrap();

    let mut markdown_files = Vec::new();

    if let Some(t) = target {
        let abs_t = if t.is_absolute() {
            t.to_path_buf()
        } else {
            repo_root.join(t)
        };
        if abs_t.is_file() && abs_t.extension().is_some_and(|ext| ext == "md") {
            markdown_files.push(abs_t);
        } else if abs_t.is_dir() {
            collect_markdown_files(&abs_t, &mut markdown_files);
        } else {
            println!(
                "{} Target not found or invalid: {:?}",
                "SKIP".yellow(),
                abs_t
            );
            return Ok(());
        }
    } else {
        if !docs_src.exists() {
            println!(
                "{} Docs source not found at {:?}",
                "SKIP".yellow(),
                docs_src
            );
            return Ok(());
        }
        collect_markdown_files(&docs_src, &mut markdown_files);
        collect_root_guides(repo_root, &mut markdown_files);
        collect_docs_root_guides(&docs_dir, &mut markdown_files);
    }

    for path in markdown_files {
        // Enforce max depth for files inside docs/src
        if path.starts_with(&docs_src) {
            let rel = path.strip_prefix(&docs_src).unwrap();
            let depth = rel.components().count();
            if depth > 3 {
                nesting_errors.push(path.to_path_buf());
            }
        }

        let content =
            read_utf8_path_capped(&path).with_context(|| format!("Failed to read {:?}", path))?;

        let parent_dir = path.parent().unwrap_or(repo_root);

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

            // Split into path and optional anchor
            let mut parts = target_full.splitn(2, '#');
            let target_path_str = parts.next().unwrap_or(target_full);
            let anchor = parts.next();

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
                broken_links.push((
                    path.to_path_buf(),
                    target_full.to_string(),
                    target_path,
                    "missing_file",
                ));
            } else if let Some(anchor_text) = anchor {
                if !check_anchor(&target_path, anchor_text) {
                    broken_links.push((
                        path.to_path_buf(),
                        target_full.to_string(),
                        target_path,
                        "missing_anchor",
                    ));
                }
            }
        }
    }

    println!("{} Checked {} internal links.", "DONE".green(), total_links);

    let mut failed = false;

    if !nesting_errors.is_empty() {
        println!(
            "{} Found {} inappropriately nested files (> 3 levels deep in docs/src):",
            "FAIL".red().bold(),
            nesting_errors.len()
        );
        for f in &nesting_errors {
            println!(
                "  {}",
                f.strip_prefix(repo_root).unwrap_or(f).display().yellow()
            );
        }
        failed = true;
    }

    if !broken_links.is_empty() {
        println!(
            "{} Found {} broken links:",
            "FAIL".red().bold(),
            broken_links.len()
        );
        for (source, target_str, resolved, reason) in &broken_links {
            let rel_source = source.strip_prefix(repo_root).unwrap_or(source);
            println!(
                "  {} -> {} (resolved: {:?}) [{}]",
                rel_source.display().cyan(),
                target_str.yellow(),
                resolved.display().dimmed(),
                reason.red()
            );
        }
        failed = true;
    }

    if failed {
        Err(anyhow::anyhow!("Documentation integrity check failed"))
    } else {
        println!(
            "{} All internal links and structures are valid!",
            "PASS".green().bold()
        );
        Ok(())
    }
}

fn check_anchor(path: &Path, anchor: &str) -> bool {
    let content = match read_utf8_path_capped(path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Convert anchor to a header string. Example: "phase-1:-harden-the-core-ci-link-checker"
    // This is a simplistic check, matching `# some text` ignoring case and punctuation where needed.
    // For robust matching, we look for an explicit inline `<a id="...">` or `<a name="...">` OR a header line.

    let a_id_pattern = format!("id=\"{anchor}\"");
    let a_name_pattern = format!("name=\"{anchor}\"");
    if content.contains(&a_id_pattern) || content.contains(&a_name_pattern) {
        return true;
    }

    // Try finding headers
    let lines = content.lines();
    for line in lines {
        if line.starts_with('#') {
            let text = line.trim_start_matches('#').trim();
            // Basic markdown anchor generation: lowercase, replace spaces with hyphen, remove punctuation
            let generated = text
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
                .collect::<String>()
                .replace(" ", "-");
            if generated == anchor {
                return true;
            }
        }
    }

    false
}

fn collect_markdown_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "md") {
            out.push(entry.path().to_path_buf());
        }
    }
}

fn collect_root_guides(repo_root: &Path, out: &mut Vec<std::path::PathBuf>) {
    for rel in ["README.md", "AGENTS.md", "CONTRIBUTING.md"] {
        let path = repo_root.join(rel);
        if path.is_file() {
            out.push(path);
        }
    }
}

fn collect_docs_root_guides(docs_dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if !docs_dir.is_dir() {
        return;
    }
    for entry in WalkDir::new(docs_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "md") {
            out.push(entry.path().to_path_buf());
        }
    }
}
