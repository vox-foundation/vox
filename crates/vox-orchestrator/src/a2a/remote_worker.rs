//! Background remote worker loop for Populi `remote_task_envelope` rows.

use std::sync::Arc;
use std::sync::Mutex;

use crate::process_util::quiet_command;

use tracing::Instrument as _;

use super::envelope::{
    REMOTE_TASK_ENVELOPE_TYPE, REMOTE_TASK_RESULT_TYPE, RemoteTaskEnvelope, RemoteTaskResult,
};

const DEFAULT_POPULI_RECEIVER_AGENT: u64 = 2;
const DEFAULT_POPULI_SENDER_AGENT: u64 = 1;
const DEFAULT_MESH_WORKER_NODE_ID: &str = "vox-orch-worker";

#[derive(Debug, Default)]
struct RemotePayloadContext {
    session_id: Option<String>,
    thread_id: Option<String>,
    context_envelope_json: Option<String>,
    harness_spec_json: Option<String>,
}

fn parse_remote_payload_context(payload: &str) -> RemotePayloadContext {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        return RemotePayloadContext::default();
    };
    let session_id = value
        .get("session_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);
    let thread_id = value
        .get("thread_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);
    let context_envelope_json = value.get("context_envelope_json").and_then(|v| {
        if let Some(s) = v.as_str() {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        } else if v.is_object() {
            serde_json::to_string(v).ok()
        } else {
            None
        }
    });
    let harness_spec_json = value.get("harness_spec_json").and_then(|v| {
        if let Some(s) = v.as_str() {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        } else if v.is_object() {
            serde_json::to_string(v).ok()
        } else {
            None
        }
    });
    RemotePayloadContext {
        session_id,
        thread_id,
        context_envelope_json,
        harness_spec_json,
    }
}

/// Process a single `remote_task_envelope` inbox row inside a tracing span.
///
/// The span records `vox.mesh.trace_id` from the W3C `traceparent` carried on
/// the inbox message (S1 level: field attachment only; full context propagation
/// is S2).
/// Outcome of running a dispatched `.vox` source on the worker (Track B).
struct DispatchedExecParts {
    success: bool,
    result: Option<String>,
    error: Option<String>,
}

/// Build the legacy echo result (back-compat / `no-exec` policy): acknowledges
/// the payload byte count without executing anything.
fn echo_result(envelope: &RemoteTaskEnvelope) -> RemoteTaskResult {
    RemoteTaskResult {
        idempotency_key: envelope.idempotency_key.clone(),
        task_id: Some(envelope.task_id),
        success: true,
        result: Some(format!(
            "remote worker accepted payload ({} bytes)",
            envelope.payload.len()
        )),
        error: None,
    }
}

/// Execute a dispatched `.vox` source on this worker.
///
/// Returns `None` when execution is declined by node policy
/// (`policy == "no-exec"`) so the caller falls back to [`echo_result`].
/// Otherwise returns the real execution outcome — including refusals (invalid
/// base64, or a missing/mismatched BLAKE3 integrity hash) as `success: false`
/// **without spawning**. Source-only (`.vox` text) via `vox run --mode interp`
/// with repeatable `--caps` scoped to a per-dispatch tempdir. Reaching this path
/// already implies the poller's
/// `populi_remote_execute_experimental` gate is on.
///
/// SECURITY: this runs attacker-influenced code behind the experimental gate,
/// a non-`no-exec` policy, mandatory integrity verification, and interpreter
/// `--caps` on a unique tempdir. Forwarded secrets are never injected.
/// Curated host env preserved across `env_clear()` so `vox`/the runtime still
/// works; everything else (including the worker's OWN secrets) is stripped from
/// the dispatched subprocess. Only keys present in the worker env are passed.
fn baseline_passthrough_env() -> Vec<(String, String)> {
    const KEYS: &[&str] = &[
        "PATH",
        "HOME",
        "TMPDIR",
        "TMP",
        "TEMP",
        "USER",
        "LOGNAME",
        "LANG",
        "LC_ALL",
        "LD_LIBRARY_PATH",
        "DYLD_LIBRARY_PATH",
        // Windows essentials:
        "SystemRoot",
        "windir",
        "SystemDrive",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "PATHEXT",
        "ComSpec",
        "NUMBER_OF_PROCESSORS",
        "PROCESSOR_ARCHITECTURE",
        "OS",
    ];
    KEYS.iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| ((*k).to_string(), v)))
        .collect()
}

/// Harden a dispatched subprocess: `env_clear()` (so dispatched code can't read
/// the worker's own environment / secrets), restore the curated baseline, then
/// add any caller-supplied env (the source lane always passes empty).
fn harden_dispatch_env(cmd: &mut std::process::Command, secret_env: &[(String, String)]) {
    cmd.env_clear();
    for (k, v) in baseline_passthrough_env() {
        cmd.env(k, v);
    }
    for (k, v) in secret_env {
        cmd.env(k, v);
    }
}

fn parse_populi_agent_id(raw: Option<&str>, default: u64) -> u64 {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(default)
}

fn mesh_worker_node_id() -> String {
    match vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshNodeId).expose() {
        Some(id) if !id.trim().is_empty() => id.to_string(),
        _ => DEFAULT_MESH_WORKER_NODE_ID.to_string(),
    }
}

fn run_dispatched_source(
    source_b64: &str,
    expected_blake3_hex: Option<&str>,
    policy: &str,
    secret_env: &[(String, String)],
) -> Option<DispatchedExecParts> {
    if policy == "no-exec" {
        return None;
    }
    let refuse = |error: String| {
        Some(DispatchedExecParts {
            success: false,
            result: None,
            error: Some(error),
        })
    };

    let source_bytes = match base64::engine::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        source_b64,
    ) {
        Ok(b) => b,
        Err(e) => return refuse(format!("invalid base64 source: {e}")),
    };
    // Integrity is mandatory for any source we are about to execute.
    let Some(expected_hex) = expected_blake3_hex else {
        return refuse("exec_source_blake3_hex is required to execute dispatched source".into());
    };
    let actual_hex = blake3::hash(&source_bytes).to_hex().to_string();
    if actual_hex != expected_hex {
        return refuse(format!(
            "integrity error: source hash mismatch (expected {expected_hex}, got {actual_hex})"
        ));
    }

    let job_dir = std::env::temp_dir().join(format!(
        "vox-dispatch-{}",
        vox_foundation::primitives::id::simple_hex_id()
    ));
    if let Err(e) = std::fs::create_dir_all(&job_dir) {
        return refuse(format!("failed to create dispatch tmp dir: {e}"));
    }
    let tmp_file = job_dir.join("job.vox");
    if let Err(e) = std::fs::write(&tmp_file, &source_bytes) {
        let _ = std::fs::remove_dir_all(&job_dir);
        return refuse(format!("failed to write dispatch tmp file: {e}"));
    }
    let caps_root = std::fs::canonicalize(&job_dir).unwrap_or_else(|_| job_dir.clone());
    // Use the always-available interpreter (`--mode interp`) rather than
    // `--mode script` (which requires a `script-execution` feature build), so a
    // worker can run dispatched source without a specially-built `vox` binary.
    let mut cmd = quiet_command("vox");
    cmd.arg("run")
        .arg("--mode")
        .arg("interp")
        .arg("--caps")
        .arg(format!("fs:rw={}", caps_root.display()))
        .arg("--caps")
        .arg("time:real")
        .arg(&tmp_file)
        .current_dir(&job_dir);
    harden_dispatch_env(&mut cmd, secret_env);
    cmd.env("TMPDIR", &job_dir);
    let output = cmd.output();
    let _ = std::fs::remove_dir_all(&job_dir);

    Some(parts_from_output(output))
}

/// Turn a finished (or failed) subprocess into [`DispatchedExecParts`], applying
/// the 10 MiB combined-output truncation.
fn parts_from_output(output: std::io::Result<std::process::Output>) -> DispatchedExecParts {
    match output {
        Ok(out) => {
            const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;
            let mut stdout = out.stdout;
            let mut stderr = out.stderr;
            if stdout.len() + stderr.len() > MAX_OUTPUT_BYTES {
                if stderr.len() > MAX_OUTPUT_BYTES / 2 {
                    stderr.truncate(MAX_OUTPUT_BYTES / 2);
                }
                let remaining = MAX_OUTPUT_BYTES.saturating_sub(stderr.len());
                if stdout.len() > remaining {
                    stdout.truncate(remaining);
                }
            }
            let combined =
                String::from_utf8_lossy(&stdout).into_owned() + &String::from_utf8_lossy(&stderr);
            let success = out.status.success();
            DispatchedExecParts {
                success,
                result: Some(combined),
                error: if success {
                    None
                } else {
                    Some(format!("Exit code: {:?}", out.status.code()))
                },
            }
        }
        Err(e) => DispatchedExecParts {
            success: false,
            result: None,
            error: Some(format!("failed to spawn `vox`: {e}")),
        },
    }
}

async fn process_one_envelope(
    orchestrator: &crate::orchestrator::Orchestrator,
    client: &vox_populi::http_client::PopuliHttpClient,
    sender_agent: u64,
    receiver_agent: u64,
    msg: vox_populi::transport::A2AStoredMessage,
    node_id: &str,
) {
    let envelope = match serde_json::from_str::<RemoteTaskEnvelope>(&msg.payload) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!(
                message_id = msg.id,
                error = %e,
                "populi remote worker: invalid envelope JSON; acking to drain poison-pill"
            );
            let _ = client
                .relay_a2a_ack(&receiver_agent.to_string(), msg.id)
                .await;
            return;
        }
    };

    // Parse the W3C traceparent into a structured context (P0-T8).
    let trace_ctx = msg
        .traceparent
        .as_deref()
        .and_then(crate::a2a::traceparent::parse);
    let trace_id = trace_ctx
        .as_ref()
        .map(|c| c.trace_id.as_str())
        .unwrap_or("");
    let parent_id = trace_ctx
        .as_ref()
        .map(|c| c.parent_id.as_str())
        .unwrap_or("");
    let exec_lease_id = envelope.exec_lease_id.as_deref().unwrap_or("");
    let span = tracing::info_span!(
        "populi_remote_envelope",
        task_id = envelope.task_id,
        message_id = msg.id,
        exec_lease_id,
        "vox.mesh.trace_id" = trace_id,
        "vox.mesh.parent_span_id" = parent_id,
    );

    async {
        tracing::info!("populi remote worker: processing envelope");

        let payload_context = parse_remote_payload_context(&envelope.payload);
        let envelope_session_id = envelope
            .session_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string);
        let envelope_context_json = envelope
            .context_envelope_json
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string);
        let effective_session_id = payload_context.session_id.or(envelope_session_id);
        let effective_context_json = payload_context
            .context_envelope_json
            .or(envelope_context_json);
        let effective_thread_id = payload_context.thread_id.or_else(|| {
            envelope
                .thread_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(std::string::ToString::to_string)
        });
        let effective_harness_json = payload_context.harness_spec_json.or_else(|| {
            envelope
                .harness_spec_json
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(std::string::ToString::to_string)
        });
        if let (Some(session_id), Some(context_envelope_json)) = (
            effective_session_id.as_deref(),
            effective_context_json.as_deref(),
        ) {
            match serde_json::from_str::<crate::ContextEnvelope>(context_envelope_json) {
                Ok(_) => {
                    let key = crate::socrates::session_context_envelope_key(session_id);
                    crate::sync_lock::rw_write(&*orchestrator.context_store).set(
                        crate::types::AgentId(0),
                        key,
                        context_envelope_json,
                        3600,
                    );
                    let seeded = orchestrator.attach_session_retrieval_envelope_if_present(
                        crate::types::TaskId(envelope.task_id),
                        &Some(session_id.to_string()),
                    );
                    tracing::debug!(
                        message_id = msg.id,
                        task_id = envelope.task_id,
                        session_id,
                        thread_id = effective_thread_id.as_deref(),
                        seeded,
                        "populi remote worker: seeded context store and attempted Socrates attach"
                    );
                }
                Err(err) => {
                    tracing::debug!(
                        message_id = msg.id,
                        error = %err,
                        payload_len = envelope.payload.len(),
                        "populi remote worker: context_envelope_json parse failed"
                    );
                }
            }
        }
        if let Some(harness_spec_json) = effective_harness_json.as_deref() {
            match serde_json::from_str::<crate::AgentHarnessSpec>(harness_spec_json) {
                Ok(harness) => {
                    let expectations = crate::HarnessIngestExpectations {
                        repository_id: envelope.repository_id.as_str(),
                        session_id: effective_session_id.as_deref(),
                        thread_id: effective_thread_id.as_deref(),
                    };
                    if let Err(errs) = crate::validate_agent_harness_ingest(&harness, expectations)
                    {
                        tracing::warn!(
                            message_id = msg.id,
                            task_id = envelope.task_id,
                            errors = %errs.join("; "),
                            "populi remote worker: harness_spec_json failed validation"
                        );
                    } else {
                        tracing::debug!(
                            message_id = msg.id,
                            task_id = envelope.task_id,
                            harness_id = %harness.harness_id,
                            thread_id = effective_thread_id.as_deref(),
                            "populi remote worker: accepted portable harness contract"
                        );
                    }
                }
                Err(err) => tracing::warn!(
                    message_id = msg.id,
                    task_id = envelope.task_id,
                    error = %err,
                    "populi remote worker: harness_spec_json parse failed"
                ),
            }
        }
        // Lease-gated submit: orchestrator holds `task:{task_id}` and passes `exec_lease_id` in the envelope.
        // The worker must not grant a second lease (would conflict on scope) or renew/release as the wrong claimer.
        let orchestrator_holds_lease = !exec_lease_id.is_empty();

        let mut worker_owned_lease_id: Option<String> = None;
        if orchestrator_holds_lease {
            // No worker-side exec lease RPCs; orchestrator renews/releases.
        } else {
            // Legacy / demo: worker acquires a lease keyed like the orchestrator (`task:{task_id}`), not idempotency.
            let scope_key = format!("task:{}", envelope.task_id);
            let lease = match client
                .exec_lease_grant(&vox_populi::transport::RemoteExecLeaseGrantRequest {
                    claimer_node_id: node_id.to_string(),
                    scope_key,
                })
                .await
            {
                Ok(l) => l,
                Err(e) => {
                    tracing::debug!(
                        message_id = msg.id,
                        error = %e,
                        "populi remote worker: lease grant failed; leave inbox row for retry"
                    );
                    return;
                }
            };
            worker_owned_lease_id = Some(lease.lease_id.clone());
            let _ = client
                .exec_lease_renew(&vox_populi::transport::RemoteExecLeaseRenewRequest {
                    lease_id: lease.lease_id,
                    claimer_node_id: node_id.to_string(),
                })
                .await;
        }

        // When the envelope carries `.vox` source, run it via the interpreter.
        // No exec content ⇒ legacy echo. Bundle execution is deleted.
        let result_payload = {
            let policy = "source-only";
            let secret_env: Vec<(String, String)> = Vec::new();

            let executed = if let Some(source_b64) = envelope.exec_source_b64.as_deref() {
                run_dispatched_source(
                    source_b64,
                    envelope.exec_source_blake3_hex.as_deref(),
                    policy,
                    &secret_env,
                )
                .map(|parts| ("source", parts))
            } else {
                None
            };

            match executed {
                Some((kind, parts)) => {
                    tracing::info!(
                        task_id = envelope.task_id,
                        success = parts.success,
                        kind,
                        "populi remote worker: executed dispatched payload"
                    );
                    RemoteTaskResult {
                        idempotency_key: envelope.idempotency_key.clone(),
                        task_id: Some(envelope.task_id),
                        success: parts.success,
                        result: parts.result,
                        error: parts.error,
                    }
                }
                None => echo_result(&envelope),
            }
        };
        let result_json = match serde_json::to_string(&result_payload) {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!(
                    message_id = msg.id,
                    error = %e,
                    "populi remote worker: result serialization failed"
                );
                if let Some(ref lid) = worker_owned_lease_id {
                    let _ = client
                        .exec_lease_release(&vox_populi::transport::RemoteExecLeaseReleaseRequest {
                            lease_id: lid.clone(),
                            claimer_node_id: node_id.to_string(),
                        })
                        .await;
                }
                return;
            }
        };

        let deliver_req = vox_populi::transport::A2ADeliverRequest {
            sender_agent_id: receiver_agent.to_string(),
            receiver_agent_id: sender_agent.to_string(),
            message_type: REMOTE_TASK_RESULT_TYPE.to_string(),
            payload: result_json,
            idempotency_key: Some(format!(
                "remote-result-{}-{}",
                envelope.task_id, envelope.idempotency_key
            )),
            privacy_class: envelope.privacy_class.clone(),
            payload_blake3_hex: None,
            worker_ed25519_sig_b64: None,
            jwe_payload: None,
            task_kind: None,
            model_id: None,
            traceparent: msg.traceparent.clone(),
            priority: 128,
        };
        // Mesh first when this node has a mesh identity and a trusted, addressable
        // peer (plan Task 3.1). The mailbox queues durably before it dials, so a
        // peer that is switched off is a delay rather than the delivery failure the
        // HTTP call below turns it into. HTTP stays as the fallback; deleting it is
        // Phase 6, after this is proven on two machines.
        let deliver_res: Result<(), ()> = if crate::a2a::mesh_relay::try_relay(&deliver_req).await {
            Ok(())
        } else {
            client
                .relay_a2a(&deliver_req)
                .await
                .map(|_| ())
                .map_err(|_| ())
        };
        if deliver_res.is_err() {
            tracing::debug!(
                message_id = msg.id,
                "vox.mesh.trace_id" = trace_id,
                "populi remote worker: result delivery failed; leave source row for retry"
            );
            if let Some(ref lid) = worker_owned_lease_id {
                let _ = client
                    .exec_lease_release(&vox_populi::transport::RemoteExecLeaseReleaseRequest {
                        lease_id: lid.clone(),
                        claimer_node_id: node_id.to_string(),
                    })
                    .await;
            }
            return;
        }

        tracing::info!(
            task_id = envelope.task_id,
            message_id = msg.id,
            "vox.mesh.trace_id" = trace_id,
            "populi remote worker: envelope processed and acked"
        );
        let _ = client
            .relay_a2a_ack(&receiver_agent.to_string(), msg.id)
            .await;
        if let Some(ref lid) = worker_owned_lease_id {
            let _ = client
                .exec_lease_release(&vox_populi::transport::RemoteExecLeaseReleaseRequest {
                    lease_id: lid.clone(),
                    claimer_node_id: node_id.to_string(),
                })
                .await;
        }
    }
    .instrument(span)
    .await;
}

async fn run_remote_worker_tick(
    orchestrator: &crate::orchestrator::Orchestrator,
    client: &vox_populi::http_client::PopuliHttpClient,
    receiver_agent: u64,
    sender_agent: u64,
) {
    // Carry any mail queued for a peer that was off last tick. Nothing arrives
    // to trigger this retry on its own: the message is already ours to deliver.
    let flushed = crate::a2a::mesh_relay::flush_pending().await;
    if flushed > 0 {
        tracing::info!(flushed, "populi remote worker: delivered queued mesh mail");
    }

    let Ok(inbox) = client.relay_a2a_inbox(&receiver_agent.to_string()).await else {
        tracing::debug!(
            receiver_agent = receiver_agent,
            "populi remote worker: inbox HTTP failed"
        );
        return;
    };

    let node_id = mesh_worker_node_id();
    for msg in inbox.messages {
        if msg.message_type != REMOTE_TASK_ENVELOPE_TYPE {
            continue;
        }
        let trace_id = msg
            .traceparent
            .as_deref()
            .and_then(|tp| tp.split('-').nth(1))
            .filter(|s| s.len() == 32)
            .map(str::to_string);
        let span = tracing::info_span!(
            "populi.remote_worker.process_envelope",
            message_id = msg.id,
            "vox.mesh.trace_id" = trace_id.as_deref().unwrap_or(""),
        );
        process_one_envelope(
            orchestrator,
            client,
            sender_agent,
            receiver_agent,
            msg,
            &node_id,
        )
        .instrument(span)
        .await;
    }
}

/// Spawn a periodic worker poll loop that consumes `remote_task_envelope` rows.
pub fn spawn_populi_remote_worker_poller(
    orchestrator: Arc<crate::orchestrator::Orchestrator>,
    join_slot: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
) {
    let Ok(mut guard) = join_slot.lock() else {
        tracing::error!("populi remote worker: join_slot mutex poisoned; poller not restarted");
        return;
    };
    if let Some(h) = guard.take() {
        h.abort();
    }
    let orch = orchestrator.clone();
    *guard = Some(tokio::spawn(async move {
        loop {
            let (interval_secs, run, url, timeout_ms, receiver_agent, sender_agent) = {
                let cfg = crate::sync_lock::rw_read(&*orch.config).clone();
                if !cfg.populi_remote_execute_experimental
                    || cfg.populi_remote_worker_poll_interval_secs == 0
                {
                    (5_u64, false, String::new(), 500_u64, 0_u64, 0_u64)
                } else {
                    let maybe_url = cfg
                        .populi_control_url
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                    if let Some(url) = maybe_url {
                        let receiver_agent = parse_populi_agent_id(
                            cfg.populi_remote_execute_receiver_agent.as_deref(),
                            DEFAULT_POPULI_RECEIVER_AGENT,
                        );
                        let sender_agent = parse_populi_agent_id(
                            cfg.populi_remote_execute_sender_agent.as_deref(),
                            DEFAULT_POPULI_SENDER_AGENT,
                        );
                        (
                            cfg.populi_remote_worker_poll_interval_secs.max(1),
                            true,
                            url,
                            cfg.populi_http_timeout_ms.max(500),
                            receiver_agent,
                            sender_agent,
                        )
                    } else {
                        (5_u64, false, String::new(), 500_u64, 0_u64, 0_u64)
                    }
                }
            };

            if run {
                let client = vox_populi::http_client::PopuliHttpClient::new_with_timeout(
                    &url,
                    std::time::Duration::from_millis(timeout_ms),
                )
                .with_env_token();
                run_remote_worker_tick(&orch, &client, receiver_agent, sender_agent).await;
            }
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        }
    }));
}

/// One-shot remote worker tick using current orchestrator config.
pub async fn populi_remote_worker_tick_once(orchestrator: &crate::orchestrator::Orchestrator) {
    let cfg = crate::sync_lock::rw_read(&*orchestrator.config).clone();
    if !cfg.populi_remote_execute_experimental || cfg.populi_remote_worker_poll_interval_secs == 0 {
        return;
    }
    let Some(url) = cfg
        .populi_control_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return;
    };
    let receiver_agent = parse_populi_agent_id(
        cfg.populi_remote_execute_receiver_agent.as_deref(),
        DEFAULT_POPULI_RECEIVER_AGENT,
    );
    let sender_agent = parse_populi_agent_id(
        cfg.populi_remote_execute_sender_agent.as_deref(),
        DEFAULT_POPULI_SENDER_AGENT,
    );
    let timeout_ms = cfg.populi_http_timeout_ms.max(500);
    let client = vox_populi::http_client::PopuliHttpClient::new_with_timeout(
        url,
        std::time::Duration::from_millis(timeout_ms),
    )
    .with_env_token();
    run_remote_worker_tick(orchestrator, &client, receiver_agent, sender_agent).await;
}

#[cfg(test)]
mod tests {
    use super::{parse_remote_payload_context, run_dispatched_source};
    use serial_test::serial;

    fn b64(s: &str) -> String {
        base64::engine::Engine::encode(&base64::engine::general_purpose::STANDARD, s.as_bytes())
    }

    fn blake3_hex(s: &str) -> String {
        blake3::hash(s.as_bytes()).to_hex().to_string()
    }

    /// `vox` reachable on PATH? Tests that actually spawn it skip cleanly when not.
    fn vox_on_path() -> bool {
        std::process::Command::new("vox")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn no_exec_policy_returns_none_for_echo_fallback() {
        let src = "pub fn main() { print(\"x\") }";
        let got = run_dispatched_source(&b64(src), Some(&blake3_hex(src)), "no-exec", &[]);
        assert!(got.is_none(), "no-exec policy must decline (caller echoes)");
    }

    #[test]
    fn missing_integrity_hash_refuses_without_spawning() {
        let src = "pub fn main() { print(\"x\") }";
        let parts = run_dispatched_source(&b64(src), None, "permissive", &[])
            .expect("a decision is returned");
        assert!(!parts.success);
        assert!(parts.result.is_none());
        assert!(
            parts.error.as_deref().unwrap_or("").contains("blake3"),
            "error should explain the missing integrity hash, got {:?}",
            parts.error
        );
    }

    #[test]
    fn hash_mismatch_refuses_without_spawning() {
        let src = "pub fn main() { print(\"x\") }";
        let parts = run_dispatched_source(&b64(src), Some("deadbeef"), "permissive", &[])
            .expect("a decision is returned");
        assert!(!parts.success);
        assert!(parts.result.is_none());
        assert!(
            parts
                .error
                .as_deref()
                .unwrap_or("")
                .contains("hash mismatch"),
            "error should report a hash mismatch, got {:?}",
            parts.error
        );
    }

    #[test]
    fn invalid_base64_refuses_without_spawning() {
        let parts = run_dispatched_source("!!!not base64!!!", Some("deadbeef"), "permissive", &[])
            .expect("a decision is returned");
        assert!(!parts.success);
        assert!(parts.error.as_deref().unwrap_or("").contains("base64"));
    }

    #[test]
    #[serial(vox_spawn)]
    fn executes_source_and_returns_real_stdout() {
        if !vox_on_path() {
            eprintln!("skipping: `vox` not on PATH");
            return;
        }
        let src = "pub fn main() {\n    print(\"hello from \" + str(2 + 3))\n}\n";
        let parts = run_dispatched_source(&b64(src), Some(&blake3_hex(src)), "permissive", &[])
            .expect("executed");
        assert!(parts.success, "expected success, error={:?}", parts.error);
        assert!(
            parts
                .result
                .as_deref()
                .unwrap_or("")
                .contains("hello from 5"),
            "expected real stdout, got {:?}",
            parts.result
        );
    }

    #[test]
    #[serial(vox_spawn)]
    fn built_exec_source_fields_round_trip_to_real_execution() {
        // End-to-end: the sender helper builds (b64, hash); the worker re-verifies
        // the hash and executes — proving the dispatch contract closes the loop.
        if !vox_on_path() {
            eprintln!("skipping: `vox` not on PATH");
            return;
        }
        let src = "pub fn main() {\n    print(\"answer \" + str(6 * 7))\n}\n";
        let (b64, hex) = crate::a2a::exec_source::build_exec_source_fields(src);
        let parts = run_dispatched_source(&b64, Some(&hex), "permissive", &[]).expect("executed");
        assert!(parts.success, "expected success, error={:?}", parts.error);
        assert!(
            parts.result.as_deref().unwrap_or("").contains("answer 42"),
            "expected real stdout from built source, got {:?}",
            parts.result
        );
    }

    #[test]
    #[serial(vox_spawn)]
    fn nonzero_exit_sets_success_false() {
        if !vox_on_path() {
            eprintln!("skipping: `vox` not on PATH");
            return;
        }
        // Not valid Vox — `vox run` exits non-zero.
        let src = "this is not valid vox source @@@";
        let parts = run_dispatched_source(&b64(src), Some(&blake3_hex(src)), "permissive", &[])
            .expect("executed");
        assert!(!parts.success, "a failing script must report success=false");
        assert!(parts.error.is_some());
    }

    #[test]
    fn parse_remote_payload_context_extracts_session_and_context() {
        let payload = serde_json::json!({
            "task_description": "x",
            "session_id": "  sid-123 ",
            "thread_id": " thread-123 ",
            "context_envelope_json": "{\"schema_version\":1}",
            "harness_spec_json": "{\"schema_version\":1}"
        })
        .to_string();
        let parsed = parse_remote_payload_context(&payload);
        assert_eq!(parsed.session_id.as_deref(), Some("sid-123"));
        assert_eq!(parsed.thread_id.as_deref(), Some("thread-123"));
        assert_eq!(
            parsed.context_envelope_json.as_deref(),
            Some("{\"schema_version\":1}")
        );
        assert_eq!(
            parsed.harness_spec_json.as_deref(),
            Some("{\"schema_version\":1}")
        );
    }

    #[test]
    fn parse_remote_payload_context_handles_missing_fields() {
        let parsed = parse_remote_payload_context("{\"task_description\":\"x\"}");
        assert!(parsed.session_id.is_none());
        assert!(parsed.thread_id.is_none());
        assert!(parsed.context_envelope_json.is_none());
        assert!(parsed.harness_spec_json.is_none());
    }

    #[test]
    fn parse_remote_payload_context_serializes_object_form_context_envelope() {
        let payload = serde_json::json!({
            "session_id": "sid-obj",
            "context_envelope_json": {
                "schema_version": 1,
                "envelope_type": "retrieval_evidence"
            }
        })
        .to_string();
        let parsed = parse_remote_payload_context(&payload);
        assert_eq!(parsed.session_id.as_deref(), Some("sid-obj"));
        let context = parsed
            .context_envelope_json
            .as_deref()
            .expect("context json should be captured");
        let as_value: serde_json::Value = serde_json::from_str(context).expect("valid json");
        assert_eq!(as_value["schema_version"], 1);
        assert_eq!(as_value["envelope_type"], "retrieval_evidence");
    }
}
