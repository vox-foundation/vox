//! The GUI's backend selector: the local orchestrator daemon (default, unchanged behaviour) or a
//! remote `vox serve` gateway.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use vox_orchestrator::orch_daemon::{ORCH_CLIENT_READ_DEADLINE, OrchDaemonClient};

use super::error::AxisError;
use super::gateway_client::{GatewayClient, ServerProfile};
use super::manifest::{self, SurfaceAvailability};
use crate::commands::daemon::PersistentDaemon;

pub enum Backend {
    Local(Arc<PersistentDaemon>),
    Remote(Arc<GatewayClient>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BackendStatusDto {
    pub mode: &'static str,
    pub url: Option<String>,
    pub profile: Option<ServerProfile>,
    pub surfaces: Vec<SurfaceAvailability>,
}

pub struct BackendState {
    local: Arc<PersistentDaemon>,
    remote: tokio::sync::RwLock<Option<Arc<GatewayClient>>>,
    remote_flag: AtomicBool,
    relay: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl BackendState {
    pub fn new(local: Arc<PersistentDaemon>) -> Self {
        Self {
            local,
            remote: tokio::sync::RwLock::new(None),
            remote_flag: AtomicBool::new(false),
            relay: std::sync::Mutex::new(None),
        }
    }

    pub fn local(&self) -> Arc<PersistentDaemon> {
        self.local.clone()
    }

    /// Synchronous mode check for the invoke gate and the local stream loops.
    pub fn is_remote(&self) -> bool {
        self.remote_flag.load(Ordering::Acquire)
    }

    pub async fn current(&self) -> Backend {
        match self.remote.read().await.as_ref() {
            Some(client) => Backend::Remote(client.clone()),
            None => Backend::Local(self.local.clone()),
        }
    }

    pub async fn set_remote(
        &self,
        client: Arc<GatewayClient>,
        relay: Option<tokio::task::JoinHandle<()>>,
    ) {
        *self.remote.write().await = Some(client);
        self.replace_relay(relay);
        self.remote_flag.store(true, Ordering::Release);
    }

    pub async fn set_local(&self) {
        self.remote_flag.store(false, Ordering::Release);
        self.replace_relay(None);
        *self.remote.write().await = None;
    }

    fn replace_relay(&self, next: Option<tokio::task::JoinHandle<()>>) {
        let previous = std::mem::replace(&mut *self.relay.lock().expect("relay mutex"), next);
        if let Some(task) = previous {
            task.abort();
        }
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, AxisError> {
        self.call_with_deadline(method, params, ORCH_CLIENT_READ_DEADLINE, None)
            .await
    }

    pub async fn call_with_deadline(
        &self,
        method: &str,
        params: Value,
        deadline: Duration,
        permission_mode: Option<String>,
    ) -> Result<Value, AxisError> {
        match self.current().await {
            Backend::Local(daemon) => {
                let addr = daemon
                    .ensure()
                    .await
                    .map_err(AxisError::DaemonUnavailable)?;
                let mut client = match daemon.token().await {
                    Some(token) => OrchDaemonClient::with_token(addr, token),
                    None => OrchDaemonClient::new(addr),
                };
                if let Some(mode) = permission_mode {
                    client = client.with_permission_mode(mode);
                }
                client
                    .call_with_deadline(method, params, deadline)
                    .await
                    .map_err(|e| AxisError::Daemon(e.to_string()))
            }
            Backend::Remote(gateway) => gateway.call_daemon_method(method, params, deadline).await,
        }
    }

    pub async fn status(&self) -> BackendStatusDto {
        let backends = manifest::command_backends();
        let surfaces = manifest::registry_surfaces();
        match self.current().await {
            Backend::Local(_) => BackendStatusDto {
                mode: "local",
                url: None,
                profile: None,
                surfaces: manifest::surface_availability(backends, surfaces, None),
            },
            Backend::Remote(gateway) => {
                let profile = gateway.profile().clone();
                BackendStatusDto {
                    mode: "remote",
                    url: Some(gateway.base_url().to_string()),
                    surfaces: manifest::surface_availability(
                        backends,
                        surfaces,
                        Some(&profile.axis_surfaces),
                    ),
                    profile: Some(profile),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::gateway_client::test_support::{duplex_client, profile};
    use crate::remote::manifest::UnavailableReason;
    use vox_foundation::protocol::orch_daemon_method;

    #[tokio::test]
    async fn starts_local_with_every_surface_available() {
        let state = BackendState::new(Arc::new(PersistentDaemon::default()));
        assert!(!state.is_remote());
        assert!(matches!(state.current().await, Backend::Local(_)));
        let status = state.status().await;
        assert_eq!(status.mode, "local");
        assert_eq!(status.url, None);
        assert!(status.profile.is_none());
        assert!(!status.surfaces.is_empty());
        assert!(
            status
                .surfaces
                .iter()
                .all(|s| s.available && s.reason.is_none())
        );
    }

    #[tokio::test]
    async fn remote_mode_reports_profile_gates_surfaces_and_refuses_unrouted_methods() {
        let (client, _server) = duplex_client(profile(&["mesh", "models"])).await;
        let state = BackendState::new(Arc::new(PersistentDaemon::default()));
        state.set_remote(Arc::new(client), None).await;
        assert!(state.is_remote());

        let status = state.status().await;
        assert_eq!(status.mode, "remote");
        assert_eq!(status.url.as_deref(), Some("https://vox.lan:3921/"));
        let reason = |s: &str| {
            status
                .surfaces
                .iter()
                .find(|a| a.surface == s)
                .unwrap()
                .reason
        };
        assert_eq!(reason("mesh"), None);
        assert_eq!(reason("models"), Some(UnavailableReason::NotRemoteCapable));
        assert_eq!(reason("tasks"), Some(UnavailableReason::NotInProfile));

        let err = state
            .call(orch_daemon_method::SUBAGENT_TREE, serde_json::json!({}))
            .await
            .unwrap_err();
        assert_eq!(
            err,
            AxisError::NotRemoteCapable {
                method: orch_daemon_method::SUBAGENT_TREE.into()
            }
        );

        state.set_local().await;
        assert!(!state.is_remote());
        assert!(matches!(state.current().await, Backend::Local(_)));
    }
}
