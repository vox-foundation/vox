use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::types::AgentId;
pub use crate::types::{A2AMessage, A2AMessageType, MessageId};
use crate::types::{MessagePriority, ThreadId, VcsContext};

/// Stable A2A wire type for remote task execution envelopes.
pub const REMOTE_TASK_ENVELOPE_TYPE: &str = "remote_task_envelope";
/// Stable A2A wire type for remote task execution acknowledgements.
pub const REMOTE_TASK_ACK_TYPE: &str = "remote_task_ack";
/// Stable A2A wire type for remote task execution results.
pub const REMOTE_TASK_RESULT_TYPE: &str = "remote_task_result";

/// Envelope sent across mesh A2A relay to request remote execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteTaskEnvelope {
    /// Idempotency key used by receivers to deduplicate requests.
    pub idempotency_key: String,
    /// Local task id from the originating orchestrator.
    pub task_id: u64,
    /// Originating repository id.
    pub repository_id: String,
    /// Requested capability hints encoded as JSON.
    pub capability_requirements_json: String,
    /// Opaque task payload contract for the receiver.
    pub payload: String,
}

/// Ack payload for a remote task envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteTaskAck {
    /// Idempotency key from the original envelope.
    pub idempotency_key: String,
    /// Whether receiver accepted the envelope.
    pub accepted: bool,
    /// Optional diagnostic detail.
    pub detail: Option<String>,
}

/// Result payload for a remote task envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteTaskResult {
    /// Idempotency key from the original envelope.
    pub idempotency_key: String,
    /// Whether remote execution succeeded.
    pub success: bool,
    /// Optional result payload.
    pub result: Option<String>,
    /// Optional error detail.
    pub error: Option<String>,
}

/// Database-persisted A2A message row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbA2AMessage {
    pub id: u64,
    pub message_uuid: String,
    pub sender_agent: String,
    pub receiver_agent: String,
    pub msg_type: String,
    pub payload: String,
    pub priority: i64,
    pub thread_id: Option<String>,
    pub acknowledged: bool,
    pub created_at: String,
    pub repository_id: String,
}

/// Relay a message to another mens node via HTTP.
pub async fn relay_to_mesh(
    client: &vox_populi::http_client::PopuliHttpClient,
    sender: AgentId,
    receiver: AgentId,
    msg_type: A2AMessageType,
    payload: impl Into<String>,
) -> Result<(), String> {
    client
        .relay_a2a(&vox_populi::transport::A2ADeliverRequest {
            sender_agent_id: sender.0.to_string(),
            receiver_agent_id: receiver.0.to_string(),
            message_type: msg_type.to_string(),
            payload: payload.into(),
        })
        .await
        .map_err(|e: vox_populi::PopuliRegistryError| e.to_string())
}

/// Relay a structured remote task envelope over the mesh A2A transport.
pub async fn relay_remote_task_envelope(
    client: &vox_populi::http_client::PopuliHttpClient,
    sender: AgentId,
    receiver: AgentId,
    envelope: &RemoteTaskEnvelope,
) -> Result<(), String> {
    let payload = serde_json::to_string(envelope).map_err(|e| e.to_string())?;
    client
        .relay_a2a(&vox_populi::transport::A2ADeliverRequest {
            sender_agent_id: sender.0.to_string(),
            receiver_agent_id: receiver.0.to_string(),
            message_type: REMOTE_TASK_ENVELOPE_TYPE.to_string(),
            payload,
        })
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Send a message to the database with circuit breaker protection.
pub async fn send_to_db_with_breaker(
    db: &vox_db::VoxDb,
    sender: AgentId,
    receiver: AgentId,
    msg_type: A2AMessageType,
    payload: impl Into<String> + Clone,
    priority: MessagePriority,
    thread_id: Option<ThreadId>,
    repository_id: &str,
) -> Result<String, String> {
    db.breaker()
        .call(|| async {
            send_to_db(
                db,
                sender,
                receiver,
                msg_type,
                payload.clone(),
                priority,
                thread_id,
                repository_id,
            )
            .await
        })
        .await
}

/// Send a message to the database for delivery (cross-node).
pub async fn send_to_db(
    store: &vox_db::VoxDb,
    sender: AgentId,
    receiver: AgentId,
    msg_type: A2AMessageType,
    payload: impl Into<String>,
    priority: MessagePriority,
    thread_id: Option<ThreadId>,
    repository_id: &str,
) -> Result<String, String> {
    let uuid = uuid::Uuid::new_v4().to_string();
    let priority_val = match priority {
        MessagePriority::Low => 0,
        MessagePriority::Normal => 1,
        MessagePriority::High => 2,
        MessagePriority::Critical => 3,
    };
    let payload = payload.into();
    let thread_str = thread_id.map(|t| t.0);

    store
        .send_a2a_message(
            &uuid,
            &sender.0.to_string(),
            &receiver.0.to_string(),
            msg_type.into_str(),
            &payload,
            priority_val,
            thread_str.as_deref(),
            repository_id,
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(uuid)
}

/// Poll for new unacknowledged messages for an agent from the database.
pub async fn poll_inbox_from_db(
    store: &vox_db::VoxDb,
    agent_id: AgentId,
    repository_id: &str,
) -> Result<Vec<DbA2AMessage>, String> {
    let rows = store
        .poll_a2a_inbox(&agent_id.0.to_string(), repository_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut msgs = Vec::new();
    for row in rows {
        msgs.push(DbA2AMessage {
            id: row.id as u64,
            message_uuid: row.message_uuid,
            sender_agent: row.sender_agent,
            receiver_agent: row.receiver_agent,
            msg_type: row.msg_type,
            payload: row.payload,
            priority: row.priority,
            thread_id: row.thread_id,
            acknowledged: row.acknowledged,
            created_at: row.created_at,
            repository_id: row.repository_id,
        });
    }
    Ok(msgs)
}

/// Mark a message as acknowledged in the database.
pub async fn acknowledge_db_message(
    store: &vox_db::VoxDb,
    message_uuid: &str,
) -> Result<(), String> {
    store
        .acknowledge_a2a_message_by_uuid(message_uuid)
        .await
        .map_err(|e| e.to_string())
}

/// Remove old acknowledged messages from the database.
pub async fn prune_old_a2a_messages(
    store: &vox_db::VoxDb,
    older_than_days: u32,
) -> Result<u64, String> {
    store
        .prune_a2a_messages(older_than_days)
        .await
        .map_err(|e| e.to_string())
}

/// Routing hint for mens messaging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum A2ARoute {
    /// Local in-memory delivery to an agent on the same node.
    Local,
    /// Send via direct HTTP relay to node.
    Relay(String),
    /// Persist in database for polling.
    Db,
}

/// Message bus for A2A communication.
///
/// Provides inbox-based messaging with support for unicast,
/// broadcast, and multicast delivery.
pub struct MessageBus {
    /// Per-agent inboxes.
    pub(crate) inboxes:
        std::sync::RwLock<HashMap<AgentId, std::sync::RwLock<VecDeque<A2AMessage>>>>,
    /// Audit trail of all messages (most recent at back).
    audit_trail: std::sync::RwLock<Vec<A2AMessage>>,
    /// ID generator.
    id_gen: AtomicU64,
    /// Maximum inbox size per agent before oldest messages are dropped.
    max_inbox_size: usize,
    /// Number of messages dropped due to inbox overflow.
    dropped_messages: AtomicU64,
}

impl MessageBus {
    /// Create a new message bus.
    pub fn new(max_inbox_size: usize) -> Self {
        Self {
            inboxes: std::sync::RwLock::new(HashMap::new()),
            audit_trail: std::sync::RwLock::new(Vec::new()),
            id_gen: AtomicU64::new(1),
            max_inbox_size,
            dropped_messages: AtomicU64::new(0),
        }
    }

    pub(crate) fn next_id(&self) -> MessageId {
        MessageId(self.id_gen.fetch_add(1, Ordering::Relaxed))
    }

    /// Register an agent (creates their inbox).
    pub fn register_agent(&self, agent_id: AgentId) {
        let inboxes = crate::sync_lock::rw_read(&self.inboxes);
        if !inboxes.contains_key(&agent_id) {
            drop(inboxes);
            let mut inboxes = crate::sync_lock::rw_write(&self.inboxes);
            inboxes
                .entry(agent_id)
                .or_insert_with(|| std::sync::RwLock::new(VecDeque::new()));
        }
    }

    /// Send a message to a specific agent.
    pub fn send(
        &self,
        sender: AgentId,
        receiver: AgentId,
        msg_type: A2AMessageType,
        payload: impl Into<String>,
    ) -> MessageId {
        let id = self.next_id();
        let msg = A2AMessage::new(id, sender, Some(receiver), msg_type, payload);

        // Deliver to receiver's inbox
        {
            let inboxes = crate::sync_lock::rw_read(&self.inboxes);
            if let Some(inbox_lock) = inboxes.get(&receiver) {
                let mut inbox = crate::sync_lock::rw_write(inbox_lock);
                if inbox.len() >= self.max_inbox_size {
                    inbox.pop_front(); // Drop oldest
                    self.dropped_messages.fetch_add(1, Ordering::Relaxed);
                }
                inbox.push_back(msg.clone());
            } else {
                drop(inboxes);
                let mut inboxes = crate::sync_lock::rw_write(&self.inboxes);
                let inbox_lock = inboxes
                    .entry(receiver)
                    .or_insert_with(|| std::sync::RwLock::new(VecDeque::new()));
                let mut inbox = crate::sync_lock::rw_write(inbox_lock);
                if inbox.len() >= self.max_inbox_size {
                    inbox.pop_front();
                    self.dropped_messages.fetch_add(1, Ordering::Relaxed);
                }
                inbox.push_back(msg.clone());
            }
        }

        // Audit trail
        crate::sync_lock::rw_write(&self.audit_trail).push(msg);

        tracing::debug!(
            from = %sender,
            to = %receiver,
            msg_id = %id,
            "A2A message sent"
        );

        id
    }

    /// Broadcast a message to all registered agents (except sender).
    pub fn broadcast(
        &self,
        sender: AgentId,
        msg_type: A2AMessageType,
        payload: impl Into<String>,
    ) -> MessageId {
        let id = self.next_id();
        let payload = payload.into();
        let msg = A2AMessage::new(id, sender, None, msg_type, payload);

        // Deliver to all inboxes except sender
        let agents: Vec<AgentId> = {
            let inboxes = crate::sync_lock::rw_read(&self.inboxes);
            inboxes.keys().copied().collect()
        };
        for agent_id in agents {
            if agent_id != sender {
                let inboxes = crate::sync_lock::rw_read(&self.inboxes);
                if let Some(inbox_lock) = inboxes.get(&agent_id) {
                    let mut inbox = crate::sync_lock::rw_write(inbox_lock);
                    if inbox.len() >= self.max_inbox_size {
                        inbox.pop_front();
                        self.dropped_messages.fetch_add(1, Ordering::Relaxed);
                    }
                    inbox.push_back(msg.clone());
                }
            }
        }

        crate::sync_lock::rw_write(&self.audit_trail).push(msg);
        id
    }

    /// Send to a group of agents.
    pub fn send_to_group(
        &self,
        sender: AgentId,
        receivers: &[AgentId],
        msg_type: A2AMessageType,
        payload: impl Into<String>,
    ) -> MessageId {
        let id = self.next_id();
        let payload = payload.into();

        for &receiver in receivers {
            let msg = A2AMessage::new(id, sender, Some(receiver), msg_type.clone(), &payload);
            let inboxes = crate::sync_lock::rw_read(&self.inboxes);
            if let Some(inbox_lock) = inboxes.get(&receiver) {
                let mut inbox = crate::sync_lock::rw_write(inbox_lock);
                if inbox.len() >= self.max_inbox_size {
                    inbox.pop_front();
                    self.dropped_messages.fetch_add(1, Ordering::Relaxed);
                }
                inbox.push_back(msg);
            }
        }

        let audit_msg = A2AMessage::new(id, sender, None, msg_type, payload);
        crate::sync_lock::rw_write(&self.audit_trail).push(audit_msg);
        id
    }

    /// Get unacknowledged messages for an agent, sorted by priority (highest first).
    pub fn inbox(&self, agent_id: AgentId) -> Vec<A2AMessage> {
        let inboxes = crate::sync_lock::rw_read(&self.inboxes);
        let mut msgs: Vec<_> = inboxes
            .get(&agent_id)
            .map(|inbox_lock| {
                let inbox = crate::sync_lock::rw_read(inbox_lock);
                inbox
                    .iter()
                    .filter(|m| {
                        if m.acknowledged {
                            return false;
                        }
                        if m.is_expired() {
                            return false;
                        }
                        true
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        // Sort descending by priority (Critical=3 > High=2 > Normal=1 > Low=0).
        msgs.sort_by(|a, b| b.priority.cmp(&a.priority));
        msgs
    }

    /// Get all messages for an agent (including acknowledged).
    pub fn inbox_all(&self, agent_id: AgentId) -> Vec<A2AMessage> {
        let inboxes = crate::sync_lock::rw_read(&self.inboxes);
        inboxes
            .get(&agent_id)
            .map(|inbox_lock| {
                let inbox = crate::sync_lock::rw_read(inbox_lock);
                inbox.iter().cloned().collect()
            })
            .unwrap_or_default()
    }

    /// Retrieve all messages in a specific thread, across all agents.
    pub fn messages_in_thread(&self, thread_id: &ThreadId) -> Vec<A2AMessage> {
        let audit = crate::sync_lock::rw_read(&self.audit_trail);
        let mut msgs: Vec<_> = audit
            .iter()
            .filter(|m| m.thread_id.as_ref() == Some(thread_id))
            .cloned()
            .collect();
        msgs.sort_by_key(|m| m.timestamp_ms);
        msgs
    }

    /// Retrieve an agent's inbox filtered to a specific thread.
    pub fn inbox_for_thread(&self, agent_id: AgentId, thread_id: &ThreadId) -> Vec<A2AMessage> {
        let inboxes = crate::sync_lock::rw_read(&self.inboxes);
        inboxes
            .get(&agent_id)
            .map(|inbox_lock| {
                let inbox = crate::sync_lock::rw_read(inbox_lock);
                inbox
                    .iter()
                    .filter(|m| m.thread_id.as_ref() == Some(thread_id) && !m.acknowledged)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Send a VCS-context-annotated message to an agent.
    /// This is the primary Phase 3A mechanism for sharing exact code state.
    pub fn send_with_vcs_context(
        &self,
        sender: AgentId,
        receiver: AgentId,
        msg_type: A2AMessageType,
        payload: impl Into<String>,
        vcs_context: VcsContext,
        priority: MessagePriority,
        thread_id: Option<ThreadId>,
    ) -> MessageId {
        let id = self.next_id();
        let msg = A2AMessage::new(id, sender, Some(receiver), msg_type, payload)
            .with_priority(priority)
            .with_vcs_context(vcs_context);
        let msg = if let Some(tid) = thread_id {
            msg.in_thread(tid)
        } else {
            msg
        };

        {
            let inboxes = crate::sync_lock::rw_read(&self.inboxes);
            if let Some(inbox_lock) = inboxes.get(&receiver) {
                let mut inbox = crate::sync_lock::rw_write(inbox_lock);
                if inbox.len() >= self.max_inbox_size {
                    inbox.pop_front();
                    self.dropped_messages.fetch_add(1, Ordering::Relaxed);
                }
                inbox.push_back(msg.clone());
            }
        }
        crate::sync_lock::rw_write(&self.audit_trail).push(msg);
        id
    }

    /// Send a conflict-detected notice (Critical priority, auto-annotated).
    pub fn send_conflict_notice(
        &self,
        sender: AgentId,
        receiver: AgentId,
        path: &str,
        snapshot_before: Option<u64>,
    ) -> MessageId {
        let ctx = VcsContext {
            snapshot_before,
            snapshot_after: None,
            touched_paths: vec![path.parse().unwrap_or_default()],
            change_id: None,
            op_id: None,
            content_hash: None,
        };
        self.send_with_vcs_context(
            sender,
            receiver,
            A2AMessageType::ConflictDetected,
            format!("Conflict detected on {path}"),
            ctx,
            MessagePriority::Critical,
            None,
        )
    }

    /// Acknowledge a message in an agent's inbox.
    pub fn acknowledge(&self, agent_id: AgentId, message_id: MessageId) -> bool {
        let inboxes = crate::sync_lock::rw_read(&self.inboxes);
        if let Some(inbox_lock) = inboxes.get(&agent_id) {
            let mut inbox = crate::sync_lock::rw_write(inbox_lock);
            if let Some(msg) = inbox.iter_mut().find(|m| m.id == message_id) {
                msg.acknowledged = true;
                return true;
            }
        }
        false
    }

    /// Get the audit trail (all messages ever sent).
    pub fn audit_trail(&self) -> Vec<A2AMessage> {
        crate::sync_lock::rw_read(&self.audit_trail).clone()
    }

    /// Get audit trail messages since a given timestamp.
    pub fn audit_since(&self, since_ms: u64) -> Vec<A2AMessage> {
        crate::sync_lock::rw_read(&self.audit_trail)
            .iter()
            .filter(|m| m.timestamp_ms >= since_ms)
            .cloned()
            .collect()
    }

    /// Count unacknowledged messages for an agent.
    pub fn unread_count(&self, agent_id: AgentId) -> usize {
        let inboxes = crate::sync_lock::rw_read(&self.inboxes);
        inboxes
            .get(&agent_id)
            .map(|inbox_lock| {
                let inbox = crate::sync_lock::rw_read(inbox_lock);
                inbox.iter().filter(|m| !m.acknowledged).count()
            })
            .unwrap_or(0)
    }

    /// Total messages in the audit trail.
    pub fn total_messages(&self) -> usize {
        crate::sync_lock::rw_read(&self.audit_trail).len()
    }

    /// Total count of dropped inbox messages due to per-agent inbox capacity.
    pub fn dropped_messages(&self) -> u64 {
        self.dropped_messages.load(Ordering::Relaxed)
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new(100)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_ids_strictly_increasing_for_correlation() {
        let bus = MessageBus::new(10);
        let a1 = AgentId(1);
        let a2 = AgentId(2);
        bus.register_agent(a1);
        bus.register_agent(a2);
        let id1 = bus.send(a1, a2, A2AMessageType::FreeForm, "a");
        let id2 = bus.send(a1, a2, A2AMessageType::FreeForm, "b");
        assert!(
            id2.0 > id1.0,
            "monotonic ids support delivery correlation / dedup policies"
        );
    }

    #[test]
    fn send_and_receive() {
        let bus = MessageBus::new(100);
        let a1 = AgentId(1);
        let a2 = AgentId(2);

        bus.register_agent(a1);
        bus.register_agent(a2);

        let id = bus.send(a1, a2, A2AMessageType::ProgressUpdate, "50% done");

        assert_eq!(bus.unread_count(a2), 1);
        assert_eq!(bus.unread_count(a1), 0);

        let inbox = bus.inbox(a2);
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].id, id);
        assert_eq!(inbox[0].payload, "50% done");
    }

    #[test]
    fn broadcast_reaches_all() {
        let bus = MessageBus::new(100);
        let a1 = AgentId(1);
        let a2 = AgentId(2);
        let a3 = AgentId(3);

        bus.register_agent(a1);
        bus.register_agent(a2);
        bus.register_agent(a3);

        bus.broadcast(a1, A2AMessageType::CompletionNotice, "Task done");

        // sender should NOT receive
        assert_eq!(bus.unread_count(a1), 0);
        // others should receive
        assert_eq!(bus.unread_count(a2), 1);
        assert_eq!(bus.unread_count(a3), 1);
    }

    #[test]
    fn acknowledge_marks_read() {
        let bus = MessageBus::new(100);
        let a1 = AgentId(1);
        let a2 = AgentId(2);

        bus.register_agent(a1);
        bus.register_agent(a2);

        let id = bus.send(a1, a2, A2AMessageType::HelpRequest, "Need help");
        assert_eq!(bus.unread_count(a2), 1);

        bus.acknowledge(a2, id);
        assert_eq!(bus.unread_count(a2), 0);
        // Still in inbox_all
        assert_eq!(bus.inbox_all(a2).len(), 1);
    }

    #[test]
    fn audit_trail() {
        let bus = MessageBus::new(100);
        let a1 = AgentId(1);
        let a2 = AgentId(2);

        bus.register_agent(a1);
        bus.register_agent(a2);

        bus.send(a1, a2, A2AMessageType::FreeForm, "hello");
        bus.send(a2, a1, A2AMessageType::FreeForm, "hi back");

        assert_eq!(bus.total_messages(), 2);
        assert_eq!(bus.audit_trail().len(), 2);
    }

    #[test]
    fn inbox_overflow() {
        let bus = MessageBus::new(2); // very small inbox
        let a1 = AgentId(1);
        let a2 = AgentId(2);

        bus.register_agent(a1);
        bus.register_agent(a2);

        bus.send(a1, a2, A2AMessageType::FreeForm, "msg1");
        bus.send(a1, a2, A2AMessageType::FreeForm, "msg2");
        bus.send(a1, a2, A2AMessageType::FreeForm, "msg3");

        // Only 2 messages should remain (oldest dropped)
        let inbox = bus.inbox_all(a2);
        assert_eq!(inbox.len(), 2);
        assert_eq!(inbox[0].payload, "msg2"); // msg1 was dropped
    }

    #[test]
    fn priority_sorted_inbox() {
        use crate::types::MessagePriority;
        let bus = MessageBus::new(100);
        let a1 = AgentId(1);
        let a2 = AgentId(2);
        bus.register_agent(a1);
        bus.register_agent(a2);

        // Send lower-priority first, then Critical.
        let id_low = bus.next_id();
        let low_msg = A2AMessage::new(id_low, a1, Some(a2), A2AMessageType::FreeForm, "low")
            .with_priority(MessagePriority::Low);
        {
            let mut inboxes = crate::sync_lock::rw_write(&bus.inboxes);
            let inbox_lock = inboxes
                .entry(a2)
                .or_insert_with(|| std::sync::RwLock::new(VecDeque::new()));
            crate::sync_lock::rw_write(inbox_lock).push_back(low_msg.clone());
        }
        crate::sync_lock::rw_write(&bus.audit_trail).push(low_msg);

        let id_crit = bus.next_id();
        let crit_msg = A2AMessage::new(
            id_crit,
            a1,
            Some(a2),
            A2AMessageType::ErrorReport,
            "critical!",
        )
        .with_priority(MessagePriority::Critical);
        {
            let mut inboxes = crate::sync_lock::rw_write(&bus.inboxes);
            let inbox_lock = inboxes
                .entry(a2)
                .or_insert_with(|| std::sync::RwLock::new(VecDeque::new()));
            crate::sync_lock::rw_write(inbox_lock).push_back(crit_msg.clone());
        }
        crate::sync_lock::rw_write(&bus.audit_trail).push(crit_msg);

        let inbox = bus.inbox(a2);
        assert_eq!(inbox.len(), 2);
        // Critical should come first.
        assert_eq!(inbox[0].priority, MessagePriority::Critical);
        assert_eq!(inbox[1].priority, MessagePriority::Low);
    }

    #[test]
    fn thread_message_grouping() {
        use crate::types::{MessagePriority, ThreadId, VcsContext};
        let bus = MessageBus::new(100);
        let a1 = AgentId(1);
        let a2 = AgentId(2);
        bus.register_agent(a1);
        bus.register_agent(a2);

        let thread = ThreadId::from("thread-abc");
        let ctx = VcsContext {
            snapshot_before: Some(1),
            snapshot_after: Some(2),
            touched_paths: vec!["src/parser.rs".parse().unwrap()],
            change_id: None,
            op_id: None,
            content_hash: None,
        };

        bus.send_with_vcs_context(
            a1,
            a2,
            A2AMessageType::ConflictDetected,
            "merge conflict on parser",
            ctx,
            MessagePriority::High,
            Some(thread.clone()),
        );
        // Send a message NOT in the thread.
        bus.send(a1, a2, A2AMessageType::FreeForm, "unrelated");

        let threaded = bus.messages_in_thread(&thread);
        assert_eq!(threaded.len(), 1);
        assert!(threaded[0].vcs_context.is_some());
        let ctx_back = threaded[0].vcs_context.as_ref().unwrap();
        assert_eq!(ctx_back.snapshot_before, Some(1));
    }

    #[test]
    fn conflict_notice_is_critical_priority() {
        let bus = MessageBus::new(100);
        let a1 = AgentId(1);
        let a2 = AgentId(2);
        bus.register_agent(a1);
        bus.register_agent(a2);

        bus.send_conflict_notice(a1, a2, "src/lib.rs", Some(42));
        let inbox = bus.inbox(a2);
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].msg_type, A2AMessageType::ConflictDetected);
        use crate::types::MessagePriority;
        assert_eq!(inbox[0].priority, MessagePriority::Critical);
    }
}
