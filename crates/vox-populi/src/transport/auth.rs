use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use subtle::ConstantTimeEq;

/// Role implied by the bearer token presented to the populi control plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopuliBearerRole {
    /// Legacy / operator token (`VOX_MESH_TOKEN`).
    Mesh,
    /// Worker-scoped token (`VOX_MESH_WORKER_TOKEN`).
    Worker,
    /// Submitter token for workload delivery (`VOX_MESH_SUBMITTER_TOKEN`).
    Submitter,
    /// Admin token (`VOX_MESH_ADMIN_TOKEN`).
    Admin,
}

/// Request auth context attached after the bearer middleware runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopuliAuthContext {
    /// Control plane has no bearer requirement (tests / explicit open).
    FullAccess,
    /// Authenticated with a classified role.
    Role(PopuliBearerRole),
}

/// Join / heartbeat / leave / list / inbox / ack (not deliver-only submitter tokens).
#[must_use]
pub fn auth_allows_worker_plane(ctx: PopuliAuthContext) -> bool {
    match ctx {
        PopuliAuthContext::FullAccess => true,
        PopuliAuthContext::Role(PopuliBearerRole::Mesh)
        | PopuliAuthContext::Role(PopuliBearerRole::Worker)
        | PopuliAuthContext::Role(PopuliBearerRole::Admin) => true,
        PopuliAuthContext::Role(PopuliBearerRole::Submitter) => false,
    }
}

/// A2A deliver (submitter + legacy mesh/admin).
#[must_use]
pub fn auth_allows_deliver(ctx: PopuliAuthContext) -> bool {
    match ctx {
        PopuliAuthContext::FullAccess => true,
        PopuliAuthContext::Role(PopuliBearerRole::Mesh)
        | PopuliAuthContext::Role(PopuliBearerRole::Submitter)
        | PopuliAuthContext::Role(PopuliBearerRole::Admin) => true,
        PopuliAuthContext::Role(PopuliBearerRole::Worker) => false,
    }
}

/// Admin-only routes (mesh legacy token or `VOX_MESH_ADMIN_TOKEN`).
#[must_use]
pub fn auth_allows_admin_route(ctx: PopuliAuthContext) -> bool {
    match ctx {
        PopuliAuthContext::FullAccess => true,
        PopuliAuthContext::Role(PopuliBearerRole::Mesh)
        | PopuliAuthContext::Role(PopuliBearerRole::Admin) => true,
        PopuliAuthContext::Role(PopuliBearerRole::Worker)
        | PopuliAuthContext::Role(PopuliBearerRole::Submitter) => false,
    }
}

/// Resolved populi bearer material from Clavis / env (captured at router build time).
#[derive(Clone, Debug)]
pub struct PopuliMeshAuthRuntime {
    mesh: Option<Arc<str>>,
    worker: Option<Arc<str>>,
    submitter: Option<Arc<str>>,
    admin: Option<Arc<str>>,
    /// When set, HS256 JWT bearer is accepted (`role`, `jti`, `exp` claims).
    pub(crate) jwt_hmac: Option<Arc<str>>,
}

impl Default for PopuliMeshAuthRuntime {
    fn default() -> Self {
        Self {
            mesh: None,
            worker: None,
            submitter: None,
            admin: None,
            jwt_hmac: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct MeshJwtClaims {
    role: String,
    jti: String,
    exp: u64,
}

fn bearer_looks_like_jwt(token: &str) -> bool {
    let mut n = 0usize;
    for p in token.split('.') {
        if p.is_empty() {
            return false;
        }
        n += 1;
    }
    n == 3
}

fn mesh_jwt_role_from_claim(s: &str) -> Option<PopuliBearerRole> {
    match s.trim().to_ascii_lowercase().as_str() {
        "mesh" => Some(PopuliBearerRole::Mesh),
        "worker" => Some(PopuliBearerRole::Worker),
        "submitter" => Some(PopuliBearerRole::Submitter),
        "admin" => Some(PopuliBearerRole::Admin),
        _ => None,
    }
}

impl PopuliMeshAuthRuntime {
    /// Test / single-token mode: only [`PopuliBearerRole::Mesh`] matches `token`.
    #[must_use]
    pub fn legacy_mesh_token_only(token: impl AsRef<str>) -> Self {
        let t = token.as_ref().trim();
        Self {
            mesh: if t.is_empty() {
                None
            } else {
                Some(Arc::from(t.to_string().into_boxed_str()))
            },
            ..Default::default()
        }
    }

    /// JWT-only (or combined) auth for tests / embedding when secrets are not read from Clavis.
    #[must_use]
    pub fn with_jwt_hmac_only(secret: impl Into<String>) -> Self {
        let s = secret.into();
        let t = s.trim().to_string();
        Self {
            jwt_hmac: if t.is_empty() {
                None
            } else {
                Some(Arc::from(t.into_boxed_str()))
            },
            ..Default::default()
        }
    }

    /// Read tokens via [`vox_clavis::resolve_secret`] (same precedence as other mesh callers).
    #[must_use]
    pub fn from_env() -> Self {
        let mesh = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshToken)
            .expose()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| Arc::from(s.to_string().into_boxed_str()));
        let worker = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshWorkerToken)
            .expose()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| Arc::from(s.to_string().into_boxed_str()));
        let submitter = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshSubmitterToken)
            .expose()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| Arc::from(s.to_string().into_boxed_str()));
        let admin = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshAdminToken)
            .expose()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| Arc::from(s.to_string().into_boxed_str()));
        let jwt_hmac = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshJwtHmacSecret)
            .expose()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| Arc::from(s.to_string().into_boxed_str()));
        Self {
            mesh,
            worker,
            submitter,
            admin,
            jwt_hmac,
        }
    }

    /// Single-token legacy deployments: only `VOX_MESH_TOKEN` is configured.
    #[must_use]
    pub fn legacy_shared_mesh_only(&self) -> bool {
        self.mesh.is_some()
            && self.worker.is_none()
            && self.submitter.is_none()
            && self.admin.is_none()
    }

    /// Any bearer requirement is active.
    #[must_use]
    pub fn requires_bearer(&self) -> bool {
        self.mesh.is_some()
            || self.worker.is_some()
            || self.submitter.is_some()
            || self.admin.is_some()
            || self.jwt_hmac.is_some()
    }

    /// Classify a presented bearer token, if it matches a configured secret.
    #[must_use]
    pub fn classify_bearer(&self, presented: impl AsRef<str>) -> Option<PopuliBearerRole> {
        let t = presented.as_ref().trim();
        if t.is_empty() {
            return None;
        }
        if self
            .mesh
            .as_ref()
            .is_some_and(|expected| bearer_token_eq(expected, t))
        {
            return Some(PopuliBearerRole::Mesh);
        }
        if self
            .worker
            .as_ref()
            .is_some_and(|expected| bearer_token_eq(expected, t))
        {
            return Some(PopuliBearerRole::Worker);
        }
        if self
            .submitter
            .as_ref()
            .is_some_and(|expected| bearer_token_eq(expected, t))
        {
            return Some(PopuliBearerRole::Submitter);
        }
        if self
            .admin
            .as_ref()
            .is_some_and(|expected| bearer_token_eq(expected, t))
        {
            return Some(PopuliBearerRole::Admin);
        }
        None
    }

    /// Validate an HS256 mesh JWT (`role`, `jti`, `exp`) and apply a bounded `jti` replay table (unix **seconds**).
    #[must_use]
    pub fn try_authorize_jwt(
        &self,
        token: &str,
        now_unix_secs: u64,
        jti_seen: &mut HashMap<String, u64>,
    ) -> Option<PopuliBearerRole> {
        let secret = self.jwt_hmac.as_deref()?;
        if secret.is_empty() {
            return None;
        }
        if !bearer_looks_like_jwt(token) {
            return None;
        }
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.leeway = 60;
        validation.validate_exp = true;
        let key = jsonwebtoken::DecodingKey::from_secret(secret.as_bytes());
        let data =
            jsonwebtoken::decode::<MeshJwtClaims>(token, &key, &validation).ok()?;
        let claims = data.claims;
        if claims.jti.trim().is_empty() {
            return None;
        }
        jti_seen.retain(|_, exp| *exp > now_unix_secs);
        if jti_seen.contains_key(&claims.jti) {
            return None;
        }
        // Bound replay table to keep memory predictable on long-lived control planes.
        const MAX_JTI: usize = 50_000;
        if jti_seen.len() >= MAX_JTI {
            jti_seen.clear();
            tracing::warn!("populi mesh jwt jti replay table exceeded {MAX_JTI}; cleared");
        }
        jti_seen.insert(claims.jti, claims.exp);
        mesh_jwt_role_from_claim(&claims.role)
    }
}

pub(super) fn populi_control_token_from_env() -> Option<String> {
    vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshToken)
        .expose()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
}

/// Constant-time comparison when lengths match (avoids early return on length for the equal-length case).
pub(super) fn bearer_token_eq(expected: &str, presented: &str) -> bool {
    let a = expected.as_bytes();
    let b = presented.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}
