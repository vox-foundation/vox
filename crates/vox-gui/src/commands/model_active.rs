//! Active-model preference (Models surface), served by the daemon — `GET /api/v2/models/preferences`
//! and `vox_models_preference_set` — so it works against a local or a remote Vox server.

use std::sync::{Arc, Mutex, OnceLock};

use serde_json::json;
use vox_orchestrator_mcp::gui_ops::dto::models::ModelsPreferencesDto;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, tool_data};

/// GUI-process display fallback when the server is unreachable or has no row. Replaces a
/// prior `unsafe { std::env::set_var("VOX_MODEL", ...) }`: mutating the process environment
/// from an async fn in a multi-threaded Tauri process is a data-race hazard, and it collided
/// with `VOX_MODEL` as a genuine user-set launch-time default. Not daemon routing state (that
/// is `mcp_chat_model_override`).
fn active_model_override() -> &'static Mutex<Option<String>> {
    static OVERRIDE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

/// What [`get_active_model_impl`] returns when the server preference is absent or
/// unreachable: the in-process override, else the user-set `VOX_MODEL` launch-time default.
fn active_model_fallback() -> Option<String> {
    if let Some(v) = active_model_override().lock().unwrap().clone() {
        return Some(v);
    }
    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxModel)
        .expose()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub(crate) async fn get_active_model_impl<B: BackendCalls>(
    b: &B,
) -> Result<Option<String>, String> {
    let prefs = match b.api_v2_get("/models/preferences").await {
        Ok(env) => api_data::<ModelsPreferencesDto>(env).ok(),
        Err(_) => None,
    };
    Ok(prefs
        .and_then(|p| p.active_model)
        .or_else(active_model_fallback))
}

pub(crate) async fn set_active_model_impl<B: BackendCalls>(
    b: &B,
    model_id: String,
) -> Result<(), String> {
    let trimmed = model_id.trim().to_string();
    if trimmed.is_empty() {
        return Err("model_id must not be empty".into());
    }
    let args = json!({ "key": "active_model", "value": trimmed });
    // A tool-level rejection (unknown model, profile denial) goes back to the UI; an
    // unreachable server keeps the degrade-to-display-override behaviour.
    if let Ok(env) = b.call_tool("vox_models_preference_set", args).await {
        tool_data::<String>("vox_models_preference_set", env)?;
    }
    *active_model_override().lock().unwrap() = Some(trimmed);
    Ok(())
}

#[tauri::command]
pub async fn get_active_model(
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<Option<String>, String> {
    get_active_model_impl(&backend.current().await).await
}

#[tauri::command]
pub async fn set_active_model(
    model_id: String,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<(), String> {
    set_active_model_impl(&backend.current().await, model_id).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    fn priority() -> serde_json::Value {
        json!({"efficiency": 25, "precision": 30, "latency": 15, "availability": 15, "balance": 10, "mobile": 5})
    }

    #[test]
    fn active_model_fallback_returns_in_process_override_when_set() {
        *active_model_override().lock().unwrap() = None;
        assert_eq!(active_model_fallback(), None);

        *active_model_override().lock().unwrap() = Some("mens/just-set-model".to_string());
        assert_eq!(
            active_model_fallback(),
            Some("mens/just-set-model".to_string())
        );

        *active_model_override().lock().unwrap() = None;
    }

    #[tokio::test]
    async fn get_reads_daemon_preferences_route() {
        let b = FakeBackend::new().on_get(
            "/models/preferences",
            Ok(json!({"v": 1, "data": {"active_model": "openai/gpt-x", "routing_priority": priority(), "selection_policy": "{}"}})),
        );
        assert_eq!(
            get_active_model_impl(&b).await.unwrap().as_deref(),
            Some("openai/gpt-x")
        );
        assert_eq!(b.calls()[0].0, "get:/models/preferences");
    }

    /// Override-sensitive paths share one test: the override is a process-wide static.
    #[tokio::test]
    async fn fallback_and_set_semantics() {
        *active_model_override().lock().unwrap() = Some("prior/model".into());
        let absent = FakeBackend::new().on_get(
            "/models/preferences",
            Ok(json!({"v": 1, "data": {"active_model": null, "routing_priority": priority(), "selection_policy": "{}"}})),
        );
        assert_eq!(
            get_active_model_impl(&absent).await.unwrap().as_deref(),
            Some("prior/model")
        );

        let rejected = FakeBackend::new().on_tool(
            "vox_models_preference_set",
            Ok(json!({"success": false, "error": "model nope not found in registry"})),
        );
        assert!(
            set_active_model_impl(&rejected, "nope".into())
                .await
                .is_err()
        );
        assert_eq!(
            active_model_override().lock().unwrap().as_deref(),
            Some("prior/model")
        );

        let offline = FakeBackend::new(); // every call errors: server unreachable
        set_active_model_impl(&offline, "local/llama".into())
            .await
            .unwrap();
        assert_eq!(
            active_model_override().lock().unwrap().as_deref(),
            Some("local/llama")
        );

        let ok = FakeBackend::new().on_tool(
            "vox_models_preference_set",
            Ok(json!({"success": true, "data": "active_model"})),
        );
        set_active_model_impl(&ok, " openai/gpt-x ".into())
            .await
            .unwrap();
        assert_eq!(
            ok.calls()[0].1,
            json!({"key": "active_model", "value": "openai/gpt-x"})
        );
        assert!(set_active_model_impl(&ok, "  ".into()).await.is_err());
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("model_active.rs");
        assert!(!src.contains(concat!("vox", "_db")) && !src.contains(concat!("tur", "so::")));
    }
}
