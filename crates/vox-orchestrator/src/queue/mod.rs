//! Priority-ordered per-agent task queues with dependency tracking.
//!
//! [`AgentQueue`](crate::queue::AgentQueue) is the unit the orchestrator uses for dequeue, pause, and completion accounting.

mod drain;
mod priority;

use std::collections::VecDeque;

use crate::contract::TaskCapabilityHints;
use crate::types::{AgentId, AgentTask, TaskId};

/// Per-agent priority task queue.
///
/// Tasks are stored in priority order (Urgent > Normal > Background).
/// Within the same priority level, tasks are FIFO.
#[derive(Debug)]
pub struct AgentQueue {
    /// The agent that owns this queue.
    pub id: AgentId,
    /// Human-readable name for this agent/queue.
    pub name: String,
    /// Ordered queue of pending tasks.
    pub(crate) tasks: VecDeque<AgentTask>,
    /// The task currently being executed (if any).
    pub(crate) in_progress: Option<AgentTask>,
    /// IDs of completed tasks (for dependency resolution).
    pub(crate) completed: Vec<TaskId>,
    /// Whether this queue is paused (no dequeue).
    pub paused: bool,
    /// Last time this agent was active (enqueued, dequeued, or completed a task).
    pub last_active: std::time::SystemTime,
    /// ID of the AI agent session mapped to this queue, if any.
    pub agent_session_id: Option<String>,
    /// Hardware capabilities this agent queue provides (for GPU-aware routing).
    pub capabilities: TaskCapabilityHints,
    /// Active skills bound to this agent queue with their EWMA reliability scores.
    pub active_skills: std::collections::HashMap<String, f64>,
    /// Workflow context ID/name if this agent is dedicated to a specific durable workflow.
    pub workflow_context: Option<String>,
    /// Optional identifier linking this agent to a specific provider endpoint, used for reliability metrics.
    pub endpoint_reliability_key: Option<String>,
    /// Count of recent shard-validation failures routed through this agent.
    pub recent_shard_validation_failures: u32,
    /// If set, reducer tasks are de-prioritized until this unix-ms timestamp.
    pub reducer_cooldown_until_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use crate::types::TaskPriority;

    use super::*;

    fn make_task(id: u64, priority: TaskPriority) -> AgentTask {
        AgentTask::new(TaskId(id), format!("task-{}", id), priority, vec![])
    }

    #[test]
    fn enqueue_respects_priority() {
        let mut q = AgentQueue::new(AgentId(1), "test");
        q.enqueue(make_task(1, TaskPriority::Normal));
        q.enqueue(make_task(2, TaskPriority::Urgent));
        q.enqueue(make_task(3, TaskPriority::Background));

        // Urgent should be first
        let first = q.dequeue().expect("should have task");
        assert_eq!(first.id, TaskId(2));
        let second = q.dequeue().expect("should have task");
        assert_eq!(second.id, TaskId(1));
    }

    #[test]
    fn fifo_within_same_priority() {
        let mut q = AgentQueue::new(AgentId(1), "test");
        q.enqueue(make_task(1, TaskPriority::Normal));
        q.enqueue(make_task(2, TaskPriority::Normal));
        q.enqueue(make_task(3, TaskPriority::Normal));

        assert_eq!(q.dequeue().unwrap().id, TaskId(1));
    }

    #[test]
    fn cancel_task() {
        let mut q = AgentQueue::new(AgentId(1), "test");
        q.enqueue(make_task(1, TaskPriority::Normal));
        q.enqueue(make_task(2, TaskPriority::Normal));

        let cancelled = q.cancel(TaskId(1));
        assert!(cancelled.is_some());
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn mark_complete_and_unblock() {
        let mut q = AgentQueue::new(AgentId(1), "test");

        // Task 2 depends on task 1
        let t1 = make_task(1, TaskPriority::Normal);
        let t2 = make_task(2, TaskPriority::Normal).depends_on(TaskId(1));

        q.enqueue(t1);
        q.enqueue(t2);

        // Dequeue task 1 (task 2 is blocked)
        let active = q.dequeue().expect("task 1 ready");
        assert_eq!(active.id, TaskId(1));

        // Task 2 should still be blocked
        assert!(q.is_blocked(TaskId(2)));

        // Complete task 1 — should unblock task 2
        q.mark_complete(TaskId(1));
        assert!(!q.is_blocked(TaskId(2)));

        // Now task 2 should be dequeue-able
        let next = q.dequeue().expect("task 2 should be unblocked");
        assert_eq!(next.id, TaskId(2));
    }

    #[test]
    fn paused_queue_blocks_dequeue() {
        let mut q = AgentQueue::new(AgentId(1), "test");
        q.enqueue(make_task(1, TaskPriority::Normal));
        q.pause();
        assert!(q.dequeue().is_none());
        q.resume();
        assert!(q.dequeue().is_some());
    }

    #[test]
    fn reorder_changes_position() {
        let mut q = AgentQueue::new(AgentId(1), "test");
        q.enqueue(make_task(1, TaskPriority::Background));
        q.enqueue(make_task(2, TaskPriority::Normal));

        // Promote task 1 to urgent
        q.reorder(TaskId(1), TaskPriority::Urgent);

        let first = q.dequeue().unwrap();
        assert_eq!(first.id, TaskId(1));
    }

    #[test]
    fn markdown_output() {
        let mut q = AgentQueue::new(AgentId(1), "parser");
        q.enqueue(make_task(1, TaskPriority::Normal));
        q.enqueue(make_task(2, TaskPriority::Urgent));

        let md = q.to_markdown();
        assert!(md.contains("Agent A-01"));
        assert!(md.contains("parser"));
        assert!(md.contains("task-1"));
        assert!(md.contains("task-2"));
    }

    #[test]
    fn drain_empties_queue() {
        let mut q = AgentQueue::new(AgentId(1), "test");
        q.enqueue(make_task(1, TaskPriority::Normal));
        q.enqueue(make_task(2, TaskPriority::Normal));

        let drained = q.drain_tasks();
        assert_eq!(drained.len(), 2);
        assert!(q.is_empty());
    }

    #[test]
    fn enqueue_dedup_prevents_duplicate() {
        let mut q = AgentQueue::new(AgentId(1), "test");
        let t1 = make_task(1, TaskPriority::Normal);
        let t2 = AgentTask::new(TaskId(2), "task-1", TaskPriority::Urgent, vec![]); // same desc as t1
        assert!(q.enqueue_dedup(t1));
        assert!(
            !q.enqueue_dedup(t2),
            "duplicate description should be rejected"
        );
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn enqueue_dedup_case_insensitive() {
        let mut q = AgentQueue::new(AgentId(1), "test");
        let t1 = AgentTask::new(TaskId(1), "Fix Parser", TaskPriority::Normal, vec![]);
        let t2 = AgentTask::new(TaskId(2), "fix parser", TaskPriority::Urgent, vec![]);
        assert!(q.enqueue_dedup(t1));
        assert!(
            !q.enqueue_dedup(t2),
            "case-insensitive duplicate should be rejected"
        );
    }

    #[test]
    fn retry_task_increments_count() {
        let mut q = AgentQueue::new(AgentId(1), "test");
        let task = make_task(1, TaskPriority::Normal);
        let ok = q.retry_task(task, 3);
        assert!(ok);
        assert_eq!(q.len(), 1);
        let dequeued = q.dequeue().unwrap();
        assert_eq!(dequeued.retry_count, 1);
    }

    #[test]
    fn retry_task_respects_max_retries() {
        let mut q = AgentQueue::new(AgentId(1), "test");
        let mut task = make_task(42, TaskPriority::Normal);
        task.retry_count = 3;
        let ok = q.retry_task(task, 3);
        assert!(!ok, "should refuse when retry_count >= max_retries");
        assert!(q.is_empty());
    }

    #[test]
    fn attach_socrates_context_updates_queued_task() {
        let mut q = AgentQueue::new(AgentId(1), "test");
        q.enqueue(make_task(7, TaskPriority::Normal));
        let attached = q.attach_socrates_context(
            TaskId(7),
            crate::socrates::SocratesTaskContext {
                factual_mode: true,
                required_citations: 1,
                evidence_count: 2,
                contradiction_hints: 1,
                retrieval_tier: Some("hybrid".to_string()),
                retrieval_used_vector: true,
                retrieval_used_lexical_fallback: false,
                ..Default::default()
            },
        );
        assert!(attached);
        let t = q.tasks().iter().find(|t| t.id == TaskId(7)).expect("task");
        let soc = t.socrates.as_ref().expect("socrates");
        assert_eq!(soc.retrieval_tier.as_deref(), Some("hybrid"));
        assert!(soc.retrieval_used_vector);
    }
}
