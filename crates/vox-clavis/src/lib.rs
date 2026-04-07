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
use resolver::{ResolveOptions, SecretResolver};
pub use resolver::ResolveProfile;
pub use spec::{
    Capability, Profile, RequirementMode, RequirementSet, RotationPolicy, SecretBundle, SecretClass,
    SecretId, SecretMaterialKind, SecretMetadata, SecretSpec, Workflow, WorkflowRequirements,
    all_bundle_doc_names, all_specs, capabilities_for_secret, managed_secret_env_names, required_for,
    required_for_profile, requirements_for_bundle, requirements_for_profile,
    requirements_for_profile_mode,
};
pub use types::{ResolutionStatus, ResolvedSecret, SecretSource};

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
        match std::env::var("VOX_CLAVIS_CUTOVER_PHASE")
            .or_else(|_| std::env::var("VOX_CLAVIS_MIGRATION_PHASE"))
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
        match std::env::var("VOX_CLAVIS_BACKEND")
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
    SecretResolver::new(backend).resolve(
        id,
        &options,
    )
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
            if std::env::var("VOX_TURSO_URL").is_ok() {
                return resolve_vox_cloud(id, cloudless_options);
            }
            if std::env::var("INFISICAL_TOKEN").is_ok()
                || std::env::var("INFISICAL_SERVICE_TOKEN").is_ok()
            {
                return resolve_infisical(id, profile);
            }
            if std::env::var("VAULT_ADDR").is_ok() && std::env::var("VAULT_TOKEN").is_ok() {
                return resolve_vault(id, profile);
            }
            // fallback to cloud automatically if keyring has a master key
            if keyring::Entry::new("vox-clavis-vault", "master").is_ok() {
                return resolve_vox_cloud(id, cloudless_options);
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
