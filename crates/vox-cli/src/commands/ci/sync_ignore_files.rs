use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

/// Derived files to generate from `.voxignore`.
const DERIVED_FILES: &[(&str, &str)] = &[
    (
        ".cursorignore",
        "# .cursorignore \u{2014} DERIVED from .voxignore (SSOT)\n# DO NOT EDIT DIRECTLY. Run `vox ci sync-ignore-files` to regenerate.\n# See: docs/src/archive/research-2026-q1/multi-repo-context-isolation-research-2026.md \u{A7}3\n\n",
    ),
    (
        ".aiignore",
        "# .aiignore \u{2014} DERIVED from .voxignore (SSOT)\n# Consumed by JetBrains AI Assistant (must be enabled in Settings | Tools | AI Assistant)\n# DO NOT EDIT DIRECTLY. Run `vox ci sync-ignore-files` to regenerate.\n# See: docs/src/archive/research-2026-q1/multi-repo-context-isolation-research-2026.md \u{A7}3\n\n",
    ),
    (
        ".aiexclude",
        "# .aiexclude \u{2014} DERIVED from .voxignore (SSOT)\n# Consumed by Gemini/Android Studio Code Assist\n# DO NOT EDIT DIRECTLY. Run `vox ci sync-ignore-files` to regenerate.\n# See: docs/src/archive/research-2026-q1/multi-repo-context-isolation-research-2026.md \u{A7}3\n\n",
    ),
];

fn extract_ignore_patterns(voxignore_content: &str) -> String {
    let mut out = String::new();
    for line in voxignore_content.lines() {
        if line.starts_with("# .voxignore \u{2014} SINGLE SOURCE OF TRUTH")
            || line.starts_with("# IMPORTANT: This file is the SSOT")
            || line.starts_with("# should exclude from AI context.")
            || line.starts_with("# vox ci sync-ignore-files")
            || line.starts_with("# to regenerate all derived")
            || line.starts_with("# GitHub Copilot content exclusion")
            || line.starts_with("# See: docs/")
            || line.starts_with("#   vox ci sync-ignore-files")
            || line.starts_with("# .aiexclude directly.")
            || line.starts_with("# GitHub Settings")
            || line.is_empty() && out.is_empty()
        {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub(crate) fn run(root: &Path, verify: bool) -> Result<()> {
    let voxignore_path = root.join(".voxignore");
    if !voxignore_path.exists() {
        if verify {
            return Err(anyhow!(
                ".voxignore not found in workspace root. It is required as the SSOT for AI context exclusion."
            ));
        } else {
            // Nothing to do if it doesn't exist and we're not verifying strictly
            return Ok(());
        }
    }

    let voxignore_content = fs::read_to_string(&voxignore_path)?;
    let parsed_patterns = extract_ignore_patterns(&voxignore_content);

    for (filename, header) in DERIVED_FILES {
        let target_path = root.join(filename);
        let expected_content = format!("{}{}", header, parsed_patterns)
            .replace("\u{2014}", "—")
            .replace("\u{A7}", "§");

        // Use standard double-quotes with unicode escapes above, but here we can just write it.
        // Rust string literals handle unicode characters well. I used unicode escapes above just to be safe.
        // Actually replacing it to fix any potential encoding issues.

        if verify {
            if !target_path.exists() {
                return Err(anyhow!(
                    "Derived ignore file {} is missing. Please run `vox ci sync-ignore-files`.",
                    filename
                ));
            }
            let current_content = fs::read_to_string(&target_path)?;
            if current_content != expected_content {
                return Err(anyhow!(
                    "Derived ignore file {} is out of sync with .voxignore. Please run `vox ci sync-ignore-files`.",
                    filename
                ));
            }
        } else {
            fs::write(&target_path, expected_content)?;
            println!("Synchronized {} from .voxignore", filename);
        }
    }

    if verify {
        println!("All derived ignore files are perfectly in sync with .voxignore.");
    }

    Ok(())
}
