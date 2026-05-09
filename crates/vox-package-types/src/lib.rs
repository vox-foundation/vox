//! Pure-data L1 leaf for vox-package: Vox.toml manifest types, vox.lock
//! types, package_kind enum, resolver request/response types. No async,
//! no DB, no compiler dependency.

/// `[deploy.coolify]` manifest shapes (serde-only).
pub mod deploy_coolify;
/// `vox.lock` lockfile format.
pub mod lockfile;
/// `Vox.toml` manifest parsing and serialization.
pub mod manifest;
/// Enumerates artifact kinds that VoxPM can manage as packages.
pub mod package_kind;
/// Semantic versions and dependency resolution helpers.
pub mod resolver;

pub use deploy_coolify::{
    CoolifyDeployConfig, CoolifyEnvReconciliationMode, CoolifyEnvVarDetail, CoolifyEnvVarSpec,
};
pub use lockfile::Lockfile;
pub use manifest::{DependencySpec, DeploySection, DetailedDependency, ManifestError, VoxManifest};
pub use package_kind::PackageKind;
pub use resolver::{SemVer, VersionReq};
