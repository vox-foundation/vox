use crate::drive::flags::{DriveFlags, read_token_from_file};
use crate::drive::listener::{
    DriveHandler, DriveHttpRequest, DriveHttpResponse, ListenerHandle, bind_loopback,
};
use crate::drive::session_io::{DriveSessionFile, write_child_session};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Injected before page JS so drive never reads/writes the user's pin key.
pub const DRIVE_ISOLATION_SCRIPT: &str = r#"
window.__VOX_DRIVE_LIVE__ = true;
(function () {
  var KEY = "vox_chat_model.v1";
  var proto = Storage.prototype;
  var getItem = proto.getItem;
  var setItem = proto.setItem;
  var removeItem = proto.removeItem;
  proto.getItem = function (k) {
    if (k === KEY) return null;
    return getItem.call(this, k);
  };
  proto.setItem = function (k, v) {
    if (k === KEY) return;
    return setItem.call(this, k, v);
  };
  proto.removeItem = function (k) {
    if (k === KEY) return;
    return removeItem.call(this, k);
  };
})();
"#;

pub struct DriveRuntime {
    pub mode: Mutex<String>,
    pub handle: Mutex<Option<ListenerHandle>>,
    pending: Arc<Mutex<HashMap<String, mpsc::SyncSender<DriveHttpResponse>>>>,
}

impl DriveRuntime {
    pub fn off() -> Self {
        Self {
            mode: Mutex::new("off".into()),
            handle: Mutex::new(None),
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Live from process start so `get_drive_mode` cannot race the listener bind.
    pub fn pending_live() -> Self {
        Self {
            mode: Mutex::new("live".into()),
            handle: Mutex::new(None),
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveRequestEvent {
    pub id: String,
    pub verb: String,
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveRespondArgs {
    pub id: String,
    pub status: u16,
    pub body: String,
}

pub fn start_live<R: Runtime>(
    app: &AppHandle<R>,
    flags: DriveFlags,
    runtime: &DriveRuntime,
) -> Result<(), String> {
    let token = read_token_from_file().ok_or_else(|| {
        "VOX_GUI_DRIVE_TOKEN_PATH missing or empty (never pass the token via env)".to_string()
    })?;
    let bound = bind_loopback(&token).map_err(|e| e.to_string())?;
    let handle = bound.handle.clone();
    let pending = Arc::clone(&runtime.pending);
    let app_cb = app.clone();
    let handler: DriveHandler = Arc::new(move |req: DriveHttpRequest| {
        if req.verb == "show" {
            reveal_window(&app_cb);
            return DriveHttpResponse {
                status: 200,
                body: r#"{"ok":true,"plane":"live"}"#.into(),
            };
        }
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::sync_channel(1);
        pending
            .lock()
            .expect("drive pending")
            .insert(id.clone(), tx);
        let body: serde_json::Value =
            serde_json::from_str(&req.body).unwrap_or(serde_json::Value::Null);
        let _ = app_cb.emit(
            "drive://request",
            DriveRequestEvent {
                id,
                verb: req.verb,
                body,
            },
        );
        match rx.recv_timeout(Duration::from_secs(90)) {
            Ok(resp) => resp,
            Err(RecvTimeoutError::Timeout) => DriveHttpResponse {
                status: 504,
                body: r#"{"error":"timeout"}"#.into(),
            },
            Err(_) => DriveHttpResponse {
                status: 503,
                body: r#"{"error":"host_gone"}"#.into(),
            },
        }
    });
    handle.set_handler(handler);
    handle.set_ready(false);

    let store_root = std::env::var("VOX_GUI_DRIVE_STORE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let token_path = std::env::var("VOX_GUI_DRIVE_TOKEN_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("gui-drive.token"));
    write_child_session(&DriveSessionFile {
        schema_version: 1,
        pid: std::process::id(),
        port: handle.addr().port(),
        token_path,
        store_path: store_root.join(".vox/store.db"),
        show: flags.show,
    })?;

    apply_window_isolation(app, flags.show);
    *runtime.mode.lock().expect("drive mode") = "live".into();
    *runtime.handle.lock().expect("drive handle") = Some(handle);
    std::mem::forget(bound);
    Ok(())
}

fn reveal_window<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_skip_taskbar(false);
            let _ = window.set_focus();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = app.set_dock_visibility(true);
        }
    });
}

/// Hide + skip Dock before the rest of setup so the drive Axis does not flash.
pub fn hide_drive_window_early<R: Runtime>(app: &AppHandle<R>) {
    apply_window_isolation(app, false);
}

fn apply_window_isolation<R: Runtime>(app: &AppHandle<R>, show: bool) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.eval("window.__VOX_DRIVE_LIVE__=true;");
        let _ = window.set_title("Axis (drive)");
        if !show {
            let _ = window.hide();
            let _ = window.set_skip_taskbar(true);
        }
    }
    if !show {
        #[cfg(target_os = "macos")]
        {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let _ = app.set_dock_visibility(false);
        }
    }
}

#[tauri::command]
pub fn get_drive_mode(runtime: tauri::State<'_, DriveRuntime>) -> String {
    runtime.mode.lock().expect("drive mode").clone()
}

#[tauri::command]
pub fn drive_set_ready(runtime: tauri::State<'_, DriveRuntime>) {
    if let Some(handle) = runtime.handle.lock().expect("drive handle").as_ref() {
        handle.set_ready(true);
    }
}

#[tauri::command]
pub fn drive_respond(runtime: tauri::State<'_, DriveRuntime>, args: DriveRespondArgs) {
    if let Some(tx) = runtime
        .pending
        .lock()
        .expect("drive pending")
        .remove(&args.id)
    {
        let _ = tx.send(DriveHttpResponse {
            status: args.status,
            body: args.body,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_live_is_live_before_listener() {
        let rt = DriveRuntime::pending_live();
        assert_eq!(rt.mode.lock().expect("mode").as_str(), "live");
        assert!(rt.handle.lock().expect("handle").is_none());
    }

    #[test]
    fn off_is_off() {
        let rt = DriveRuntime::off();
        assert_eq!(rt.mode.lock().expect("mode").as_str(), "off");
    }

    #[test]
    fn isolation_script_blocks_shared_pin_key() {
        assert!(DRIVE_ISOLATION_SCRIPT.contains("__VOX_DRIVE_LIVE__"));
        assert!(DRIVE_ISOLATION_SCRIPT.contains("vox_chat_model.v1"));
        assert!(DRIVE_ISOLATION_SCRIPT.contains("proto.setItem"));
    }
}
