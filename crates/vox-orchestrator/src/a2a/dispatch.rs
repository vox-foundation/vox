//! HTTP relay and database persistence for A2A messages.

use crate::types::{AgentId, A2AMessageType, MessagePriority, ThreadId};

use super::envelope::{
    DbA2AMessage, RemoteTaskEnvelope, REMOTE_TASK_ENVELOPE_TYPE,
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
        })
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
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
