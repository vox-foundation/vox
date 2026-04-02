//! HTTP relay and database persistence for A2A messages.

use crate::types::{A2AMessageType, AgentId, CompletionAttestation, MessagePriority, ThreadId};

use super::envelope::{
    DbA2AMessage, REMOTE_TASK_CANCEL_TYPE, REMOTE_TASK_ENVELOPE_TYPE, REMOTE_TASK_RESULT_TYPE,
    RemoteTaskCancel, RemoteTaskEnvelope, RemoteTaskResult,
};

fn fnv1a64(parts: &[&str]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for p in parts {
        for b in p.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

fn default_mesh_idempotency_key(
    sender: AgentId,
    receiver: AgentId,
    msg_type: &A2AMessageType,
    payload: &str,
) -> String {
    format!(
        "mesh-{sender}-{receiver}-{:016x}",
        fnv1a64(&[&msg_type.to_string(), payload])
    )
}

/// Relay a message to another mens node via HTTP.
pub async fn relay_to_mesh(
    client: &vox_populi::http_client::PopuliHttpClient,
    sender: AgentId,
    receiver: AgentId,
    msg_type: A2AMessageType,
    payload: impl Into<String>,
) -> Result<(), String> {
    let payload = payload.into();
    let idempotency_key = default_mesh_idempotency_key(sender, receiver, &msg_type, &payload);
    client
        .relay_a2a(&vox_populi::transport::A2ADeliverRequest {
            sender_agent_id: sender.0.to_string(),
            receiver_agent_id: receiver.0.to_string(),
            message_type: msg_type.to_string(),
            payload,
            idempotency_key: Some(idempotency_key),
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

/// Relay a remote-task cancel hint to the mesh receiver (best-effort).
pub async fn relay_remote_task_cancel(
    client: &vox_populi::http_client::PopuliHttpClient,
    sender: AgentId,
    receiver: AgentId,
    cancel: &RemoteTaskCancel,
) -> Result<(), String> {
    let payload = serde_json::to_string(cancel).map_err(|e| e.to_string())?;
    let idempotency_key = format!(
        "orch-remote-cancel-{}-{}",
        cancel.task_id, cancel.idempotency_key
    );
    client
        .relay_a2a(&vox_populi::transport::A2ADeliverRequest {
            sender_agent_id: sender.0.to_string(),
            receiver_agent_id: receiver.0.to_string(),
            message_type: REMOTE_TASK_CANCEL_TYPE.to_string(),
            payload,
            idempotency_key: Some(idempotency_key),
            privacy_class: None,
            payload_blake3_hex: None,
            worker_ed25519_sig_b64: None,
        })
        .await
        .map(|_| ())
        .map_err(|e: vox_populi::PopuliRegistryError| e.to_string())
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
    max_messages: usize,
    orchestrator: &crate::Orchestrator,
) {
    // Walk newest-first inbox pages so deep `remote_task_result` rows are not stuck behind unrelated mail.
    const MAX_PAGES: usize = 32;

    let mut budget = max_messages.max(1);
    let page_size = max_messages.max(1);
    let mut before_message_id: Option<u64> = None;

    'pages: for _ in 0..MAX_PAGES {
        if budget == 0 {
            break 'pages;
        }
        let Ok(inbox) = client
            .relay_a2a_inbox_limited(
                &parent_inbox_agent_id.to_string(),
                Some(page_size),
                before_message_id,
            )
            .await
        else {
            tracing::debug!(
                parent_inbox_agent_id = parent_inbox_agent_id,
                "populi remote result poll: inbox HTTP failed"
            );
            return;
        };

        if inbox.messages.is_empty() {
            break;
        }

        let page_len = inbox.messages.len();
        let next_cursor = inbox.messages.last().map(|m| m.id);

        for msg in inbox.messages.into_iter().take(page_size) {
            if budget == 0 {
                break 'pages;
            }
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
            let delegate = orchestrator
                .agent_assigned_to_task(task_id)
                .and_then(|aid| orchestrator.agent_queue(aid))
                .and_then(|ql| {
                    let q = crate::sync_lock::rw_read(&*ql);
                    q.current_task()
                        .filter(|t| t.id == task_id)
                        .and_then(|t| t.populi_remote_delegate.clone())
                });
            if let Some(delegate) = delegate.as_ref() {
                if delegate.idempotency_key != result.idempotency_key {
                    tracing::debug!(
                        task_id = tid,
                        message_id = msg.id,
                        expected_idempotency = %delegate.idempotency_key,
                        received_idempotency = %result.idempotency_key,
                        "populi remote result: stale idempotency for task; acking row"
                    );
                    let _ = client
                        .relay_a2a_ack(&parent_inbox_agent_id.to_string(), msg.id)
                        .await;
                    budget = budget.saturating_sub(1);
                    continue;
                }
            }
            let terminal_res = if result.success {
                orchestrator
                    .complete_task_with_attestation(
                        task_id,
                        Some(CompletionAttestation {
                            checks_passed: vec!["peer_review_approved".to_string()],
                            ..Default::default()
                        }),
                    )
                    .await
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

            if should_ack {
                if let Err(e) = client
                    .relay_a2a_ack(&parent_inbox_agent_id.to_string(), msg.id)
                    .await
                {
                    tracing::debug!(
                        error = %e,
                        message_id = msg.id,
                        "populi remote result: ack failed"
                    );
                }
                budget = budget.saturating_sub(1);
                if let Some(delegate) = delegate
                    && let (Some(lease_id), Some(claimer_node_id)) =
                        (delegate.lease_id, delegate.claimer_node_id)
                {
                    let _ = client
                        .exec_lease_release(&vox_populi::transport::RemoteExecLeaseReleaseRequest {
                            lease_id,
                            claimer_node_id,
                        })
                        .await;
                }
            }
        }

        before_message_id = next_cursor;
        if before_message_id.is_none() || page_len < page_size {
            break;
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
