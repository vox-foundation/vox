//! HTTP client for the mesh control plane ([`crate::transport`]).
//!
//! Base URL should include scheme and host, e.g. `http://127.0.0.1:9847` (no trailing slash).
//! When the control plane is protected with **`VOX_MESH_TOKEN`**, set the same value via
//! [`MeshHttpClient::with_bearer`] (or [`MeshHttpClient::with_env_token`]).

use std::time::Duration;

use crate::transport::{LeaveRequest, A2ADeliverRequest};
use crate::{MeshRegistryFile, NodeRecord, MeshRegistryError};

/// Call the mesh HTTP API (join / list / heartbeat / leave).
#[derive(Debug, Clone)]
pub struct MeshHttpClient {
    client: reqwest::Client,
    base: String,
    bearer: Option<String>,
}

impl MeshHttpClient {
    /// Hosted / BaaS control plane entrypoint: same as [`Self::new`], but documents org-scoped HTTPS
    /// bases (see `docs/src/adr/009-mesh-hosted-baas.md`). **Never** embed secrets in the URL.
    #[must_use]
    pub fn for_hosted_control_plane(base: impl Into<String>) -> Self {
        Self::new(base)
    }

    /// New client; `base` is normalized (trailing `/` stripped). No `Authorization` header.
    #[must_use]
    pub fn new(base: impl Into<String>) -> Self {
        Self::new_with_timeout(base, Duration::from_secs(30))
    }

    /// Like [`Self::new`] with an explicit request timeout (federation / control plane).
    #[must_use]
    pub fn new_with_timeout(base: impl Into<String>, timeout: Duration) -> Self {
        let mut base = base.into();
        while base.ends_with('/') {
            base.pop();
        }
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client for mesh HTTP");
        Self {
            client,
            base,
            bearer: None,
        }
    }

    /// Attach `Authorization: Bearer …` to every request (e.g. matches server **`VOX_MESH_TOKEN`**).
    #[must_use]
    pub fn with_bearer(mut self, token: impl Into<String>) -> Self {
        let t = token.into();
        self.bearer = if t.trim().is_empty() { None } else { Some(t) };
        self
    }

    /// If **`VOX_MESH_TOKEN`** is set and non-empty, same as [`Self::with_bearer`] with that value.
    #[must_use]
    pub fn with_env_token(self) -> Self {
        match std::env::var("VOX_MESH_TOKEN") {
            Ok(t) if !t.trim().is_empty() => self.with_bearer(t),
            _ => self,
        }
    }

    fn auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.bearer {
            Some(t) => rb.bearer_auth(t),
            None => rb,
        }
    }

    /// `GET /v1/mesh/nodes`
    pub async fn list_nodes(&self) -> Result<MeshRegistryFile, MeshRegistryError> {
        let url = format!("{}/v1/mesh/nodes", self.base);
        let v = self
            .auth(self.client.get(url))
            .send()
            .await
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?
            .error_for_status()
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?;
        Ok(v)
    }

    /// `POST /v1/mesh/join`
    pub async fn join(&self, node: &NodeRecord) -> Result<NodeRecord, MeshRegistryError> {
        let url = format!("{}/v1/mesh/join", self.base);
        let v = self
            .auth(self.client.post(url).json(node))
            .send()
            .await
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?
            .error_for_status()
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?;
        Ok(v)
    }

    /// `POST /v1/mesh/heartbeat`
    pub async fn heartbeat(&self, node: &NodeRecord) -> Result<NodeRecord, MeshRegistryError> {
        let url = format!("{}/v1/mesh/heartbeat", self.base);
        let v = self
            .auth(self.client.post(url).json(node))
            .send()
            .await
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?
            .error_for_status()
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?;
        Ok(v)
    }

    /// `POST /v1/mesh/leave` — returns `true` if the node was present and removed.
    pub async fn leave(&self, node_id: &str) -> Result<bool, MeshRegistryError> {
        let url = format!("{}/v1/mesh/leave", self.base);
        let resp = self
            .auth(self.client.post(url).json(&LeaveRequest {
                id: node_id.to_string(),
            }))
            .send()
            .await
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?;
        match resp.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(true),
            reqwest::StatusCode::NOT_FOUND => Ok(false),
            _ => Err(MeshRegistryError::Http(format!(
                "leave: unexpected status {}",
                resp.status()
            ))),
        }
    }

    /// `POST /v1/mesh/a2a/deliver` — forward an A2A message to a remote node.
    pub async fn relay_a2a(&self, req: &A2ADeliverRequest) -> Result<(), MeshRegistryError> {
        let url = format!("{}/v1/mesh/a2a/deliver", self.base);
        self.auth(self.client.post(url).json(req))
            .send()
            .await
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?
            .error_for_status()
            .map_err(|e| MeshRegistryError::Http(e.to_string()))?;
        Ok(())
    }
}
