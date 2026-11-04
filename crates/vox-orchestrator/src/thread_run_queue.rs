//! Thread Run Queue, Mid-Turn `promote_to_steer`, `LimitRecoveryState` & `awaited: bool` Settlement (`T3-STREAM-GUEST-04`).
//!
//! Adapted from `pingdotgg/t3code`:
//! - `apps/server/src/orchestration-v2/ThreadRunQueue.ts` (FIFO queue while turn active + `promoteToSteer`)
//! - `apps/server/src/orchestration-v2/Decider.limitRecovery.ts` (`limit_recovery` auto-resumption at `retryAt`)
//! - `apps/server/src/orchestration-v2/Decider.backgroundWork.ts` (`awaited: bool` settlement gate)

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Delivery mode of a queued prompt in `ThreadRunQueue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueuedRunDeliveryMode {
    /// Wait in FIFO order until the current turn finishes.
    FollowUpTurn,
    /// Promoted mid-turn to inject `[MID-TURN STEER INTERVENTION]` into the active turn's next tool iteration.
    MidTurnSteer,
}

/// A prompt entry waiting in a thread's `ThreadRunQueue`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedRunEntry {
    /// Unique queue entry ID.
    pub entry_id: String,
    /// Thread ID.
    pub thread_id: String,
    /// Prompt text.
    pub prompt: String,
    /// Delivery mode (`FollowUpTurn` or `MidTurnSteer`).
    pub mode: QueuedRunDeliveryMode,
}

/// Provider rate-limit / quota pause state (`Decider.limitRecovery.ts` equivalent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitRecoveryState {
    /// Provider or model identifier that hit the rate/usage limit.
    pub provider_id: String,
    /// Human-readable reason from the provider error.
    pub message: String,
    /// Unix timestamp (ms) when the turn should automatically resume.
    pub retry_at_ms: u64,
}

/// Background child task registration (`awaited: bool` settlement gate).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundWorkItem {
    /// Unique background task ID.
    pub work_id: String,
    /// If `true`, the parent turn stays open until this item completes (`background-work.completed`).
    /// If `false` (fire-and-forget), the parent turn may complete immediately.
    pub awaited: bool,
    /// Whether the background work has completed.
    pub completed: bool,
}

/// Per-thread run queue, mid-turn steering buffer, limit-recovery state, and background-work gate.
#[derive(Debug, Default, Clone)]
pub struct ThreadRunQueue {
    queues: HashMap<String, VecDeque<QueuedRunEntry>>,
    limit_recovery: HashMap<String, LimitRecoveryState>,
    background_work: HashMap<String, Vec<BackgroundWorkItem>>,
}

impl ThreadRunQueue {
    /// Create a new empty `ThreadRunQueue`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue a follow-up prompt while a thread turn is active.
    pub fn enqueue_prompt(
        &mut self,
        thread_id: impl Into<String>,
        entry_id: impl Into<String>,
        prompt: impl Into<String>,
    ) -> QueuedRunEntry {
        let thread_id = thread_id.into();
        let entry = QueuedRunEntry {
            entry_id: entry_id.into(),
            thread_id: thread_id.clone(),
            prompt: prompt.into(),
            mode: QueuedRunDeliveryMode::FollowUpTurn,
        };
        self.queues
            .entry(thread_id)
            .or_default()
            .push_back(entry.clone());
        entry
    }

    /// Promote a queued follow-up entry to an immediate mid-turn steer (`thread.queue.promote-to-steer`).
    pub fn promote_to_steer(&mut self, thread_id: &str, entry_id: &str) -> Option<String> {
        let queue = self.queues.get_mut(thread_id)?;
        let pos = queue.iter().position(|e| e.entry_id == entry_id)?;
        let mut entry = queue.remove(pos)?;
        entry.mode = QueuedRunDeliveryMode::MidTurnSteer;
        Some(format!(
            "[MID-TURN STEER INTERVENTION]: {}",
            entry.prompt.trim()
        ))
    }

    /// Pop the next queued follow-up turn when the active turn completes.
    pub fn pop_next_follow_up(&mut self, thread_id: &str) -> Option<QueuedRunEntry> {
        self.queues.get_mut(thread_id)?.pop_front()
    }

    /// Record a provider rate-limit pause (`thread.limit-recovery.detected`).
    pub fn set_limit_recovery(&mut self, thread_id: impl Into<String>, state: LimitRecoveryState) {
        self.limit_recovery.insert(thread_id.into(), state);
    }

    /// Check whether `thread_id` has reached its `retry_at_ms` and can auto-resume (`now_ms >= retry_at_ms`).
    pub fn take_ready_limit_recovery(
        &mut self,
        thread_id: &str,
        now_ms: u64,
    ) -> Option<LimitRecoveryState> {
        let ready = self
            .limit_recovery
            .get(thread_id)
            .is_some_and(|s| now_ms >= s.retry_at_ms);
        if ready {
            self.limit_recovery.remove(thread_id)
        } else {
            None
        }
    }

    /// Register a background work item (`awaited: true` blocks turn completion; `awaited: false` does not).
    pub fn register_background_work(
        &mut self,
        thread_id: impl Into<String>,
        work_id: impl Into<String>,
        awaited: bool,
    ) {
        self.background_work
            .entry(thread_id.into())
            .or_default()
            .push(BackgroundWorkItem {
                work_id: work_id.into(),
                awaited,
                completed: false,
            });
    }

    /// Mark a background work item as completed (`thread.background-work.completed`).
    pub fn complete_background_work(&mut self, thread_id: &str, work_id: &str) -> bool {
        let Some(items) = self.background_work.get_mut(thread_id) else {
            return false;
        };
        for item in items {
            if item.work_id == work_id {
                item.completed = true;
                return true;
            }
        }
        false
    }

    /// Returns `true` if the parent turn can settle (i.e. zero incomplete `awaited: true` items remain).
    pub fn can_settle_turn(&self, thread_id: &str) -> bool {
        self.background_work
            .get(thread_id)
            .map(|items| items.iter().all(|item| !item.awaited || item.completed))
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::{LimitRecoveryState, ThreadRunQueue};

    #[test]
    fn thread_run_queue_supports_fifo_and_promote_to_steer() {
        let mut q = ThreadRunQueue::new();
        q.enqueue_prompt("t1", "e1", "Run cargo test next");
        q.enqueue_prompt("t1", "e2", "Actually use --lib only");

        let steer = q
            .promote_to_steer("t1", "e2")
            .expect("promote e2 to mid-turn steer");
        assert_eq!(
            steer,
            "[MID-TURN STEER INTERVENTION]: Actually use --lib only"
        );

        let next = q.pop_next_follow_up("t1").expect("e1 still in queue");
        assert_eq!(next.entry_id, "e1");
        assert!(q.pop_next_follow_up("t1").is_none());
    }

    #[test]
    fn limit_recovery_and_awaited_background_work_gate_settlement() {
        let mut q = ThreadRunQueue::new();
        q.set_limit_recovery(
            "t1",
            LimitRecoveryState {
                provider_id: "anthropic".to_string(),
                message: "Rate limit exceeded".to_string(),
                retry_at_ms: 5_000,
            },
        );
        assert!(q.take_ready_limit_recovery("t1", 4_999).is_none());
        assert!(q.take_ready_limit_recovery("t1", 5_000).is_some());

        // Fire-and-forget (`awaited: false`) does not block settlement
        q.register_background_work("t1", "dev-server", false);
        assert!(q.can_settle_turn("t1"));

        // Awaited subagent (`awaited: true`) blocks settlement until completed
        q.register_background_work("t1", "subagent-1", true);
        assert!(!q.can_settle_turn("t1"));
        assert!(q.complete_background_work("t1", "subagent-1"));
        assert!(q.can_settle_turn("t1"));
    }
}
