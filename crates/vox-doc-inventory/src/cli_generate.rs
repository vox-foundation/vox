//! Shared `main` logic for `doc-inventory-generate` and `vox-doc-inventory-generate` binaries.

use std::path::{Path, PathBuf};

// vox:defactored-from vox-repository 2026-11-12 (avoids vox-doc-inventory -> vox-repository edge)
fn discover_repo_root_from(start: &Path) -> PathBuf {
    let mut probe = start.to_path_buf();
    loop {
        if probe.join("AGENTS.md").is_file() && probe.join("Cargo.toml").is_file() {
            return probe;
        }
        if !probe.pop() {
            break;
        }
    }
    let mut probe = start.to_path_buf();
    loop {
        if probe.join("Vox.toml").is_file() || probe.join(".git").exists() {
            return probe;
        }
        if !probe.pop() {
            break;
        }
    }
    start.to_path_buf()
}

fn parse_inventory_cli_args<I>(args: I) -> anyhow::Result<(Option<PathBuf>, Option<PathBuf>)>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let mut repo_root: Option<PathBuf> = None;
    let mut out_path: Option<PathBuf> = None;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--repo-root" => {
                let Some(val) = iter.next() else {
                    anyhow::bail!("missing directory argument for --repo-root");
                };
                repo_root = Some(PathBuf::from(val));
            }
            "--output" => {
                let Some(val) = iter.next() else {
                    anyhow::bail!("missing path argument for --output");
                };
                out_path = Some(PathBuf::from(val));
            }
            _ => anyhow::bail!(
                "usage: vox-doc-inventory-generate | doc-inventory-generate [--repo-root <dir>] [--output <path>]\n\
                 default: {}",
                crate::DEFAULT_INVENTORY_PATH
            ),
        }
    }
    Ok((repo_root, out_path))
}

/// Parse argv and write inventory JSON (default path or `--output <path>`).
pub fn run_generate_inventory_cli() -> anyhow::Result<()> {
    let (explicit_root, explicit_out) = parse_inventory_cli_args(std::env::args().skip(1))?;
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = explicit_root.unwrap_or_else(|| discover_repo_root_from(&cwd));
    let out = match explicit_out {
        None => root.join(crate::DEFAULT_INVENTORY_PATH),
        Some(p) if p.is_absolute() => p,
        Some(p) => root.join(p),
    };
    crate::generate(&root, &out)?;
    println!("Wrote {}", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_optional_repo_root_and_output_flags() {
        let (root, out) = parse_inventory_cli_args(vec![
            "--repo-root".to_string(),
            "repo".to_string(),
            "--output".to_string(),
            "out.json".to_string(),
        ])
        .unwrap();
        assert_eq!(root, Some(PathBuf::from("repo")));
        assert_eq!(out, Some(PathBuf::from("out.json")));
    }

    #[test]
    fn rejects_unknown_cli_flag() {
        assert!(parse_inventory_cli_args(vec!["--bogus".to_string()]).is_err());
    }
}
