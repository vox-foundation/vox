use crate::backend::SecretBackend;
use crate::spec::{SecretId, SecretSpec};
use crate::types::{ResolutionStatus, ResolvedSecret, SecretSource};

#[derive(Default)]
pub struct ResolveOptions {
    pub include_auth_json: bool,
    pub include_populi_env: bool,
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
        let (env_value, env_source, env_status) = crate::sources::env::resolve_env(spec);
        if env_value.is_some() {
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

        match self.backend.resolve(spec.id, spec) {
            Ok(Some(v)) => {
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
            && spec.id == SecretId::VoxMeshToken
            && let Some((v, source)) = crate::sources::populi_env::read_mesh_token_from_populi_env()
        {
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
