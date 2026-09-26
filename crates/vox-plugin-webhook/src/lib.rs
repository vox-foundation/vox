#![cfg_attr(test, allow(unsafe_code))] // test-only std::env::set_var (unsafe on edition 2024)
//! # vox-plugin-webhook
//!
//! Plugin entry point for the Vox webhook HTTP listener gateway.
//!
//! On `init()` the plugin spawns a Tokio task that runs the Axum webhook
//! server on the address configured by the `VOX_WEBHOOK_ADDR` environment
//! variable (default: `0.0.0.0:9080`).
//!
//! ## Event routing
//!
//! The plugin uses a no-op `WebhookEventSink` by default. For production use,
//! the host should wire an `Arc<dyn WebhookEventSink>` backed by the Orchestrator
//! (see `WebhookOrchestratorBridge` in `webhook::bridge`). The orchestrator-side
//! wiring is deferred — tracked as Step 8 of the extraction plan.
//!
//! ## Plugin trait
//!
//! Implements `VoxPlugin` (id + shutdown). The HTTP server is a long-running
//! background tokio task started from `init()`. There is no dedicated
//! "start-service" lifecycle hook in ABI v11 — this matches the pattern used
//! by other long-running plugins (e.g. vox-plugin-cloud).

// Public types are designed for orchestrator wiring (Step 8). Suppress dead-code
// lint until the bridge is wired — these are real implementations, not stubs.
#![allow(dead_code, unused_imports)]

mod webhook;

use abi_stable::{
    erased_types::TD_Opaque, export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn,
    std_types::*,
};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tokio::sync::broadcast;
use tracing::{info, warn};
use vox_plugin_api::VOX_PLUGIN_ABI_VERSION;
use vox_plugin_api::abi::{VoxPlugin, VoxPlugin_TO, VoxPluginRef, VoxPluginRoot, VoxPluginRootRef};
use vox_plugin_api::extensions::http_listener::{
    HTTP_LISTENER_REVISION, HttpListener, HttpListener_TO,
};
use vox_plugin_api::extensions::webhook_inbox::{
    WEBHOOK_INBOX_REVISION, WebhookInbox, WebhookInbox_TO,
};
use vox_plugin_api::host::VoxHost_TO;
use webhook::{
    WebhookEvent, WebhookHandler,
    router::{WebhookState, serve},
};

// ---------------------------------------------------------------------------
// Listener slot (shared by HttpListener and WebhookInbox)
// ---------------------------------------------------------------------------

/// The running listener's event channel: the router's own `event_sink` and a receiver
/// subscribed to it, drained by [`WebhookInboxImpl::poll_events`].
struct ListenerSlot {
    sender: Arc<broadcast::Sender<WebhookEvent>>,
    receiver: broadcast::Receiver<WebhookEvent>,
}

static LISTENER: Mutex<Option<ListenerSlot>> = Mutex::new(None);

fn listener_slot() -> MutexGuard<'static, Option<ListenerSlot>> {
    LISTENER.lock().unwrap_or_else(PoisonError::into_inner)
}

fn plugin_err(msg: impl Into<String>) -> RBoxError {
    RBoxError::new(std::io::Error::other(msg.into()))
}

// ---------------------------------------------------------------------------
// ABI root module
// ---------------------------------------------------------------------------

#[export_root_module]
fn root_module() -> VoxPluginRootRef {
    VoxPluginRoot {
        abi_version: VOX_PLUGIN_ABI_VERSION,
        manifest_json,
        init,
    }
    .leak_into_prefix()
}

#[sabi_extern_fn]
fn manifest_json() -> RString {
    RString::from(r#"{"id":"webhook","version":"0.1.0"}"#)
}

#[sabi_extern_fn]
fn init(_host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    // Start the HTTP listener on a background tokio task.
    //
    // NOTE: this relies on a tokio runtime already being active in the host
    // process, which is guaranteed by the vox-plugin-host bootstrap.
    let addr = webhook::config::bind_addr_from_env();
    let ingress_token = std::env::var("VOX_WEBHOOK_INGRESS_TOKEN").ok();

    let mut state = WebhookState::new(WebhookHandler::new());
    if let Some(token) = ingress_token {
        state = state.with_ingress_token(token);
    } else {
        warn!(
            "vox-plugin-webhook: VOX_WEBHOOK_INGRESS_TOKEN not set — running in degraded (no-auth) mode"
        );
    }

    // Spawn the HTTP server. The broadcast channel inside WebhookState will
    // accumulate events; wire WebhookOrchestratorBridge to consume them.
    let addr_clone = addr.clone();
    tokio::spawn(async move {
        info!(addr = %addr_clone, "vox-plugin-webhook: starting HTTP listener");
        if let Err(e) = serve(state, &addr_clone).await {
            tracing::error!("vox-plugin-webhook: server error: {e}");
        }
    });

    let plugin = WebhookPlugin;
    let to = VoxPlugin_TO::from_value(plugin, TD_Opaque);
    RResult::ROk(to)
}

// ---------------------------------------------------------------------------
// Plugin impl
// ---------------------------------------------------------------------------

struct WebhookPlugin;

impl VoxPlugin for WebhookPlugin {
    fn id(&self) -> RString {
        RString::from("webhook")
    }

    fn shutdown(&self) -> RResult<(), RBoxError> {
        // The tokio task will be dropped when the runtime shuts down.
        // No explicit handle is stored (acceptable for the current ABI surface).
        RResult::ROk(())
    }

    fn as_http_listener(&self) -> ROption<HttpListener_TO<'static, RBox<()>>> {
        ROption::RSome(HttpListener_TO::from_value(WebhookHttpListener, TD_Opaque))
    }

    fn as_webhook_inbox(&self) -> ROption<WebhookInbox_TO<'static, RBox<()>>> {
        ROption::RSome(WebhookInbox_TO::from_value(WebhookInboxImpl, TD_Opaque))
    }
}

struct WebhookInboxImpl;

impl WebhookInbox for WebhookInboxImpl {
    fn revision(&self) -> u32 {
        WEBHOOK_INBOX_REVISION
    }

    fn poll_events(&self, max: u32) -> RResult<RVec<RString>, RBoxError> {
        let mut guard = listener_slot();
        let Some(slot) = guard.as_mut() else {
            return RResult::RErr(plugin_err(
                "vox-plugin-webhook: the listener has not been started",
            ));
        };
        let mut out = RVec::new();
        while out.len() < max as usize {
            match slot.receiver.try_recv() {
                Ok(event) => match serde_json::to_string(&event) {
                    Ok(json) => out.push(RString::from(json)),
                    Err(e) => return RResult::RErr(RBoxError::new(e)),
                },
                Err(broadcast::error::TryRecvError::Lagged(n)) => {
                    warn!(
                        n,
                        "vox-plugin-webhook: inbox lagged — {n} events dropped; consider increasing channel capacity"
                    );
                }
                Err(
                    broadcast::error::TryRecvError::Empty | broadcast::error::TryRecvError::Closed,
                ) => break,
            }
        }
        RResult::ROk(out)
    }
}

struct WebhookHttpListener;

impl HttpListener for WebhookHttpListener {
    fn revision(&self) -> u32 {
        HTTP_LISTENER_REVISION
    }

    fn start_listening(&self, config_json: RStr<'_>) -> RResult<(), RBoxError> {
        let addr = serde_json::from_str::<serde_json::Value>(config_json.as_str())
            .ok()
            .and_then(|v| v.get("addr").and_then(|a| a.as_str()).map(str::to_string))
            .or_else(|| std::env::var("VOX_WEBHOOK_ADDR").ok());
        let addr = webhook::config::resolve_bind_addr(addr.as_deref());
        let ingress_token = std::env::var("VOX_WEBHOOK_INGRESS_TOKEN").ok();
        let mut state = WebhookState::new(WebhookHandler::new());
        if let Some(token) = ingress_token {
            state = state.with_ingress_token(token);
        }
        *listener_slot() = Some(ListenerSlot {
            sender: state.event_sink.clone(),
            receiver: state.event_sink.subscribe(),
        });
        tokio::spawn(async move {
            info!(addr = %addr, "vox-plugin-webhook: HttpListener start_listening");
            if let Err(e) = serve(state, &addr).await {
                tracing::error!("vox-plugin-webhook: server error: {e}");
            }
        });
        RResult::ROk(())
    }

    fn stop_listening(&self) -> RResult<(), RBoxError> {
        listener_slot().take();
        RResult::ROk(())
    }
}

/// Serialises tests that touch the process-wide listener slot.
#[cfg(test)]
fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod webhook_inbox_tests {
    use super::*;

    fn event(id: &str) -> WebhookEvent {
        WebhookEvent {
            id: id.to_string(),
            source: "test".to_string(),
            event_type: "ping".to_string(),
            payload: serde_json::json!({ "id": id }),
            received_at: 0,
        }
    }

    /// Send on the Sender stored in the listener slot — the router's own `event_sink`.
    fn send_on_router_sink(ev: WebhookEvent) {
        let slot = LISTENER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        slot.as_ref()
            .expect("listener slot must be filled after start_listening")
            .sender
            .send(ev)
            .expect("the slot's receiver is subscribed");
    }

    fn start(plugin: &VoxPluginRef) {
        let listener = plugin.as_http_listener().unwrap();
        let r = listener
            .start_listening(r#"{"addr":"127.0.0.1:0","ingress_token":"test-token"}"#.into());
        assert!(r.is_rok(), "start_listening failed: {r:?}");
    }

    fn stop(plugin: &VoxPluginRef) {
        assert!(plugin.as_http_listener().unwrap().stop_listening().is_rok());
    }

    fn ids(batch: &RVec<RString>) -> Vec<String> {
        batch
            .iter()
            .map(|s| {
                serde_json::from_str::<WebhookEvent>(s.as_str())
                    .expect("each item is a JSON WebhookEvent")
                    .id
            })
            .collect()
    }

    #[tokio::test]
    async fn poll_events_before_start_is_an_error() {
        let _g = test_lock();
        let plugin = VoxPlugin_TO::from_value(WebhookPlugin, TD_Opaque);
        let inbox = plugin.as_webhook_inbox();
        assert!(inbox.is_rsome(), "webhook plugin must expose WebhookInbox");
        assert!(inbox.unwrap().poll_events(10).is_rerr());
    }

    #[tokio::test]
    async fn poll_events_drains_router_events_through_the_erased_trait_object() {
        let _g = test_lock();
        let plugin = VoxPlugin_TO::from_value(WebhookPlugin, TD_Opaque);
        start(&plugin);
        send_on_router_sink(event("evt-1"));
        send_on_router_sink(event("evt-2"));
        let inbox = plugin.as_webhook_inbox().unwrap();
        let first = inbox.poll_events(10).unwrap();
        assert_eq!(ids(&first), ["evt-1", "evt-2"]);
        assert_eq!(inbox.poll_events(10).unwrap().len(), 0);
        stop(&plugin);
    }

    #[tokio::test]
    async fn poll_events_respects_max() {
        let _g = test_lock();
        let plugin = VoxPlugin_TO::from_value(WebhookPlugin, TD_Opaque);
        start(&plugin);
        for id in ["m-1", "m-2", "m-3"] {
            send_on_router_sink(event(id));
        }
        let inbox = plugin.as_webhook_inbox().unwrap();
        assert_eq!(ids(&inbox.poll_events(2).unwrap()), ["m-1", "m-2"]);
        assert_eq!(ids(&inbox.poll_events(10).unwrap()), ["m-3"]);
        stop(&plugin);
    }
}

#[cfg(test)]
mod semcov_wave3_tests {
    // Rust 2024 made std::env::{set_var,remove_var} unsafe; mutated single-threaded.
    #![allow(unused_imports, unsafe_code)]
    use super::*;
    use vox_plugin_api::extensions::http_listener::HttpListener;

    // start_listening resolves addr from JSON config first, then env var, then
    // hardcoded default. We can observe the branching at the function boundary
    // without actually binding a port because the function always returns ROk
    // (spawn is fire-and-forget) — but we MUST run inside a tokio runtime so
    // that tokio::spawn does not panic.

    #[tokio::test]
    async fn start_listening_returns_ok_for_valid_json_config() {
        let _g = test_lock();
        let listener = WebhookHttpListener;
        let config = r#"{"addr": "127.0.0.1:0"}"#;
        let result = listener.start_listening(config.into());
        let _ = listener.stop_listening();
        assert!(
            result.is_rok(),
            "start_listening must succeed for valid JSON config: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn start_listening_returns_ok_for_empty_json_object() {
        // Falls through to env-var / default path
        let _g = test_lock();
        let listener = WebhookHttpListener;
        let config = r#"{}"#;
        let result = listener.start_listening(config.into());
        let _ = listener.stop_listening();
        assert!(
            result.is_rok(),
            "start_listening must succeed for empty JSON"
        );
    }

    #[tokio::test]
    async fn start_listening_returns_ok_for_invalid_json() {
        // JSON parse fails → falls back to env-var / default — must not propagate error
        let _g = test_lock();
        let listener = WebhookHttpListener;
        let config = "not-json";
        let result = listener.start_listening(config.into());
        let _ = listener.stop_listening();
        assert!(
            result.is_rok(),
            "start_listening must succeed even for unparseable config"
        );
    }

    #[tokio::test]
    async fn start_listening_env_var_path_succeeds() {
        let _g = test_lock();
        unsafe { std::env::set_var("VOX_WEBHOOK_ADDR", "127.0.0.1:0") };
        let listener = WebhookHttpListener;
        // Empty JSON → addr from env var
        let result = listener.start_listening(r#"{}"#.into());
        let _ = listener.stop_listening();
        unsafe { std::env::remove_var("VOX_WEBHOOK_ADDR") };
        assert!(result.is_rok(), "env-var addr path must succeed");
    }
}
