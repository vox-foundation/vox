//! MCP tools: submit Populi / training-style work through the orchestrator (compatibility shim).

use serde::Deserialize;
use serde_json::json;

use crate::params::ToolResult;
use crate::server::ServerState;
use vox_orchestrator::{TaskCapabilityHints, TaskPriority};

/// Arguments for `vox_train_submit`.
#[derive(Debug, Deserialize)]
pub struct TrainSubmitParams {
    /// Human-readable training goal (stored on the orchestrator task).
    pub description: String,
    /// When true, route toward CUDA-capable agent queues when configured.
    #[serde(default)]
    pub require_cuda: bool,
    /// When true, route toward Metal-capable agent queues when configured.
    #[serde(default)]
    pub require_metal: bool,
    /// Optional minimum VRAM hint (MiB) for routing.
    pub min_vram_mb: Option<u32>,
}

/// Enqueue a background orchestrator task tagged for training; returns canonical `vox populi train` hint.
pub async fn train_submit(state: &ServerState, params: TrainSubmitParams) -> String {
    let prefer_gpu_compute = params.require_cuda || params.require_metal;
    let caps = TaskCapabilityHints {
        gpu_cuda: params.require_cuda,
        gpu_metal: params.require_metal,
        min_vram_mb: params.min_vram_mb,
        prefer_gpu_compute,
        ..Default::default()
    };

    let desc = format!(
        "[Populi train orchestration] {}\n\nRun locally: `vox populi train --backend qlora --tokenizer hf --device cuda|metal|cpu` (see docs/src/architecture/populi-training-ssot.md).",
        params.description
    );

    let mut orch = state.orchestrator.lock().await;
    match orch
        .submit_task_with_agent(
            desc,
            vec![],
            Some(TaskPriority::Background),
            None,
            Some(caps),
        )
        .await
    {
        Ok(task_id) => ToolResult::ok(json!({
            "task_id": task_id.0,
            "hint": "Training execution remains in Populi CLI; this task records intent and routes GPU-capable agents when configured.",
            "canonical_cli": "vox populi train",
        }))
        .to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(format!("{e}")).to_json(),
    }
}
