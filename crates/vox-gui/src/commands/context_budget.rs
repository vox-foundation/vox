//! Tauri command for the chat context meter. Served by the daemon
//! (`GET /api/v2/orchestrator/context-budget`), which owns the compaction config and the
//! `llm_interactions` history.

use std::sync::Arc;

use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::orchestrator::ContextBudgetPayload;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query};

pub(crate) async fn context_budget_impl<B: BackendCalls>(
    b: &B,
    session_id: Option<String>,
) -> Result<ContextBudgetPayload, String> {
    let path = path_with_query(
        "/orchestrator/context-budget",
        &[("session_id", session_id)],
    );
    api_data(b.api_v2_get(&path).await?)
}

/// The active context-window budget; with `session_id`, `used_tokens` is the size of that
/// session's latest model interaction.
#[tauri::command]
pub async fn get_context_budget(
    backend: State<'_, Arc<BackendState>>,
    session_id: Option<String>,
) -> Result<ContextBudgetPayload, String> {
    context_budget_impl(&backend.current().await, session_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;
    use serde_json::json;

    #[tokio::test]
    async fn reads_the_context_budget_route() {
        let payload = json!({
            "max_context_tokens": 128_000, "reserved_tokens": 10_000,
            "threshold_tokens": 102_400, "usable_tokens": 118_000,
            "strategy": "balanced", "used_tokens": 42
        });
        let b = FakeBackend::new()
            .on_get(
                "/orchestrator/context-budget?session_id=chat%3A1",
                Ok(json!({ "v": 1, "data": payload })),
            )
            .on_get(
                "/orchestrator/context-budget",
                Ok(json!({ "v": 1, "data": payload })),
            );
        let budget = context_budget_impl(&b, Some("chat:1".into()))
            .await
            .unwrap();
        assert_eq!(budget.used_tokens, 42);
        assert_eq!(budget.threshold_tokens, 102_400);
        assert_eq!(
            context_budget_impl(&b, None).await.unwrap().strategy,
            "balanced"
        );
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("context_budget.rs");
        assert!(!src.contains(concat!("vox", "_db")));
    }
}
