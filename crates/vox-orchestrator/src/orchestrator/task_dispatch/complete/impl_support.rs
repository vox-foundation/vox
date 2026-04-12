use crate::orchestrator::Orchestrator;
use crate::orchestrator::persistence_outbox::PERSISTENCE_OUTBOX_KEY;
use crate::types::{AgentId, TaskId};

impl Orchestrator {
    pub(crate) fn extract_phase_label(desc: &str) -> String {
        const PREFIX: &str = "[PHASE:";
        if let Some(start) = desc.find(PREFIX) {
            let suffix = &desc[start + PREFIX.len()..];
            if let Some(end) = suffix.find(']') {
                return suffix[..end].trim().to_ascii_lowercase();
            }
        }
        "single_shot".to_string()
    }

    pub(crate) fn classify_verification_failures(
        reason_lower: &str,
    ) -> Vec<crate::reconstruction::VerificationFailureKind> {
        let mut kinds = Vec::new();
        if reason_lower.contains("compile") || reason_lower.contains("cargo check") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Compile);
        }
        if reason_lower.contains("test") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Tests);
        }
        if reason_lower.contains("contract") || reason_lower.contains("schema") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Contract);
        }
        if reason_lower.contains("doc") || reason_lower.contains("ssot") {
            kinds.push(crate::reconstruction::VerificationFailureKind::DocsSsot);
        }
        if reason_lower.contains("regression") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Regression);
        }
        if reason_lower.contains("grounding") || reason_lower.contains("citation") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Grounding);
        }
        if reason_lower.contains("contradiction") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Contradiction);
        }
        if kinds.is_empty() {
            kinds.push(crate::reconstruction::VerificationFailureKind::Unknown);
        }
        kinds
    }

    pub(crate) fn failure_kind_tag(
        kind: crate::reconstruction::VerificationFailureKind,
    ) -> &'static str {
        match kind {
            crate::reconstruction::VerificationFailureKind::Compile => "compile",
            crate::reconstruction::VerificationFailureKind::Tests => "tests",
            crate::reconstruction::VerificationFailureKind::Contract => "contract",
            crate::reconstruction::VerificationFailureKind::DocsSsot => "docs_ssot",
            crate::reconstruction::VerificationFailureKind::Regression => "regression",
            crate::reconstruction::VerificationFailureKind::Grounding => "grounding",
            crate::reconstruction::VerificationFailureKind::Contradiction => "contradiction",
            crate::reconstruction::VerificationFailureKind::Unknown => "unknown",
        }
    }

    pub(crate) fn record_task_loop_metric(
        &self,
        task_id: TaskId,
        phase: &str,
        outcome: &str,
        debug_iterations: u8,
    ) {
        let key = format!("task_loop_metrics/{}", task_id.0);
        let event = serde_json::json!({
            "task_id": task_id.0,
            "phase": phase,
            "outcome": outcome,
            "debug_iterations": debug_iterations,
            "ts_unix_ms": crate::types::now_unix_ms()
        });
        if let Ok(raw) = serde_json::to_string(&event) {
            crate::sync_lock::rw_read(&*self.context_store).set(AgentId(0), key, raw, 0);
        }
    }

    pub(crate) fn record_persistence_degradation(&self, lane: &str, error: &str) {
        self.record_persistence_degradation_with_meta(lane, error, None);
    }

    pub(crate) fn record_persistence_degradation_with_meta(
        &self,
        lane: &str,
        error: &str,
        replay_meta: Option<serde_json::Value>,
    ) {
        let key = format!("orchestrator/persistence_health/{lane}");
        let store = crate::sync_lock::rw_read(&*self.context_store);
        let previous = store
            .get(&key)
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
        let count = previous
            .as_ref()
            .and_then(|v| v.get("degraded_count"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1);
        let payload = serde_json::json!({
            "lane": lane,
            "status": "degraded",
            "degraded_count": count,
            "last_error": error.chars().take(600).collect::<String>(),
            "last_error_unix_ms": crate::types::now_unix_ms(),
        });
        if let Ok(raw) = serde_json::to_string(&payload) {
            store.set(AgentId(0), key, raw, 0);
        }
        self.enqueue_persistence_outbox(lane, error, replay_meta);
        tracing::warn!(lane = lane, error = %error, "persistence lane degraded");
    }

    pub(crate) fn enqueue_persistence_outbox(
        &self,
        lane: &str,
        error: &str,
        replay_meta: Option<serde_json::Value>,
    ) {
        let key = PERSISTENCE_OUTBOX_KEY.to_string();
        let store = crate::sync_lock::rw_read(&*self.context_store);
        let mut queue = store
            .get(&key)
            .and_then(|raw| serde_json::from_str::<Vec<serde_json::Value>>(&raw).ok())
            .unwrap_or_default();
        let mut entry = serde_json::json!({
            "lane": lane,
            "error": error.chars().take(600).collect::<String>(),
            "first_seen_unix_ms": crate::types::now_unix_ms(),
            "retry_count": 0u64,
        });
        if let Some(meta) = replay_meta
            && let Some(obj) = entry.as_object_mut()
        {
            obj.insert("replay".to_string(), meta);
        }
        queue.push(entry);
        const MAX_OUTBOX_ITEMS: usize = 200;
        if queue.len() > MAX_OUTBOX_ITEMS {
            let drop_n = queue.len().saturating_sub(MAX_OUTBOX_ITEMS);
            queue.drain(0..drop_n);
        }
        if let Ok(raw) = serde_json::to_string(&queue) {
            store.set(AgentId(0), key, raw, 0);
        }
    }

    pub(crate) async fn persist_with_retry_meta<E, F, Fut>(
        &self,
        lane: &str,
        replay_meta: Option<serde_json::Value>,
        mut op: F,
    ) where
        E: std::fmt::Display,
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<(), E>>,
    {
        const MAX_ATTEMPTS: usize = 3;
        for attempt in 1..=MAX_ATTEMPTS {
            match op().await {
                Ok(()) => {
                    self.ack_persistence_lane_recovery(lane);
                    return;
                }
                Err(e) if attempt < MAX_ATTEMPTS => {
                    let delay_ms = 25u64 * attempt as u64;
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    tracing::debug!(lane = lane, attempt = attempt, error = %e, "persistence retry");
                }
                Err(e) => {
                    self.record_persistence_degradation_with_meta(
                        lane,
                        &e.to_string(),
                        replay_meta.clone(),
                    );
                }
            }
        }
    }

    pub(crate) fn ack_persistence_lane_recovery(&self, lane: &str) {
        let store = crate::sync_lock::rw_read(&*self.context_store);
        if crate::orchestrator::persistence_outbox::ack_persistence_outbox_lane(&store, lane) {
            tracing::debug!(
                lane = lane,
                "persistence lane recovered; acknowledged outbox item"
            );
        }
    }
}
