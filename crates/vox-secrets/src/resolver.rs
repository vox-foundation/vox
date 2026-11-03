use crate::backend::SecretBackend;
use crate::spec::{SecretId, SecretSpec};
use crate::types::{ResolutionStatus, ResolvedSecret, SecretSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResolveProfile {
    #[default]
    DevLenient,
    CiStrict,
    ProdStrict,
    HardCutStrict,
}

impl ResolveProfile {
    #[must_use]
    pub const fn is_strict(self) -> bool {
        matches!(
            self,
            ResolveProfile::CiStrict | ResolveProfile::ProdStrict | ResolveProfile::HardCutStrict
        )
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ResolveProfile::DevLenient => "dev",
            ResolveProfile::CiStrict => "ci",
            ResolveProfile::ProdStrict => "prod",
            ResolveProfile::HardCutStrict => "hardcut",
        }
    }
}

#[derive(Clone)]
pub struct ResolveOptions {
    pub include_env: bool,
    pub include_auth_json: bool,
    pub include_populi_env: bool,
    pub profile: ResolveProfile,
    pub caller_context: String,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            include_env: true,
            include_auth_json: true,
            include_populi_env: true,
            profile: ResolveProfile::DevLenient,
            caller_context: "process".to_string(),
        }
    }
}

pub struct SecretResolver<B: SecretBackend> {
    backend: B,
}

impl<B: SecretBackend> SecretResolver<B> {
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    #[must_use]
    pub fn resolve(&self, id: SecretId, opts: &ResolveOptions) -> ResolvedSecret {
        let spec = id.spec();
        let resolved = self.resolve_spec(*spec, opts);
        self.maybe_audit(&resolved, opts);
        resolved
    }

    fn maybe_audit(&self, resolved: &ResolvedSecret, opts: &ResolveOptions) {
        let profile = opts.profile;
        let audit_enabled = match profile {
            ResolveProfile::ProdStrict | ResolveProfile::HardCutStrict => true,
            _ => std::env::var("VOX_SECRETS_AUDIT_LOG")
                .ok()
                .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(false),
        };

        if audit_enabled {
            let spec = resolved.id.spec();
            let _ = self.backend.write_audit_log(
                spec.canonical_env,
                &format!("{:?}", resolved.status),
                resolved.source.map(|s| match s {
                    SecretSource::EnvCanonical => "EnvCanonical",
                    SecretSource::EnvAlias => "EnvAlias",
                    SecretSource::SecureStore => "SecureStore",
                    SecretSource::AuthJson => "AuthJson",
                    SecretSource::LegacyAuthToken => "LegacyAuthToken",
                    SecretSource::PopuliEnv => "PopuliEnv",
                    SecretSource::ExternalBackend => "ExternalBackend",
                }),
                &format!("{:?}", profile),
                &opts.caller_context,
                resolved.detail.as_deref(),
            );
        }
    }

    fn resolve_spec(&self, spec: SecretSpec, opts: &ResolveOptions) -> ResolvedSecret {
        let metadata = spec.id.metadata();
        let strict_profile = opts.profile.is_strict();
        if opts.include_env {
            let (env_value, env_source, env_status) = crate::sources::env::resolve_env(spec);
            if env_value.is_some() {
                if strict_profile && matches!(env_status, ResolutionStatus::DeprecatedAliasUsed) {
                    return ResolvedSecret {
                        id: spec.id,
                        value: None,
                        source: env_source,
                        status: ResolutionStatus::RejectedLegacyAlias,
                        remediation: spec.remediation,
                        detail: Some("deprecated alias is blocked in strict profile".to_string()),
                    };
                }
                if let Some(src) = env_source
                    && !metadata.allows_source(src, strict_profile)
                {
                    return ResolvedSecret {
                        id: spec.id,
                        value: None,
                        source: Some(src),
                        status: ResolutionStatus::RejectedSourcePolicy,
                        remediation: spec.remediation,
                        detail: Some(format!("source {:?} blocked by strict source policy", src)),
                    };
                }
                // Surface env-vs-auth.json conflicts so users who ran
                // `vox secrets set <reg> …` aren't silently overridden by
                // a stale env var. Detail is informational; env keeps
                // precedence (operator-override convention).
                let mut detail: Option<String> = None;
                if let Some(reg) = spec.auth_registry
                    && let Some((auth_value, _)) =
                        crate::sources::auth_json::read_registry_token(reg)
                    && let (Some(env_v), Some(auth_v)) = (
                        env_value.as_ref().map(secrecy::ExposeSecret::expose_secret),
                        Some(secrecy::ExposeSecret::expose_secret(&auth_value)),
                    )
                    && env_v != auth_v
                {
                    detail = Some(format!(
                        "env `{}` is shadowing a different value stored via \
                         `vox secrets set {reg}` — unset the env or run \
                         `vox secrets set {reg} <token>` again to align",
                        spec.canonical_env
                    ));
                }
                return ResolvedSecret {
                    id: spec.id,
                    value: env_value,
                    source: env_source,
                    status: env_status,
                    remediation: spec.remediation,
                    detail,
                };
            }
            if matches!(env_status, ResolutionStatus::InvalidEmpty) {
                return ResolvedSecret {
                    id: spec.id,
                    value: None,
                    source: None,
                    status: ResolutionStatus::InvalidEmpty,
                    remediation: spec.remediation,
                    detail: None,
                };
            }
        }

        match self.backend.resolve(
            spec.id,
            spec,
            Some(opts.profile.as_str()),
            &opts.caller_context,
        ) {
            Ok(Some(v)) => {
                if !metadata.allows_source(SecretSource::ExternalBackend, strict_profile) {
                    return ResolvedSecret {
                        id: spec.id,
                        value: None,
                        source: Some(SecretSource::ExternalBackend),
                        status: ResolutionStatus::RejectedSourcePolicy,
                        remediation: spec.remediation,
                        detail: Some(
                            "external backend blocked by strict source policy".to_string(),
                        ),
                    };
                }
                return ResolvedSecret {
                    id: spec.id,
                    value: Some(v),
                    source: Some(SecretSource::ExternalBackend),
                    status: ResolutionStatus::Present,
                    remediation: spec.remediation,
                    detail: None,
                };
            }
            Ok(None) => {}
            Err(e) => {
                return ResolvedSecret {
                    id: spec.id,
                    value: None,
                    source: None,
                    status: ResolutionStatus::BackendUnavailable,
                    remediation: spec.remediation,
                    detail: Some(e.to_string()),
                };
            }
        }

        if opts.include_auth_json
            && let Some(reg) = spec.auth_registry
            && let Some((v, source)) = crate::sources::auth_json::read_registry_token(reg)
        {
            if !metadata.allows_source(source, strict_profile) {
                return ResolvedSecret {
                    id: spec.id,
                    value: None,
                    source: Some(source),
                    status: ResolutionStatus::RejectedSourcePolicy,
                    remediation: spec.remediation,
                    detail: Some(format!(
                        "source {:?} blocked by strict source policy",
                        source
                    )),
                };
            }
            return ResolvedSecret {
                id: spec.id,
                value: Some(v),
                source: Some(source),
                status: ResolutionStatus::Present,
                remediation: spec.remediation,
                detail: None,
            };
        }

        if opts.include_populi_env
            && crate::spec::secret_reads_populi_env_file(spec.id)
            && let Some((v, source)) =
                crate::sources::populi_env::read_populi_env_key(spec.canonical_env)
        {
            if !metadata.allows_source(source, strict_profile) {
                return ResolvedSecret {
                    id: spec.id,
                    value: None,
                    source: Some(source),
                    status: ResolutionStatus::RejectedSourcePolicy,
                    remediation: spec.remediation,
                    detail: Some(format!(
                        "source {:?} blocked by strict source policy",
                        source
                    )),
                };
            }
            return ResolvedSecret {
                id: spec.id,
                value: Some(v),
                source: Some(source),
                status: ResolutionStatus::Present,
                remediation: spec.remediation,
                detail: None,
            };
        }

        let status = if spec.policy.required {
            ResolutionStatus::MissingRequired
        } else {
            ResolutionStatus::MissingOptional
        };
        ResolvedSecret {
            id: spec.id,
            value: None,
            source: None,
            status,
            remediation: spec.remediation,
            detail: None,
        }
    }

    /// Resolve a portable `clavis://<provider>/<key>` URI reference (`CLAVIS-02`).
    pub fn resolve_clavis_uri(
        &self,
        uri: &str,
        opts: &ResolveOptions,
    ) -> Result<ResolvedSecret, String> {
        let id = parse_clavis_uri(uri)?;
        Ok(self.resolve(id, opts))
    }
}

/// Parse a portable `clavis://<provider>/<key>` URI into its canonical [`SecretId`] (`CLAVIS-02`).
///
/// Supported forms:
/// - `clavis://<provider>/<key>` (e.g., `clavis://openrouter/api_key`, `clavis://gemini/api_key`, `clavis://github/token`)
/// - `clavis://env/<CANONICAL_OR_ALIAS_ENV>` (e.g., `clavis://env/OPENROUTER_API_KEY`)
/// - `clavis://vox/<CANONICAL_ENV_LOWER_OR_UPPER>` (e.g., `clavis://vox/vox_mesh_token`)
pub fn parse_clavis_uri(uri: &str) -> Result<SecretId, String> {
    let trimmed = uri.trim();
    let Some(rest) = trimmed.strip_prefix("clavis://") else {
        return Err(format!(
            "invalid Clavis URI `{trimmed}`: must start with `clavis://`"
        ));
    };
    let Some((provider, key)) = rest.split_once('/') else {
        return Err(format!(
            "invalid Clavis URI `{trimmed}`: expected `clavis://<provider>/<key>`"
        ));
    };
    let provider_norm = provider.trim().to_ascii_lowercase();
    let key_norm = key.trim();
    if provider_norm.is_empty() || key_norm.is_empty() {
        return Err(format!(
            "invalid Clavis URI `{trimmed}`: provider and key must be non-empty"
        ));
    }

    // 1. Direct provider/key shorthand mappings for the most common developer providers.
    let direct = match (
        provider_norm.as_str(),
        key_norm.to_ascii_lowercase().as_str(),
    ) {
        ("openrouter", "api_key" | "key" | "openrouter_api_key") => {
            Some(SecretId::OpenRouterApiKey)
        }
        ("gemini" | "google", "api_key" | "key" | "gemini_api_key") => Some(SecretId::GeminiApiKey),
        ("openai", "api_key" | "key" | "openai_api_key") => Some(SecretId::OpenaiApiKey),
        ("anthropic", "api_key" | "key" | "anthropic_api_key") => Some(SecretId::AnthropicApiKey),
        ("groq", "api_key" | "key" | "groq_api_key") => Some(SecretId::GroqApiKey),
        ("deepseek", "api_key" | "key" | "deepseek_api_key") => Some(SecretId::DeepSeekApiKey),
        ("mistral", "api_key" | "key" | "mistral_api_key") => Some(SecretId::MistralApiKey),
        ("cerebras", "api_key" | "key" | "cerebras_api_key") => Some(SecretId::CerebrasApiKey),
        ("sambanova", "api_key" | "key" | "sambanova_api_key") => Some(SecretId::SambaNovaApiKey),
        ("together", "api_key" | "key" | "together_api_key") => Some(SecretId::TogetherApiKey),
        ("huggingface" | "hf", "token" | "api_key" | "hf_token") => {
            Some(SecretId::HuggingFaceToken)
        }
        ("tavily", "api_key" | "key" | "tavily_api_key") => Some(SecretId::TavilyApiKey),
        ("github" | "forge", "token" | "forge_token" | "github_token") => {
            Some(SecretId::ForgeToken)
        }
        ("runpod", "api_key" | "key" | "vox_runpod_api_key") => Some(SecretId::VoxRunpodApiKey),
        ("vast", "api_key" | "key" | "vox_vast_api_key") => Some(SecretId::VoxVastApiKey),
        ("turso" | "db", "url" | "vox_db_url") => Some(SecretId::VoxDbUrl),
        ("turso" | "db", "token" | "vox_db_token") => Some(SecretId::VoxDbToken),
        ("mesh" | "populi", "token" | "vox_mesh_token") => Some(SecretId::VoxMeshToken),
        _ => None,
    };
    if let Some(id) = direct {
        return Ok(id);
    }

    // 2. Fallback: match against canonical_env, aliases, deprecated_aliases, or `<PROVIDER>_<KEY>`.
    let candidate_upper = key_norm.to_ascii_uppercase().replace('-', "_");
    let combined_upper = format!(
        "{}_{}",
        provider_norm.to_ascii_uppercase().replace('-', "_"),
        candidate_upper
    );
    for spec in crate::spec::all_specs() {
        if spec.canonical_env == candidate_upper
            || spec.canonical_env == combined_upper
            || spec
                .aliases
                .iter()
                .any(|a| *a == candidate_upper || *a == combined_upper)
            || spec
                .deprecated_aliases
                .iter()
                .any(|a| *a == candidate_upper || *a == combined_upper)
            || spec
                .auth_registry
                .is_some_and(|r| r.eq_ignore_ascii_case(&provider_norm))
        {
            return Ok(spec.id);
        }
    }

    Err(format!(
        "unrecognized Clavis URI `{trimmed}`: no managed SecretSpec matched provider `{provider_norm}` and key `{key_norm}`"
    ))
}

#[cfg(test)]
mod clavis_uri_tests {
    use super::*;

    #[test]
    fn parse_clavis_uri_resolves_provider_shorthands_and_env_names() {
        assert_eq!(
            parse_clavis_uri("clavis://openrouter/api_key").unwrap(),
            SecretId::OpenRouterApiKey
        );
        assert_eq!(
            parse_clavis_uri("clavis://gemini/api_key").unwrap(),
            SecretId::GeminiApiKey
        );
        assert_eq!(
            parse_clavis_uri("clavis://env/ANTHROPIC_API_KEY").unwrap(),
            SecretId::AnthropicApiKey
        );
        assert_eq!(
            parse_clavis_uri("clavis://vox/mesh_token").unwrap(),
            SecretId::VoxMeshToken
        );
    }

    #[test]
    fn parse_clavis_uri_rejects_malformed_or_unknown_refs() {
        assert!(parse_clavis_uri("http://openrouter/api_key").is_err());
        assert!(parse_clavis_uri("clavis://openrouter").is_err());
        assert!(parse_clavis_uri("clavis://unknown_provider/nonexistent_key").is_err());
    }
}
