//! Tauri commands backing the Vox Axis "Harness Health" surface (chat harness continuous eval
//! design, 2026-08-02). Read-only and served by the daemon (`GET /api/v2/harness/eval/*`); all
//! writes to these tables happen via `vox harness eval --live`/`publish` (CLI).

use std::sync::Arc;

use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::harness::{HarnessEvalRunDto, RegressionFlagDto};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query};

pub(crate) async fn history_impl<B: BackendCalls>(
    b: &B,
    limit: Option<usize>,
) -> Result<Vec<HarnessEvalRunDto>, String> {
    let path = path_with_query(
        "/harness/eval/history",
        &[("limit", limit.map(|l| l.to_string()))],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn regressions_impl<B: BackendCalls>(
    b: &B,
) -> Result<Vec<RegressionFlagDto>, String> {
    api_data(b.api_v2_get("/harness/eval/regressions").await?)
}

/// Recent harness eval runs with a per-category pass/fail rollup.
#[tauri::command]
pub async fn harness_eval_history(
    backend: State<'_, Arc<BackendState>>,
    limit: Option<usize>,
) -> Result<Vec<HarnessEvalRunDto>, String> {
    history_impl(&backend.current().await, limit).await
}

/// Regressions between the two most recent runs (empty with fewer than two runs).
#[tauri::command]
pub async fn harness_eval_regressions(
    backend: State<'_, Arc<BackendState>>,
) -> Result<Vec<RegressionFlagDto>, String> {
    regressions_impl(&backend.current().await).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;
    use serde_json::json;

    #[tokio::test]
    async fn reads_go_to_the_harness_eval_routes() {
        let b = FakeBackend::new()
            .on_get(
                "/harness/eval/history?limit=10",
                Ok(json!({ "v": 1, "data": [{
                    "run_id": "r1", "git_sha": "abc1234", "triggered_by": "local",
                    "pass_count": 9, "fail_count": 1, "skip_count": 0,
                    "total_cost_usd": 0.05, "started_at_ms": 1000,
                    "category_breakdown": []
                }] })),
            )
            .on_get(
                "/harness/eval/regressions",
                Ok(json!({ "v": 1, "data": [] })),
            );
        let runs = history_impl(&b, Some(10)).await.unwrap();
        assert_eq!(runs[0].pass_count, 9);
        assert!(regressions_impl(&b).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_route_error_surfaces_its_code() {
        let b = FakeBackend::new().on_get(
            "/harness/eval/regressions",
            Ok(json!({ "error": { "code": "harness_eval_error", "message": "bad sha" } })),
        );
        let e = regressions_impl(&b).await.unwrap_err();
        assert!(e.starts_with("harness_eval_error"), "{e}");
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("harness_eval.rs");
        assert!(!src.contains(concat!("vox", "_db")));
        assert!(!src.contains(concat!("vox", "_cli")));
    }
}
