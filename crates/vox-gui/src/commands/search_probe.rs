//! Research engine settings and search-provider probes, served by the daemon
//! (`GET /api/v2/research/engine-status`, `vox_research_engine_config_set`,
//! `vox_research_engine_probe`): provider keys, quotas and probes are the **server's**, so the
//! panel describes the engine that will actually run research.

use std::sync::Arc;

use serde_json::json;
use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::research::{
    ResearchEngineConfigDto, ResearchEngineStatusDto,
};
pub use vox_search::probe::ProviderProbeResult;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, tool_data};

const CONFIG_SET_TOOL: &str = "vox_research_engine_config_set";
const PROBE_TOOL: &str = "vox_research_engine_probe";

pub(crate) async fn engine_status_impl<B: BackendCalls>(
    b: &B,
) -> Result<ResearchEngineStatusDto, String> {
    api_data(b.api_v2_get("/research/engine-status").await?)
}

pub(crate) async fn save_config_impl<B: BackendCalls>(
    b: &B,
    config: ResearchEngineConfigDto,
) -> Result<(), String> {
    tool_data(
        CONFIG_SET_TOOL,
        b.call_tool(CONFIG_SET_TOOL, json!({ "config": config }))
            .await?,
    )
}

pub(crate) async fn probe_one_impl<B: BackendCalls>(
    b: &B,
    provider: String,
    query: String,
) -> Result<ProviderProbeResult, String> {
    tool_data(
        PROBE_TOOL,
        b.call_tool(PROBE_TOOL, json!({ "provider": provider, "query": query }))
            .await?,
    )
}

pub(crate) async fn probe_all_impl<B: BackendCalls>(
    b: &B,
    query: String,
) -> Result<Vec<ProviderProbeResult>, String> {
    tool_data(
        PROBE_TOOL,
        b.call_tool(PROBE_TOOL, json!({ "query": query })).await?,
    )
}

#[tauri::command]
pub async fn get_research_engine_status(
    backend: State<'_, Arc<BackendState>>,
) -> Result<ResearchEngineStatusDto, String> {
    engine_status_impl(&backend.current().await).await
}

#[tauri::command]
pub async fn save_research_engine_config(
    backend: State<'_, Arc<BackendState>>,
    config: ResearchEngineConfigDto,
) -> Result<(), String> {
    save_config_impl(&backend.current().await, config).await
}

#[tauri::command]
pub async fn probe_search_provider(
    backend: State<'_, Arc<BackendState>>,
    provider: String,
    query: String,
) -> Result<ProviderProbeResult, String> {
    probe_one_impl(&backend.current().await, provider, query).await
}

#[tauri::command]
pub async fn probe_all_search_providers(
    backend: State<'_, Arc<BackendState>>,
    query: String,
) -> Result<Vec<ProviderProbeResult>, String> {
    probe_all_impl(&backend.current().await, query).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    #[test]
    fn engine_config_accepts_camel_case_from_the_ui() {
        let cfg: ResearchEngineConfigDto = serde_json::from_str(
            r#"{"activeLane":"deep","fastTimeoutMs":2200,"enabledProviders":["arxiv"]}"#,
        )
        .unwrap();
        assert_eq!(cfg.active_lane.as_deref(), Some("deep"));
        assert_eq!(cfg.fast_timeout_ms, Some(2200));
        assert_eq!(cfg.deep_timeout_ms, None);
        assert_eq!(cfg.enabled_providers, Some(vec!["arxiv".to_string()]));
    }

    #[tokio::test]
    async fn engine_commands_go_to_the_route_and_ops() {
        let b = FakeBackend::new()
            .on_get(
                "/research/engine-status",
                Ok(json!({ "v": 1, "data": {
                    "active_lane": "fast", "fast_timeout_ms": 1, "deep_timeout_ms": 2,
                    "providers": [], "free_key_offers": []
                }})),
            )
            .on_tool(
                CONFIG_SET_TOOL,
                Ok(json!({ "success": true, "data": null })),
            )
            .on_tool(
                PROBE_TOOL,
                Ok(json!({ "success": false, "error": "Query cannot be empty" })),
            );
        assert_eq!(engine_status_impl(&b).await.unwrap().active_lane, "fast");
        let cfg = ResearchEngineConfigDto {
            active_lane: Some("deep".into()),
            ..Default::default()
        };
        save_config_impl(&b, cfg).await.unwrap();
        let err = probe_one_impl(&b, "wikipedia".into(), " ".into())
            .await
            .unwrap_err();
        assert!(err.ends_with("Query cannot be empty"), "{err}");
        probe_all_impl(&b, "q".into()).await.unwrap_err();
        let calls = b.calls();
        assert_eq!(calls[1].1["config"]["active_lane"], "deep");
        assert_eq!(calls[2].1["provider"], "wikipedia");
        assert!(calls[3].1.get("provider").is_none());
    }
}
