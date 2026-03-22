//! Correlation-scoped question routing between agents (`ask` / `answer`).
//!
//! [`QARouter`](crate::qa::QARouter) stores pending prompts keyed by [`CorrelationId`](crate::types::CorrelationId) so asynchronous
//! replies can find the original asker without a global mailbox.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::sync_lock;
use crate::types::{AgentId, CorrelationId, CorrelationIdGenerator};

/// Question waiting for a correlated answer from another agent.
pub struct PendingQuestion {
    /// Agent that asked the question.
    pub from: AgentId,
    /// Agent expected to answer (unicast).
    pub to: AgentId,
    /// Full question text.
    pub question: String,
    /// When the question was registered (for timeouts).
    pub asked_at: Instant,
}

/// In-memory ask queue with monotonic correlation ids.
#[derive(Clone)]
pub struct QARouter {
    pending: Arc<RwLock<HashMap<CorrelationId, PendingQuestion>>>,
    correlator: Arc<CorrelationIdGenerator>,
}

impl QARouter {
    /// Empty router; correlations start from a fresh generator.
    pub fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            correlator: Arc::new(CorrelationIdGenerator::new()),
        }
    }

    /// Registers a pending question and returns its id for later `answer`.
    pub fn ask(&self, from: AgentId, to: AgentId, question: impl Into<String>) -> CorrelationId {
        let corr_id = self.correlator.next();
        let q = PendingQuestion {
            from,
            to,
            question: question.into(),
            asked_at: Instant::now(),
        };
        sync_lock::rw_write(&self.pending).insert(corr_id, q);
        corr_id
    }

    /// Completes a round-trip; returns the original asker if the id was valid.
    pub fn answer(&self, corr_id: CorrelationId, _answer: &str) -> Option<AgentId> {
        let q = sync_lock::rw_write(&self.pending).remove(&corr_id)?;
        Some(q.from)
    }

    /// Lists open questions addressed to `to_agent` (for inbox UIs).
    pub fn pending_questions(&self, to_agent: AgentId) -> Vec<(CorrelationId, String)> {
        sync_lock::rw_read(&self.pending)
            .iter()
            .filter(|(_, q)| q.to == to_agent)
            .map(|(k, q)| (*k, q.question.clone()))
            .collect()
    }
}

impl Default for QARouter {
    fn default() -> Self {
        Self::new()
    }
}
