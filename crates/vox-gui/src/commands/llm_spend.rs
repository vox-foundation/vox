//! Recorded LLM spend vs the budget caps for the Runtime settings surface, served by the daemon
//! (`GET /api/v2/llm/spend`): spend and caps are the **server's**, so a remote Axis shows what
//! the connected Vox server actually charges against.

use std::sync::Arc;

use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::attention::LlmSpendDto;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query};

pub(crate) async fn get_llm_spend_impl<B: BackendCalls>(
    b: &B,
    session_id: Option<String>,
) -> Result<LlmSpendDto, String> {
    let path = path_with_query("/llm/spend", &[("session_id", session_id)]);
    api_data(b.api_v2_get(&path).await?)
}

/// Read recorded LLM spend (session/day/total) + the budget caps. `session_id` scopes the
/// per-session figure; spend is zero when the server has no DB attached (caps still shown).
#[tauri::command]
pub async fn get_llm_spend(
    backend: State<'_, Arc<BackendState>>,
    session_id: Option<String>,
) -> Result<LlmSpendDto, String> {
    get_llm_spend_impl(&backend.current().await, session_id).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    #[tokio::test]
    async fn spend_reads_the_session_scoped_route() {
        let b = FakeBackend::new().on_get(
            "/llm/spend?session_id=s1",
            Ok(json!({"data": {
                "sessionUsd": 0.5, "dayUsd": 1.25, "totalUsd": 9.0,
                "dailyBudgetUsd": 5.0, "perSessionBudgetUsd": 1.0
            }})),
        );
        let spend = get_llm_spend_impl(&b, Some("s1".into())).await.unwrap();
        assert_eq!(spend.day_usd, 1.25);
        assert_eq!(spend.per_session_budget_usd, 1.0);
        assert_eq!(b.calls()[0].0, "get:/llm/spend?session_id=s1");
    }

    #[tokio::test]
    async fn spend_without_session_omits_the_query() {
        let b =
            FakeBackend::new().on_get("/llm/spend", Ok(json!({"data": LlmSpendDto::default()})));
        get_llm_spend_impl(&b, None).await.unwrap();
        assert_eq!(b.calls()[0].0, "get:/llm/spend");
    }

    #[test]
    fn spend_and_user_config_have_no_direct_db_access() {
        for src in [include_str!("llm_spend.rs"), include_str!("user_config.rs")] {
            let (body, _) = src.split_once("#[cfg(test)]").unwrap();
            assert!(!body.contains("vox_db"));
            assert!(!body.contains("GuiDbPool"));
        }
    }
}
