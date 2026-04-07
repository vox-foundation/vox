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
}

#[derive(Clone, Copy)]
pub struct ResolveOptions {
    pub include_env: bool,
    pub include_auth_json: bool,
    pub include_populi_env: bool,
    pub profile: ResolveProfile,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            include_env: true,
            include_auth_json: true,
            include_populi_env: true,
            profile: ResolveProfile::DevLenient,
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
        self.resolve_spec(spec, opts)
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
                return ResolvedSecret {
                    id: spec.id,
                    value: env_value,
                    source: env_source,
                    status: env_status,
                    remediation: spec.remediation,
                    detail: None,
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

        match self.backend.resolve(spec.id, spec) {
            Ok(Some(v)) => {
                if !metadata.allows_source(SecretSource::ExternalBackend, strict_profile) {
                    return ResolvedSecret {
                        id: spec.id,
                        value: None,
                        source: Some(SecretSource::ExternalBackend),
                        status: ResolutionStatus::RejectedSourcePolicy,
                        remediation: spec.remediation,
                        detail: Some("external backend blocked by strict source policy".to_string()),
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
                    detail: Some(format!("source {:?} blocked by strict source policy", source)),
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
                    detail: Some(format!("source {:?} blocked by strict source policy", source)),
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
}
