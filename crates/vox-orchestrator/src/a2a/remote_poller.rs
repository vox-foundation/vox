//! Background drain of Populi `remote_task_result` inbox rows — embedder-agnostic entry points.

use std::sync::Arc;
use std::sync::Mutex;

async fn renew_populi_exec_leases(
    orchestrator: &crate::orchestrator::Orchestrator,
    client: &vox_populi::http_client::PopuliHttpClient,
    receiver_agent: Option<u64>,
    sender_agent: u64,
) {
    let assignments = orchestrator.task_assignments_copy();
    for (task_id, agent_id) in assignments {
        let Some(queue_lock) = orchestrator.agent_queue(agent_id) else {
            continue;
        };
        let delegate = {
            let q = crate::sync_lock::rw_read(&*queue_lock);
            q.current_task()
                .filter(|t| t.id == task_id)
                .and_then(|t| t.populi_remote_delegate.clone())
        };
        let Some(delegate) = delegate else {
            continue;
        };
        let (Some(lease_id), Some(claimer_node_id)) =
            (delegate.lease_id.clone(), delegate.claimer_node_id.clone())
        else {
            continue;
        };

        let renew_res = client
            .exec_lease_renew(&vox_populi::transport::RemoteExecLeaseRenewRequest {
                lease_id,
                claimer_node_id,
            })
            .await;
        match renew_res {
            Ok(()) => continue,
            Err(err) => {
                let terminal_lease_loss =
                    err.is_http_status(404) || err.is_http_status(409) || err.is_http_status(403);
                if !terminal_lease_loss {
                    tracing::debug!(
                        task_id = task_id.0,
                        error = %err,
                        "populi lease renew transient failure; preserving remote hold"
                    );
                    continue;
                }
            }
        }

        let _ = orchestrator.fallback_populi_remote_task_locally(task_id, "lease_renew_failed");
        if let Some(recv_id) = receiver_agent {
            let cancel = crate::a2a::RemoteTaskCancel {
                idempotency_key: delegate.idempotency_key.clone(),
                task_id: task_id.0,
                reason: Some("lease_renew_failed".to_string()),
            };
            let _ = crate::a2a::relay_remote_task_cancel(
                client,
                crate::types::AgentId(sender_agent),
                crate::types::AgentId(recv_id),
                &cancel,
            )
            .await;
        }
    }
}

/// Spawn a periodic poll loop that applies [`super::dispatch::drain_populi_remote_task_results`].
///
/// `join_slot` is typically an embedder-owned `Arc<Mutex<Option<JoinHandle>>>` so the same
/// orchestrator can be re-rooted (abort prior handle, spawn fresh).
pub fn spawn_populi_remote_result_poller(
    orchestrator: Arc<crate::orchestrator::Orchestrator>,
    join_slot: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
) {
    let mut guard = join_slot.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(h) = guard.take() {
        h.abort();
    }
    let orch = orchestrator.clone();
    *guard = Some(tokio::spawn(async move {
        loop {
            let (
                sleep_secs,
                run_drain,
                url_opt,
                timeout_ms,
                parent_agent,
                receiver_agent,
                max_messages,
            ) = {
                let cfg = crate::sync_lock::rw_read(&*orch.config).clone();
                if !cfg.populi_remote_execute_experimental
                    || cfg.populi_remote_result_poll_interval_secs == 0
                {
                    (5u64, false, None, 500u64, 1u64, None, 64usize)
                } else {
                    let url = cfg
                        .populi_control_url
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string());
                    let parent_agent = cfg
                        .populi_remote_execute_sender_agent
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(1_u64);
                    let interval_secs = cfg.populi_remote_result_poll_interval_secs.max(1);
                    let timeout_ms = cfg.populi_http_timeout_ms.max(500);
                    let receiver_agent = cfg
                        .populi_remote_execute_receiver_agent
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .and_then(|s| s.parse::<u64>().ok());
                    (
                        interval_secs,
                        url.is_some(),
                        url,
                        timeout_ms,
                        parent_agent,
                        receiver_agent,
                        cfg.populi_remote_result_max_messages_per_poll.max(1),
                    )
                }
            };

            if run_drain {
                if let Some(url) = url_opt {
                    let client = vox_populi::http_client::PopuliHttpClient::new_with_timeout(
                        &url,
                        std::time::Duration::from_millis(timeout_ms),
                    )
                    .with_env_token();
                    renew_populi_exec_leases(&orch, &client, receiver_agent, parent_agent).await;
                    super::drain_populi_remote_task_results(
                        &client,
                        parent_agent,
                        max_messages,
                        &orch,
                    )
                    .await;
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
        }
    }));
}

/// One-shot drain using current [`crate::config::OrchestratorConfig`] (for tests and manual ticks).
pub async fn populi_remote_result_poll_once(orchestrator: &crate::orchestrator::Orchestrator) {
    let cfg = crate::sync_lock::rw_read(&*orchestrator.config).clone();
    if !cfg.populi_remote_execute_experimental {
        return;
    }
    let url = match cfg
        .populi_control_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(u) => u.to_string(),
        None => return,
    };
    let timeout_ms = cfg.populi_http_timeout_ms.max(500);
    let parent_agent = cfg
        .populi_remote_execute_sender_agent
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1_u64);
    let receiver_agent = cfg
        .populi_remote_execute_receiver_agent
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u64>().ok());
    let max_messages = cfg.populi_remote_result_max_messages_per_poll.max(1);
    let client = vox_populi::http_client::PopuliHttpClient::new_with_timeout(
        &url,
        std::time::Duration::from_millis(timeout_ms),
    )
    .with_env_token();
    renew_populi_exec_leases(orchestrator, &client, receiver_agent, parent_agent).await;
    super::drain_populi_remote_task_results(&client, parent_agent, max_messages, orchestrator)
        .await;
}
