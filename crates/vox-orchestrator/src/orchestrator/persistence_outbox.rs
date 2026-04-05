//! Persistence degradation outbox: periodic drain/prune and health counters.
//!
//! Entries are stored at [`PERSISTENCE_OUTBOX_KEY`] as a JSON array of objects
//! with `lane`, `error`, `first_seen_unix_ms`, and `retry_count` (see enqueue path in
//! `task_dispatch/complete.rs`).

use crate::context::ContextStore;
use crate::types::AgentId;
use std::collections::BTreeMap;

/// Context key for the outbox queue (must match persistence callsites).
pub(crate) const PERSISTENCE_OUTBOX_KEY: &str = "orchestrator/persistence_outbox";

/// Health / observability for the last lifecycle pass.
pub(crate) const PERSISTENCE_OUTBOX_LIFECYCLE_KEY: &str =
    "orchestrator/persistence_outbox_lifecycle";
const MAX_REPLAY_PER_TICK: usize = 20;

#[derive(Debug, Clone)]
pub(crate) struct PersistenceOutboxLifecycleParams {
    /// After this many ms since `first_seen`, an item is considered stale and gains a retry per tick.
    pub stale_after_ms: u64,
    /// Drop items whose first_seen is older than this many ms.
    pub max_age_ms: u64,
    /// Drop items whose `retry_count` is at or above this threshold (after any increment this tick).
    pub max_retries: u64,
}

impl Default for PersistenceOutboxLifecycleParams {
    fn default() -> Self {
        Self {
            stale_after_ms: 300_000, // 5 minutes
            max_age_ms: 604_800_000, // 7 days
            max_retries: 128,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct OutboxLifecycleStats {
    pub queued: usize,
    pub pruned_last_run: u64,
    pub retried_last_run: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ReplayStats {
    replayed_last_run: u64,
    replay_failed_last_run: u64,
    replay_failed_by_op: BTreeMap<String, u64>,
}

/// One maintenance pass: load outbox, prune by age/attempts, bump retries on stale rows, persist.
pub(crate) fn run_persistence_outbox_lifecycle_pass(
    store: &ContextStore,
    now_ms: u64,
    params: &PersistenceOutboxLifecycleParams,
) -> OutboxLifecycleStats {
    let key = PERSISTENCE_OUTBOX_KEY.to_string();
    let mut queue = store
        .get(&key)
        .and_then(|raw| serde_json::from_str::<Vec<serde_json::Value>>(&raw).ok())
        .unwrap_or_default();

    let mut pruned_last_run: u64 = 0;
    let mut retried_last_run: u64 = 0;
    let mut kept: Vec<serde_json::Value> = Vec::with_capacity(queue.len());

    for item in queue.drain(..) {
        let Some(obj) = item.as_object().cloned() else {
            pruned_last_run = pruned_last_run.saturating_add(1);
            continue;
        };

        let lane = obj
            .get("lane")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let error = obj
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let first_seen = first_seen_unix_ms(&obj).unwrap_or(now_ms);
        let mut retry_count = obj
            .get("retry_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);

        let age_ms = now_ms.saturating_sub(first_seen);

        if age_ms >= params.max_age_ms {
            pruned_last_run = pruned_last_run.saturating_add(1);
            continue;
        }
        if retry_count >= params.max_retries {
            pruned_last_run = pruned_last_run.saturating_add(1);
            continue;
        }

        if age_ms >= params.stale_after_ms {
            retry_count = retry_count.saturating_add(1);
            retried_last_run = retried_last_run.saturating_add(1);
            if retry_count >= params.max_retries {
                pruned_last_run = pruned_last_run.saturating_add(1);
                continue;
            }
        }

        let mut normalized = obj;
        normalized.insert("lane".to_string(), serde_json::Value::String(lane));
        normalized.insert("error".to_string(), serde_json::Value::String(error));
        normalized.insert(
            "first_seen_unix_ms".to_string(),
            serde_json::Value::from(first_seen),
        );
        normalized.insert(
            "retry_count".to_string(),
            serde_json::Value::from(retry_count),
        );
        normalized.insert("schema_version".to_string(), serde_json::Value::from(1));
        kept.push(serde_json::Value::Object(normalized));
    }

    let stats = OutboxLifecycleStats {
        queued: kept.len(),
        pruned_last_run,
        retried_last_run,
    };

    if let Ok(raw) = serde_json::to_string(&kept) {
        store.set(AgentId(0), key, raw, 0);
    }

    let health = serde_json::json!({
        "queued": stats.queued,
        "pruned_last_run": stats.pruned_last_run,
        "retried_last_run": stats.retried_last_run,
        "last_run_unix_ms": now_ms,
    });
    if let Ok(raw) = serde_json::to_string(&health) {
        store.set(
            AgentId(0),
            PERSISTENCE_OUTBOX_LIFECYCLE_KEY.to_string(),
            raw,
            0,
        );
    }

    stats
}

/// Acknowledge one recovered lane by removing the oldest matching entry.
pub(crate) fn ack_persistence_outbox_lane(store: &ContextStore, lane: &str) -> bool {
    let lane = lane.trim();
    if lane.is_empty() {
        return false;
    }
    let key = PERSISTENCE_OUTBOX_KEY.to_string();
    let mut queue = store
        .get(&key)
        .and_then(|raw| serde_json::from_str::<Vec<serde_json::Value>>(&raw).ok())
        .unwrap_or_default();

    let idx = queue.iter().enumerate().find_map(|(i, item)| {
        item.as_object().and_then(|o| {
            o.get("lane")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| *s == lane)
                .map(|_| i)
        })
    });
    let Some(i) = idx else {
        return false;
    };
    queue.remove(i);
    if let Ok(raw) = serde_json::to_string(&queue) {
        store.set(AgentId(0), key, raw, 0);
    }
    true
}

fn first_seen_unix_ms(obj: &serde_json::Map<String, serde_json::Value>) -> Option<u64> {
    if let Some(v) = obj.get("first_seen_unix_ms") {
        if let Some(n) = v.as_u64() {
            return Some(n);
        }
    }
    obj.get("first_seen").and_then(|v| v.as_u64()).or_else(|| {
        obj.get("first_seen")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
    })
}

impl crate::orchestrator::Orchestrator {
    pub(crate) async fn tick_persistence_outbox_lifecycle(&self) {
        let now_ms = crate::types::now_unix_ms();
        let params = PersistenceOutboxLifecycleParams::default();
        let mut stats = {
            let store = crate::sync_lock::rw_read(&*self.context_store);
            run_persistence_outbox_lifecycle_pass(&store, now_ms, &params)
        };
        let replay_stats = self.try_replay_persistence_outbox().await;
        let store = crate::sync_lock::rw_read(&*self.context_store);
        stats.queued = store
            .get(PERSISTENCE_OUTBOX_KEY)
            .and_then(|raw| serde_json::from_str::<Vec<serde_json::Value>>(&raw).ok())
            .map(|v| v.len())
            .unwrap_or(0);

        let health = serde_json::json!({
            "queued": stats.queued,
            "pruned_last_run": stats.pruned_last_run,
            "retried_last_run": stats.retried_last_run,
            "replayed_last_run": replay_stats.replayed_last_run,
            "replay_failed_last_run": replay_stats.replay_failed_last_run,
            "replay_failed_by_op": replay_stats.replay_failed_by_op,
            "last_run_unix_ms": now_ms,
        });
        if let Ok(raw) = serde_json::to_string(&health) {
            store.set(
                AgentId(0),
                PERSISTENCE_OUTBOX_LIFECYCLE_KEY.to_string(),
                raw,
                0,
            );
        }

        if stats.pruned_last_run > 0
            || stats.retried_last_run > 0
            || replay_stats.replayed_last_run > 0
            || replay_stats.replay_failed_last_run > 0
        {
            tracing::debug!(
                queued = stats.queued,
                pruned = stats.pruned_last_run,
                retried = stats.retried_last_run,
                replayed = replay_stats.replayed_last_run,
                replay_failed = replay_stats.replay_failed_last_run,
                "persistence outbox lifecycle tick"
            );
        }
    }

    async fn try_replay_persistence_outbox(&self) -> ReplayStats {
        let Some(db) = self.db() else {
            return ReplayStats::default();
        };
        let key = PERSISTENCE_OUTBOX_KEY.to_string();
        let raw = {
            let store = crate::sync_lock::rw_read(&*self.context_store);
            match store.get(&key) {
                Some(raw) => raw,
                None => return ReplayStats::default(),
            }
        };
        let mut queue = serde_json::from_str::<Vec<serde_json::Value>>(&raw).unwrap_or_default();
        if queue.is_empty() {
            return ReplayStats::default();
        }

        let mut replayed = 0u64;
        let mut replay_failed = 0u64;
        let mut replay_failed_by_op: BTreeMap<String, u64> = BTreeMap::new();
        let mut kept = Vec::with_capacity(queue.len());
        for entry in queue.drain(..) {
            if replayed as usize >= MAX_REPLAY_PER_TICK {
                kept.push(entry);
                continue;
            }
            if replay_one_entry(&db, &entry).await {
                replayed = replayed.saturating_add(1);
            } else {
                replay_failed = replay_failed.saturating_add(1);
                let op = replay_op_label(&entry);
                *replay_failed_by_op.entry(op).or_insert(0) += 1;
                kept.push(entry);
            }
        }
        let store = crate::sync_lock::rw_read(&*self.context_store);
        if let Ok(raw) = serde_json::to_string(&kept) {
            store.set(AgentId(0), key, raw, 0);
        }
        ReplayStats {
            replayed_last_run: replayed,
            replay_failed_last_run: replay_failed,
            replay_failed_by_op,
        }
    }
}

fn replay_op_label(entry: &serde_json::Value) -> String {
    let op = entry
        .get("replay")
        .and_then(serde_json::Value::as_object)
        .and_then(|replay| {
            replay
                .get("op")
                .and_then(serde_json::Value::as_str)
                .or_else(|| {
                    if replay.get("repository_id").is_some() && replay.get("kind").is_some() {
                        Some("append_orchestration_lineage_event")
                    } else {
                        None
                    }
                })
        });
    op.unwrap_or("unknown").to_string()
}

async fn replay_one_entry(db: &std::sync::Arc<vox_db::VoxDb>, entry: &serde_json::Value) -> bool {
    let Some(obj) = entry.as_object() else {
        return false;
    };
    let Some(replay) = obj.get("replay").and_then(serde_json::Value::as_object) else {
        return false;
    };
    let op = replay
        .get("op")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            // Backward-compat for lineage entries created before explicit replay op tagging.
            if replay.get("repository_id").is_some() && replay.get("kind").is_some() {
                Some("append_orchestration_lineage_event")
            } else {
                None
            }
        });

    match op {
        Some("append_orchestration_lineage_event") => {
            let Some(repository_id) = replay
                .get("repository_id")
                .and_then(serde_json::Value::as_str)
            else {
                return false;
            };
            let Some(kind) = replay.get("kind").and_then(serde_json::Value::as_str) else {
                return false;
            };
            let Some(task_id) = replay.get("task_id").and_then(serde_json::Value::as_i64) else {
                return false;
            };
            let agent_id = replay.get("agent_id").and_then(serde_json::Value::as_i64);
            let session_id = replay.get("session_id").and_then(serde_json::Value::as_str);
            let workflow_id = replay
                .get("workflow_id")
                .and_then(serde_json::Value::as_str);
            let plan_session_id = replay
                .get("plan_session_id")
                .and_then(serde_json::Value::as_str);
            let plan_node_id = replay
                .get("plan_node_id")
                .and_then(serde_json::Value::as_str);
            let payload_json = replay
                .get("payload_json")
                .and_then(serde_json::Value::as_str);

            db.append_orchestration_lineage_event(
                repository_id,
                kind,
                task_id,
                agent_id,
                session_id,
                workflow_id,
                plan_session_id,
                plan_node_id,
                payload_json,
            )
            .await
            .is_ok()
        }
        Some("record_task_reliability_observation") => {
            let Some(agent_id) = replay.get("agent_id").and_then(serde_json::Value::as_str) else {
                return false;
            };
            let Some(success) = replay.get("success").and_then(serde_json::Value::as_bool) else {
                return false;
            };
            db.record_task_reliability_observation(agent_id, success)
                .await
                .is_ok()
        }
        Some("record_trust_observation") => {
            let Some(entity_type) = replay
                .get("entity_type")
                .and_then(serde_json::Value::as_str)
            else {
                return false;
            };
            let Some(entity_id) = replay.get("entity_id").and_then(serde_json::Value::as_str)
            else {
                return false;
            };
            let Some(dimension) = replay.get("dimension").and_then(serde_json::Value::as_str)
            else {
                return false;
            };
            let observation = vox_db::TrustObservationInput {
                entity_type,
                entity_id,
                dimension,
                domain: replay.get("domain").and_then(serde_json::Value::as_str),
                task_class: replay.get("task_class").and_then(serde_json::Value::as_str),
                provider: replay.get("provider").and_then(serde_json::Value::as_str),
                model_id: replay.get("model_id").and_then(serde_json::Value::as_str),
                repository_id: replay
                    .get("repository_id")
                    .and_then(serde_json::Value::as_str),
                source_kind: replay
                    .get("source_kind")
                    .and_then(serde_json::Value::as_str),
                observation_value: replay
                    .get("observation_value")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.5),
                confidence_weight: replay
                    .get("confidence_weight")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(1.0),
                sample_size: replay
                    .get("sample_size")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(1) as i64,
                artifact_ref: replay
                    .get("artifact_ref")
                    .and_then(serde_json::Value::as_str),
                metadata_json: replay
                    .get("metadata_json")
                    .and_then(serde_json::Value::as_str),
                ewma_alpha: replay
                    .get("ewma_alpha")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.10),
            };
            db.record_trust_observation(observation).await.is_ok()
        }
        Some("record_plan_node_attempt") => {
            let Some(plan_session_id) = replay
                .get("plan_session_id")
                .and_then(serde_json::Value::as_str)
            else {
                return false;
            };
            let Some(version) = replay.get("version").and_then(serde_json::Value::as_i64) else {
                return false;
            };
            let Some(node_id) = replay.get("node_id").and_then(serde_json::Value::as_str) else {
                return false;
            };
            let attempt_no = replay
                .get("attempt_no")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(1);
            let Some(outcome) = replay.get("outcome").and_then(serde_json::Value::as_str) else {
                return false;
            };
            let task_id = replay.get("task_id").and_then(serde_json::Value::as_str);
            let error_text = replay.get("error_text").and_then(serde_json::Value::as_str);
            let latency_ms = replay.get("latency_ms").and_then(serde_json::Value::as_i64);
            db.record_plan_node_attempt(
                plan_session_id,
                version,
                node_id,
                attempt_no,
                task_id,
                outcome,
                error_text,
                latency_ms,
            )
            .await
            .is_ok()
        }
        Some("set_plan_node_status") => {
            let Some(plan_session_id) = replay
                .get("plan_session_id")
                .and_then(serde_json::Value::as_str)
            else {
                return false;
            };
            let Some(version) = replay.get("version").and_then(serde_json::Value::as_i64) else {
                return false;
            };
            let Some(node_id) = replay.get("node_id").and_then(serde_json::Value::as_str) else {
                return false;
            };
            let Some(status) = replay.get("status").and_then(serde_json::Value::as_str) else {
                return false;
            };
            db.set_plan_node_status(plan_session_id, version, node_id, status)
                .await
                .is_ok()
        }
        Some("append_plan_version") => {
            let Some(plan_session_id) = replay
                .get("plan_session_id")
                .and_then(serde_json::Value::as_str)
            else {
                return false;
            };
            let Some(version) = replay.get("version").and_then(serde_json::Value::as_i64) else {
                return false;
            };
            let parent_version = replay
                .get("parent_version")
                .and_then(serde_json::Value::as_i64);
            let origin = replay.get("origin").and_then(serde_json::Value::as_str);
            let metadata_json = replay
                .get("metadata_json")
                .and_then(serde_json::Value::as_str);
            db.append_plan_version(
                plan_session_id,
                version,
                parent_version,
                origin,
                metadata_json,
            )
            .await
            .is_ok()
        }
        Some("insert_victory_verdict") => {
            let Some(task_id) = replay.get("task_id").and_then(serde_json::Value::as_str) else {
                return false;
            };
            let Some(verdict_val) = replay.get("verdict") else {
                return false;
            };
            let Ok(verdict) = serde_json::from_value::<crate::VictoryVerdict>(verdict_val.clone())
            else {
                return false;
            };
            db.insert_victory_verdict(task_id, &verdict).await.is_ok()
        }
        Some("insert_observer_event") => {
            let Some(session_id) = replay.get("session_id").and_then(serde_json::Value::as_str)
            else {
                return false;
            };
            let Some(task_id) = replay.get("task_id").and_then(serde_json::Value::as_str) else {
                return false;
            };
            let Some(report_val) = replay.get("report") else {
                return false;
            };
            let Ok(report) = serde_json::from_value::<crate::ObservationReport>(report_val.clone())
            else {
                return false;
            };
            db.insert_observer_event(session_id, task_id, &report)
                .await
                .is_ok()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn seed_outbox(store: &ContextStore, items: Vec<serde_json::Value>) {
        let raw = serde_json::to_string(&items).unwrap();
        store.set(AgentId(0), PERSISTENCE_OUTBOX_KEY.to_string(), raw, 0);
    }

    fn load_outbox(store: &ContextStore) -> Vec<serde_json::Value> {
        store
            .get(PERSISTENCE_OUTBOX_KEY)
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn load_health(store: &ContextStore) -> serde_json::Value {
        serde_json::from_str(
            &store
                .get(PERSISTENCE_OUTBOX_LIFECYCLE_KEY)
                .expect("lifecycle key set"),
        )
        .unwrap()
    }

    async fn mem_db() -> Arc<vox_db::VoxDb> {
        Arc::new(
            vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
                .await
                .expect("memory db"),
        )
    }

    #[test]
    fn ack_persistence_outbox_lane_removes_one_oldest_match() {
        let store = ContextStore::new();
        seed_outbox(
            &store,
            vec![
                serde_json::json!({"lane":"lineage/task_failed","error":"a","first_seen_unix_ms":1u64,"retry_count":0u64}),
                serde_json::json!({"lane":"trust/observation","error":"b","first_seen_unix_ms":2u64,"retry_count":0u64}),
                serde_json::json!({"lane":"lineage/task_failed","error":"c","first_seen_unix_ms":3u64,"retry_count":0u64}),
            ],
        );
        assert!(ack_persistence_outbox_lane(&store, "lineage/task_failed"));
        let q = load_outbox(&store);
        assert_eq!(q.len(), 2);
        assert_eq!(q[0]["lane"], "trust/observation");
        assert_eq!(q[1]["lane"], "lineage/task_failed");
    }

    #[test]
    fn fresh_entry_not_retried_until_stale() {
        let store = ContextStore::new();
        let now = 10_000u64;
        seed_outbox(
            &store,
            vec![serde_json::json!({
                "lane": "test",
                "error": "e",
                "first_seen_unix_ms": now,
                "retry_count": 0u64,
            })],
        );
        let params = PersistenceOutboxLifecycleParams {
            stale_after_ms: 5_000,
            max_age_ms: 1_000_000,
            max_retries: 10,
        };
        let s = run_persistence_outbox_lifecycle_pass(&store, now, &params);
        assert_eq!(s.queued, 1);
        assert_eq!(s.retried_last_run, 0);
        assert_eq!(s.pruned_last_run, 0);
        let q = load_outbox(&store);
        assert_eq!(q[0]["retry_count"], 0);
    }

    #[test]
    fn stale_entry_increments_retry_each_tick() {
        let store = ContextStore::new();
        let first = 1_000u64;
        seed_outbox(
            &store,
            vec![serde_json::json!({
                "lane": "lane-a",
                "error": "boom",
                "first_seen_unix_ms": first,
                "retry_count": 0u64,
            })],
        );
        let params = PersistenceOutboxLifecycleParams {
            stale_after_ms: 1_000,
            max_age_ms: 1_000_000,
            max_retries: 10,
        };
        let s = run_persistence_outbox_lifecycle_pass(&store, 5_000, &params);
        assert_eq!(s.retried_last_run, 1);
        assert_eq!(load_outbox(&store)[0]["retry_count"], 1);

        let s2 = run_persistence_outbox_lifecycle_pass(&store, 6_000, &params);
        assert_eq!(s2.retried_last_run, 1);
        assert_eq!(load_outbox(&store)[0]["retry_count"], 2);
    }

    #[test]
    fn prunes_when_max_retries_reached_after_increment() {
        let store = ContextStore::new();
        seed_outbox(
            &store,
            vec![serde_json::json!({
                "lane": "x",
                "error": "e",
                "first_seen_unix_ms": 0u64,
                "retry_count": 2u64,
            })],
        );
        let params = PersistenceOutboxLifecycleParams {
            stale_after_ms: 1,
            max_age_ms: 1_000_000,
            max_retries: 3,
        };
        let s = run_persistence_outbox_lifecycle_pass(&store, 10_000, &params);
        assert_eq!(s.pruned_last_run, 1);
        assert_eq!(s.queued, 0);
        assert!(load_outbox(&store).is_empty());
    }

    #[test]
    fn prunes_by_max_age() {
        let store = ContextStore::new();
        seed_outbox(
            &store,
            vec![serde_json::json!({
                "lane": "x",
                "error": "e",
                "first_seen_unix_ms": 100u64,
                "retry_count": 0u64,
            })],
        );
        let params = PersistenceOutboxLifecycleParams {
            stale_after_ms: 1_000_000,
            max_age_ms: 1_000,
            max_retries: 100,
        };
        let s = run_persistence_outbox_lifecycle_pass(&store, 2_000, &params);
        assert_eq!(s.pruned_last_run, 1);
        assert_eq!(s.queued, 0);
    }

    #[test]
    fn backward_compat_first_seen_alias() {
        let store = ContextStore::new();
        seed_outbox(
            &store,
            vec![serde_json::json!({
                "lane": "legacy",
                "error": "e",
                "first_seen": 500u64,
                "retry_count": 0u64,
            })],
        );
        let params = PersistenceOutboxLifecycleParams {
            stale_after_ms: 100,
            max_age_ms: 10_000,
            max_retries: 5,
        };
        run_persistence_outbox_lifecycle_pass(&store, 1_000, &params);
        let q = load_outbox(&store);
        assert_eq!(q[0]["first_seen_unix_ms"], 500);
        assert_eq!(q[0]["retry_count"], 1);
    }

    #[test]
    fn health_payload_contains_counters() {
        let store = ContextStore::new();
        seed_outbox(
            &store,
            vec![serde_json::json!({
                "lane": "h",
                "error": "e",
                "first_seen_unix_ms": 0u64,
                "retry_count": 0u64,
            })],
        );
        let params = PersistenceOutboxLifecycleParams {
            stale_after_ms: 1,
            max_age_ms: 9_999_999,
            max_retries: 100,
        };
        run_persistence_outbox_lifecycle_pass(&store, 5_000, &params);
        let h = load_health(&store);
        assert_eq!(h["queued"], 1);
        assert_eq!(h["retried_last_run"], 1);
        assert_eq!(h["last_run_unix_ms"], 5_000);
    }

    #[test]
    fn non_object_entries_dropped_as_pruned() {
        let store = ContextStore::new();
        seed_outbox(
            &store,
            vec![serde_json::json!("not-an-object"), serde_json::json!({})],
        );
        let params = PersistenceOutboxLifecycleParams::default();
        let s = run_persistence_outbox_lifecycle_pass(&store, 0, &params);
        assert_eq!(s.pruned_last_run, 1);
        assert_eq!(s.queued, 1);
    }

    #[tokio::test]
    async fn replay_lineage_entry_succeeds() {
        let db = mem_db().await;
        let entry = serde_json::json!({
            "lane": "lineage/task_completed",
            "error": "x",
            "first_seen_unix_ms": 1u64,
            "retry_count": 0u64,
            "replay": {
                "op": "append_orchestration_lineage_event",
                "repository_id": "repo-test",
                "kind": "task_completed",
                "task_id": 42i64,
                "agent_id": 7i64,
                "session_id": "s-1",
                "workflow_id": serde_json::Value::Null,
                "plan_session_id": serde_json::Value::Null,
                "plan_node_id": serde_json::Value::Null,
                "payload_json": "{\"ok\":true}"
            }
        });
        assert!(replay_one_entry(&db, &entry).await);
    }

    #[tokio::test]
    async fn replay_trust_and_reliability_entries_persist_rows() {
        let db = mem_db().await;
        let reliability_entry = serde_json::json!({
            "lane": "trust/reliability_observation",
            "error": "x",
            "first_seen_unix_ms": 1u64,
            "retry_count": 0u64,
            "replay": {
                "op": "record_task_reliability_observation",
                "agent_id": "9",
                "success": true
            }
        });
        assert!(replay_one_entry(&db, &reliability_entry).await);
        let reliability_rows = db
            .list_agent_reliability()
            .await
            .expect("list_agent_reliability");
        assert!(
            reliability_rows.iter().any(|(agent_id, _)| agent_id == "9"),
            "expected reliability row for agent 9"
        );

        let trust_entry = serde_json::json!({
            "lane": "trust/observation",
            "error": "x",
            "first_seen_unix_ms": 1u64,
            "retry_count": 0u64,
            "replay": {
                "op": "record_trust_observation",
                "entity_type": "agent",
                "entity_id": "9",
                "dimension": "task_completion",
                "domain": "verify",
                "task_class": "verify",
                "provider": serde_json::Value::Null,
                "model_id": serde_json::Value::Null,
                "repository_id": "repo-test",
                "source_kind": "task_completed",
                "observation_value": 1.0,
                "confidence_weight": 1.0,
                "sample_size": 1,
                "artifact_ref": "task-42",
                "metadata_json": serde_json::Value::Null,
                "ewma_alpha": 0.10
            }
        });
        assert!(replay_one_entry(&db, &trust_entry).await);
        let trust_rows = db
            .list_trust_observations(
                Some("agent"),
                Some("task_completion"),
                Some("verify"),
                Some("repo-test"),
                Some("task-42"),
                None,
                10,
            )
            .await
            .expect("list_trust_observations");
        assert!(
            !trust_rows.is_empty(),
            "expected trust observation row inserted by replay"
        );
    }

    #[tokio::test]
    async fn replay_plan_entries_update_state() {
        let db = mem_db().await;
        db.create_plan_session("ps-1", None, "goal", "strategy")
            .await
            .expect("create_plan_session");
        db.upsert_plan_node("ps-1", 1, "n1", "desc", "[]", "{}", "pending", None)
            .await
            .expect("upsert_plan_node");

        let attempt_entry = serde_json::json!({
            "lane": "plan/node_attempt",
            "error": "x",
            "first_seen_unix_ms": 1u64,
            "retry_count": 0u64,
            "replay": {
                "op": "record_plan_node_attempt",
                "plan_session_id": "ps-1",
                "version": 1i64,
                "node_id": "n1",
                "attempt_no": 1i64,
                "task_id": "123",
                "outcome": "completed",
                "error_text": serde_json::Value::Null,
                "latency_ms": serde_json::Value::Null
            }
        });
        assert!(replay_one_entry(&db, &attempt_entry).await);

        let status_entry = serde_json::json!({
            "lane": "plan/node_status",
            "error": "x",
            "first_seen_unix_ms": 1u64,
            "retry_count": 0u64,
            "replay": {
                "op": "set_plan_node_status",
                "plan_session_id": "ps-1",
                "version": 1i64,
                "node_id": "n1",
                "status": "completed"
            }
        });
        assert!(replay_one_entry(&db, &status_entry).await);
        let nodes = db
            .load_plan_nodes_with_status("ps-1", 1)
            .await
            .expect("load_plan_nodes_with_status");
        let status = nodes
            .iter()
            .find(|n| n.node_id == "n1")
            .map(|n| n.status.clone())
            .unwrap_or_default();
        assert_eq!(status, "completed");

        let version_entry = serde_json::json!({
            "lane": "plan/replan_version",
            "error": "x",
            "first_seen_unix_ms": 1u64,
            "retry_count": 0u64,
            "replay": {
                "op": "append_plan_version",
                "plan_session_id": "ps-1",
                "version": 2i64,
                "parent_version": 1i64,
                "origin": "task_failed",
                "metadata_json": "{\"task_id\":1}"
            }
        });
        assert!(replay_one_entry(&db, &version_entry).await);
        let head = db.load_plan_head("ps-1").await.expect("load_plan_head");
        assert_eq!(head, Some(2));
    }

    #[tokio::test]
    async fn replay_unknown_or_invalid_entry_returns_false() {
        let db = mem_db().await;
        let unknown = serde_json::json!({
            "replay": { "op": "unknown_op" }
        });
        assert!(!replay_one_entry(&db, &unknown).await);
        assert_eq!(replay_op_label(&unknown), "unknown_op");

        let missing_required = serde_json::json!({
            "replay": { "op": "set_plan_node_status", "plan_session_id": "ps-1" }
        });
        assert!(!replay_one_entry(&db, &missing_required).await);
        assert_eq!(replay_op_label(&missing_required), "set_plan_node_status");

        let missing_replay = serde_json::json!({ "lane": "x" });
        assert_eq!(replay_op_label(&missing_replay), "unknown");
    }
}
