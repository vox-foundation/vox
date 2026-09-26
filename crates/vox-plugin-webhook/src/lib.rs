//! # vox-plugin-webhook
//!
//! Plugin entry point for the Vox webhook HTTP listener gateway.
//!
//! ## Lifecycle (fail closed, D-14)
//!
//! Loading the plugin starts nothing: `init()` only constructs the plugin object.
//! A listener exists only after the host calls `HttpListener::start_listening`
//! with a JSON config carrying a non-blank `ingress_token` (and optionally `addr`;
//! else `VOX_WEBHOOK_ADDR`, else `0.0.0.0:9080`). Without a token the call is
//! refused and nothing is bound. The listener runs on a tokio runtime owned by
//! this plugin, binds synchronously so bind errors reach the caller, and
//! `stop_listening` (or `shutdown`) aborts it.
//!
//! ## Event routing (D-10)
//!
//! Accepted events are drained by the host through the `WebhookInbox` extension
//! (`as_webhook_inbox().poll_events(max)`), one JSON-serialized event per item.

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
    task: tokio::task::JoinHandle<()>,
}

static LISTENER: Mutex<Option<ListenerSlot>> = Mutex::new(None);

fn listener_slot() -> MutexGuard<'static, Option<ListenerSlot>> {
    LISTENER.lock().unwrap_or_else(PoisonError::into_inner)
}

fn plugin_err(msg: impl Into<String>) -> RBoxError {
    RBoxError::new(std::io::Error::other(msg.into()))
}

/// The plugin's own tokio runtime. A cdylib links its own tokio, so it must not rely
/// on the host's runtime context across the dylib boundary.
fn rt() -> &'static tokio::runtime::Runtime {
    static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("vox-webhook-rt")
            .build()
            .expect("failed to build webhook plugin tokio runtime")
    })
}

/// Abort the running listener, if any.
fn stop_listener() {
    if let Some(slot) = listener_slot().take() {
        slot.task.abort();
    }
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
    // Side-effect free (D-14): nothing listens until the host calls start_listening.
    RResult::ROk(new_plugin())
}

fn new_plugin() -> VoxPluginRef {
    VoxPlugin_TO::from_value(WebhookPlugin, TD_Opaque)
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
        // The plugin-owned runtime outlives the plugin object, so stop explicitly.
        stop_listener();
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
        match start_listener(config_json.as_str()) {
            Ok(()) => RResult::ROk(()),
            Err(msg) => RResult::RErr(plugin_err(msg)),
        }
    }

    fn stop_listening(&self) -> RResult<(), RBoxError> {
        stop_listener();
        RResult::ROk(())
    }
}

/// Fail closed: refuse without a non-blank `ingress_token`, bind synchronously, and
/// fill the listener slot only once everything succeeded. Never logs the token or
/// the raw config.
fn start_listener(config_json: &str) -> Result<(), String> {
    let config: serde_json::Value = serde_json::from_str(config_json)
        .map_err(|e| format!("vox-plugin-webhook: start_listening config is not JSON: {e}"))?;
    let token = config
        .get("ingress_token")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or("vox-plugin-webhook: refusing to start the listener without an ingress token")?;
    let addr = config
        .get("addr")
        .and_then(|a| a.as_str())
        .map(str::to_string)
        .or_else(|| std::env::var("VOX_WEBHOOK_ADDR").ok());
    let addr = webhook::config::resolve_bind_addr(addr.as_deref());

    let mut slot = listener_slot();
    if slot.is_some() {
        return Err("vox-plugin-webhook: the listener is already running".into());
    }
    let std_listener = std::net::TcpListener::bind(&addr)
        .map_err(|e| format!("vox-plugin-webhook: cannot bind {addr}: {e}"))?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| format!("vox-plugin-webhook: cannot configure {addr}: {e}"))?;

    let state = WebhookState::new(WebhookHandler::new(), token);
    let sender = state.event_sink.clone();
    let receiver = sender.subscribe();
    let task = rt().spawn(async move {
        let listener = match tokio::net::TcpListener::from_std(std_listener) {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("vox-plugin-webhook: cannot register listener: {e}");
                return;
            }
        };
        if let Err(e) = serve(state, listener).await {
            tracing::error!("vox-plugin-webhook: server error: {e}");
        }
    });
    *slot = Some(ListenerSlot {
        sender,
        receiver,
        task,
    });
    info!(addr = %addr, "vox-plugin-webhook: listener started");
    Ok(())
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
mod listener_lifecycle_tests {
    //! D-14: the listener is fail closed and inert until the host starts it.
    use super::*;

    const GOOD: &str = r#"{"addr":"127.0.0.1:0","ingress_token":"test-token"}"#;

    fn polls_err() -> bool {
        WebhookInboxImpl.poll_events(10).is_rerr()
    }

    fn event(id: &str) -> WebhookEvent {
        WebhookEvent {
            id: id.to_string(),
            source: "test".to_string(),
            event_type: "ping".to_string(),
            payload: serde_json::json!({}),
            received_at: 0,
        }
    }

    #[test]
    fn start_listening_refuses_without_ingress_token() {
        let _g = test_lock();
        for config in [
            "{}",
            "not-json",
            r#"{"addr":"127.0.0.1:0"}"#,
            r#"{"addr":"127.0.0.1:0","ingress_token":""}"#,
            r#"{"addr":"127.0.0.1:0","ingress_token":"   "}"#,
        ] {
            let r = WebhookHttpListener.start_listening(config.into());
            assert!(r.is_rerr(), "config {config:?} must be refused");
            assert!(polls_err(), "a refused start must keep no listener slot");
        }
    }

    #[test]
    fn start_listening_twice_is_refused() {
        let _g = test_lock();
        let l = WebhookHttpListener;
        assert!(l.start_listening(GOOD.into()).is_rok());
        assert!(l.start_listening(GOOD.into()).is_rerr());
        {
            let slot = listener_slot();
            let sink = &slot.as_ref().expect("first listener kept").sender;
            sink.send(event("still-live")).expect("receiver subscribed");
        }
        let polled = WebhookInboxImpl.poll_events(10);
        assert_eq!(polled.unwrap().len(), 1, "the first listener keeps working");
        assert!(l.stop_listening().is_rok());
    }

    #[test]
    fn stop_listening_releases_the_listener() {
        let _g = test_lock();
        let l = WebhookHttpListener;
        assert!(l.start_listening(GOOD.into()).is_rok());
        assert!(l.stop_listening().is_rok());
        assert!(polls_err(), "polling after stop must be an error");
        assert!(
            l.start_listening(GOOD.into()).is_rok(),
            "restart after stop"
        );
        assert!(l.stop_listening().is_rok());
    }

    #[test]
    fn start_listening_returns_bind_errors() {
        let _g = test_lock();
        let occupied = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
        let addr = occupied.local_addr().expect("local addr");
        let config = format!(r#"{{"addr":"{addr}","ingress_token":"test-token"}}"#);
        let r = WebhookHttpListener.start_listening(config.as_str().into());
        assert!(r.is_rerr(), "binding an occupied port must fail");
        assert!(polls_err(), "a failed bind must keep no listener slot");
    }

    #[test]
    fn start_listening_needs_no_ambient_tokio_runtime() {
        let _g = test_lock();
        let l = WebhookHttpListener;
        let r = l.start_listening(GOOD.into());
        assert!(r.is_rok(), "start outside any tokio runtime failed: {r:?}");
        assert!(l.stop_listening().is_rok());
    }

    #[test]
    fn constructing_the_plugin_starts_no_listener() {
        let _g = test_lock();
        let plugin = new_plugin();
        assert_eq!(plugin.id().as_str(), "webhook");
        assert!(
            listener_slot().is_none(),
            "constructing the plugin must not listen"
        );
        assert!(polls_err());
    }
}
