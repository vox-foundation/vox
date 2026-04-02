//! Explicit multi-repository catalog and local cross-repo query helpers.
//!
//! The catalog is intentionally local-first: each local entry resolves to an ordinary
//! [`crate::RepositoryContext`], while remote entries remain adapter metadata until a caller
//! chooses a remote backend.

mod catalog;
mod query;
#[cfg(test)]
mod tests;
mod types;

pub use catalog::{
    RepoCatalogError, load_repo_catalog_from_repo, refresh_repo_catalog,
    repo_catalog_manifest_path, resolve_repo_catalog,
};
pub use query::{repo_query_file, repo_query_history, repo_query_text};
pub use types::{
    CrossRepoQueryTrace, QueryFileParams, QueryHistoryParams, QueryTextParams, RemoteAdapterHints,
    RepoAccessMode, RepoCapability, RepoCatalog, RepoCatalogRefreshResult, RepoFileRead,
    RepoFileReadResponse, RepoHistoryEntry, RepoHistoryResponse, RepoQuerySkippedRepository,
    RepoTextMatch, RepoTextSearchResponse, RepositoryDescriptor, ResolvedRepoCatalog,
    ResolvedRepositoryDescriptor,
};

pub(crate) const REPO_CATALOG_BASENAME: &str = "repositories.yaml";
pub(crate) const REPO_CATALOG_SCHEMA_VERSION: u32 = 1;
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
];
