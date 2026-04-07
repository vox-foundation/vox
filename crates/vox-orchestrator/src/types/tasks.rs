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
    /// Generating new code or scaffolding.
    CodeGen,
    /// Multi-step workflow synthesis.
    #[default]
    General,
    /// Automated Reasoning System execution.
    Ars,
    /// Up-front orchestration planning.
    Planning,
    /// Code review and critique.
    Review,
}

/// Populi mesh holds execution authority for this task; local actors must not dequeue it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PopuliRemoteDelegate {
    /// Same key as [`crate::a2a::RemoteTaskEnvelope::idempotency_key`] for cancel/result correlation.
    pub idempotency_key: String,
    /// Populi execution lease id when lease APIs are active for this task class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<String>,
    /// Claimer node identity used for lease renew/release calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimer_node_id: Option<String>,
}

/// Optional hints applied at enqueue time and merged into [`AgentTask`] for routing / telemetry.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskEnqueueHints {
    /// When set, overrides default task category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_category: Option<TaskCategory>,
    /// Estimated complexity 1–10; clamped when merged onto the task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complexity: Option<u8>,
    /// Non-binding preference string (e.g. tier hint); stored on [`AgentTask::model_preference`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_preference: Option<String>,
    /// If set, stored on [`AgentTask::model_override`] for labeling and downstream routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_override: Option<String>,
    /// Optional reconstruction campaign id for long-horizon grouped runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,
    /// Optional benchmark tier for progressive reconstruction gating.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benchmark_tier: Option<crate::reconstruction::ReconstructionBenchmarkTier>,
    /// Optional explicit specialization role for multi-agent protocol runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_role: Option<crate::reconstruction::AgentExecutionRole>,
    /// Optional logical thread id preserving branch continuity inside a session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    /// Optional portable harness contract supplied by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_spec_json: Option<String>,
    /// Optional tool declaration hints (e.g. [[tool:vox_run_tests]]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_hints: Vec<String>,
    /// Optional research intent hints (e.g. [[research:vector]]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub research_hints: Vec<String>,
    /// Optional labels for mesh capability routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_labels: Option<Vec<String>>,
    /// True if the mesh task should detach for asynchronous execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_detached: Option<bool>,
}

/// Completion-time attestation metadata supplied by clients (e.g. MCP) for policy checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletionAttestation {
    /// Human-readable completion summary used for no-write policy validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_summary: Option<String>,
    /// Optional list of checks the caller claims were run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks_passed: Vec<String>,
    /// Evidence references that must appear in the session [`crate::ContextEnvelope`] (substring match).
    /// Also see `[[voxcite:...]]` markers in [`Self::completion_summary`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_citations: Vec<String>,
    /// Optional artifacts produced by the task (workspace-relative paths preferred).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_paths: Vec<PathBuf>,
    /// Explicit declaration that output avoids placeholders / stubs.
    #[serde(default)]
    pub declared_non_placeholder: bool,
    /// Allow risky completion with explicit reason (audited and logged).
    #[serde(default)]
    pub force_risky: bool,
    /// Required when `force_risky` is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force_risky_reason: Option<String>,
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
    /// Optional logical thread id preserving branch continuity for handoff or remote execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
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
    /// Number of times this task has failed Toestub gates.
    #[serde(default)]
    pub toestub_iterations: u8,
    /// Number of times this task has failed Socrates evidence checks.
    #[serde(default)]
    pub socrates_iterations: u8,
    /// Optional tool declaration hints extracted from description (e.g. `[[tool:vox_run_tests]]`).
    #[serde(default)]
    pub tool_hints: Vec<String>,
    /// Optional research intent hints extracted from description (e.g. `[[research:vector]]`).
    #[serde(default)]
    pub research_hints: Vec<String>,
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
    /// Optional logical thread id preserving branch continuity for handoff or remote execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
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
    /// Optional campaign id for grouped reconstruction attempts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,
    /// Optional benchmark tier for this task when campaign scoring is active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benchmark_tier: Option<crate::reconstruction::ReconstructionBenchmarkTier>,
    /// Optional explicit execution role (planner/builder/verifier/reproducer/researcher).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_role: Option<crate::reconstruction::AgentExecutionRole>,
    /// Optional portable harness contract attached to the task for relay, audit, and replay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_spec_json: Option<String>,
    /// When set, this task was handed to Populi A2A remote execution; local queue must not run it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub populi_remote_delegate: Option<PopuliRemoteDelegate>,
}

impl AgentTask {
    /// Create a new task with the given parameters.
    pub fn new(
        id: TaskId,
        description: impl Into<String>,
        priority: TaskPriority,
        file_manifest: Vec<FileAffinity>,
    ) -> Self {
        let description = description.into();
        let (tool_hints, research_hints) = Self::parse_description_hints(&description);
        Self {
            id,
            description,
            priority,
            status: TaskStatus::Queued,
            file_manifest,
            depends_on: Vec::new(),
            estimated_complexity: 5,
            model_preference: None,
            model_override: None,
            task_category: TaskCategory::default(),
            debug_iterations: 0,
            toestub_iterations: 0,
            socrates_iterations: 0,
            tool_hints,
            research_hints,
            campaign_id: None,
            retry_count: 0,
            created_at: Some(Instant::now()),
            created_at_ms: now_unix_ms(),
            started_at_ms: None,
            last_expensive_op_ms: None,
            socrates: None,
            capability_requirements: None,
            session_id: None,
            thread_id: None,
            attention_weight: 0.0,
            approval_tier: None,
            plan_session_id: None,
            plan_node_id: None,
            plan_version: None,
            execution_policy_json: None,
            benchmark_tier: None,
            execution_role: None,
            harness_spec_json: None,
            populi_remote_delegate: None,
        }
    }

    /// Extract structured hints from double-bracketed tags in the description.
    ///
    /// Matches `[[tool:name]]` and `[[research:topic]]`.
    pub fn parse_description_hints(description: &str) -> (Vec<String>, Vec<String>) {
        let mut tools = Vec::new();
        let mut research = Vec::new();

        // Simple manual scan to avoid heavy regex in core task types if possible.
        let mut start = 0;
        while let Some(open) = description[start..].find("[[") {
            let open_pos = start + open;
            if let Some(close) = description[open_pos..].find("]]") {
                let close_pos = open_pos + close;
                let inner = &description[open_pos + 2..close_pos];
                if let Some(colon) = inner.find(':') {
                    let kind = &inner[..colon];
                    let value = inner[colon + 1..].trim();
                    if !value.is_empty() {
                        match kind {
                            "tool" => tools.push(value.to_string()),
                            "research" => research.push(value.to_string()),
                            _ => {}
                        }
                    }
                }
                start = close_pos + 2;
            } else {
                break;
            }
        }

        (tools, research)
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

    #[test]
    fn enqueue_hints_roundtrip_preserves_campaign_tier_and_role() {
        let hints = TaskEnqueueHints {
            task_category: Some(TaskCategory::Testing),
            complexity: Some(7),
            model_preference: Some("free".to_string()),
            model_override: Some("model-x".to_string()),
            campaign_id: Some("camp-123".to_string()),
            benchmark_tier: Some(crate::reconstruction::ReconstructionBenchmarkTier::CrateRegen),
            execution_role: Some(crate::reconstruction::AgentExecutionRole::Verifier),
            thread_id: Some("thread-123".to_string()),
            harness_spec_json: Some("{\"schema_version\":1}".to_string()),
            tool_hints: vec![],
            research_hints: vec![],
            required_labels: None,
            is_detached: None,
        };
        let json = serde_json::to_string(&hints).expect("serialize hints");
        let back: TaskEnqueueHints = serde_json::from_str(&json).expect("deserialize hints");
        assert_eq!(back.campaign_id.as_deref(), Some("camp-123"));
        assert_eq!(
            back.benchmark_tier,
            Some(crate::reconstruction::ReconstructionBenchmarkTier::CrateRegen)
        );
        assert_eq!(
            back.execution_role,
            Some(crate::reconstruction::AgentExecutionRole::Verifier)
        );
        assert_eq!(back.thread_id.as_deref(), Some("thread-123"));
        assert_eq!(
            back.harness_spec_json.as_deref(),
            Some("{\"schema_version\":1}")
        );
    }
}
