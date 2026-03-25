//! Task priority, status, categories, and agent task model.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::time::Instant;

use super::ids::{TaskId, is_zero_f64, now_unix_ms};

/// Priority level for a task. Higher priority tasks are dequeued first.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    /// Background work — lowest priority.
    Background = 0,
    /// Normal priority — default.
    Normal = 1,
    /// Urgent — preempts normal tasks.
    Urgent = 2,
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Background => write!(f, "background"),
            Self::Normal => write!(f, "normal"),
            Self::Urgent => write!(f, "urgent"),
        }
    }
}

/// Current execution status of a task.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Waiting in the queue to be picked up.
    Queued,
    /// Currently being executed by an agent.
    InProgress,
    /// Successfully completed.
    Completed,
    /// Failed with an error reason.
    Failed(String),
    /// Blocked waiting for another task to complete.
    Blocked(TaskId),
    /// Explicitly cancelled by user or system.
    Cancelled,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed(reason) => write!(f, "failed: {}", reason),
            Self::Blocked(dep) => write!(f, "blocked on {}", dep),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Kind of access an agent requires on a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessKind {
    /// Read-only access (multiple agents can hold simultaneously).
    Read,
    /// Exclusive write access (only one agent at a time).
    Write,
}

/// A file path paired with the access kind required for a task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileAffinity {
    /// Path the task touches.
    pub path: PathBuf,
    /// Required lock / sharing mode.
    pub access: AccessKind,
}

impl FileAffinity {
    /// Read-only affinity for `path`.
    pub fn read(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            access: AccessKind::Read,
        }
    }

    /// Exclusive write affinity for `path`.
    pub fn write(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            access: AccessKind::Write,
        }
    }
}

/// General category of a task to guide model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TaskCategory {
    /// Parser / syntax work.
    Parsing,
    /// Static analysis and type system work.
    TypeChecking,
    /// Debugger-driven investigation.
    Debugging,
    /// Open-ended information gathering.
    Research,
    /// Test authoring and execution.
    Testing,
    /// Default — codegen and implementation tasks.
    #[default]
    CodeGen,
    /// Code review and critique.
    Review,
}

/// Description of a task before it is assigned an ID and routed in the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDescriptor {
    /// Human-readable work summary.
    pub description: String,
    /// Optional queue priority override.
    pub priority: Option<TaskPriority>,
    /// Files read or written by this task.
    pub file_manifest: Vec<FileAffinity>,
    /// Dependencies on tasks already in the orchestrator.
    pub depends_on: Vec<TaskId>,
    /// Intra-batch dependencies by index in the same submit call.
    pub temp_deps: Vec<usize>,
    /// Optional capability requirements for routing (same semantics as [`AgentTask::capability_requirements`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_requirements: Option<crate::contract::TaskCapabilityHints>,
    /// Optional session link (for chat/workflow grouping in Mens).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// A unit of work to be executed by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Unique task identifier.
    pub id: TaskId,
    /// Human-readable description of the work.
    pub description: String,
    /// Execution priority.
    pub priority: TaskPriority,
    /// Current status.
    pub status: TaskStatus,
    /// Files this task needs to read or write.
    pub file_manifest: Vec<FileAffinity>,
    /// Tasks that must complete before this one can start.
    pub depends_on: Vec<TaskId>,
    /// Estimated complexity (1-10 scale).
    pub estimated_complexity: u8,
    /// Model preference string (if any).
    pub model_preference: Option<String>,
    /// Explicit override for the model to use.
    pub model_override: Option<String>,
    /// Task category to help select the best model.
    pub task_category: TaskCategory,
    /// Number of times this task has been re-routed due to validation failures.
    pub debug_iterations: u8,
    /// Number of retry attempts (for timeout/failure recovery).
    pub retry_count: u32,
    /// When the task was created (not serialized — reconstructed on load).
    #[serde(skip)]
    pub created_at: Option<Instant>,
    /// Unix timestamp (ms) when this task object was first created (vcs/serialization safe).
    pub created_at_ms: u64,
    /// Unix timestamp (ms) when agent began executing this task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<u64>,
    /// Unix timestamp (ms) of the last expensive operation (e.g. full build).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_expensive_op_ms: Option<u64>,
    /// Optional Socrates evidence contract for factual completion gating.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socrates: Option<crate::socrates::SocratesTaskContext>,
    /// Optional GPU / hardware routing hints for distributed execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_requirements: Option<crate::contract::TaskCapabilityHints>,
    /// Optional session link (for chat/workflow grouping in Mens).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Effective attention weight computed at gate time (Phase 15). 0.0 = not yet computed.
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    pub attention_weight: f64,
    /// Approval tier assigned by the attention gate (Phase 15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_tier: Option<crate::attention::ApprovalTier>,
    /// Optional planning session this task belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_session_id: Option<String>,
    /// Optional planning node this task implements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_node_id: Option<String>,
    /// Optional planning version for this task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_version: Option<u32>,
    /// Serialized execution policy generated by planner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_policy_json: Option<String>,
}

impl AgentTask {
    /// Create a new task with the given parameters.
    pub fn new(
        id: TaskId,
        description: impl Into<String>,
        priority: TaskPriority,
        file_manifest: Vec<FileAffinity>,
    ) -> Self {
        Self {
            id,
            description: description.into(),
            priority,
            status: TaskStatus::Queued,
            file_manifest,
            depends_on: Vec::new(),
            estimated_complexity: 5,
            model_preference: None,
            model_override: None,
            task_category: TaskCategory::default(),
            debug_iterations: 0,
            retry_count: 0,
            created_at: Some(Instant::now()),
            created_at_ms: now_unix_ms(),
            started_at_ms: None,
            last_expensive_op_ms: None,
            socrates: None,
            capability_requirements: None,
            session_id: None,
            attention_weight: 0.0,
            approval_tier: None,
            plan_session_id: None,
            plan_node_id: None,
            plan_version: None,
            execution_policy_json: None,
        }
    }

    /// Attach a session ID to this task.
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add a dependency on another task.
    pub fn depends_on(mut self, dep: TaskId) -> Self {
        self.depends_on.push(dep);
        self.status = TaskStatus::Blocked(dep);
        self
    }

    /// Set estimated complexity (clamped to 1-10).
    pub fn complexity(mut self, c: u8) -> Self {
        self.estimated_complexity = c.clamp(1, 10);
        self
    }

    /// Set task category.
    pub fn category(mut self, cat: TaskCategory) -> Self {
        self.task_category = cat;
        self
    }

    /// Check if all dependencies are in the given completed set.
    pub fn is_ready(&self, completed: &[TaskId]) -> bool {
        self.depends_on.iter().all(|dep| completed.contains(dep))
    }

    /// List of files this task will write to.
    pub fn write_files(&self) -> Vec<&PathBuf> {
        self.file_manifest
            .iter()
            .filter(|f| f.access == AccessKind::Write)
            .map(|f| &f.path)
            .collect()
    }

    /// Mark the task as started, recording the start timestamp.
    pub fn start(&mut self) -> &mut Self {
        self.started_at_ms = Some(now_unix_ms());
        self
    }

    /// Record that an expensive operation occurred during this task.
    pub fn record_expensive_op(&mut self) {
        self.last_expensive_op_ms = Some(now_unix_ms());
    }

    /// Milliseconds since the last expensive operation in this task, if any.
    pub fn elapsed_since_last_expensive_op_ms(&self) -> Option<u64> {
        self.last_expensive_op_ms
            .map(|t| now_unix_ms().saturating_sub(t))
    }
}

#[cfg(test)]
mod tests {
    use super::super::ids::TaskId;
    use super::*;

    #[test]
    fn task_priority_ordering() {
        assert!(TaskPriority::Urgent > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Background);
    }

    #[test]
    fn file_affinity_constructors() {
        let r = FileAffinity::read("foo.rs");
        assert_eq!(r.access, AccessKind::Read);
        let w = FileAffinity::write("bar.rs");
        assert_eq!(w.access, AccessKind::Write);
    }

    #[test]
    fn agent_task_dependency_check() {
        let task = AgentTask::new(TaskId(1), "test task", TaskPriority::Normal, vec![])
            .depends_on(TaskId(10))
            .depends_on(TaskId(20));

        assert!(!task.is_ready(&[TaskId(10)]));
        assert!(task.is_ready(&[TaskId(10), TaskId(20)]));
    }

    #[test]
    fn serialization_roundtrip() {
        let task = AgentTask::new(
            TaskId(1),
            "fix parser",
            TaskPriority::Urgent,
            vec![FileAffinity::write("src/parser.rs")],
        );
        let json = serde_json::to_string(&task).expect("serialize");
        let back: AgentTask = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, task.id);
        assert_eq!(back.priority, task.priority);
        assert_eq!(back.description, task.description);
    }

    #[test]
    fn task_start_sets_started_at_ms() {
        let mut task = AgentTask::new(TaskId(1), "test", TaskPriority::Normal, vec![]);
        assert!(task.started_at_ms.is_none(), "should not be started yet");
        task.start();
        assert!(
            task.started_at_ms.is_some(),
            "start() must populate started_at_ms"
        );
        let ts = task.started_at_ms.unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        assert!(
            now.saturating_sub(ts) < 5_000,
            "started_at_ms should be recent"
        );
    }

    #[test]
    fn expensive_op_elapsed_ms_is_monotone() {
        let mut task = AgentTask::new(TaskId(2), "test", TaskPriority::Normal, vec![]);
        assert!(task.elapsed_since_last_expensive_op_ms().is_none());
        task.record_expensive_op();
        let elapsed = task.elapsed_since_last_expensive_op_ms();
        assert!(elapsed.is_some(), "should have elapsed after recording");
        assert!(elapsed.unwrap() < 1_000, "should be < 1s in test");
    }

    #[test]
    fn task_start_idempotent_timestamp_stable() {
        let mut task = AgentTask::new(TaskId(3), "test", TaskPriority::Normal, vec![]);
        task.start();
        let first = task.started_at_ms.unwrap();
        task.start();
        let second = task.started_at_ms.unwrap();
        assert!(
            second >= first,
            "second start should not go backward in time"
        );
    }
}
