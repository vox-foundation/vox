//! Model commands that run on the Axis machine itself: keyed-provider search, local
//! selection explain/suggest from the cached registry, and host hardware recommendation.

use vox_orchestrator::config::CostPreference;
use vox_orchestrator::models::{
    ModelRegistry, SelectionIntent, TaskCategory, select_with_default_registry,
};
use vox_orchestrator_mcp::gui_ops::dto::models::{ModelCardDto, model_spec_to_card};

fn registry_from_cache() -> ModelRegistry {
    ModelRegistry::from_cache()
}

/// Live keyed-provider model search for Loquela autocomplete.
///
/// Fetches OpenRouter / Anthropic / Google when those keys are present (OpenRouter
/// list is TTL-cached in-process), always merges Mens FS runs, then filters/ranks
/// by `query`. Does **not** read `model-catalog.v1.json`.
#[tauri::command]
pub async fn search_model_cards(
    query: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<ModelCardDto>, String> {
    let q = query.unwrap_or_default();
    let limit = limit.unwrap_or(80).clamp(1, 500);
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let specs =
        vox_orchestrator::catalog_live::search_live_keyed_models(&repo_root, &q, limit).await;
    Ok(specs.iter().map(|m| model_spec_to_card(m, None)).collect())
}

#[tauri::command]
pub async fn get_auto_model_recommendation()
-> Result<vox_orchestrator::models::auto_select::AutoModelSelection, String> {
    // Discrete-GPU VRAM comes from the nvml-probe plugin, if installed.
    vox_orchestrator_mcp::server_state::vram_probe::register_nvml_vram_probe();
    Ok(vox_orchestrator::models::auto_select::select_optimal_local_model_for_host())
}

#[tauri::command]
pub async fn explain_model_selection(
    task: String,
    complexity: Option<u8>,
) -> Result<serde_json::Value, String> {
    let task_category = match task.to_ascii_lowercase().as_str() {
        "codegen" => TaskCategory::CodeGen,
        "research" => TaskCategory::Research,
        "review" => TaskCategory::Review,
        "debugging" => TaskCategory::Debugging,
        "parsing" => TaskCategory::Parsing,
        _ => TaskCategory::General,
    };
    let mut intent = SelectionIntent::for_task(task_category);
    if let Some(c) = complexity {
        intent.complexity = c.clamp(1, 10);
    }
    let outcome = select_with_default_registry(&intent)
        .ok_or_else(|| "no model matched selection intent".to_string())?;
    Ok(serde_json::json!({
        "model_id": outcome.model_id,
        "reason": format!("{:?}", outcome.reason),
        "effective_axes": {
            "efficiency": outcome.effective_axes.efficiency,
            "precision": outcome.effective_axes.precision,
            "latency": outcome.effective_axes.latency,
            "availability": outcome.effective_axes.availability,
            "balance": outcome.effective_axes.balance,
            "mobile": outcome.effective_axes.mobile,
        },
        "provider": outcome.model_spec.provider,
        "tier": format!("{:?}", outcome.model_spec.capabilities.tier),
        "cost_per_1k": outcome.model_spec.cost_per_1k,
    }))
}

#[tauri::command]
pub async fn suggest_model_for_task(task: String) -> Result<String, String> {
    let reg = registry_from_cache();
    let task_category = match task.to_ascii_lowercase().as_str() {
        "codegen" => TaskCategory::CodeGen,
        "research" => TaskCategory::Research,
        "review" => TaskCategory::Review,
        _ => TaskCategory::General,
    };
    reg.best_for(task_category, 5, CostPreference::Performance)
        .map(|m| m.id)
        .ok_or_else(|| "no suitable model".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn auto_model_recommendation_returns_valid_selection() {
        let res = get_auto_model_recommendation().await;
        assert!(res.is_ok());
        let sel = res.unwrap();
        assert!(!sel.selected_model_id.is_empty());
        assert!(!sel.tier_reason.is_empty());
    }

    #[tokio::test]
    async fn explain_model_selection_reports_model_and_axes() {
        let v = explain_model_selection("codegen".into(), Some(42))
            .await
            .expect("bootstrap registry selects a model");
        assert!(v["model_id"].as_str().is_some_and(|s| !s.is_empty()));
        assert!(v["effective_axes"]["efficiency"].is_number());
    }
}
