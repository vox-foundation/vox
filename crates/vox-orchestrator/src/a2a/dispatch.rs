//! HTTP relay and database persistence for A2A messages.

use crate::types::{A2AMessageType, AgentId, MessagePriority, ThreadId};

use super::envelope::{
    DbA2AMessage, REMOTE_TASK_ENVELOPE_TYPE, REMOTE_TASK_RESULT_TYPE, RemoteTaskEnvelope,
    RemoteTaskResult,
};

/// Relay a message to another mens node via HTTP.
pub async fn relay_to_mesh(
    client: &vox_populi::http_client::PopuliHttpClient,
    sender: AgentId,
    receiver: AgentId,
    msg_type: A2AMessageType,
    payload: impl Into<String>,
) -> Result<(), String> {
    client
        .relay_a2a(&vox_populi::transport::A2ADeliverRequest {
            sender_agent_id: sender.0.to_string(),
            receiver_agent_id: receiver.0.to_string(),
            message_type: msg_type.to_string(),
            payload: payload.into(),
            idempotency_key: None,
            privacy_class: None,
            payload_blake3_hex: None,
            worker_ed25519_sig_b64: None,
        })
        .await
        .map_err(|e: vox_populi::PopuliRegistryError| e.to_string())
}

/// Relay a structured remote task envelope over the mesh A2A transport.
pub async fn relay_remote_task_envelope(
    client: &vox_populi::http_client::PopuliHttpClient,
    sender: AgentId,
    receiver: AgentId,
    envelope: &RemoteTaskEnvelope,
) -> Result<(), String> {
    let payload = serde_json::to_string(envelope).map_err(|e| e.to_string())?;
    client
        .relay_a2a(&vox_populi::transport::A2ADeliverRequest {
            sender_agent_id: sender.0.to_string(),
            receiver_agent_id: receiver.0.to_string(),
            message_type: REMOTE_TASK_ENVELOPE_TYPE.to_string(),
            payload,
            idempotency_key: Some(envelope.idempotency_key.clone()),
            privacy_class: envelope.privacy_class.clone(),
            payload_blake3_hex: None,
            worker_ed25519_sig_b64: None,
        })
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn task_id_from_remote_mesh_idempotency(key: &str) -> Option<u64> {
    let rest = key.strip_prefix("orch-remote-")?;
    let (tid, _) = rest.split_once('-')?;
    tid.parse().ok()
}

/// Poll the populi HTTP inbox for [`REMOTE_TASK_RESULT_TYPE`] rows and complete/fail local tasks.
///
/// `parent_inbox_agent_id` must match the **sender** agent id used when relaying
/// [`RemoteTaskEnvelope`] so result deliveries addressed to the parent are visible here.
pub async fn drain_populi_remote_task_results(
    client: &vox_populi::http_client::PopuliHttpClient,
    parent_inbox_agent_id: u64,
    orchestrator: &crate::Orchestrator,
) {
    let Ok(inbox) = client
        .relay_a2a_inbox(&parent_inbox_agent_id.to_string())
        .await
    else {
        tracing::debug!(
            parent_inbox_agent_id = parent_inbox_agent_id,
            "populi remote result poll: inbox HTTP failed"
        );
        return;
    };
    for msg in inbox.messages {
        if msg.message_type != REMOTE_TASK_RESULT_TYPE {
            continue;
        }
        let Ok(result) = serde_json::from_str::<RemoteTaskResult>(&msg.payload) else {
            tracing::debug!(
                message_id = msg.id,
                "populi remote result: skip invalid JSON payload"
            );
            continue;
        };
        let task_id = result
            .task_id
            .or_else(|| task_id_from_remote_mesh_idempotency(&result.idempotency_key));
        let Some(tid) = task_id else {
            tracing::debug!(
                idempotency = %result.idempotency_key,
                "populi remote result: could not resolve task id"
            );
            continue;
        };
        let task_id = crate::types::TaskId(tid);
        let terminal_res = if result.success {
            orchestrator.complete_task(task_id).await
        } else {
            let detail = result
                .error
                .clone()
                .or(result.result.clone())
                .unwrap_or_else(|| "remote task failed".to_string());
            orchestrator.fail_task(task_id, detail).await
        };

        let should_ack = matches!(
            &terminal_res,
            Ok(()) | Err(crate::orchestrator::OrchestratorError::TaskNotFound(_))
        );

        match &terminal_res {
            Ok(()) => {}
            Err(crate::orchestrator::OrchestratorError::TaskNotFound(_)) => {
                tracing::debug!(
                    task_id = tid,
                    message_id = msg.id,
                    "populi remote result: task missing locally; acking stale inbox row"
                );
            }
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    task_id = tid,
                    message_id = msg.id,
                    "populi remote result: terminal transition failed; leaving row for retry"
                );
            }
        }

        if should_ack
            && let Err(e) = client
                .relay_a2a_ack(&parent_inbox_agent_id.to_string(), msg.id)
                .await
        {
            tracing::debug!(
                error = %e,
                message_id = msg.id,
                "populi remote result: ack failed"
            );
        }
    }
}

/// Send a message to the database with circuit breaker protection.
pub async fn send_to_db_with_breaker(
    db: &vox_db::VoxDb,
    sender: AgentId,
    receiver: AgentId,
    msg_type: A2AMessageType,
    payload: impl Into<String> + Clone,
    priority: MessagePriority,
    thread_id: Option<ThreadId>,
    repository_id: &str,
) -> Result<String, String> {
    db.breaker()
        .call(|| async {
            send_to_db(
                db,
                sender,
                receiver,
                msg_type,
                payload.clone(),
                priority,
                thread_id,
                repository_id,
            )
            .await
        })
        .await
}

/// Send a message to the database for delivery (cross-node).
pub async fn send_to_db(
    store: &vox_db::VoxDb,
    sender: AgentId,
    receiver: AgentId,
    msg_type: A2AMessageType,
    payload: impl Into<String>,
    priority: MessagePriority,
    thread_id: Option<ThreadId>,
    repository_id: &str,
) -> Result<String, String> {
    let uuid = uuid::Uuid::new_v4().to_string();
    let priority_val = match priority {
        MessagePriority::Low => 0,
        MessagePriority::Normal => 1,
        MessagePriority::High => 2,
        MessagePriority::Critical => 3,
    };
    let payload = payload.into();
    let thread_str = thread_id.map(|t| t.0);

    store
        .send_a2a_message(
            &uuid,
            &sender.0.to_string(),
            &receiver.0.to_string(),
            msg_type.into_str(),
            &payload,
            priority_val,
            thread_str.as_deref(),
            repository_id,
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(uuid)
}

/// Poll for new unacknowledged messages for an agent from the database.
pub async fn poll_inbox_from_db(
    store: &vox_db::VoxDb,
    agent_id: AgentId,
    repository_id: &str,
) -> Result<Vec<DbA2AMessage>, String> {
    let rows = store
        .poll_a2a_inbox(&agent_id.0.to_string(), repository_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut msgs = Vec::new();
    for row in rows {
        msgs.push(DbA2AMessage {
            id: row.id as u64,
            message_uuid: row.message_uuid,
            sender_agent: row.sender_agent,
            receiver_agent: row.receiver_agent,
            msg_type: row.msg_type,
            payload: row.payload,
            priority: row.priority,
            thread_id: row.thread_id,
            acknowledged: row.acknowledged,
            created_at: row.created_at,
            repository_id: row.repository_id,
        });
    }
    Ok(msgs)
}

/// Mark a message as acknowledged in the database.
pub async fn acknowledge_db_message(
    store: &vox_db::VoxDb,
    message_uuid: &str,
) -> Result<(), String> {
    store
        .acknowledge_a2a_message_by_uuid(message_uuid)
        .await
        .map_err(|e| e.to_string())
}

/// Remove old acknowledged messages from the database.
pub async fn prune_old_a2a_messages(
    store: &vox_db::VoxDb,
    older_than_days: u32,
) -> Result<u64, String> {
    store
        .prune_a2a_messages(older_than_days)
        .await
        .map_err(|e| e.to_string())
}
