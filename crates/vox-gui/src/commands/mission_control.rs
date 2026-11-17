//! Tauri commands for the Mission Control and Vox Concilium / Populi surfaces.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use vox_foundation::protocol::orch_daemon_method;
use vox_orchestrator::{
    ConciliumPilotActionInput, ConciliumPilotActionResult, ConciliumSnapshotDto,
    PopuliPlacementDto, SequenceBoardEntry,
};

use crate::remote::backend::BackendState;

#[path = "mission_control/concilium_impl.rs"]
mod concilium_impl;

use concilium_impl::{
    call_orchestrator_daemon, concilium_board_impl, concilium_pilot_action_impl,
    concilium_placement_impl, concilium_snapshot_impl,
};

/// One edge in the subagent delegation tree returned by `list_subagent_tree`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SubagentTreeNode {
    pub task_id: u64,
    pub agent_id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_agent_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_task_id: Option<u64>,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_turn_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McApprovalRow {
    pub approval_id: String,
    pub tool: String,
    pub summary: String,
    pub requested_at_ms: i64,
}

#[derive(Debug, Deserialize)]
pub struct SetMeshPolicyInput {
    pub task_id: u64,
    pub policy: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct MeshPolicyResult {
    pub ok: bool,
    pub message: String,
}

// toestub-ignore(skeleton/untested-pub-api) — Tauri IPC wrappers; logic unit-tested in concilium_impl
#[tauri::command]
pub async fn list_subagent_tree(
    daemon: tauri::State<'_, Arc<BackendState>>,
) -> Result<Vec<SubagentTreeNode>, String> {
    let resp = call_orchestrator_daemon(
        &daemon,
        orch_daemon_method::SUBAGENT_TREE,
        serde_json::json!({}),
    )
    .await?;
    Ok(resp
        .get("tree")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default())
}

#[tauri::command]
pub async fn list_mc_approvals(
    daemon: tauri::State<'_, Arc<BackendState>>,
) -> Result<Vec<McApprovalRow>, String> {
    let resp = call_orchestrator_daemon(
        &daemon,
        orch_daemon_method::LIST_PENDING_APPROVALS,
        serde_json::json!({}),
    )
    .await?;
    Ok(resp
        .get("approvals")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default())
}

#[tauri::command]
pub async fn set_task_mesh_policy(
    input: SetMeshPolicyInput,
    daemon: tauri::State<'_, Arc<BackendState>>,
) -> Result<MeshPolicyResult, String> {
    let resp = call_orchestrator_daemon(
        &daemon,
        orch_daemon_method::SET_MESH_POLICY,
        serde_json::json!({
            "task_id": input.task_id,
            "policy": input.policy,
        }),
    )
    .await?;
    let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let message = resp
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or(if ok { "ok" } else { "error" })
        .to_string();
    Ok(MeshPolicyResult { ok, message })
}

/// Phase 454 (CONC-03): Fetch live Vox Concilium causal swimlane snapshot via daemon RPC.
#[tauri::command]
pub async fn concilium_snapshot(
    daemon: tauri::State<'_, Arc<BackendState>>,
) -> Result<ConciliumSnapshotDto, String> {
    concilium_snapshot_impl(&daemon.current().await).await
}

/// Phase 454 (CONC-03): Fetch `#seq` Shared Sequence Board entries via daemon RPC.
#[tauri::command]
pub async fn concilium_board(
    daemon: tauri::State<'_, Arc<BackendState>>,
    since_seq: Option<u64>,
    seqs: Option<Vec<u64>>,
    channel: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SequenceBoardEntry>, String> {
    concilium_board_impl(&daemon.current().await, since_seq, seqs, channel, limit).await
}

/// Phase 454 (CONC-03): Fetch hybrid Populi compute node placement map via daemon RPC.
#[tauri::command]
pub async fn concilium_placement(
    daemon: tauri::State<'_, Arc<BackendState>>,
) -> Result<PopuliPlacementDto, String> {
    concilium_placement_impl(&daemon.current().await).await
}

/// Phase 454 (CONC-03): Dispatch a pilot steering action via daemon RPC.
#[tauri::command]
pub async fn concilium_pilot_action(
    input: ConciliumPilotActionInput,
    daemon: tauri::State<'_, Arc<BackendState>>,
) -> Result<ConciliumPilotActionResult, String> {
    concilium_pilot_action_impl(&daemon.current().await, &input).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_control_has_zero_direct_vox_db_handles() {
        let src = include_str!("mission_control.rs");
        let impl_src = include_str!("mission_control/concilium_impl.rs");
        for body in [src, impl_src] {
            let prod = body.split("\n#[cfg(test)]").next().unwrap_or(body);
            assert!(!prod.contains(concat!("vox", "_db")));
            assert!(!prod.contains(concat!("Vox", "Db")));
        }
    }

    #[test]
    fn mesh_policy_input_round_trips_json() {
        let parsed: SetMeshPolicyInput =
            serde_json::from_str(r#"{"task_id":7,"policy":"local_only"}"#).expect("parse policy");
        assert_eq!(parsed.task_id, 7);
        assert_eq!(parsed.policy, serde_json::json!("local_only"));
    }
}
