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
pub use spec::{
    Capability, Profile, RequirementMode, RequirementSet, SecretBundle, SecretId, SecretSpec,
    Workflow, WorkflowRequirements, all_bundle_doc_names, all_specs, capabilities_for_secret,
    managed_secret_env_names, required_for, required_for_profile, requirements_for_bundle,
    requirements_for_profile, requirements_for_profile_mode,
};
pub use types::{ResolutionStatus, ResolvedSecret, SecretSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendMode {
    Auto,
    EnvOnly,
    Infisical,
    Vault,
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
            _ => Self::Auto,
        }
    }
}

fn resolve_with_backend<B: backend::SecretBackend>(backend: B, id: SecretId) -> ResolvedSecret {
    SecretResolver::new(backend).resolve(
        id,
        &ResolveOptions {
            include_auth_json: true,
            include_populi_env: true,
        },
    )
}

#[must_use]
pub fn resolve_secret(id: SecretId) -> ResolvedSecret {
    match BackendMode::from_env() {
        BackendMode::EnvOnly => resolve_with_backend(backend::NoopBackend, id),
        BackendMode::Infisical => resolve_infisical(id),
        BackendMode::Vault => resolve_vault(id),
        BackendMode::Auto => {
            if std::env::var("INFISICAL_TOKEN").is_ok()
                || std::env::var("INFISICAL_SERVICE_TOKEN").is_ok()
            {
                return resolve_infisical(id);
            }
            if std::env::var("VAULT_ADDR").is_ok() && std::env::var("VAULT_TOKEN").is_ok() {
                return resolve_vault(id);
            }
            resolve_with_backend(backend::NoopBackend, id)
        }
    }
}

#[must_use]
pub fn resolve_env_only(id: SecretId) -> ResolvedSecret {
    SecretResolver::new(backend::NoopBackend).resolve(id, &ResolveOptions::default())
}

fn resolve_infisical(id: SecretId) -> ResolvedSecret {
    #[cfg(feature = "clavis-infisical")]
    {
        return resolve_with_backend(backend::infisical::InfisicalBackend, id);
    }
    #[cfg(not(feature = "clavis-infisical"))]
    {
        resolve_with_backend(
            backend::UnavailableBackend {
                reason: "clavis-infisical feature is not enabled".to_string(),
            },
            id,
        )
    }
}

fn resolve_vault(id: SecretId) -> ResolvedSecret {
    #[cfg(feature = "clavis-vault")]
    {
        return resolve_with_backend(backend::vault::VaultBackend, id);
    }
    #[cfg(not(feature = "clavis-vault"))]
    {
        resolve_with_backend(
            backend::UnavailableBackend {
                reason: "clavis-vault feature is not enabled".to_string(),
            },
            id,
        )
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
