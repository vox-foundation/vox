//! Remote Axis connect / disconnect / status commands and the remote agent-event relay.

use std::sync::Arc;

use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::broadcast;

use crate::remote::backend::{BackendState, BackendStatusDto};
use crate::remote::credentials;
use crate::remote::error::AxisError;
use crate::remote::gateway_client::{ConnectRequest, GatewayClient, parse_server_url};

#[tauri::command]
pub async fn axis_connect_remote<R: Runtime>(
    app: AppHandle<R>,
    url: String,
    token: Option<String>,
    accept_fingerprint: Option<String>,
    backend: State<'_, Arc<BackendState>>,
) -> Result<BackendStatusDto, String> {
    let base = parse_server_url(&url)?;
    let token = token
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .or_else(|| credentials::load_token(&base))
        .ok_or_else(|| AxisError::Unauthorized("enter the server's bearer token".into()))?;
    let expected_pin = accept_fingerprint
        .map(|f| f.trim().to_ascii_lowercase())
        .filter(|f| !f.is_empty())
        .or_else(|| credentials::saved_pin(&base));
    let (client, pin) = GatewayClient::connect(ConnectRequest {
        url: base.clone(),
        token: token.clone(),
        expected_pin,
    })
    .await?;
    credentials::store_token(&base, &token)?;
    credentials::save_server(&base, pin.as_deref())?;
    let client = Arc::new(client);
    let relay = spawn_agent_event_relay(app, client.subscribe_events());
    backend.set_remote(client, Some(relay)).await;
    Ok(backend.status().await)
}

#[tauri::command]
pub async fn axis_disconnect_remote(
    backend: State<'_, Arc<BackendState>>,
) -> Result<BackendStatusDto, String> {
    backend.set_local().await;
    Ok(backend.status().await)
}

#[tauri::command]
pub async fn axis_backend_status(
    backend: State<'_, Arc<BackendState>>,
) -> Result<BackendStatusDto, String> {
    Ok(backend.status().await)
}

fn spawn_agent_event_relay<R: Runtime>(
    app: AppHandle<R>,
    mut events: broadcast::Receiver<Value>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match events.recv().await {
                Ok(mut frame) => {
                    let loggable =
                        crate::commands::event_annotate::annotate_agent_event(&mut frame);
                    let _ = app.emit(crate::commands::orchestrator::AGENT_EVENTS_EVENT, frame);
                    if loggable {
                        let _ =
                            app.emit(crate::commands::event_annotate::ACTIVITY_APPENDED_EVENT, ());
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::daemon::PersistentDaemon;
    use tauri::Manager;

    fn app_with_backend() -> tauri::App<tauri::test::MockRuntime> {
        let app = tauri::test::mock_app();
        app.manage(Arc::new(BackendState::new(Arc::new(
            PersistentDaemon::default(),
        ))));
        app
    }

    #[tokio::test]
    async fn backend_status_starts_local() {
        let app = app_with_backend();
        let status = axis_backend_status(app.state()).await.expect("status");
        assert_eq!(status.mode, "local");
        assert!(status.surfaces.iter().all(|s| s.available));
    }

    #[tokio::test]
    async fn connect_rejects_plain_http_to_a_lan_host_before_any_io() {
        let app = app_with_backend();
        let err = axis_connect_remote(
            app.handle().clone(),
            "http://vox.lan:3921".into(),
            Some("t".into()),
            None,
            app.state(),
        )
        .await
        .unwrap_err();
        let v: Value = serde_json::from_str(&err).expect("json envelope");
        assert_eq!(v["error"]["code"], "insecure_url");
        assert!(!app.state::<Arc<BackendState>>().is_remote());
    }

    #[tokio::test]
    async fn disconnect_returns_to_local() {
        let app = app_with_backend();
        let status = axis_disconnect_remote(app.state()).await.expect("status");
        assert_eq!(status.mode, "local");
    }
}
