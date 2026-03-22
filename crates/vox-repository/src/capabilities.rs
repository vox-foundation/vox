//! Stack / project capability probes under a repository root.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Markers for tooling gates (Cargo, Node, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoCapabilities {
    /// `Vox.toml` present at repository root.
    pub vox_project: bool,
    /// Root `Cargo.toml` declares `[workspace]`.
    pub cargo_workspace: bool,
    /// Root `Cargo.toml` declares `[package]` (single crate or workspace member file mis-read guard).
    pub cargo_package: bool,
    /// `package.json` or `pnpm-workspace.yaml` at root.
    pub node_workspace: bool,
    /// `pyproject.toml` or `setup.py` at root.
    pub python_project: bool,
    /// `go.mod` at root.
    pub go_module: bool,
    /// Inside a Git work tree (`root` is under a `.git` ancestor or `git_root` matches).
    pub git: bool,
}

/// Probe capabilities for files under `root`.
pub fn probe_capabilities(root: &Path, in_git_work_tree: bool) -> RepoCapabilities {
    let cargo_toml = root.join("Cargo.toml");
    let mut cargo_workspace = false;
    let mut cargo_package = false;
    if cargo_toml.is_file()
        && let Ok(text) = std::fs::read_to_string(&cargo_toml)
        && let Ok(val) = toml::from_str::<toml::Value>(&text)
    {
        cargo_workspace = val.get("workspace").is_some();
        cargo_package = val.get("package").is_some();
    }
    RepoCapabilities {
        vox_project: root.join("Vox.toml").is_file(),
        cargo_workspace,
        cargo_package,
        node_workspace: root.join("package.json").is_file()
            || root.join("pnpm-workspace.yaml").is_file(),
        python_project: root.join("pyproject.toml").is_file() || root.join("setup.py").is_file(),
        go_module: root.join("go.mod").is_file(),
        git: in_git_work_tree,
    }
}
