use crate::types::{AgentTask, TaskId, TaskStatus};

use super::AgentQueue;

impl AgentQueue {
    /// Dequeue the highest-priority ready task.
    /// Returns `None` if the queue is empty, paused, or all tasks are blocked.
    pub fn dequeue(&mut self) -> Option<AgentTask> {
        if self.paused {
            return None;
        }
        // Find the first task that is ready (not blocked)
        let pos = self.tasks.iter().position(|t| match &t.status {
            TaskStatus::Queued => t.is_ready(&self.completed),
            _ => false,
        })?;
        let mut task = self.tasks.remove(pos)?;
        task.status = TaskStatus::InProgress;
        task.start(); // record wall-clock start for temporal context injection
        self.in_progress = Some(task.clone());
        self.last_active = std::time::SystemTime::now();
        Some(task)
    }

    /// Peek at the next task without removing it.
    pub fn peek(&self) -> Option<&AgentTask> {
        self.tasks.front()
    }

    /// Mark the current in-progress task as completed.
    pub fn mark_complete(&mut self, task_id: TaskId) -> bool {
        if let Some(ref task) = self.in_progress {
            if task.id == task_id {
                self.completed.push(task_id);
                self.in_progress = None;
                // Unblock tasks that depended on this one
                self.unblock(task_id);
                self.last_active = std::time::SystemTime::now();
                return true;
            }
        }
        false
    }

    /// Mark the current in-progress task as failed.
    pub fn mark_failed(&mut self, task_id: TaskId, reason: String) -> bool {
        if let Some(ref mut task) = self.in_progress {
            if task.id == task_id {
                task.status = TaskStatus::Failed(reason);
                self.in_progress = None;
                return true;
            }
        }
        false
    }

    /// Unblock all queued tasks that depended on the given task.
    pub fn unblock(&mut self, completed_task_id: TaskId) {
        for task in self.tasks.iter_mut() {
            task.depends_on.retain(|dep| *dep != completed_task_id);
            if task.depends_on.is_empty() {
                if let TaskStatus::Blocked(_) = &task.status {
                    task.status = TaskStatus::Queued;
                }
            }
        }
    }

    /// Check if a specific task is blocked on unmet dependencies.
    pub fn is_blocked(&self, task_id: TaskId) -> bool {
        self.tasks
            .iter()
            .find(|t| t.id == task_id)
            .map(|t| matches!(&t.status, TaskStatus::Blocked(_)))
            .unwrap_or(false)
    }

    /// Pause this queue — no tasks will be dequeued.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume this queue.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Whether this queue is paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Number of completed tasks.
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    /// Whether there is a task currently in progress.
    pub fn has_in_progress(&self) -> bool {
        self.in_progress.is_some()
    }

    /// Number of tasks in progress (0 or 1).
    pub fn in_progress_count(&self) -> usize {
        if self.in_progress.is_some() { 1 } else { 0 }
    }

    /// Get the currently in-progress task.
    pub fn current_task(&self) -> Option<&AgentTask> {
        self.in_progress.as_ref()
    }

    /// List of completed task IDs.
    pub fn completed_ids(&self) -> &[TaskId] {
        &self.completed
    }

    /// Drain all pending tasks out of this queue (for redistribution).
    pub fn drain_tasks(&mut self) -> Vec<AgentTask> {
        self.tasks.drain(..).collect()
    }

    /// Time out any in-progress task that has been running longer than `timeout`.
    /// Returns the timed-out task if one was found.
    pub fn timeout_in_progress(&mut self, timeout: std::time::Duration) -> Option<AgentTask> {
        let now = std::time::Instant::now();
        let now_ms = crate::types::now_unix_ms();
        if let Some(ref task) = self.in_progress {
            let expired = if let Some(created) = task.created_at {
                now.duration_since(created) >= timeout
            } else {
                now_ms.saturating_sub(task.created_at_ms) >= timeout.as_millis() as u64
            };
            if expired {
                return self.in_progress.take();
            }
        }
        None
    }

    /// Requeue a failed or timed-out task with an incremented retry count.
    /// Returns `false` if `max_retries` was already reached.
    pub fn retry_task(&mut self, mut task: AgentTask, max_retries: u32) -> bool {
        if task.retry_count >= max_retries {
            return false;
        }
        task.retry_count += 1;
        // Exponential backoff: store next-eligible time in task metadata
        task.status = TaskStatus::Queued;
        self.enqueue(task);
        true
    }

    /// Drain all tasks that have exceeded a timeout (for external rescheduling).
    pub fn drain_timed_out(&mut self, timeout: std::time::Duration) -> Vec<AgentTask> {
        let now = std::time::Instant::now();
        let now_ms = crate::types::now_unix_ms();
        let mut timed_out = Vec::new();
        let mut i = 0;
        let timeout_ms = timeout.as_millis() as u64;
        while i < self.tasks.len() {
            let task = &self.tasks[i];
            let expired = if let Some(created) = task.created_at {
                now.duration_since(created) >= timeout
            } else {
                now_ms.saturating_sub(task.created_at_ms) >= timeout_ms
            };
            if expired {
                if let Some(t) = self.tasks.remove(i) {
                    timed_out.push(t);
                }
            } else {
                i += 1;
            }
        }
        timed_out
    }
}
