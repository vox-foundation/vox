//! Workspace discovery snapshot for CLI and MCP (`vox repo status` / `vox_repo_status`).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::discover_repository_or_fallback;
use crate::workspace_layout::cargo_workspace_member_dirs;

/// JSON shape shared by **`vox repo status`** and MCP **`vox_repo_status`**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoWorkspaceStatus {
    pub root: PathBuf,
    pub repository_id: String,
    pub origin_url: Option<String>,
    pub git_root: Option<PathBuf>,
    pub has_vox_agents_dir: bool,
    pub vox_toml: Option<PathBuf>,
    pub capabilities: crate::RepoCapabilities,
    pub cargo_workspace_members: Vec<PathBuf>,
}

/// Discover repository from `cwd` and build the status payload (sorted workspace members when Cargo workspace).
pub fn repo_workspace_status_for_cwd(cwd: &Path) -> RepoWorkspaceStatus {
    let ctx = discover_repository_or_fallback(cwd);
    let cargo_workspace_members = if ctx.capabilities.cargo_workspace {
        let mut dirs = cargo_workspace_member_dirs(&ctx.root);
        dirs.sort();
        dirs
    } else {
        Vec::new()
    };
    RepoWorkspaceStatus {
        root: ctx.root,
        repository_id: ctx.repository_id,
        origin_url: ctx.origin_url,
        git_root: ctx.git_root,
        has_vox_agents_dir: ctx.has_vox_agents_dir,
        vox_toml: ctx.vox_toml,
        capabilities: ctx.capabilities,
        cargo_workspace_members,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn status_includes_repository_id() {
        let d = TempDir::new().expect("tempdir");
        fs::write(d.path().join("README.md"), "x").unwrap();
        let s = repo_workspace_status_for_cwd(d.path());
        assert!(!s.repository_id.is_empty());
        assert_eq!(s.root, d.path().canonicalize().unwrap());
    }
}
