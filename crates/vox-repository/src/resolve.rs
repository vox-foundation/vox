//! Additional root resolution modes for CI, MCP hints, and Cargo workspace manifests.
//!
//! [`super::discover_repository`] remains the primary API for Git-backed
//! [`super::RepositoryContext`]. These helpers cover walk-up heuristics that
//! tooling used to duplicate across crates.

use std::path::{Path, PathBuf};

/// Environment variable overriding the logical repo root for CI and doc tools.
pub const VOX_REPO_ROOT_ENV: &str = "VOX_REPO_ROOT";

/// Resolve repository root for `vox ci` and doc-inventory: `VOX_REPO_ROOT`, else walk up from
/// [`std::env::current_dir`] until both `AGENTS.md` and `Cargo.toml` exist, else fall back to CWD.
pub fn resolve_repo_root_for_ci() -> PathBuf {
    if let Ok(p) = std::env::var(VOX_REPO_ROOT_ENV) {
        return PathBuf::from(p);
    }
    let mut d = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if d.join("AGENTS.md").is_file() && d.join("Cargo.toml").is_file() {
            return d;
        }
        if !d.pop() {
            return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
    }
}

/// Walk up from `start` (or CWD if empty) until a directory contains `Vox.toml` or `Cargo.toml`.
///
/// If no marker is found, returns [`std::env::current_dir`] (same behavior as the former MCP helper).
pub fn find_project_manifest_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.as_os_str().is_empty() {
        std::env::current_dir().ok()?
    } else if start.is_absolute() {
        std::fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf())
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::fs::canonicalize(cwd.join(start)).unwrap_or_else(|_| cwd.join(start))
    };
    loop {
        if current.join("Vox.toml").is_file() || current.join("Cargo.toml").is_file() {
            return Some(current);
        }
        if !current.pop() {
            return std::env::current_dir().ok();
        }
    }
}

/// Walk up from `start` until `Cargo.toml` contains a `[workspace]` section.
pub fn find_cargo_workspace_root_from(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.as_os_str().is_empty() {
        std::env::current_dir().ok()?
    } else {
        std::fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf())
    };
    loop {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file()
            && let Ok(contents) = std::fs::read_to_string(&manifest)
            && contents.lines().any(|l| l.trim() == "[workspace]")
        {
            return Some(dir);
        }
        dir = dir.parent()?.to_path_buf();
    }
}

/// Walk up from [`std::env::current_dir`] until `Cargo.toml` contains a `[workspace]` section.
pub fn find_cargo_workspace_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    find_cargo_workspace_root_from(&cwd)
}

/// Resolve `path` relative to the Cargo workspace root when possible; otherwise return `path` as-is.
pub fn resolve_from_cargo_workspace(path: &Path) -> PathBuf {
    find_cargo_workspace_root()
        .map(|root| root.join(path))
        .unwrap_or_else(|| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn find_cargo_workspace_root_from_nested_workspace() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let sub = dir.path().join("ws");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        let deep = sub.join("deep");
        fs::create_dir_all(&deep).unwrap();
        let root = find_cargo_workspace_root_from(&deep).expect("workspace root");
        // Canonicalize both sides: Windows tempdir paths may use extended-length (\\?\) prefixes
        // that differ from the raw path returned by tempdir().
        let root_c = std::fs::canonicalize(&root).unwrap_or(root);
        let sub_c = std::fs::canonicalize(&sub).unwrap_or(sub);
        assert_eq!(root_c, sub_c);
    }

    #[test]
    fn find_project_manifest_root_prefers_nearest_vox_toml() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"r\"\n").unwrap();
        let nested = dir.path().join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("Vox.toml"), "[vox]\n").unwrap();
        let got = find_project_manifest_root(&nested).expect("some root");
        let got_c = std::fs::canonicalize(&got).unwrap_or(got);
        let nested_c = std::fs::canonicalize(&nested).unwrap_or(nested);
        assert_eq!(got_c, nested_c);
    }
}
