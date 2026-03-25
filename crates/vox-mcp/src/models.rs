use schemars::JsonSchema;
use serde::Deserialize;
use vox_orchestrator::types::TaskCategory;

use crate::{ServerState, ToolResult};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListModelsParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SuggestModelParams {
    pub task_category: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetModelParams {
    pub agent_id: u64,
    pub model_id: String,
}

pub async fn list_models(state: &ServerState, _params: ListModelsParams) -> String {
    let orch = &state.orchestrator;
    let handle = orch.models_handle();
    let models = vox_orchestrator::sync_lock::rw_read(&*handle).list_models();
    ToolResult::ok(models).to_json()
}

pub async fn suggest_model(state: &ServerState, params: SuggestModelParams) -> String {
    let orch = &state.orchestrator;

    // Parse task_category from string
    let category = match params.task_category.to_lowercase().as_str() {
        "parsing" => TaskCategory::Parsing,
        "typechecking" => TaskCategory::TypeChecking,
        "debugging" => TaskCategory::Debugging,
        "research" => TaskCategory::Research,
        "testing" => TaskCategory::Testing,
        "codegen" => TaskCategory::CodeGen,
        "review" => TaskCategory::Review,
        _ => TaskCategory::CodeGen, // Default fallback
    };

    let preference = vox_orchestrator::sync_lock::rw_read(&*orch.config_handle()).cost_preference;
    let complexity = 5; // Default for interactive suggestions
    let handle = orch.models_handle();
    if let Some(model) =
        vox_orchestrator::sync_lock::rw_read(&*handle).best_for(category, complexity, preference)
    {
        ToolResult::ok(model).to_json()
    } else {
        ToolResult::<String>::err("No suitable model found for category").to_json()
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetActiveMcpModelParams {
    pub model_id: String,
}

/// Persist the user's preferred MCP inline-chat model (OpenRouter id, Gemini id, Ollama tag, …).
pub async fn set_active_mcp_chat_model(
    state: &ServerState,
    params: SetActiveMcpModelParams,
) -> String {
    let mut lock = state.mcp_chat_model_override.write().unwrap();
    if params.model_id.is_empty() {
        *lock = None;
        ToolResult::ok("cleared active MCP chat model override").to_json()
    } else {
        let id = params.model_id.clone();
        *lock = Some(id.clone());
        ToolResult::ok(format!("Active MCP chat model set to {id}")).to_json()
    }
}

/// Return the active MCP chat model override, if any.
pub async fn get_active_mcp_chat_model(state: &ServerState) -> String {
    let id = state.mcp_chat_model_override.read().unwrap().clone();
    ToolResult::ok(id.unwrap_or_default()).to_json()
}

pub async fn set_model(state: &ServerState, params: SetModelParams) -> String {
    let orch = &state.orchestrator;

    let handle = orch.models_handle();
    if let Some(_) = vox_orchestrator::sync_lock::rw_read(&*handle).get(&params.model_id) {
        vox_orchestrator::sync_lock::rw_write(&*handle)
            .set_override(params.agent_id, params.model_id.clone());
        ToolResult::ok(format!(
            "Successfully overridden model to {} for agent {}",
            params.model_id, params.agent_id
        ))
        .to_json()
    } else {
        ToolResult::<String>::err(format!("Model {} not found in registry", params.model_id))
            .to_json()
    }
}
