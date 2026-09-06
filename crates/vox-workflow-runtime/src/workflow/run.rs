//! Durable workflow runner: drives the workflow body through [`WorkflowHost`].
//!
//! Task 1.4 replaces the old plan-walking loop (linearize-then-replay a
//! static `WorkflowReplayIr`) with a live bridge to the tree-walking
//! interpreter: every `Begin`/`End`/`Patch`/`Wait`/`WaitSignal` request the
//! interpreter thread emits is journaled here as a canonical
//! `workflow-journal.v1` event, and the runner's decision (`Execute` /
//! `Replay` / `Park` / `Ack`) is fed straight back. The static linearizer
//! (`workflow::plan`) is untouched — it still backs other durability probes
//! and is retired in Task 1.5, not here.

use serde_json::{Value, json};
use vox_compiler::hir::HirModule;

use super::host::{HostDecision, HostOutcome, HostRequest, WorkflowHost};
use super::tracker::{DefaultTracker, WorkflowTracker};

/// Version tag for interpreted workflow journal events emitted by this crate.
pub const WORKFLOW_JOURNAL_VERSION: u32 = 1;

/// Derive a stable, content-addressed `activity_id` from structural inputs.
///
/// The id is a BLAKE3 hex digest of `"{workflow_name}\0{activity_name}\0{position}"`,
/// truncated to 32 hex chars for readability. This is deterministic across replays as
/// long as the workflow topology (activity order and names) does not change.
pub(crate) fn derive_activity_id(
    workflow_name: &str,
    activity_name: &str,
    position: usize,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(workflow_name.as_bytes());
    hasher.update(b"\0");
    hasher.update(activity_name.as_bytes());
    hasher.update(b"\0");
    hasher.update(position.to_le_bytes().as_ref());
    let hash = hasher.finalize();
    // 16 bytes → 32 lowercase hex chars: unique enough for a workflow's activity set.
    let bytes = &hash.as_bytes()[..16];
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Execute a workflow with the default (no-op, non-durable) tracker.
pub async fn interpret_workflow(
    hir: &HirModule,
    workflow_name: &str,
) -> anyhow::Result<Vec<Value>> {
    let mut tracker = DefaultTracker;
    interpret_workflow_durable(hir, workflow_name, Vec::new(), &mut tracker).await
}

/// Execute a workflow's real body through [`WorkflowHost`], journaling every
/// Begin/End/Patch exchange with the dedicated interpreter thread.
///
/// `args` are the workflow's call arguments as journal-JSON (strict `__vox`
/// envelopes for non-primitive values; see [`vox_compiler::eval::value::VoxValue`]).
/// Returns the ordered list of `workflow-journal.v1` events recorded for this run.
pub async fn interpret_workflow_durable(
    hir: &HirModule,
    workflow_name: &str,
    args: Vec<Value>,
    tracker: &mut impl WorkflowTracker,
) -> anyhow::Result<Vec<Value>> {
    // Spawn first: `WorkflowHost::spawn` validates the workflow exists before
    // any thread starts, so a bad `workflow_name` fails here without ever
    // touching the tracker (no spurious `workflow_started` row for a run that
    // never began).
    let mut host = WorkflowHost::spawn(hir.clone(), workflow_name, args.clone())?;

    // `on_workflow_started` must land before `persist_run_args`: the
    // `VoxDbTracker` impl of the latter is an `UPDATE ... WHERE run_id = ?`
    // against the row the former just inserted.
    tracker.on_workflow_started(workflow_name, 0).await?;
    tracker.persist_run_args(&Value::Array(args)).await?;

    // `steps` is required by `workflow-journal.v1.schema.json`'s
    // `WorkflowStarted` shape. The host discovers activities dynamically as
    // the interpreter runs, so there is no static plan length to report here;
    // 0 is the honest value until Phase 3 tracks live progress differently.
    let mut journal = vec![versioned_event(json!({
        "event": "WorkflowStarted",
        "workflow": workflow_name,
        "steps": 0,
    }))];
    let mut position: usize = 0;

    while let Some(req) = host.next().await {
        match req {
            HostRequest::Patch {
                change_id,
                min,
                max,
            } => {
                handle_workflow_patch(workflow_name, &change_id, min, max, &mut journal, tracker)
                    .await?;
                host.respond(HostDecision::Ack)?;
            }
            HostRequest::Begin { name, options, .. } => {
                // G18: mesh / PopuliActivity is a loud error in the body-first
                // runner, not a silent local execution. Park unblocks the
                // interpreter thread (which is waiting on this exact
                // decision), then the host is torn down and we fail loudly.
                if activity_is_mesh(&name) {
                    host.respond(HostDecision::Park("unused".into()))?;
                    let _ = host.finish().await;
                    anyhow::bail!(
                        "mesh / PopuliActivity `{name}` is unsupported in the body-first \
                         runner; use the deferred mens path instead"
                    );
                }

                // Phase 1: honor explicit `with { activity_id: "…" }` values
                // is deferred — Phase 5 also adds duplicate-id detection.
                let _explicit_id = explicit_activity_id(options.as_ref());

                let activity_id = derive_activity_id(workflow_name, &name, position);
                position += 1;

                if tracker
                    .is_activity_completed(workflow_name, &activity_id)
                    .await?
                {
                    let prior = tracker
                        .load_activity_result(workflow_name, &activity_id)
                        .await?
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "activity `{name}` marked completed but no result stored \
                                 for `{activity_id}`"
                            )
                        })?;
                    refuse_canned_local_activity(&prior)?;
                    journal.push(versioned_event(json!({
                        "event": "ActivityReplayed",
                        "workflow": workflow_name,
                        "activity": name,
                        "activity_id": activity_id,
                        "replay_source": "workflow_activity_log",
                        "result_event": prior
                            .get("event")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown"),
                        "result": prior.clone(),
                    })));
                    host.respond(HostDecision::Replay(prior))?;
                } else {
                    tracker
                        .on_activity_started(workflow_name, &name, &activity_id)
                        .await?;
                    journal.push(versioned_event(json!({
                        "event": "ActivityStarted",
                        "workflow": workflow_name,
                        "activity": name,
                        "activity_id": activity_id,
                    })));
                    host.respond(HostDecision::Execute)?;
                }
            }
            HostRequest::End {
                name,
                result,
                failed: true,
            } => {
                // The interpreter thread is already unwinding an
                // AssertionFailed/Panic at this point (see
                // `activity_hook::dispatch_activity`); it discards whatever
                // decision we send and re-raises regardless. We still must
                // respond — the thread is blocked on `blocking_recv()` — and
                // we record the failure for observability before it does.
                let _ = result;
                let activity_id =
                    derive_activity_id(workflow_name, &name, position.saturating_sub(1));
                let error_msg = format!("activity `{name}` panicked during execution");
                tracker
                    .on_activity_attempt_failed(workflow_name, &name, &activity_id, 1, &error_msg)
                    .await?;
                journal.push(versioned_event(json!({
                    "event": "ActivityAttemptFailed",
                    "workflow": workflow_name,
                    "activity": name,
                    "activity_id": activity_id,
                    "attempt": 1,
                    "max_attempts": 1,
                    "error": error_msg,
                })));
                host.respond(HostDecision::Ack)?;
            }
            HostRequest::End {
                name,
                result,
                failed: false,
            } => {
                // `Result::Err` reaches here too (`failed` is false — see
                // `activity_hook::result_err_is_not_failed`): it is a value,
                // not a runner failure, and flows to `?` like any other Ok.
                let activity_id =
                    derive_activity_id(workflow_name, &name, position.saturating_sub(1));
                tracker
                    .on_activity_completed(workflow_name, &name, &activity_id, &result)
                    .await?;
                journal.push(versioned_event(json!({
                    "event": "ActivityCompleted",
                    "workflow": workflow_name,
                    "activity": name,
                    "activity_id": activity_id,
                    "result": result,
                })));
                host.respond(HostDecision::Ack)?;
            }
            HostRequest::Wait { .. } | HostRequest::WaitSignal { .. } => {
                // Timer wakers and signal delivery land in Phase 3/4; for now
                // any wait parks the workflow rather than either blocking
                // forever or silently completing.
                host.respond(HostDecision::Park(
                    "timers and signals land in Phase 3/4".into(),
                ))?;
            }
        }
    }

    match host.finish().await? {
        HostOutcome::Completed(return_value) => {
            tracker.on_workflow_completed(workflow_name).await?;
            journal.push(versioned_event(json!({
                "event": "WorkflowCompleted",
                "workflow": workflow_name,
                "return_value": return_value,
            })));
            Ok(journal)
        }
        // Parked in this task: return the journal as-is, without
        // `WorkflowCompleted` and without failing. There is no `WorkflowParked`
        // event in `workflow-journal.v1.schema.json` — Phase 3 adds a `parked`
        // run status instead of inventing an unschematized event here.
        HostOutcome::Parked(_reason) => Ok(journal),
        HostOutcome::Failed(msg) => anyhow::bail!("{msg}"),
    }
}

async fn handle_workflow_patch(
    workflow_name: &str,
    change_id: &str,
    min: u32,
    max: u32,
    journal: &mut Vec<Value>,
    tracker: &mut impl WorkflowTracker,
) -> anyhow::Result<u32> {
    if let Some(prior) = tracker
        .load_workflow_patch(workflow_name, change_id)
        .await?
    {
        journal.push(versioned_event(json!({
            "event": "WorkflowPatch",
            "workflow": workflow_name,
            "change_id": change_id,
            "version": prior,
            "replayed": true,
        })));
        return Ok(prior);
    }
    tracker
        .record_workflow_patch(workflow_name, change_id, max)
        .await?;
    journal.push(versioned_event(json!({
        "event": "WorkflowPatch",
        "workflow": workflow_name,
        "change_id": change_id,
        "version": max,
        "min_supported": min,
        "max_supported": max,
        "replayed": false,
    })));
    Ok(max)
}

/// Match the mesh-activity naming convention the linearizer
/// (`workflow::plan::collect_from_expr`) already uses: activities named
/// `mesh_*` are PopuliActivity / mesh calls. `HirFn` carries no
/// `execution_boundary` field today, so this predicate is name-based, exactly
/// like `plan.rs`'s `is_mesh` check.
fn activity_is_mesh(name: &str) -> bool {
    name.starts_with("mesh_")
}

/// Read an explicit `with { activity_id: "…" }` override from Begin options.
fn explicit_activity_id(options: Option<&Value>) -> Option<String> {
    options
        .and_then(|o| o.get("activity_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Refuse to replay a pre-Phase-1 canned `LocalActivity` row: those rows are
/// synthetic markers the old plan-walking runner wrote (`status: "executed"`,
/// no real return value), not a real activity body's result. Resuming a run
/// that predates this rewrite against one would silently feed the interpreter
/// a fake value instead of the real one the workflow body expects.
fn refuse_canned_local_activity(prior: &Value) -> anyhow::Result<()> {
    if prior.get("event").and_then(Value::as_str) == Some("LocalActivity")
        && prior.get("status").and_then(Value::as_str) == Some("executed")
    {
        anyhow::bail!(
            "refusing to replay a pre-Phase-1 canned LocalActivity row; \
             start a new run_id after upgrading"
        );
    }
    Ok(())
}

fn versioned_event(mut entry: Value) -> Value {
    if let Value::Object(obj) = &mut entry {
        obj.entry("journal_version".to_string())
            .or_insert_with(|| json!(WORKFLOW_JOURNAL_VERSION));
    }
    entry
}

#[cfg(test)]
use std::time::Duration;

#[cfg(test)]
async fn execute_with_retry_logic<T, Work, WorkFut, OnFailed, OnRetry>(
    max_attempts: u32,
    initial_backoff_ms: u64,
    journal: &mut Vec<Value>,
    mut on_failed: OnFailed,
    mut on_retry: OnRetry,
    mut work: Work,
) -> anyhow::Result<T>
where
    Work: FnMut() -> WorkFut,
    WorkFut: std::future::Future<Output = anyhow::Result<T>>,
    OnFailed: FnMut(u32, &anyhow::Error, &mut Vec<Value>),
    OnRetry: FnMut(u32, u64, &mut Vec<Value>),
{
    let mut next_delay_ms = initial_backoff_ms;
    for attempt in 1..=max_attempts.max(1) {
        match work().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                on_failed(attempt, &err, journal);
                if attempt >= max_attempts.max(1) {
                    return Err(err);
                }
                let delay_ms = next_delay_ms.max(1);
                on_retry(attempt, delay_ms, journal);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                next_delay_ms = next_delay_ms.saturating_mul(2).min(60_000);
            }
        }
    }

    unreachable!("workflow retry loop must return or error")
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonschema::validator_for;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn retry_helper_retries_until_success_and_records_events() {
        let mut journal = Vec::new();
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_for_closure = attempts.clone();

        let result = execute_with_retry_logic(
            3,
            1,
            &mut journal,
            |attempt, err, journal| {
                journal.push(versioned_event(json!({
                    "event": "ActivityAttemptFailed",
                    "attempt": attempt,
                    "error": err.to_string(),
                })));
            },
            |attempt, delay_ms, journal| {
                journal.push(versioned_event(json!({
                    "event": "ActivityRetryScheduled",
                    "attempt": attempt,
                    "delay_ms": delay_ms,
                })));
            },
            move || {
                let attempts = attempts_for_closure.clone();
                async move {
                    let current = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    if current < 3 {
                        anyhow::bail!("attempt {current} failed");
                    }
                    Ok::<_, anyhow::Error>(json!({"event": "MeshActivity"}))
                }
            },
        )
        .await;
        let result = result.expect("retry helper should eventually succeed");
        assert_eq!(result["event"].as_str(), Some("MeshActivity"));
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        let failure_events = journal
            .iter()
            .filter(|entry| entry["event"].as_str() == Some("ActivityAttemptFailed"))
            .count();
        let retry_events = journal
            .iter()
            .filter(|entry| entry["event"].as_str() == Some("ActivityRetryScheduled"))
            .count();
        assert_eq!(failure_events, 2);
        assert_eq!(retry_events, 2);
    }

    #[tokio::test]
    async fn retry_helper_events_conform_to_workflow_journal_v1_schema() {
        let mut journal = Vec::new();
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_for_closure = attempts.clone();

        let _ = execute_with_retry_logic(
            3,
            1,
            &mut journal,
            |attempt, err, journal| {
                journal.push(versioned_event(json!({
                    "event": "ActivityAttemptFailed",
                    "workflow": "retry_schema_demo",
                    "activity": "mesh_join",
                    "activity_id": "retry_schema_demo-0",
                    "attempt": attempt,
                    "max_attempts": 3,
                    "error": err.to_string(),
                })));
            },
            |attempt, delay_ms, journal| {
                journal.push(versioned_event(json!({
                    "event": "ActivityRetryScheduled",
                    "workflow": "retry_schema_demo",
                    "activity": "mesh_join",
                    "activity_id": "retry_schema_demo-0",
                    "attempt": attempt,
                    "next_attempt": attempt + 1,
                    "delay_ms": delay_ms,
                })));
            },
            move || {
                let attempts = attempts_for_closure.clone();
                async move {
                    let current = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    if current < 3 {
                        anyhow::bail!("attempt {current} failed");
                    }
                    Ok::<_, anyhow::Error>(json!({"event": "MeshActivity"}))
                }
            },
        )
        .await
        .expect("retry helper should eventually succeed");

        let schema_json: Value = serde_json::from_str(include_str!(
            "../../../../contracts/workflow/workflow-journal.v1.schema.json"
        ))
        .expect("parse workflow journal schema");
        let validator = validator_for(&schema_json).expect("compile workflow journal schema");
        for entry in &journal {
            if let Err(err) = validator.validate(entry) {
                panic!("retry event should validate against v1 schema: {err}; entry={entry}");
            }
        }
        assert!(
            journal
                .iter()
                .any(|entry| entry["event"].as_str() == Some("ActivityAttemptFailed")),
            "retry helper should emit ActivityAttemptFailed events"
        );
        assert!(
            journal
                .iter()
                .any(|entry| entry["event"].as_str() == Some("ActivityRetryScheduled")),
            "retry helper should emit ActivityRetryScheduled events"
        );
    }

    #[test]
    fn versioned_event_adds_contract_version() {
        let value = versioned_event(json!({
            "event": "WorkflowStarted",
            "workflow": "wf",
        }));
        assert_eq!(
            value["journal_version"].as_u64(),
            Some(WORKFLOW_JOURNAL_VERSION as u64)
        );
    }

    #[test]
    fn derive_activity_id_is_deterministic() {
        let id1 = derive_activity_id("my_workflow", "send_email", 0);
        let id2 = derive_activity_id("my_workflow", "send_email", 0);
        assert_eq!(id1, id2, "same inputs must produce same activity_id");
    }

    #[test]
    fn derive_activity_id_differs_by_position() {
        let id0 = derive_activity_id("wf", "step", 0);
        let id1 = derive_activity_id("wf", "step", 1);
        assert_ne!(id0, id1, "different positions must produce different ids");
    }

    #[test]
    fn derive_activity_id_differs_by_workflow_name() {
        let id_a = derive_activity_id("workflow_a", "step", 0);
        let id_b = derive_activity_id("workflow_b", "step", 0);
        assert_ne!(
            id_a, id_b,
            "different workflow names must produce different ids"
        );
    }

    #[test]
    fn derive_activity_id_is_32_hex_chars() {
        let id = derive_activity_id("wf", "act", 0);
        assert_eq!(id.len(), 32, "activity_id must be 32 hex chars");
        assert!(
            id.chars().all(|c| c.is_ascii_hexdigit()),
            "must be lowercase hex"
        );
    }

    #[test]
    fn activity_is_mesh_matches_mesh_prefix() {
        assert!(activity_is_mesh("mesh_join"));
        assert!(activity_is_mesh("mesh_remote_charge"));
        assert!(!activity_is_mesh("charge_card"));
        assert!(!activity_is_mesh(""));
    }

    #[test]
    fn refuse_canned_local_activity_rejects_pre_phase1_rows() {
        let canned = json!({
            "event": "LocalActivity",
            "activity": "charge_card",
            "status": "executed",
        });
        assert!(refuse_canned_local_activity(&canned).is_err());
    }

    #[test]
    fn refuse_canned_local_activity_accepts_real_return_values() {
        let real = json!({"__vox": "Ok", "value": "tx_5"});
        assert!(refuse_canned_local_activity(&real).is_ok());
    }

    #[test]
    fn explicit_activity_id_reads_the_with_option() {
        let opts = json!({"activity_id": "explicit-1"});
        assert_eq!(
            explicit_activity_id(Some(&opts)),
            Some("explicit-1".to_string())
        );
        assert_eq!(explicit_activity_id(None), None);
        assert_eq!(explicit_activity_id(Some(&json!({}))), None);
    }
}
