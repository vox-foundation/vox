//! Best-effort Populi / mens HTTP steps (feature `mens`).

use serde_json::{Value, json};

use super::types::{PopuliActivity, PopuliHttpOp};

/// Best-effort mens registration + optional control-plane HTTP (env-derived base URL only).
#[cfg(feature = "mens")]
pub async fn execute_populi_step(activity: &PopuliActivity) -> anyhow::Result<Value> {
    let _ = vox_populi::publish_local_registry_best_effort();
    let vox = vox_populi::resolve_vox_toml_best_effort();
    let env = vox_populi::populi_env_resolved(vox.as_deref());
    let timeout = std::time::Duration::from_millis(activity.timeout_ms.unwrap_or(30_000).max(250));
    if let Some(base) = env.control_addr.clone() {
        let client = vox_populi::http_client::PopuliHttpClient::new_with_timeout(
            normalize_control_base(&base),
            timeout,
        )
        .with_env_token();
        let id = env
            .node_id
            .clone()
            .unwrap_or_else(|| format!("wf-{}", activity.name.replace(' ', "_")));
        let node = vox_populi::node_record_for_current_process(id, Some(base.clone()));
        let mesh_op = populi_op_json(activity.populi_op);
        match activity.populi_op {
            PopuliHttpOp::Noop => Ok(json!({
                "event": "MeshActivity",
                "activity": activity.name,
                "activity_id": activity.activity_id,
                "mesh_op": mesh_op,
                "control": "noop",
            })),
            PopuliHttpOp::Join => match client.join(&node).await {
                Ok(n) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "join_ok",
                    "node_id": n.id,
                })),
                Err(e) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "join_err",
                    "error": e.to_string(),
                })),
            },
            PopuliHttpOp::Snapshot => match client.list_nodes().await {
                Ok(f) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "snapshot_ok",
                    "node_count": f.nodes.len(),
                    "schema_version": f.schema_version,
                })),
                Err(e) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "snapshot_err",
                    "error": e.to_string(),
                })),
            },
            PopuliHttpOp::Heartbeat => match client.heartbeat(&node).await {
                Ok(n) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "heartbeat_ok",
                    "node_id": n.id,
                })),
                Err(e) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "heartbeat_err",
                    "error": e.to_string(),
                })),
            },
        }
    } else {
        Ok(json!({
            "event": "MeshActivity",
            "activity": activity.name,
            "activity_id": activity.activity_id,
            "mesh_op": populi_op_json(activity.populi_op),
            "control": "local_registry_only",
        }))
    }
}

#[cfg(feature = "mens")]
fn populi_op_json(op: PopuliHttpOp) -> &'static str {
    match op {
        PopuliHttpOp::Heartbeat => "heartbeat",
        PopuliHttpOp::Noop => "noop",
        PopuliHttpOp::Join => "join",
        PopuliHttpOp::Snapshot => "snapshot",
    }
}

#[cfg(feature = "mens")]
fn normalize_control_base(addr: &str) -> String {
    let a = addr.trim();
    if a.starts_with("http://") || a.starts_with("https://") {
        a.to_string()
    } else {
        format!("http://{a}")
    }
}
