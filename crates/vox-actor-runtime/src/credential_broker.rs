//! Zero-trust capability-scoped credential broker for agents and MCP tools (`CLAVIS-05`).
//!
//! Agents and tool invocations never receive raw secret strings in prompts, tool arguments,
//! or LLM-visible context. Instead, [`CredentialBroker`] verifies that the caller holds a
//! matching [`vox_secrets::Capability`] (or explicit [`vox_secrets::SecretId`] grant), produces
//! a [`SealedHeaderInjection`] whose `Debug` and serialized representations are strictly redacted,
//! and scrubs outbound tool results / logs with [`CredentialBroker::scrub_tool_output`].

use std::collections::{BTreeSet, HashSet};
use std::fmt;

use serde::Serialize;
use thiserror::Error;
use vox_secrets::{Capability, SecretId};

/// Errors returned by [`CredentialBroker`] operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CredentialBrokerError {
    /// The caller's [`BrokerAgentGrant`] does not authorize the requested [`SecretId`].
    #[error(
        "agent `{agent_id}` is not authorized to broker secret `{canonical_env}` (missing required capability scope)"
    )]
    UnauthorizedScope {
        /// Identifier of the requesting agent or tool.
        agent_id: String,
        /// Canonical environment name of the requested secret.
        canonical_env: &'static str,
    },
    /// The requested secret is not present in the active Clavis resolution chain.
    #[error("secret `{canonical_env}` is not present in Clavis ({remediation})")]
    SecretUnavailable {
        /// Canonical environment name of the missing secret.
        canonical_env: &'static str,
        /// Actionable remediation hint from Clavis.
        remediation: &'static str,
    },
    /// Header construction failed due to invalid characters in the secret or header name.
    #[error("invalid HTTP header material for `{canonical_env}`")]
    InvalidHeaderMaterial {
        /// Canonical environment name of the secret.
        canonical_env: &'static str,
    },
}

/// Capability and secret-level authorization grant assigned to an agent or MCP tool invocation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BrokerAgentGrant {
    /// Capabilities granted to the agent (matched against `vox_secrets::capabilities_for_secret`).
    pub capabilities: HashSet<Capability>,
    /// Explicit per-secret overrides granted to the agent.
    pub explicit_secrets: BTreeSet<SecretId>,
}

impl BrokerAgentGrant {
    /// Create a grant authorizing a set of [`Capability`] scopes.
    #[must_use]
    pub fn with_capabilities(capabilities: impl IntoIterator<Item = Capability>) -> Self {
        Self {
            capabilities: capabilities.into_iter().collect(),
            explicit_secrets: BTreeSet::new(),
        }
    }

    /// Grant access to a specific [`SecretId`].
    #[must_use]
    pub fn allow_secret(mut self, secret_id: SecretId) -> Self {
        self.explicit_secrets.insert(secret_id);
        self
    }

    /// Check whether this grant authorizes brokering `secret_id`.
    #[must_use]
    pub fn permits(&self, secret_id: SecretId) -> bool {
        if self.explicit_secrets.contains(&secret_id) {
            return true;
        }
        let secret_caps = vox_secrets::capabilities_for_secret(secret_id);
        secret_caps
            .iter()
            .any(|cap| self.capabilities.contains(cap))
    }
}

/// Sealed HTTP header injection token that prevents accidental secret exposure in logs or Debug output.
#[derive(Clone, Serialize)]
pub struct SealedHeaderInjection {
    header_name: String,
    #[serde(skip_serializing)]
    header_value: String,
    redacted_preview: String,
    secret_id: SecretId,
}

impl fmt::Debug for SealedHeaderInjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SealedHeaderInjection")
            .field("header_name", &self.header_name)
            .field("header_value", &self.redacted_preview)
            .field("secret_id", &self.secret_id)
            .finish()
    }
}

impl SealedHeaderInjection {
    /// HTTP header name (for example `"Authorization"`).
    #[must_use]
    pub fn header_name(&self) -> &str {
        &self.header_name
    }

    /// Redacted preview safe for logs and audit telemetry.
    #[must_use]
    pub fn redacted_preview(&self) -> &str {
        &self.redacted_preview
    }

    /// Underlying [`SecretId`] that produced this header injection.
    #[must_use]
    pub const fn secret_id(&self) -> SecretId {
        self.secret_id
    }

    /// Apply the sealed header onto a `reqwest::header::HeaderMap` at the outbound transport boundary.
    pub fn apply_to_headers(
        &self,
        headers: &mut reqwest::header::HeaderMap,
    ) -> Result<(), CredentialBrokerError> {
        let canonical_env = self.secret_id.spec().canonical_env;
        let name = reqwest::header::HeaderName::from_bytes(self.header_name.as_bytes())
            .map_err(|_| CredentialBrokerError::InvalidHeaderMaterial { canonical_env })?;
        let mut value = reqwest::header::HeaderValue::from_str(&self.header_value)
            .map_err(|_| CredentialBrokerError::InvalidHeaderMaterial { canonical_env })?;
        value.set_sensitive(true);
        headers.insert(name, value);
        Ok(())
    }
}

/// Zero-trust credential broker mediating secret injection and output scrubbing for agents.
#[derive(Debug, Default, Clone)]
pub struct CredentialBroker {
    tracked_values: Vec<String>,
}

impl CredentialBroker {
    /// Create a new [`CredentialBroker`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            tracked_values: Vec::new(),
        }
    }

    /// Broker a bearer `Authorization` header from the active Clavis resolution chain.
    pub fn broker_bearer_header(
        &mut self,
        agent_id: &str,
        grant: &BrokerAgentGrant,
        secret_id: SecretId,
    ) -> Result<SealedHeaderInjection, CredentialBrokerError> {
        let spec = secret_id.spec();
        if !grant.permits(secret_id) {
            return Err(CredentialBrokerError::UnauthorizedScope {
                agent_id: agent_id.to_string(),
                canonical_env: spec.canonical_env,
            });
        }
        let resolved = vox_secrets::resolve_secret(secret_id);
        let Some(raw) = resolved.expose() else {
            return Err(CredentialBrokerError::SecretUnavailable {
                canonical_env: spec.canonical_env,
                remediation: spec.remediation,
            });
        };
        self.seal_bearer_value(secret_id, raw, &resolved.redacted())
    }

    /// Broker a bearer `Authorization` header using a pre-resolved secret value (after verifying capability scope).
    pub fn broker_bearer_with_resolved(
        &mut self,
        agent_id: &str,
        grant: &BrokerAgentGrant,
        secret_id: SecretId,
        raw_secret: &str,
    ) -> Result<SealedHeaderInjection, CredentialBrokerError> {
        let spec = secret_id.spec();
        if !grant.permits(secret_id) {
            return Err(CredentialBrokerError::UnauthorizedScope {
                agent_id: agent_id.to_string(),
                canonical_env: spec.canonical_env,
            });
        }
        let prefix: String = raw_secret.chars().take(4).collect();
        let redacted = if raw_secret.len() > 4 {
            format!("{prefix}…[REDACTED]")
        } else {
            "[REDACTED]".to_string()
        };
        self.seal_bearer_value(secret_id, raw_secret, &redacted)
    }

    fn seal_bearer_value(
        &mut self,
        secret_id: SecretId,
        raw_secret: &str,
        redacted_secret: &str,
    ) -> Result<SealedHeaderInjection, CredentialBrokerError> {
        let canonical_env = secret_id.spec().canonical_env;
        let trimmed = raw_secret.trim();
        if trimmed.is_empty() || trimmed.contains(['\r', '\n']) {
            return Err(CredentialBrokerError::InvalidHeaderMaterial { canonical_env });
        }
        if trimmed.len() >= 4 && !self.tracked_values.iter().any(|v| v == trimmed) {
            self.tracked_values.push(trimmed.to_string());
        }
        Ok(SealedHeaderInjection {
            header_name: "Authorization".to_string(),
            header_value: format!("Bearer {trimmed}"),
            redacted_preview: format!("Bearer {redacted_secret}"),
            secret_id,
        })
    }

    /// Scrub any known or brokered secret values from tool outputs, stderr, or telemetry payloads.
    #[must_use]
    pub fn scrub_tool_output(&self, raw_output: &str) -> String {
        let patterns: Vec<&str> = self.tracked_values.iter().map(String::as_str).collect();
        let val = serde_json::Value::String(raw_output.to_string());
        match vox_secrets::redact::redact_secrets_from_value(&val, &patterns) {
            serde_json::Value::String(scrubbed) => scrubbed,
            _ => raw_output.to_string(),
        }
    }

    /// Check whether `text` contains any known or brokered secret material.
    #[must_use]
    pub fn contains_leaked_secret(&self, text: &str) -> bool {
        let patterns: Vec<&str> = self.tracked_values.iter().map(String::as_str).collect();
        vox_secrets::redact::contains_secret_material(text, &patterns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_agent_grant_is_rejected() {
        let mut broker = CredentialBroker::new();
        let grant = BrokerAgentGrant::with_capabilities([Capability::GpuCloud]);
        let err = broker
            .broker_bearer_with_resolved(
                "agent-worker-1",
                &grant,
                SecretId::OpenRouterApiKey,
                "sk-or-v1-secret-token-value-12345",
            )
            .expect_err("GpuCloud capability must not authorize OpenRouterApiKey");
        assert_eq!(
            err,
            CredentialBrokerError::UnauthorizedScope {
                agent_id: "agent-worker-1".to_string(),
                canonical_env: "OPENROUTER_API_KEY",
            }
        );
    }

    #[test]
    fn authorized_capability_brokers_sealed_header_and_scrubs_output() {
        let mut broker = CredentialBroker::new();
        let grant = BrokerAgentGrant::with_capabilities([Capability::ChatCloudPrimary]);
        let raw_secret = "sk-or-v1-super-sensitive-99887766";
        let sealed = broker
            .broker_bearer_with_resolved(
                "agent-chat-1",
                &grant,
                SecretId::OpenRouterApiKey,
                raw_secret,
            )
            .expect("ChatCloudPrimary capability authorizes OpenRouterApiKey");

        assert_eq!(sealed.header_name(), "Authorization");
        assert_eq!(sealed.secret_id(), SecretId::OpenRouterApiKey);
        let debug_str = format!("{sealed:?}");
        assert!(
            !debug_str.contains(raw_secret),
            "Debug output must never contain raw secret: {debug_str}"
        );
        let json_str = serde_json::to_string(&sealed).expect("serialize sealed injection");
        assert!(
            !json_str.contains(raw_secret),
            "Serialized output must never contain raw secret: {json_str}"
        );

        let mut headers = reqwest::header::HeaderMap::new();
        sealed
            .apply_to_headers(&mut headers)
            .expect("apply sealed header");
        let auth_val = headers
            .get("Authorization")
            .expect("Authorization header present");
        assert!(auth_val.is_sensitive());

        let leaked_log = format!("tool error: upstream rejected token {raw_secret} on retry");
        assert!(broker.contains_leaked_secret(&leaked_log));
        let scrubbed = broker.scrub_tool_output(&leaked_log);
        assert!(!scrubbed.contains(raw_secret));
        assert!(scrubbed.contains("[REDACTED]"));
    }
}
