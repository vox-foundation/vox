//! Inter-agent bulletin and A2A message types.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

use super::ids::{AgentId, CorrelationId, TaskId, now_unix_ms};

/// Messages exchanged via the shared bulletin board.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessage {
    /// An agent changed a file.
    FileChanged {
        /// Path that changed.
        path: PathBuf,
        /// Agent that performed the edit.
        agent: AgentId,
        /// Short description of the change.
        summary: String,
    },
    /// An agent completed a task.
    TaskCompleted {
        /// Finished task.
        task_id: TaskId,
        /// Agent that completed it.
        agent_id: AgentId,
    },
    /// A dependency is now satisfied — unblock waiting tasks.
    DependencyReady {
        /// Task that became runnable.
        task_id: TaskId,
    },
    /// Interrupt: an agent should pause or re-plan.
    Interrupt {
        /// Target agent.
        agent_id: AgentId,
        /// Operator or scheduler reason.
        reason: String,
    },
    /// An agent was spawned in the pool.
    AgentSpawned {
        /// New agent id.
        agent_id: AgentId,
        /// Worker display name.
        name: String,
    },
    /// A task was assigned to an agent.
    TaskAssigned {
        /// Assignee.
        agent_id: AgentId,
        /// Task now owned by the agent.
        task_id: TaskId,
    },
    /// A file lock was acquired.
    LockAcquired {
        /// Lock holder.
        agent_id: AgentId,
        /// Locked path.
        path: PathBuf,
    },
    /// A task failed.
    TaskFailed {
        /// Agent that was executing the task.
        agent_id: AgentId,
        /// Failed task id.
        task_id: TaskId,
        /// Failure message.
        error: String,
    },
    /// A task was flagged as suspect by user.
    TaskDoubted {
        /// Suspected agent.
        agent_id: AgentId,
        /// Doubted task id.
        task_id: TaskId,
        /// Optional reason.
        reason: Option<String>,
    },
    /// Phase 9: A question directed from one agent to another/user.
    Question {
        /// Asking agent.
        from: AgentId,
        /// Intended answerer.
        to: AgentId,
        /// Question body.
        question: String,
        /// Correlation id for matching answers.
        correlation_id: CorrelationId,
    },
    /// Phase 9: An answer back to the requesting agent.
    Answer {
        /// Answering agent.
        from: AgentId,
        /// Original asker.
        to: AgentId,
        /// Answer body.
        answer: String,
        /// Matches the question's correlation id.
        correlation_id: CorrelationId,
    },
    /// Phase 9: A persistent announcement across all agents.
    Broadcast {
        /// Sender.
        from: AgentId,
        /// Announcement text.
        message: String,
    },
    /// Phase 9: A context key update notification.
    ContextUpdate {
        /// Agent publishing the update.
        from: AgentId,
        /// Context key.
        key: String,
        /// Serialized value.
        value: String,
    },
    /// A resource lock was acquired.
    ResourceLockAcquired {
        agent_id: AgentId,
        resource_id: String,
    },
    /// A resource lock was released.
    ResourceLockReleased {
        agent_id: AgentId,
        resource_id: String,
    },
    /// A structured agent-to-agent message (Integrates A2A into Bulletin).
    A2A(A2AMessage),
    /// Orchestrator policy escalated an action to HITL (D5+D9).
    EscalationRequired {
        session_id: String,
        grade: String,
        action_description: String,
    },
    /// A sub-agent was dispatched for a subtask (D4).
    SubAgentDispatched {
        parent_agent_id: AgentId,
        child_task_description: String,
        chain_depth: u32,
    },
    /// Agent spawn chain depth exceeded the safety limit (D4).
    ChainDepthAlert { current_depth: u32, max_depth: u32 },
}

/// Unique identifier for a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub u64);

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "M-{:06}", self.0)
    }
}

/// The type of A2A message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum A2AMessageType {
    /// A plan handoff from one agent to another.
    PlanHandoff,
    /// Request to claim scope over a set of files.
    ScopeRequest,
    /// Grant scope over requested files.
    ScopeGrant,
    /// Progress update on current work.
    ProgressUpdate,
    /// Request for help from another agent.
    HelpRequest,
    /// Notification that a task/objective is complete.
    CompletionNotice,
    /// Report of an error or failure.
    ErrorReport,
    /// Free-form text message.
    FreeForm,
    /// A file conflict has been detected — receiver should inspect.
    ConflictDetected,
    /// A file conflict has been resolved.
    ConflictResolved,
    /// A VCS state change event (branch, commit, merge).
    VcsEvent,
    /// Request to cancel a task the receiver is working on.
    CancelRequest,
    /// Snapshot ID exchange for before/after context sharing.
    SnapshotShare,
    /// Broadcast a unified news item to all publishers.
    BroadcastNews,
    /// MENS Observer requests validation research from Socrates.
    SocratesResearchRequest,
    /// Request for a generic resource lock.
    ResourceLockRequest,
    /// Grant for a generic resource lock.
    ResourceLockGrant,
}

impl A2AMessageType {
    /// Return the snake_case string representation of the message type.
    pub fn into_str(&self) -> &'static str {
        match self {
            Self::PlanHandoff => "plan_handoff",
            Self::ScopeRequest => "scope_request",
            Self::ScopeGrant => "scope_grant",
            Self::ProgressUpdate => "progress_update",
            Self::HelpRequest => "help_request",
            Self::CompletionNotice => "completion_notice",
            Self::ErrorReport => "error_report",
            Self::FreeForm => "free_form",
            Self::ConflictDetected => "conflict_detected",
            Self::ConflictResolved => "conflict_resolved",
            Self::VcsEvent => "vcs_event",
            Self::CancelRequest => "cancel_request",
            Self::SnapshotShare => "snapshot_share",
            Self::BroadcastNews => "broadcast_news",
            Self::SocratesResearchRequest => "socrates_research_request",
            Self::ResourceLockRequest => "resource_lock_request",
            Self::ResourceLockGrant => "resource_lock_grant",
        }
    }
}

impl std::fmt::Display for A2AMessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.into_str())
    }
}

/// Priority of an A2A message, mirroring task priority but for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MessagePriority {
    /// Background info, no urgency.
    Low = 0,
    /// Default priority.
    #[default]
    Normal = 1,
    /// Time-sensitive — process before Normal messages.
    High = 2,
    /// Circuit-breaker level — must be acted on immediately.
    Critical = 3,
}

/// A unique identifier for a conversation thread between agents.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThreadId(pub String);

impl ThreadId {
    /// Create a new random thread ID.
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        Self(format!("thread-{nonce:08x}"))
    }

    /// Create from an existing string.
    pub fn from(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for ThreadId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ThreadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// VCS context attached to an A2A message.
/// Allows agents to share the exact code state they are discussing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcsContext {
    /// Snapshot ID before the operation (if applicable).
    pub snapshot_before: Option<u64>,
    /// Snapshot ID after the operation (if applicable).
    pub snapshot_after: Option<u64>,
    /// Files touched by the operation.
    pub touched_paths: Vec<PathBuf>,
    /// Logical change ID this message relates to.
    pub change_id: Option<u64>,
    /// Operation log entry ID this message relates to.
    pub op_id: Option<u64>,
    /// Content hash of the primary file being discussed.
    pub content_hash: Option<String>,
}

/// Envelope metadata for traceability and precedence (system > policy > user > peer).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Trace ID for this message chain.
    pub trace_id: Option<String>,
    /// Parent trace (e.g. handoff or task that triggered this).
    pub parent_trace_id: Option<String>,
    /// Intent/request ID for grouping related messages.
    pub intent_id: Option<String>,
    /// Hash of the goal/objective this message relates to.
    pub goal_hash: Option<String>,
    /// Sender role (e.g. "system", "user", "agent").
    pub sender_role: Option<String>,
    /// Trust level for provenance (e.g. "trusted", "untrusted").
    /// Deprecated: use `trust_tier`. Kept for Arca backward-compat.
    pub trust_level: Option<String>,
    /// Typed trust tier from Phase 15 attention system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_tier: Option<crate::attention::TrustTier>,
    /// Expiry timestamp in unix milliseconds (None = no expiry).
    pub expiry_ms: Option<u64>,
}

/// A structured message between agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    /// Unique message ID.
    pub id: MessageId,
    /// Sender agent.
    pub sender: AgentId,
    /// Receiver (None = broadcast to all).
    pub receiver: Option<AgentId>,
    /// Message type.
    pub msg_type: A2AMessageType,
    /// Message payload (structured or free-form text).
    pub payload: String,
    /// Correlation ID for threading related messages.
    pub correlation_id: Option<String>,
    /// Whether the receiver has acknowledged this message.
    pub acknowledged: bool,
    /// Timestamp in unix milliseconds.
    pub timestamp_ms: u64,
    /// Optional envelope for traceability and conflict-aware handling.
    #[serde(default)]
    pub envelope: Option<MessageEnvelope>,
    /// Message priority — higher priority messages should be processed first.
    #[serde(default)]
    pub priority: MessagePriority,
    /// Conversation thread ID — groups related messages together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<ThreadId>,
    /// VCS context — the exact code state this message relates to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vcs_context: Option<VcsContext>,
    /// Time-to-live in milliseconds. If None, default 300_000 (5 min) applies at read time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
}

impl A2AMessage {
    /// Create a new message.
    pub fn new(
        id: MessageId,
        sender: AgentId,
        receiver: Option<AgentId>,
        msg_type: A2AMessageType,
        payload: impl Into<String>,
    ) -> Self {
        let timestamp_ms = now_unix_ms();

        Self {
            id,
            sender,
            receiver,
            msg_type,
            payload: payload.into(),
            correlation_id: None,
            acknowledged: false,
            timestamp_ms,
            envelope: None,
            priority: MessagePriority::Normal,
            thread_id: None,
            vcs_context: None,
            ttl_ms: None,
        }
    }

    /// Create a new message with a custom time-to-live.
    pub fn new_with_ttl(
        id: MessageId,
        sender: AgentId,
        receiver: Option<AgentId>,
        msg_type: A2AMessageType,
        payload: impl Into<String>,
        ttl_ms: u64,
    ) -> Self {
        let mut msg = Self::new(id, sender, receiver, msg_type, payload);
        msg.ttl_ms = Some(ttl_ms);
        msg
    }

    /// Whether this message has exceeded its TTL (default 300_000ms / 5 min).
    pub fn is_expired(&self) -> bool {
        let ttl = self.ttl_ms.unwrap_or(300_000);
        self.elapsed_ms() > ttl
    }

    /// Attach envelope metadata for traceability.
    pub fn with_envelope(mut self, envelope: MessageEnvelope) -> Self {
        self.envelope = Some(envelope);
        self
    }

    /// Set message priority.
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Assign to a conversation thread.
    pub fn in_thread(mut self, thread_id: ThreadId) -> Self {
        self.thread_id = Some(thread_id);
        self
    }

    /// Attach VCS context (snapshot IDs, touched paths, change ID).
    pub fn sub_vcs_context(mut self, ctx: VcsContext) -> Self {
        self.vcs_context = Some(ctx);
        self
    }

    /// Attach VCS context (snapshot IDs, touched paths, change ID).
    pub fn with_vcs_context(mut self, ctx: VcsContext) -> Self {
        self.vcs_context = Some(ctx);
        self
    }

    /// Milliseconds since this message was created.
    pub fn elapsed_ms(&self) -> u64 {
        now_unix_ms().saturating_sub(self.timestamp_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ids::AgentId;
    use super::*;

    #[test]
    fn message_serialization_roundtrip() {
        let msg = AgentMessage::FileChanged {
            path: PathBuf::from("src/lib.rs"),
            agent: AgentId(1),
            summary: "added TryCatch variant".to_string(),
        };
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: AgentMessage = serde_json::from_str(&json).expect("deserialize");
        match back {
            AgentMessage::FileChanged {
                path,
                agent,
                summary,
            } => {
                assert_eq!(path, PathBuf::from("src/lib.rs"));
                assert_eq!(agent, AgentId(1));
                assert_eq!(summary, "added TryCatch variant");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn a2a_message_elapsed_ms_grows_over_time() {
        let msg = A2AMessage::new(
            MessageId(1),
            AgentId(1),
            Some(AgentId(2)),
            A2AMessageType::FreeForm,
            "hello",
        );
        let elapsed = msg.elapsed_ms();
        assert!(
            elapsed < 1_000,
            "freshly created message should have elapsed < 1000ms, got {elapsed}"
        );
    }
}
