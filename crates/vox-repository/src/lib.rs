//! Discover the logical repository root (Git work tree or fallback), compute a stable
//! [`RepositoryContext::repository_id`], and probe stack capabilities for MCP/orchestrator wiring.

mod agent_scope;
mod capabilities;
mod discover;
mod error;
mod git_root;
pub mod gpu_inventory;
mod id;
mod path_safety;
pub mod populi_toml;
mod repo_catalog;
mod repo_query_trace;
mod repo_workspace_status;
mod resolve;
mod skill_scaffold;
mod workspace_layout;
mod workspace_path_migration;

pub use agent_scope::{
    agents_dir, agents_glob_repo_relative, load_agent_scopes, normalize_task_path,
};
pub use capabilities::{
    RepoCapabilities, TaskCapabilityHints, merge_agent_capabilities, probe_host_capabilities,
};
pub use discover::{discover_repository, discover_repository_or_fallback};
pub use error::RepositoryError;
pub use git_root::find_git_work_tree;
pub use gpu_inventory::{GpuInventorySnapshot, probe_nvidia_gpu_inventory_best_effort};
pub use id::compute_repository_id;
pub use path_safety::{
    path_relative_to_repo_root, resolve_local_path_under_repo_root,
    resolve_strict_repo_relative_path,
};
pub use populi_toml::{
    MeshToml, MeshTomlError, VoxMeshToml, VoxMeshTomlError, read_vox_populi_toml,
};
pub use repo_catalog::{
    CrossRepoQueryTrace, QueryFileParams, QueryHistoryParams, QueryTextParams,
    REPO_CATALOG_SCHEMA_VERSION, RemoteAdapterHints, RepoAccessMode, RepoCapability, RepoCatalog,
    RepoCatalogError, RepoCatalogRefreshResult, RepoFileRead, RepoFileReadResponse,
    RepoHistoryEntry, RepoHistoryResponse, RepoQuerySkippedRepository, RepoTextMatch,
    RepoTextSearchResponse, RepositoryDescriptor, ResolvedRepoCatalog,
    ResolvedRepositoryDescriptor, load_repo_catalog_from_repo, refresh_repo_catalog,
    repo_catalog_manifest_path, repo_query_file, repo_query_history, repo_query_text,
    resolve_repo_catalog,
};
pub use repo_query_trace::{
    repo_query_file_with_plane, repo_query_history_with_plane, repo_query_text_with_plane,
};
pub use repo_workspace_status::{RepoWorkspaceStatus, repo_workspace_status_for_cwd};
pub use resolve::{
    VOX_REPO_ROOT_ENV, find_cargo_workspace_root, find_cargo_workspace_root_from,
    find_project_manifest_root, resolve_from_cargo_workspace, resolve_repo_root_for_ci,
};
pub use skill_scaffold::{skill_markdown_filename, skill_markdown_for_project};
pub use workspace_layout::{
    cargo_workspace_member_dirs, go_roots, node_workspace_packages, python_roots,
};
pub use workspace_path_migration::{
    migrate_legacy_memory_shard_into_vox_memory, migrate_legacy_sessions_into_vox,
};

use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn repository_id_stable_for_same_directory() {
        let d = TempDir::new().expect("tempdir");
        let a = discover_repository(d.path()).expect("discover");
        let b = discover_repository(d.path()).expect("discover");
        assert_eq!(a.repository_id, b.repository_id);
        assert_eq!(a.root, b.root);
    }

    #[test]
    fn cargo_workspace_member_dirs_expands_crates_glob() {
        let d = TempDir::new().expect("tempdir");
        fs::write(
            d.path().join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*"]
resolver = "2"
"#,
        )
        .expect("write root");
        let c = d.path().join("crates");
        fs::create_dir_all(c.join("alpha")).expect("mkdir");
        fs::write(
            c.join("alpha").join("Cargo.toml"),
            "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("write crate");
        fs::create_dir_all(c.join("beta")).expect("mkdir");
        fs::write(
            c.join("beta").join("Cargo.toml"),
            "[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("write crate");
        let mut dirs = cargo_workspace_member_dirs(d.path());
        dirs.sort();
        assert_eq!(dirs.len(), 2);
        let names: Vec<String> = dirs
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }

    #[test]
    fn node_workspace_packages_expands_glob() {
        let d = TempDir::new().expect("tempdir");
        fs::write(
            d.path().join("package.json"),
            r#"{"name":"root","workspaces":["packages/*"]}"#,
        )
        .expect("root pkg");
        let pkg_a = d.path().join("packages").join("a");
        fs::create_dir_all(&pkg_a).expect("mkdir");
        fs::write(pkg_a.join("package.json"), "{}").expect("child pkg");
        let mut pkgs = node_workspace_packages(d.path());
        pkgs.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].0, "a");
        assert_eq!(pkgs[0].1, pkg_a);
    }
}

/// Fully resolved repository context for tooling (MCP, sessions, affinity).
#[derive(Debug, Clone)]
pub struct RepositoryContext {
    /// Canonical repository root (Git work tree root, or resolved starting directory).
    pub root: PathBuf,
    /// Git work tree root when inside a Git repository.
    pub git_root: Option<PathBuf>,
    /// Stable hex id (blake3 over origin + root path when available).
    pub repository_id: String,
    /// `remote.origin.url` when present.
    pub origin_url: Option<String>,
    /// Detected stack / project markers under `root`.
    pub capabilities: RepoCapabilities,
    /// `.vox/agents` exists (opt-in agent scope files).
    pub has_vox_agents_dir: bool,
    /// Path to `Vox.toml` when present at `root`.
    pub vox_toml: Option<PathBuf>,
}
