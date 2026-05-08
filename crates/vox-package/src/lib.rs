//! Package and schema management for Vox projects (hashing, namespaces, normalized paths, storage).
//!
//! The `CodeStore` API (**Arca** internal storage) is backed by Turso (libSQL-compatible).
//! Application code should use **`vox_db::Codex`** / `VoxDb` as the public facade (see ADR 004).
#![allow(clippy::collapsible_if)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::manual_unwrap_or_default)]

/// Content-addressed artifact cache (`.vox-cache`).
pub mod artifact_cache;
/// `[deploy.coolify]` manifest shapes (serde-only).
pub mod deploy_coolify;
/// `vox.lock` lockfile format.
pub mod lockfile;
/// `Vox.toml` manifest parsing and serialization.
pub mod manifest;
/// HTTP client for the package registry API.
pub mod registry;
/// Semantic versions and dependency resolution helpers.
pub mod resolver;
/// Multi-package workspace discovery from `Vox.toml` / members.
pub mod workspace;

pub use artifact_cache::{ArtifactCache, CacheLookup, CacheManifest};
pub use deploy_coolify::{
    CoolifyDeployConfig, CoolifyEnvReconciliationMode, CoolifyEnvVarDetail, CoolifyEnvVarSpec,
};

pub use lockfile::Lockfile;
pub use manifest::{DependencySpec, DeploySection, DetailedDependency, ManifestError, VoxManifest};
pub use registry::{
    DownloadResponse, PublishDependency, PublishRequest, RegistryClient, RegistryPackageInfo,
    SearchResult,
};
pub use resolver::{SemVer, VersionReq};
pub use vox_db::hash::content_hash;
pub use workspace::VoxWorkspace;
