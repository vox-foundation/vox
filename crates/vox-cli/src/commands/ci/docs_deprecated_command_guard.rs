//! Fail docs / agent registries that embed retired `cargo test` shapes (compiler monolith drift).

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::Path;

fn scan_file(root: &Path, path: &Path, failures: &mut Vec<String>) -> Result<()> {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let body = fs::read_to_string(path).with_context(|| format!("read {}", rel.display()))?;
    for (line_idx, line) in body.lines().enumerate() {
        let line_no = line_idx + 1;
        if line.contains("cargo test -p vox-parser") {
            failures.push(format!(
                "{}:{}: retired crate in command example; use `-p vox-compiler --test golden_examples_strict_parse`",
                rel.display(),
                line_no
            ));
        }
        if line.contains("vox-compiler") && line.contains("parity_test") {
            failures.push(format!(
                "{}:{}: wrong integration test name for vox-compiler strict-parse; use `--test golden_examples_strict_parse`",
                rel.display(),
                line_no
            ));
        }
    }
    Ok(())
}

/// Scan markdown under `docs/` plus selected agent/script registries for stale cargo-test snippets.
pub fn verify(root: &Path) -> Result<()> {
    let mut failures = Vec::new();

    let docs = root.join("docs");
    if docs.is_dir() {
        let mut stack = vec![docs];
        while let Some(dir) = stack.pop() {
            for entry in
                fs::read_dir(&dir).with_context(|| format!("read_dir {}", dir.display()))?
            {
                let entry = entry?;
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|e| e.to_str()) == Some("md") {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name == "SUMMARY.md" || name.contains("-ARCHIVED.md") {
                        continue;
                    }
                    scan_file(root, &p, &mut failures)?;
                }
            }
        }
    }

    let script_registry = root.join("docs/agents/script-registry.json");
    if script_registry.is_file() {
        scan_file(root, &script_registry, &mut failures)?;
    }

    let scripts_readme = root.join("scripts/README.md");
    if scripts_readme.is_file() {
        scan_file(root, &scripts_readme, &mut failures)?;
    }

    if !failures.is_empty() {
        for f in &failures {
            eprintln!("{}", f);
        }
        return Err(anyhow!(
            "docs_deprecated_command_guard: {} stale cargo-test reference(s)",
            failures.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn detects_vox_parser_cargo_invocation() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let md = tmp.path().join("docs").join("x.md");
        fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
        let mut f = fs::File::create(&md).expect("create");
        writeln!(f, "run `cargo test -p vox-parser --lib`").expect("write");
        let mut failures = Vec::new();
        scan_file(tmp.path(), &md, &mut failures).expect("scan");
        assert!(!failures.is_empty());
    }

    #[test]
    fn detects_parity_test_with_vox_compiler() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let md = tmp.path().join("docs").join("y.md");
        fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
        let mut f = fs::File::create(&md).expect("create");
        writeln!(f, "`cargo test -p vox-compiler --test parity_test`").expect("write");
        let mut failures = Vec::new();
        scan_file(tmp.path(), &md, &mut failures).expect("scan");
        assert!(!failures.is_empty());
    }
}
