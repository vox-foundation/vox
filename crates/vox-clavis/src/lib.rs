pub mod backend;
pub mod errors;
pub mod policy;
pub mod resolver;
pub mod sources;
pub mod spec;
#[cfg(test)]
mod tests;
mod types;

pub use errors::SecretError;
pub use policy::{MissingBehavior, SecretPolicy};
pub use resolver::ResolveProfile;
use resolver::{ResolveOptions, SecretResolver};
pub use spec::{
    Capability, Profile, RequirementMode, RequirementSet, RotationPolicy, SecretBundle,
    SecretClass, SecretId, SecretMaterialKind, SecretMetadata, SecretSpec, Workflow,
    WorkflowRequirements, all_bundle_doc_names, all_specs, capabilities_for_secret,
    managed_secret_env_names, required_for, required_for_profile, requirements_for_bundle,
    requirements_for_profile, requirements_for_profile_mode,
};
pub use types::{ResolutionStatus, ResolvedSecret, SecretSource};

pub const OPERATOR_CLAVIS_CUTOVER_PHASE: &str = "VOX_CLAVIS_CUTOVER_PHASE";
pub const OPERATOR_CLAVIS_MIGRATION_PHASE: &str = "VOX_CLAVIS_MIGRATION_PHASE";
pub const OPERATOR_CLAVIS_HARD_CUT: &str = "VOX_CLAVIS_HARD_CUT";
pub const OPERATOR_CLAVIS_AUTO_PREFER_VAULT: &str = "VOX_CLAVIS_AUTO_PREFER_VAULT";
pub const OPERATOR_CLAVIS_KEK_REF: &str = "VOX_CLAVIS_KEK_REF";
pub const OPERATOR_CLAVIS_KEK_VERSION: &str = "VOX_CLAVIS_KEK_VERSION";
pub const OPERATOR_CLAVIS_AUTO_VAULT: &str = "VOX_CLAVIS_AUTO_VAULT";
pub const OPERATOR_CLAVIS_VAULT_URL: &str = "VOX_CLAVIS_VAULT_URL";
pub const OPERATOR_CLAVIS_VAULT_PATH: &str = "VOX_CLAVIS_VAULT_PATH";
pub const OPERATOR_CLAVIS_VAULT_TOKEN: &str = "VOX_CLAVIS_VAULT_TOKEN";
pub const OPERATOR_ACCOUNT_ID: &str = "VOX_ACCOUNT_ID";
pub const OPERATOR_CLAVIS_PROFILE: &str = "VOX_CLAVIS_PROFILE";
pub const OPERATOR_CLAVIS_BACKEND: &str = "VOX_CLAVIS_BACKEND";
pub const OPERATOR_INFISICAL_TOKEN: &str = "INFISICAL_TOKEN";
pub const OPERATOR_INFISICAL_SERVICE_TOKEN: &str = "INFISICAL_SERVICE_TOKEN";
pub const OPERATOR_VAULT_ADDR: &str = "VAULT_ADDR";
pub const OPERATOR_VAULT_TOKEN: &str = "VAULT_TOKEN";
pub const OPERATOR_TURSO_URL: &str = "VOX_TURSO_URL";
pub const OPERATOR_TURSO_TOKEN: &str = "VOX_TURSO_TOKEN";

pub const OPERATOR_SCIENTIA_CROSSREF_MAILTO: &str = "VOX_SCIENTIA_CROSSREF_MAILTO";
pub const OPERATOR_SCHOLARLY_ADAPTER: &str = "VOX_SCHOLARLY_ADAPTER";
pub const OPERATOR_SCHOLARLY_JOB_LOCK_OWNER: &str = "VOX_SCHOLARLY_JOB_LOCK_OWNER";
pub const OPERATOR_ZENODO_HTTP_MAX_ATTEMPTS: &str = "VOX_ZENODO_HTTP_MAX_ATTEMPTS";
pub const OPERATOR_ZENODO_API_BASE: &str = "VOX_ZENODO_API_BASE";
pub const OPERATOR_OPENREVIEW_HTTP_MAX_ATTEMPTS: &str = "VOX_OPENREVIEW_HTTP_MAX_ATTEMPTS";
pub const OPERATOR_ZENODO_STAGING_DIR: &str = "VOX_ZENODO_STAGING_DIR";
pub const OPERATOR_ZENODO_UPLOAD_ALLOWLIST: &str = "VOX_ZENODO_UPLOAD_ALLOWLIST";
pub const OPERATOR_SYNDICATION_TEMPLATE_PROFILE: &str = "VOX_SYNDICATION_TEMPLATE_PROFILE";
pub const OPERATOR_NEWS_PUBLISH_ARMED: &str = "VOX_NEWS_PUBLISH_ARMED";
pub const OPERATOR_NEWS_SITE_BASE_URL: &str = "VOX_NEWS_SITE_BASE_URL";
pub const OPERATOR_NEWS_RSS_FEED_PATH: &str = "VOX_NEWS_RSS_FEED_PATH";

/// Array of system operator tuning environment variables.
pub const OPERATOR_TUNING_ENVS: &[&str] = &[
    OPERATOR_CLAVIS_CUTOVER_PHASE,
    OPERATOR_CLAVIS_MIGRATION_PHASE,
    OPERATOR_CLAVIS_HARD_CUT,
    OPERATOR_CLAVIS_AUTO_PREFER_VAULT,
    OPERATOR_CLAVIS_KEK_REF,
    OPERATOR_CLAVIS_KEK_VERSION,
    OPERATOR_CLAVIS_AUTO_VAULT,
    OPERATOR_CLAVIS_VAULT_URL,
    OPERATOR_CLAVIS_VAULT_PATH,
    OPERATOR_CLAVIS_VAULT_TOKEN,
    OPERATOR_ACCOUNT_ID,
    OPERATOR_CLAVIS_PROFILE,
    OPERATOR_CLAVIS_BACKEND,
    OPERATOR_INFISICAL_TOKEN,
    OPERATOR_INFISICAL_SERVICE_TOKEN,
    OPERATOR_VAULT_ADDR,
    OPERATOR_VAULT_TOKEN,
    OPERATOR_TURSO_URL,
    OPERATOR_TURSO_TOKEN,
    OPERATOR_SCIENTIA_CROSSREF_MAILTO,
    OPERATOR_SCHOLARLY_ADAPTER,
    OPERATOR_SCHOLARLY_JOB_LOCK_OWNER,
    OPERATOR_ZENODO_HTTP_MAX_ATTEMPTS,
    OPERATOR_ZENODO_API_BASE,
    OPERATOR_OPENREVIEW_HTTP_MAX_ATTEMPTS,
    OPERATOR_ZENODO_STAGING_DIR,
    OPERATOR_ZENODO_UPLOAD_ALLOWLIST,
    OPERATOR_SYNDICATION_TEMPLATE_PROFILE,
    OPERATOR_NEWS_PUBLISH_ARMED,
    OPERATOR_NEWS_SITE_BASE_URL,
    OPERATOR_NEWS_RSS_FEED_PATH,
    "VOX_DB_URL",
    "VOX_DB_TOKEN",
    "VOX_ACCOUNT_ID",
    "VOX_MODEL",
    "VOX_BUDGET_USD",
    "VOX_DATA_DIR",
    "VOX_MCP_BINARY",
    "VOX_GAMIFY_ENABLED",
    "VOX_GAMIFY_MODE",
    "VOX_WEB_RUN_MODE",
    "VOX_WEB_TANSTACK_START",
    "VOX_MESH_ENABLED",
    "VOX_MESH_MODE",
    "VOX_MESH_NODE_ID",
    "VOX_MESH_LABELS",
    "VOX_MESH_CONTROL_ADDR",
    "VOX_MESH_REGISTRY_PATH",
    "VOX_MESH_ADVERTISE_GPU",
    "VOX_MESH_SCOPE_ID",
    "VOX_MESH_BOOTSTRAP_TOKEN",
    "VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS",
    "VOX_MESH_SERVER_STALE_PRUNE_MS",
    "VOX_MESH_A2A_MAX_MESSAGES",
    "VOX_MESH_A2A_LEASE_MS",
    "VOX_ORCHESTRATOR_MESH_CONTROL_URL",
    "VOX_OPENCLAW_URL",
    "VOX_OPENCLAW_WS_URL",
    "OPENROUTER_MODEL",
    "OPENAI_MODEL",
    "OPENAI_BASE_URL",
    "GEMINI_MODEL",
    "OLLAMA_URL",
    "OLLAMA_MODEL",
    "VOX_ORCHESTRATOR_PLAN_LLM_SYNTHESIS",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendMode {
    Auto,
    EnvOnly,
    Infisical,
    Vault,
    VoxCloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum CutoverPhase {
    #[default]
    Shadow,
    Canary,
    Enforce,
    Decommission,
}

impl CutoverPhase {
    #[must_use]
    fn from_env() -> Self {
        match std::env::var(crate::OPERATOR_CLAVIS_CUTOVER_PHASE)
            .or_else(|_| std::env::var(crate::OPERATOR_CLAVIS_MIGRATION_PHASE))
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("shadow") => Self::Shadow,
            Some("canary") => Self::Canary,
            Some("enforce") => Self::Enforce,
            Some("decommission") => Self::Decommission,
            _ => Self::Shadow,
        }
    }

    #[must_use]
    const fn legacy_sources_allowed(self, profile: ResolveProfile) -> bool {
        match self {
            CutoverPhase::Shadow => true,
            CutoverPhase::Canary => !profile.is_strict(),
            CutoverPhase::Enforce | CutoverPhase::Decommission => false,
        }
    }

    #[must_use]
    const fn force_vox_cloud_backend(self) -> bool {
        matches!(self, CutoverPhase::Decommission)
    }
}

impl BackendMode {
    #[must_use]
    pub fn from_env() -> Self {
        match std::env::var(crate::OPERATOR_CLAVIS_BACKEND)
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("env_only") | Some("env") => Self::EnvOnly,
            Some("infisical") => Self::Infisical,
            Some("vault") => Self::Vault,
            Some("vox_cloud") | Some("voxcloud") => Self::VoxCloud,
            _ => Self::Auto,
        }
    }
}

fn resolve_with_backend<B: backend::SecretBackend>(
    backend: B,
    id: SecretId,
    options: ResolveOptions,
) -> ResolvedSecret {
    SecretResolver::new(backend).resolve(id, &options)
}

#[must_use]
pub fn resolve_secret(id: SecretId) -> ResolvedSecret {
    let profile = resolve_profile_from_env();
    let phase = CutoverPhase::from_env();
    let legacy_allowed = phase.legacy_sources_allowed(profile);
    let cloudless_options = ResolveOptions {
        include_env: legacy_allowed,
        include_auth_json: legacy_allowed,
        include_populi_env: legacy_allowed,
        profile,
    };
    let default_options = ResolveOptions {
        include_env: legacy_allowed,
        include_auth_json: legacy_allowed,
        include_populi_env: legacy_allowed,
        profile,
    };
    if phase.force_vox_cloud_backend() {
        return resolve_vox_cloud(id, cloudless_options);
    }
    match BackendMode::from_env() {
        BackendMode::EnvOnly => resolve_with_backend(backend::NoopBackend, id, default_options),
        BackendMode::Infisical => resolve_infisical(id, profile),
        BackendMode::Vault => resolve_vault(id, profile),
        BackendMode::VoxCloud => resolve_vox_cloud(id, cloudless_options),
        BackendMode::Auto => {
            let prefer_vault = std::env::var(crate::OPERATOR_CLAVIS_AUTO_PREFER_VAULT)
                .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(false);

            if prefer_vault
                || std::env::var(crate::OPERATOR_CLAVIS_AUTO_VAULT).is_ok()
                || std::env::var(crate::OPERATOR_CLAVIS_VAULT_URL).is_ok()
                || std::env::var(crate::OPERATOR_CLAVIS_VAULT_PATH).is_ok()
            {
                return resolve_vox_cloud(id, cloudless_options);
            }

            // Legacy compat check for Auto trigger (mirroring vox_vault's compat check).
            // NOTE: Do NOT add VOX_DB_URL here—Codex and Vault are separate planes (env-vars.md).
            // This fallback is deprecated and will be removed once users migrate to explicit Clavis signals.
            let legacy_turso_fallback = legacy_allowed && (std::env::var("VOX_TURSO_URL").is_ok() || std::env::var("TURSO_URL").is_ok());
            if legacy_turso_fallback {
                eprintln!("warning: Clavis is falling back to vox_cloud based on legacy VOX_TURSO_URL signal. This is DEPRECATED.");
                eprintln!("remediation: Set VOX_CLAVIS_VAULT_URL or VOX_CLAVIS_AUTO_VAULT instead.");
                return resolve_vox_cloud(id, cloudless_options);
            }

            if std::env::var(crate::OPERATOR_INFISICAL_TOKEN).is_ok()
                || std::env::var(crate::OPERATOR_INFISICAL_SERVICE_TOKEN).is_ok()
            {
                return resolve_infisical(id, profile);
            }
            if std::env::var(crate::OPERATOR_VAULT_ADDR).is_ok() && std::env::var(crate::OPERATOR_VAULT_TOKEN).is_ok() {
                return resolve_vault(id, profile);
            }
            // fallback to cloud automatically if keyring has a master key with a set value
            if let Ok(entry) = keyring::Entry::new("vox-clavis-vault", "master") {
                if entry.get_password().is_ok() {
                    return resolve_vox_cloud(id, cloudless_options);
                }
            }
            resolve_with_backend(backend::NoopBackend, id, default_options)
        }
    }
}

fn resolve_vox_cloud(id: SecretId, options: ResolveOptions) -> ResolvedSecret {
    match backend::vox_vault::VoxCloudBackend::new() {
        Ok(backend) => resolve_with_backend(backend, id, options),
        Err(e) => resolve_with_backend(
            backend::UnavailableBackend {
                reason: format!("VoxCloud backend failed to init: {}", e),
            },
            id,
            options,
        ),
    }
}

#[must_use]
pub fn resolve_env_only(id: SecretId) -> ResolvedSecret {
    SecretResolver::new(backend::NoopBackend).resolve(id, &ResolveOptions::default())
}

fn resolve_infisical(id: SecretId, profile: ResolveProfile) -> ResolvedSecret {
    #[cfg(feature = "clavis-infisical")]
    {
        return resolve_with_backend(
            backend::infisical::InfisicalBackend,
            id,
            ResolveOptions {
                include_env: true,
                include_auth_json: true,
                include_populi_env: true,
                profile,
            },
        );
    }
    #[cfg(not(feature = "clavis-infisical"))]
    {
        resolve_with_backend(
            backend::UnavailableBackend {
                reason: "clavis-infisical feature is not enabled".to_string(),
            },
            id,
            ResolveOptions {
                include_env: true,
                include_auth_json: true,
                include_populi_env: true,
                profile,
            },
        )
    }
}

fn resolve_vault(id: SecretId, profile: ResolveProfile) -> ResolvedSecret {
    #[cfg(feature = "clavis-vault")]
    {
        return resolve_with_backend(
            backend::vault::VaultBackend,
            id,
            ResolveOptions {
                include_env: true,
                include_auth_json: true,
                include_populi_env: true,
                profile,
            },
        );
    }
    #[cfg(not(feature = "clavis-vault"))]
    {
        resolve_with_backend(
            backend::UnavailableBackend {
                reason: "clavis-vault feature is not enabled".to_string(),
            },
            id,
            ResolveOptions {
                include_env: true,
                include_auth_json: true,
                include_populi_env: true,
                profile,
            },
        )
    }
}

fn resolve_profile_from_env() -> ResolveProfile {
    match std::env::var("VOX_CLAVIS_PROFILE")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("ci") | Some("cistrict") | Some("ci_strict") => ResolveProfile::CiStrict,
        Some("prod") | Some("prodstrict") | Some("prod_strict") => ResolveProfile::ProdStrict,
        Some("hardcut") | Some("hard_cut") | Some("hard_cut_strict") | Some("hardcutstrict") => {
            ResolveProfile::HardCutStrict
        }
        _ => ResolveProfile::DevLenient,
    }
}

pub fn set_registry_token(
    registry: &str,
    token: &str,
    username: Option<String>,
) -> Result<std::path::PathBuf, SecretError> {
    sources::auth_json::write_registry_token(registry, token, username)
}

#[must_use]
pub fn get_registry_token(registry: &str) -> Option<String> {
    sources::auth_json::read_registry_token(registry)
        .map(|(s, _)| secrecy::ExposeSecret::expose_secret(&s).to_string())
}

pub fn migrate_auth_store_to_secure_store() -> Result<usize, SecretError> {
    sources::auth_json::migrate_to_secure_store()
}
