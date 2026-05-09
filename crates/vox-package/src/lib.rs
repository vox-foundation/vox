//! Package and schema management for Vox projects (hashing, namespaces, normalized paths, storage).
//!
//! The `CodeStore` API (**Arca** internal storage) is backed by Turso (libSQL-compatible).
//! Application code should use **`vox_db::Codex`** / `VoxDb` as the public facade (see ADR 004).
#![allow(clippy::collapsible_if)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::manual_unwrap_or_default)]

/// Content-addressed artifact cache (`.vox-cache`).
pub mod artifact_cache;
/// HTTP client for the package registry API.
pub mod registry;
/// Multi-package workspace discovery from `Vox.toml` / members.
pub mod workspace;

// Re-export all type-only items from the pure-data L1 crate.
pub use vox_package_types::{
    CoolifyDeployConfig, CoolifyEnvReconciliationMode, CoolifyEnvVarDetail, CoolifyEnvVarSpec,
    DependencySpec, DeploySection, DetailedDependency, Lockfile, ManifestError, PackageKind,
    SemVer, VersionReq, VoxManifest, deploy_coolify, lockfile, manifest, package_kind, resolver,
};

pub use artifact_cache::{ArtifactCache, CacheLookup, CacheManifest};
pub use registry::{
    DownloadResponse, PublishDependency, PublishRequest, RegistryClient, RegistryPackageInfo,
    SearchResult,
};
pub use vox_db::hash::content_hash;
pub use workspace::VoxWorkspace;
