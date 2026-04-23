use anyhow::{Context, Result};
use std::path::Path;

pub fn check_frozen_crates(root: &Path) -> Result<()> {
    let frozen_path = root.join("crates").join("_frozen.md");
    if !frozen_path.exists() {
        // If _frozen.md doesn't exist, we don't enforce it.
        return Ok(());
    }

    let frozen_content = std::fs::read_to_string(&frozen_path)
        .with_context(|| "failed to read crates/_frozen.md")?;

    let mut approved_crates = Vec::new();
    for line in frozen_content.lines() {
        if let Some(idx) = line.find("`vox-") {
            let start = idx + 1; // skip the backtick
            if let Some(end) = line[start..].find('`') {
                let name = &line[start..start + end];
                approved_crates.push(name.to_string());
            }
        }
    }

    let crates_dir = root.join("crates");
    let mut violations = Vec::new();
    for entry in std::fs::read_dir(&crates_dir).with_context(|| "failed to read crates/ dir")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let crate_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            // Ignore _ prefixes or dotfiles
            if crate_name.starts_with('_') || crate_name.starts_with('.') {
                continue;
            }
            if !approved_crates.contains(&crate_name) {
                violations.push(crate_name);
            }
        }
    }

    if !violations.is_empty() {
        println!(
            "⚠️ Notice: The following crates are peripheral or experimental (not in Frozen Core):"
        );
        for v in violations {
            println!("  - {}", v);
        }
    }

    Ok(())
}
