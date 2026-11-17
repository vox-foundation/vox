//! Broadcast bulletin board and append-only `#seq` Shared Sequence Board (`CONC-02`).

use std::sync::Arc;

use tokio::sync::broadcast;

use crate::types::AgentMessage;

#[path = "bulletin/concilium_board.rs"]
mod concilium_board;
#[path = "bulletin/mentions.rs"]
mod mentions;
#[path = "bulletin/pilot_actions.rs"]
mod pilot_actions;

use concilium_board::BulletinConciliumInner;

const DEFAULT_BULLETIN_CAPACITY: usize = 256;

/// Cross-agent communication channel using broadcast pub/sub plus the
/// persistent `#seq` Shared Sequence Board and `0 tok/s` doorbell registry.
#[derive(Clone)]
pub struct BulletinBoard {
    sender: broadcast::Sender<AgentMessage>,
    concilium: Arc<BulletinConciliumInner>,
}

impl BulletinBoard {
    /// Phase 454 (CONC-02): Create a bulletin board with broadcast channel `capacity`.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            concilium: Arc::new(BulletinConciliumInner::default()),
        }
    }

    /// Phase 454 (CONC-02): Publish a broadcast [`AgentMessage`] to active subscribers.
    pub fn publish(&self, msg: AgentMessage) {
        let _ = self.sender.send(msg);
    }

    /// Phase 454 (CONC-02): Subscribe to live broadcast [`AgentMessage`] notifications.
    pub fn subscribe(&self) -> broadcast::Receiver<AgentMessage> {
        self.sender.subscribe()
    }

    /// Phase 454 (CONC-02): Return the current count of broadcast receivers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for BulletinBoard {
    fn default() -> Self {
        Self::new(DEFAULT_BULLETIN_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentId, TaskId};

    #[tokio::test]
    async fn publish_and_receive() {
        let board = BulletinBoard::new(16);
        let (mut rx1, mut rx2) = (board.subscribe(), board.subscribe());
        assert_eq!(board.subscriber_count(), 2);
        board.publish(AgentMessage::TaskCompleted {
            task_id: TaskId(1),
            agent_id: AgentId(1),
        });
        assert!(rx1.recv().await.is_ok() && rx2.recv().await.is_ok());
    }
}
