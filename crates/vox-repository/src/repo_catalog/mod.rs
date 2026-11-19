//! Explicit multi-repository catalog and local cross-repo query helpers.
//!
//! The catalog is intentionally local-first: each local entry resolves to an ordinary
//! [`crate::RepositoryContext`], while remote entries remain adapter metadata until a caller
//! chooses a remote backend.

mod catalog;
pub mod federation;
mod query;
#[cfg(test)]
mod tests;
mod types;

mod voxignore;

pub use catalog::{
    RepoCatalogError, add_repository_to_catalog, add_repository_to_catalog_with_home,
    external_repo_cache_root, external_repo_cache_root_under, get_active_repository_context,
    get_active_repository_context_with_home, global_library_root, global_library_root_under,
    load_repo_catalog_from_repo, onboard_repository_harness, onboard_repository_harness_with_home,
    refresh_repo_catalog, remove_repository_from_catalog, repo_catalog_manifest_path,
    resolve_repo_catalog, save_repo_catalog, switch_active_repository,
    switch_active_repository_with_home, vox_user_home_dir,
};
pub use federation::{
    ContractDrift, ContractFileDigest, CrossRepoParityReport, FederatedCrateNode, FederatedEdge,
    FederatedGraphSummary, RepoContractStatus, SatelliteSyncSummary, StandaloneSovereigntyReport,
    default_federation_catalog, refresh_federated_symbol_graph, render_local_cargo_patch_block,
    sync_federated_satellites, verify_cross_repo_contract_parity, verify_standalone_sovereignty,
};
pub use query::{repo_query_file, repo_query_history, repo_query_text};
pub use types::{
    ActiveRepoContext, CrossRepoQueryTrace, HarnessOnboardSummary, QueryFileParams,
    QueryHistoryParams, QueryTextParams, RemoteAdapterHints, RepoAccessMode, RepoCapability,
    RepoCatalog, RepoCatalogRefreshResult, RepoFileRead, RepoFileReadResponse, RepoHistoryEntry,
    RepoHistoryResponse, RepoQuerySkippedRepository, RepoTextMatch, RepoTextSearchResponse,
    RepositoryDescriptor, ResolvedRepoCatalog, ResolvedRepositoryDescriptor,
};
pub(crate) use voxignore::VoxIgnore;

pub(crate) const REPO_CATALOG_BASENAME: &str = "repositories.yaml";
pub const REPO_CATALOG_SCHEMA_VERSION: u32 = 1;
pub(crate) const MAX_FILES_PER_REPO_DEFAULT: usize = 50_000;
pub(crate) const MAX_FILE_BYTES_DEFAULT: usize = 256 * 1024;
pub(crate) const MAX_TEXT_MATCHES_PER_REPO_DEFAULT: usize = 50;
pub(crate) const MAX_FILE_READ_BYTES_DEFAULT: usize = 128 * 1024;
pub(crate) const MAX_HISTORY_COMMITS_DEFAULT: usize = 20;

pub(crate) const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".venv",
    ".vox",
    ".next",
    "vendor",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    ".eggs",
    "coverage",
    ".turbo",
    ".vercel",
    ".svelte-kit",
];
