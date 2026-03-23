use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::types::AgentId;
pub use crate::types::{A2AMessage, A2AMessageType, MessageId};
use crate::types::{MessagePriority, ThreadId, VcsContext};


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



/// Relay a message to another mesh node via HTTP.
pub async fn relay_to_mesh(
    client: &vox_mesh::http_client::MeshHttpClient,
    sender: AgentId,
    receiver: AgentId,
    msg_type: A2AMessageType,
    payload: impl Into<String>,
) -> Result<(), String> {
    client
        .relay_a2a(&vox_mesh::transport::A2ADeliverRequest {
            sender_agent_id: sender.0.to_string(),
            receiver_agent_id: receiver.0.to_string(),
            message_type: msg_type.to_string(),
            payload: payload.into(),
        })
        .await
        .map_err(|e: vox_mesh::MeshRegistryError| e.to_string())
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
                db.store().connection(),
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
    conn: &turso::Connection,
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

    conn.execute(
        "INSERT INTO a2a_messages (
            message_uuid, sender_agent, receiver_agent, msg_type, payload, priority, thread_id, repository_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            uuid.as_str(),
            sender.0.to_string(),
            receiver.0.to_string(),
            msg_type.into_str(),
            payload.as_str(),
            priority_val,
            thread_str,
            repository_id,
        ),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(uuid)
}

/// Poll for new unacknowledged messages for an agent from the database.
pub async fn poll_inbox_from_db(
    conn: &turso::Connection,
    agent_id: AgentId,
    repository_id: &str,
) -> Result<Vec<DbA2AMessage>, String> {
    let mut rows = conn
        .query(
            "SELECT id, message_uuid, sender_agent, receiver_agent, msg_type, payload, priority, thread_id, acknowledged, created_at, repository_id
             FROM a2a_messages
             WHERE receiver_agent = ?1 AND acknowledged = 0 AND repository_id = ?2
             ORDER BY priority DESC, created_at ASC",
            (agent_id.0.to_string(), repository_id),
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut msgs = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
        msgs.push(DbA2AMessage {
            id: row.get::<i64>(0).map_err(|e| e.to_string())? as u64,
            message_uuid: row.get(1).map_err(|e| e.to_string())?,
            sender_agent: row.get(2).map_err(|e| e.to_string())?,
            receiver_agent: row.get(3).map_err(|e| e.to_string())?,
            msg_type: row.get(4).map_err(|e| e.to_string())?,
            payload: row.get(5).map_err(|e| e.to_string())?,
            priority: row.get(6).map_err(|e| e.to_string())?,
            thread_id: row.get(7).map_err(|e| e.to_string())?,
            acknowledged: row.get::<i64>(8).map_err(|e| e.to_string())? != 0,
            created_at: row.get(9).map_err(|e| e.to_string())?,
            repository_id: row.get(10).map_err(|e| e.to_string())?,
        });
    }
    Ok(msgs)
}

/// Mark a message as acknowledged in the database.
pub async fn acknowledge_db_message(
    conn: &turso::Connection,
    message_uuid: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE a2a_messages SET acknowledged = 1 WHERE message_uuid = ?1",
        (message_uuid,),
    )
    .await
    .map_err(|e| e.to_string())
    .map(|_| ())
}

/// Remove old acknowledged messages from the database.
pub async fn prune_old_a2a_messages(
    conn: &turso::Connection,
    older_than_days: u32,
) -> Result<u64, String> {
    let result = conn.execute(
        "DELETE FROM a2a_messages WHERE acknowledged = 1 AND created_at < datetime('now', '-' || ?1 || ' days')",
        (older_than_days,),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(result as u64)
}

/// Routing hint for mesh messaging.
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
#[derive(Debug)]
pub struct MessageBus {
    /// Per-agent inboxes.
    pub(crate) inboxes: HashMap<AgentId, VecDeque<A2AMessage>>,
    /// Audit trail of all messages (most recent at back).
    audit_trail: Vec<A2AMessage>,
    /// ID generator.
    id_gen: AtomicU64,
    /// Maximum inbox size per agent before oldest messages are dropped.
    max_inbox_size: usize,
}

impl MessageBus {
    /// Create a new message bus.
    pub fn new(max_inbox_size: usize) -> Self {
        Self {
            inboxes: HashMap::new(),
            audit_trail: Vec::new(),
            id_gen: AtomicU64::new(1),
            max_inbox_size,
        }
    }

    pub(crate) fn next_id(&self) -> MessageId {
        MessageId(self.id_gen.fetch_add(1, Ordering::Relaxed))
    }

    /// Register an agent (creates their inbox).
    pub fn register_agent(&mut self, agent_id: AgentId) {
        self.inboxes.entry(agent_id).or_default();
    }

    /// Send a message to a specific agent.
    pub fn send(
        &mut self,
        sender: AgentId,
        receiver: AgentId,
        msg_type: A2AMessageType,
        payload: impl Into<String>,
    ) -> MessageId {
        let id = self.next_id();
        let msg = A2AMessage::new(id, sender, Some(receiver), msg_type, payload);

        // Deliver to receiver's inbox
        let inbox = self.inboxes.entry(receiver).or_default();
        if inbox.len() >= self.max_inbox_size {
            inbox.pop_front(); // Drop oldest
        }
        inbox.push_back(msg.clone());

        // Audit trail
        self.audit_trail.push(msg);

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
        &mut self,
        sender: AgentId,
        msg_type: A2AMessageType,
        payload: impl Into<String>,
    ) -> MessageId {
        let id = self.next_id();
        let payload = payload.into();
        let msg = A2AMessage::new(id, sender, None, msg_type, payload);

        // Deliver to all inboxes except sender
        let agents: Vec<AgentId> = self.inboxes.keys().copied().collect();
        for agent_id in agents {
            if agent_id != sender {
                let inbox = self.inboxes.entry(agent_id).or_default();
                if inbox.len() >= self.max_inbox_size {
                    inbox.pop_front();
                }
                inbox.push_back(msg.clone());
            }
        }

        self.audit_trail.push(msg);
        id
    }

    /// Send to a group of agents.
    pub fn send_to_group(
        &mut self,
        sender: AgentId,
        receivers: &[AgentId],
        msg_type: A2AMessageType,
        payload: impl Into<String>,
    ) -> MessageId {
        let id = self.next_id();
        let payload = payload.into();

        for &receiver in receivers {
            let msg = A2AMessage::new(id, sender, Some(receiver), msg_type.clone(), &payload);
            let inbox = self.inboxes.entry(receiver).or_default();
            if inbox.len() >= self.max_inbox_size {
                inbox.pop_front();
            }
            inbox.push_back(msg);
        }

        let audit_msg = A2AMessage::new(id, sender, None, msg_type, payload);
        self.audit_trail.push(audit_msg);
        id
    }

    /// Get unacknowledged messages for an agent, sorted by priority (highest first).
    pub fn inbox(&self, agent_id: AgentId) -> Vec<&A2AMessage> {
        let mut msgs: Vec<_> = self
            .inboxes
            .get(&agent_id)
            .map(|inbox| {
                inbox.iter().filter(|m| {
                    if m.acknowledged {
                        return false;
                    }
                    if m.elapsed_ms() > 300_000 {
                        // Message is stale (>5 min); skip silently — callers rely on freshness filtering.
                        let _ = m.id; // acknowledge the drop without tracing dep
                        return false;
                    }
                    true
                }).collect()
            })
            .unwrap_or_default();
        // Sort descending by priority (Critical=3 > High=2 > Normal=1 > Low=0).
        msgs.sort_by(|a, b| b.priority.cmp(&a.priority));
        msgs
    }

    /// Get all messages for an agent (including acknowledged).
    pub fn inbox_all(&self, agent_id: AgentId) -> Vec<&A2AMessage> {
        self.inboxes
            .get(&agent_id)
            .map(|inbox| inbox.iter().collect())
            .unwrap_or_default()
    }

    /// Retrieve all messages in a specific thread, across all agents.
    pub fn messages_in_thread(&self, thread_id: &ThreadId) -> Vec<&A2AMessage> {
        let mut msgs: Vec<_> = self
            .audit_trail
            .iter()
            .filter(|m| m.thread_id.as_ref() == Some(thread_id))
            .collect();
        msgs.sort_by_key(|m| m.timestamp_ms);
        msgs
    }

    /// Retrieve an agent's inbox filtered to a specific thread.
    pub fn inbox_for_thread(&self, agent_id: AgentId, thread_id: &ThreadId) -> Vec<&A2AMessage> {
        self.inboxes
            .get(&agent_id)
            .map(|inbox| {
                inbox
                    .iter()
                    .filter(|m| m.thread_id.as_ref() == Some(thread_id) && !m.acknowledged)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Send a VCS-context-annotated message to an agent.
    /// This is the primary Phase 3A mechanism for sharing exact code state.
    pub fn send_with_vcs_context(
        &mut self,
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

        let inbox = self.inboxes.entry(receiver).or_default();
        if inbox.len() >= self.max_inbox_size {
            inbox.pop_front();
        }
        inbox.push_back(msg.clone());
        self.audit_trail.push(msg);
        id
    }

    /// Send a conflict-detected notice (Critical priority, auto-annotated).
    pub fn send_conflict_notice(
        &mut self,
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
    pub fn acknowledge(&mut self, agent_id: AgentId, message_id: MessageId) -> bool {
        if let Some(inbox) = self.inboxes.get_mut(&agent_id) {
            if let Some(msg) = inbox.iter_mut().find(|m| m.id == message_id) {
                msg.acknowledged = true;
                return true;
            }
        }
        false
    }

    /// Get the audit trail (all messages ever sent).
    pub fn audit_trail(&self) -> &[A2AMessage] {
        &self.audit_trail
    }

    /// Get audit trail messages since a given timestamp.
    pub fn audit_since(&self, since_ms: u64) -> Vec<&A2AMessage> {
        self.audit_trail
            .iter()
            .filter(|m| m.timestamp_ms >= since_ms)
            .collect()
    }

    /// Count unacknowledged messages for an agent.
    pub fn unread_count(&self, agent_id: AgentId) -> usize {
        self.inboxes
            .get(&agent_id)
            .map(|inbox| inbox.iter().filter(|m| !m.acknowledged).count())
            .unwrap_or(0)
    }

    /// Total messages in the audit trail.
    pub fn total_messages(&self) -> usize {
        self.audit_trail.len()
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
    fn send_and_receive() {
        let mut bus = MessageBus::new(100);
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
        let mut bus = MessageBus::new(100);
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
        let mut bus = MessageBus::new(100);
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
        let mut bus = MessageBus::new(100);
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
        let mut bus = MessageBus::new(2); // very small inbox
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
        let mut bus = MessageBus::new(100);
        let a1 = AgentId(1);
        let a2 = AgentId(2);
        bus.register_agent(a1);
        bus.register_agent(a2);

        // Send lower-priority first, then Critical.
        let id_low = bus.next_id();
        let low_msg = A2AMessage::new(id_low, a1, Some(a2), A2AMessageType::FreeForm, "low")
            .with_priority(MessagePriority::Low);
        bus.inboxes
            .entry(a2)
            .or_default()
            .push_back(low_msg.clone());
        bus.audit_trail.push(low_msg);

        let id_crit = bus.next_id();
        let crit_msg = A2AMessage::new(
            id_crit,
            a1,
            Some(a2),
            A2AMessageType::ErrorReport,
            "critical!",
        )
        .with_priority(MessagePriority::Critical);
        bus.inboxes
            .entry(a2)
            .or_default()
            .push_back(crit_msg.clone());
        bus.audit_trail.push(crit_msg);

        let inbox = bus.inbox(a2);
        assert_eq!(inbox.len(), 2);
        // Critical should come first.
        assert_eq!(inbox[0].priority, MessagePriority::Critical);
        assert_eq!(inbox[1].priority, MessagePriority::Low);
    }

    #[test]
    fn thread_message_grouping() {
        use crate::types::{MessagePriority, ThreadId, VcsContext};
        let mut bus = MessageBus::new(100);
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
        let mut bus = MessageBus::new(100);
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
