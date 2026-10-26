//! `vox ci dom-test-env-ban` — ban @vitest-environment / jsdom imports in unit tests.
//!
//! Default is **strict** (fail closed). Pass `--advisory` only for migration sweeps.

use anyhow::{Result, bail};
use std::path::Path;
use walkdir::WalkDir;

fn is_unit_test(path: &str) -> bool {
    let p = path.replace('\\', "/");
    (p.contains(".test.") || p.contains(".spec."))
        && !p.contains("/e2e/")
        && !p.contains(".browser.")
}

fn is_violation(text: &str) -> bool {
    text.contains("@vitest-environment")
        || text.contains("from 'jsdom'")
        || text.contains("from \"jsdom\"")
        || text.contains("from 'happy-dom'")
        || text.contains("from \"happy-dom\"")
}

pub struct DomTestEnvBanArgs {
    /// When true, print violations but exit 0.
    pub advisory: bool,
}

pub fn run(root: &Path, args: DomTestEnvBanArgs) -> Result<()> {
    let ui = root.join("crates/vox-gui/ui");
    if !ui.is_dir() {
        println!("dom-test-env-ban: skip (no ui tree)");
        return Ok(());
    }
    let mut bad = Vec::new();
    for entry in WalkDir::new(ui.join("src"))
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if !is_unit_test(&rel_s) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        if is_violation(&text) {
            bad.push(rel_s);
        }
    }
    if bad.is_empty() {
        println!("dom-test-env-ban: ok");
        return Ok(());
    }
    eprintln!("dom-test-env-ban: {} violation(s):", bad.len());
    for b in bad.iter().take(40) {
        eprintln!("  - {b}");
    }
    if bad.len() > 40 {
        eprintln!("  … and {} more", bad.len() - 40);
    }
    if args.advisory {
        println!(
            "dom-test-env-ban: advisory ({} hits; omit --advisory to fail closed)",
            bad.len()
        );
        return Ok(());
    }
    bail!(
        "unit tests must not set @vitest-environment / import jsdom — use project vitest config or *.browser.test.* ({} hits)",
        bad.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pragma() {
        assert!(is_violation("// @vitest-environment jsdom\n"));
        assert!(!is_violation("export const x = 1;\n"));
    }
}
