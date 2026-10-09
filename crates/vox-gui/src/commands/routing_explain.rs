//! Routing explanation and health for the Models surface, computed by the daemon
//! (`GET /api/v2/models/routing-explain`, `GET /api/v2/models/routing-health`) from its
//! scoreboard-backed registry, so it works against a local or a remote Vox server.

use std::sync::Arc;

use serde_json::Value;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query};

pub(crate) async fn explain_routing_impl<B: BackendCalls>(
    b: &B,
    task: String,
    mode: String,
    complexity: u8,
) -> Result<Value, String> {
    let path = path_with_query(
        "/models/routing-explain",
        &[
            ("task", Some(task)),
            ("mode", Some(mode)),
            ("complexity", Some(complexity.to_string())),
        ],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn get_routing_health_impl<B: BackendCalls>(b: &B) -> Result<Value, String> {
    api_data(b.api_v2_get("/models/routing-health").await?)
}

/// How task dispatch would choose now for `mode` / `task` / `complexity` (1–10). Not chat: chat resolves
/// through `models::select::decide` (premium aliases, confidence gating, per-request axes).
#[tauri::command]
pub async fn explain_routing(
    task: String,
    mode: String,
    complexity: u8,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<Value, String> {
    explain_routing_impl(&backend.current().await, task, mode, complexity).await
}

/// The registry's routing health, checked now.
#[tauri::command]
pub async fn get_routing_health(
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<Value, String> {
    get_routing_health_impl(&backend.current().await).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    #[tokio::test]
    async fn explain_forwards_labels_and_surfaces_server_errors() {
        let path = "/models/routing-explain?task=codegen&mode=balanced&complexity=4";
        let b = FakeBackend::new().on_get(path, Ok(json!({"v": 1, "data": {"complexity": 4}})));
        let v = explain_routing_impl(&b, "codegen".into(), "balanced".into(), 4)
            .await
            .unwrap();
        assert_eq!(v["complexity"], 4);
        assert_eq!(b.calls()[0].0, format!("get:{path}"));

        let bad = FakeBackend::new().on_get(
            "/models/routing-explain?task=codegen&mode=x&complexity=4",
            Ok(json!({"v": 1, "error": {"code": "bad_request", "message": "unknown mode"}})),
        );
        let e = explain_routing_impl(&bad, "codegen".into(), "x".into(), 4)
            .await
            .unwrap_err();
        assert!(e.contains("unknown mode"), "{e}");
    }

    #[tokio::test]
    async fn health_reads_the_daemon_route() {
        let b = FakeBackend::new().on_get(
            "/models/routing-health",
            Ok(json!({"v": 1, "data": {"ok": true}})),
        );
        assert_eq!(get_routing_health_impl(&b).await.unwrap()["ok"], true);
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("routing_explain.rs");
        assert!(!src.contains(concat!("vox", "_db")));
    }
}
