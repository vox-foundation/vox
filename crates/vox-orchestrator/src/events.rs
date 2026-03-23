//! Real-time event bus for agent activity broadcasting.
//!
//! Publishes structured `AgentEvent`s over a tokio broadcast channel.
//! Consumers (dashboard SSE, monitors, gamify hooks) subscribe and receive
//! events as they happen — no polling, no JSONL heuristics.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

use crate::types::{AgentId, TaskId};

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

/// Monotonically increasing event ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub u64);

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E-{:06}", self.0)
    }
}

/// What an agent is currently doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActivity {
    /// Writing code / editing files.
    Writing,
    /// Reading / searching files.
    Reading,
    /// Running a command or tool.
    Executing,
    /// Thinking / planning (waiting for LLM response).
    Thinking,
    /// Waiting for user input or permission.
    WaitingForInput,
    /// Idle — no active task.
    Idle,
}

impl std::fmt::Display for AgentActivity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Writing => write!(f, "writing"),
            Self::Reading => write!(f, "reading"),
            Self::Executing => write!(f, "executing"),
            Self::Thinking => write!(f, "thinking"),
            Self::WaitingForInput => write!(f, "waiting_for_input"),
            Self::Idle => write!(f, "idle"),
        }
    }
}

impl std::str::FromStr for AgentActivity {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "writing" => Ok(Self::Writing),
            "reading" => Ok(Self::Reading),
            "executing" => Ok(Self::Executing),
            "thinking" => Ok(Self::Thinking),
            "waiting_for_input" => Ok(Self::WaitingForInput),
            "idle" => Ok(Self::Idle),
            _ => Err(format!("unknown activity: {s}")),
        }
    }
}

/// A structured event emitted by the orchestrator.
///
/// Each event carries a unique ID, timestamp, and typed payload.
/// This replaces Pixel Agents' heuristic-based JSONL parsing with
/// deterministic, structured events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    /// Unique event identifier.
    pub id: EventId,
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Event payload.
    pub kind: AgentEventKind,
}

/// The different kinds of events the orchestrator can emit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEventKind {
    /// A new agent was spawned.
    AgentSpawned {
        agent_id: AgentId,
        name: String,
    },
    /// An agent was retired/removed.
    AgentRetired {
        agent_id: AgentId,
    },
    /// An agent's activity changed.
    ActivityChanged {
        agent_id: AgentId,
        activity: AgentActivity,
    },

    /// A task was submitted to the queue.
    TaskSubmitted {
        task_id: TaskId,
        agent_id: AgentId,
        description: String,
    },
    /// A task started executing.
    TaskStarted {
        task_id: TaskId,
        agent_id: AgentId,
    },
    /// A task completed successfully.
    TaskCompleted {
        task_id: TaskId,
        agent_id: AgentId,
    },
    /// A task failed.
    TaskFailed {
        task_id: TaskId,
        agent_id: AgentId,
        error: String,
    },

    /// A file lock was acquired.
    LockAcquired {
        agent_id: AgentId,
        path: PathBuf,
        exclusive: bool,
    },
    /// A file lock was released.
    LockReleased {
        agent_id: AgentId,
        path: PathBuf,
    },

    /// An agent went idle (no pending tasks).
    AgentIdle {
        agent_id: AgentId,
    },
    /// An agent started working again.
    AgentBusy {
        agent_id: AgentId,
    },

    /// An inter-agent message was sent.
    MessageSent {
        from: AgentId,
        to: Option<AgentId>,
        summary: String,
    },

    /// A cost was incurred (LLM API call).
    ///
    /// **MCP:** when Codex is attached, persisted usage is SSOT in `provider_usage`; bus emission is
    /// gated by **`VOX_MCP_LLM_COST_EVENTS`** (default off with DB) to avoid dashboards double-counting.
    CostIncurred {
        agent_id: AgentId,
        provider: String,
        model: String,
        input_tokens: u32,
        output_tokens: u32,
        cost_usd: f64,
    },

    /// Auto-continuation was triggered for an idle agent.
    ContinuationTriggered {
        agent_id: AgentId,
        strategy: String,
    },

    /// A plan handoff between agents.
    PlanHandoff {
        from: AgentId,
        to: AgentId,
        plan_summary: String,
    },

    /// A scope violation was detected.
    ScopeViolation {
        agent_id: AgentId,
        path: PathBuf,
        reason: String,
    },

    /// Context window compaction was triggered.
    CompactionTriggered {
        agent_id: AgentId,
        tokens_before: usize,
        tokens_after: usize,
        strategy: String,
    },

    /// Pre-compaction memory flush completed.
    MemoryFlushed {
        agent_id: AgentId,
        facts_flushed: usize,
    },

    /// A new session was created.
    SessionCreated {
        agent_id: AgentId,
        session_id: String,
    },

    /// A session was reset.
    SessionReset {
        agent_id: AgentId,
        session_id: String,
        turns_cleared: usize,
    },

    /// Workspace / DB snapshot captured for undo or conflict tracking.
    SnapshotCaptured {
        agent_id: AgentId,
        snapshot_id: String,
        file_count: usize,
        description: String,
    },
    /// Overlapping edits detected between agents.
    ConflictDetected {
        path: PathBuf,
        agent_ids: Vec<AgentId>,
        conflict_id: String,
    },
    /// Undo stack applied.
    OperationUndone {
        agent_id: AgentId,
        operation_id: String,
    },
    /// Redo stack applied.
    OperationRedone {
        agent_id: AgentId,
        operation_id: String,
    },
    /// Handoff could not be completed (e.g. spawn failure).
    AgentHandoffRejected {
        from: AgentId,
        reason: String,
    },
    /// Handoff completed; target agent resumed work.
    AgentHandoffAccepted {
        agent_id: AgentId,
        from: AgentId,
        plan_summary: String,
    },
    /// Load balancer moved tasks due to urgent queue depth.
    UrgentRebalanceTriggered {
        moved: usize,
    },
    /// Streaming LLM token chunk (debug / UI).
    TokenStreamed {
        agent_id: AgentId,
        text: String,
    },

    /// Prompt injection / safety gate rejected input (MCP).
    InjectionDetected {
        detail: String,
    },
    /// Canonicalized prompt produced warnings (MCP task submit).
    PromptConflictDetected {
        task_id: TaskId,
        warnings: Vec<String>,
    },
    /// Durable workflow lifecycle (MCP / dashboard).
    WorkflowStarted {
        workflow_id: String,
    },
    WorkflowCompleted {
        workflow_id: String,
    },
    WorkflowFailed {
        workflow_id: String,
        error: String,
    },
    ActivityStarted {
        activity_id: String,
    },
    ActivityCompleted {
        activity_id: String,
    },
    ActivityRetried {
        activity_id: String,
        attempt: u32,
    },
    ConflictResolved {
        conflict_id: String,
        resolution_strategy: String,
    },
    WorkspaceCreated {
        agent_id: AgentId,
        root: PathBuf,
    },
}

// ---------------------------------------------------------------------------
// Event bus
// ---------------------------------------------------------------------------

/// Thread-safe event bus for broadcasting agent events.
///
/// Uses a tokio broadcast channel under the hood. Multiple consumers
/// (dashboard, monitor, gamify hooks) can subscribe independently.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<AgentEvent>,
    id_gen: std::sync::Arc<AtomicU64>,
}

impl EventBus {
    /// Create a new event bus with the given channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            id_gen: std::sync::Arc::new(AtomicU64::new(1)),
        }
    }

    /// Emit an event. Returns the assigned EventId.
    pub fn emit(&self, kind: AgentEventKind) -> EventId {
        let id = EventId(self.id_gen.fetch_add(1, Ordering::Relaxed));
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let event = AgentEvent {
            id,
            timestamp_ms,
            kind,
        };

        tracing::debug!(event_id = %id, "event emitted: {:?}", event.kind);
        let _ = self.sender.send(event);
        id
    }

    /// Subscribe to events. Returns a receiver for all future events.
    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.sender.subscribe()
    }

    /// Number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Get the next event ID that will be assigned.
    pub fn next_event_id(&self) -> u64 {
        self.id_gen.load(Ordering::Relaxed)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn emit_and_receive() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        let id = bus.emit(AgentEventKind::AgentSpawned {
            agent_id: AgentId(1),
            name: "builder".to_string(),
        });

        assert_eq!(id, EventId(1));

        let event = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv())
            .await
            .expect("should not timeout")
            .expect("should receive");

        assert_eq!(event.id, EventId(1));
        assert!(event.timestamp_ms > 0);
        match event.kind {
            AgentEventKind::AgentSpawned { agent_id, name } => {
                assert_eq!(agent_id, AgentId(1));
                assert_eq!(name, "builder");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[tokio::test]
    async fn multiple_subscribers_receive() {
        let bus = EventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        assert_eq!(bus.subscriber_count(), 2);

        bus.emit(AgentEventKind::AgentIdle {
            agent_id: AgentId(2),
        });

        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }

    #[test]
    fn event_serialization_roundtrip() {
        let event = AgentEvent {
            id: EventId(42),
            timestamp_ms: 1234567890,
            kind: AgentEventKind::CostIncurred {
                agent_id: AgentId(1),
                provider: "openrouter".to_string(),
                model: "claude-3".to_string(),
                input_tokens: 100,
                output_tokens: 50,
                cost_usd: 0.005,
            },
        };

        let json = serde_json::to_string(&event).expect("serialize");
        let back: AgentEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, EventId(42));
    }

    #[test]
    fn sequential_event_ids() {
        let bus = EventBus::new(16);
        let id1 = bus.emit(AgentEventKind::AgentIdle {
            agent_id: AgentId(1),
        });
        let id2 = bus.emit(AgentEventKind::AgentBusy {
            agent_id: AgentId(1),
        });
        assert_eq!(id1, EventId(1));
        assert_eq!(id2, EventId(2));
    }
}
