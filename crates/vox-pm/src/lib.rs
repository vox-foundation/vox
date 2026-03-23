//! Package and schema management for Vox projects (hashing, namespaces, normalized paths, storage).
//!
//! The `CodeStore` API (**Arca** internal storage) is backed by Turso (libSQL-compatible).
//! Application code should use **`vox_db::Codex`** / `VoxDb` as the public facade (see ADR 004).
#![allow(clippy::collapsible_if)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::manual_unwrap_or_default)]

/// SHA3-512 content hashes encoded as Base32Hex (CAS object identity).
pub mod hash;
/// Dot-separated logical namespaces for named bindings in the store.
pub mod namespace;
/// Source normalization before hashing (comments/whitespace stripping).
pub mod normalize;
/// Arca SQL schema manifest + baseline V1 (`schema_version` records a single baseline row).
pub mod schema;
/// Turso-backed [`CodeStore`](store::CodeStore) and related types.
pub mod store;

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
pub use store::{
    AgentDefEntry, ArtifactEntry, BehaviorEventEntry, BuilderSessionEntry, CodeStore,
    CodexChangeLogEntry, CommandFrequencyEntry, ComponentEntry, EmbeddingEntry,
    EndpointReliabilityEntry, ExecutionEntry, KnowledgeNodeSummary, LearnedPatternEntry,
    LogExecutionParams, LogInteractionParams, MemoryEntry, PackageSearchResult,
    PublishArtifactParams, RegisterAgentParams, ReviewEntry, SaveMemoryParams, SaveSnippetParams,
    ScheduledEntry, SessionTurnEntry, SkillExecutionParams, SkillManifestEntry,
    SkillReliabilityReport, SnippetEntry, StoreError, TrainingPair, TypedStreamEventEntry,
    UserEntry,
};

pub use hash::content_hash;
pub use lockfile::Lockfile;
pub use manifest::{DependencySpec, DeploySection, DetailedDependency, ManifestError, VoxManifest};
pub use registry::{
    DownloadResponse, PublishDependency, PublishRequest, RegistryClient, RegistryPackageInfo,
    SearchResult,
};
pub use resolver::{SemVer, VersionReq};
pub use workspace::VoxWorkspace;
