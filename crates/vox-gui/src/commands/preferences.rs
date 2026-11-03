//! Axis `gui.*` preferences (plan 07 D1). Remote mode keeps them client-side in
//! [`AxisLocalPrefs`]; local mode reaches the workspace DB through the local daemon's
//! loopback-only `vox_gui_preference_*` ops.

use std::sync::Arc;

use serde_json::json;

use crate::remote::backend::{Backend, BackendState};
use crate::remote::calls::{BackendCalls, tool_data};
use crate::remote::local_prefs::AxisLocalPrefs;

/// Local mode never writes the client file: a denial or an unreachable daemon is surfaced
/// to the UI, so preferences have exactly one home per mode.
pub(crate) async fn get_gui_preference_impl<B: BackendCalls>(
    b: &B,
    remote: bool,
    local: &AxisLocalPrefs,
    key: String,
) -> Result<Option<String>, String> {
    if remote {
        return local.get(&key);
    }
    let env = b
        .call_tool("vox_gui_preference_get", json!({ "key": key }))
        .await?;
    tool_data::<Option<String>>("vox_gui_preference_get", env)
}

pub(crate) async fn set_gui_preference_impl<B: BackendCalls>(
    b: &B,
    remote: bool,
    local: &AxisLocalPrefs,
    key: String,
    value: String,
) -> Result<(), String> {
    if remote {
        return local.set(&key, &value);
    }
    let env = b
        .call_tool(
            "vox_gui_preference_set",
            json!({ "key": key, "value": value }),
        )
        .await?;
    tool_data::<String>("vox_gui_preference_set", env).map(|_| ())
}

#[tauri::command]
pub async fn get_gui_preference(
    key: String,
    app: tauri::AppHandle,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<Option<String>, String> {
    let local = AxisLocalPrefs::for_app(&app)?;
    let b = backend.current().await;
    let remote = matches!(b, Backend::Remote(_));
    get_gui_preference_impl(&b, remote, &local, key).await
}

#[tauri::command]
pub async fn set_gui_preference(
    key: String,
    value: String,
    app: tauri::AppHandle,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<(), String> {
    let local = AxisLocalPrefs::for_app(&app)?;
    let b = backend.current().await;
    let remote = matches!(b, Backend::Remote(_));
    set_gui_preference_impl(&b, remote, &local, key, value).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    fn store(dir: &tempfile::TempDir) -> (AxisLocalPrefs, std::path::PathBuf) {
        let path = dir.path().join("axis-preferences.json");
        (AxisLocalPrefs::at(path.clone()), path)
    }

    #[tokio::test]
    async fn remote_mode_uses_only_the_client_store() {
        let dir = tempfile::tempdir().unwrap();
        let (local, _) = store(&dir);
        let b = FakeBackend::new();
        set_gui_preference_impl(&b, true, &local, "gui.theme".into(), "dark".into())
            .await
            .unwrap();
        let v = get_gui_preference_impl(&b, true, &local, "gui.theme".into())
            .await
            .unwrap();
        assert_eq!(v.as_deref(), Some("dark"));
        assert!(b.calls().is_empty());
    }

    #[tokio::test]
    async fn local_mode_goes_through_the_daemon_ops() {
        let dir = tempfile::tempdir().unwrap();
        let (local, path) = store(&dir);
        let b = FakeBackend::new()
            .on_tool(
                "vox_gui_preference_get",
                Ok(json!({"success": true, "data": "light"})),
            )
            .on_tool(
                "vox_gui_preference_set",
                Ok(json!({"success": true, "data": "gui.theme"})),
            );
        let v = get_gui_preference_impl(&b, false, &local, "gui.theme".into())
            .await
            .unwrap();
        assert_eq!(v.as_deref(), Some("light"));
        set_gui_preference_impl(&b, false, &local, "gui.theme".into(), "dark".into())
            .await
            .unwrap();
        let calls = b.calls();
        assert_eq!(calls[0].0, "tool:vox_gui_preference_get");
        assert_eq!(calls[1].0, "tool:vox_gui_preference_set");
        assert_eq!(calls[1].1, json!({"key": "gui.theme", "value": "dark"}));
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn local_mode_surfaces_denials_without_writing_the_client_file() {
        let dir = tempfile::tempdir().unwrap();
        let (local, path) = store(&dir);
        let denied = FakeBackend::new().on_tool(
            "vox_gui_preference_set",
            Ok(json!({"error": {"code": "tool_not_in_profile", "message": "not served"}})),
        );
        let e = set_gui_preference_impl(&denied, false, &local, "gui.theme".into(), "x".into())
            .await
            .unwrap_err();
        assert!(e.starts_with("tool_not_in_profile"), "{e}");
        assert!(!path.exists());

        let down =
            FakeBackend::new().on_tool("vox_gui_preference_set", Err("unreachable".to_string()));
        let e = set_gui_preference_impl(&down, false, &local, "gui.theme".into(), "x".into())
            .await
            .unwrap_err();
        assert_eq!(e, "unreachable");
        assert!(!path.exists());
    }

    #[test]
    fn preferences_module_has_no_direct_db_access() {
        let src = include_str!("preferences.rs");
        let (body, _) = src.split_once("#[cfg(test)]").unwrap();
        assert!(!body.contains("vox_db"));
    }
}
