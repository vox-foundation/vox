//! Reactive subscription manager for table-level change notifications.
//!
//! Uses `tokio::sync::broadcast` channels to notify subscribers when
//! a table's data has been mutated. This powers SSE-based reactive queries.
//!
//! Locks use [`tokio::sync::RwLock`] so callers in async contexts never block
//! the executor on contended `std::sync` primitives.
//!
//! # Architecture
//!
//! ```text
//!   @mutation insert_task()
//!       │
//!       ▼
//!   SubscriptionManager::notify("tasks")
//!       │
//!       ▼
//!   broadcast::Sender<()> ──► all Receivers for "tasks"
//!       │
//!       ▼
//!   SSE endpoint re-runs @query list_tasks()
//!       │
//!       ▼
//!   Client gets updated result
//! ```

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, broadcast};

/// Default capacity for broadcast channels per table.
const DEFAULT_CHANNEL_CAPACITY: usize = 64;
/// Bounded replay ring capacity per `Stream[T]` channel (`Last-Event-ID` support, `STREAM-01`).
const STREAM_REPLAY_CAPACITY: usize = 256;

/// Sequenced SSE event stored in the bounded replay buffer (`STREAM-01`, `CC-02`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SequencedStreamEvent {
    pub id: u64,
    pub channel: String,
    pub data: String,
}

/// Manages per-table broadcast channels for reactive query subscriptions.
///
/// When a `@mutation` commits, it calls [`SubscriptionManager::notify`] with the affected table names.
/// SSE subscription endpoints hold `Receiver` handles and re-run their queries
/// when notified.
///
/// **Two channel pools, two use cases:**
///   - `channels` (signal — `broadcast::Sender<()>`) drives reactive
///     table invalidations (`@mutation` commits → "something changed"
///     → SSE re-fetch by client).
///   - `payload_channels` (payload — `broadcast::Sender<String>`) drives
///     actor `broadcast(msg)` → `subscribe(Actor)` SSE wire delivery so
///     subscribers receive the actual message string. Kept separate so
///     the table-invalidation lane doesn't have to allocate a String
///     per notify call.
#[derive(Clone)]
pub struct SubscriptionManager {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<()>>>>,
    payload_channels: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
    replay_buffers: Arc<RwLock<HashMap<String, VecDeque<SequencedStreamEvent>>>>,
    next_event_id: Arc<AtomicU64>,
}

impl SubscriptionManager {
    /// Create a new, empty subscription manager.
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            payload_channels: Arc::new(RwLock::new(HashMap::new())),
            replay_buffers: Arc::new(RwLock::new(HashMap::new())),
            next_event_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Subscribe to an actor's payload-bearing broadcast channel.
    /// Returns a `broadcast::Receiver<String>` that yields each
    /// payload emitted via [`SubscriptionManager::notify_payload`].
    ///
    /// Channel keys are the actor's bare name (matches what the codegen
    /// emits at the `subscribe(Actor)` call site).
    pub async fn subscribe_payload(&self, channel: &str) -> broadcast::Receiver<String> {
        {
            let channels = self.payload_channels.read().await;
            if let Some(sender) = channels.get(channel) {
                return sender.subscribe();
            }
        }
        let mut channels = self.payload_channels.write().await;
        let sender = channels
            .entry(channel.to_string())
            .or_insert_with(|| broadcast::channel(DEFAULT_CHANNEL_CAPACITY).0);
        sender.subscribe()
    }

    /// Push a payload to every payload subscriber of `channel`.
    /// Called by the actor handler body's `broadcast(msg)` lowering.
    pub async fn notify_payload(&self, channel: &str, payload: String) {
        // Lazily create the channel if no one's subscribed yet — keeps
        // the broadcast lossless for any subscriber that joins later
        // (within the channel's buffered capacity).
        let needs_create = self.payload_channels.read().await.get(channel).is_none();
        if needs_create {
            let mut channels = self.payload_channels.write().await;
            channels
                .entry(channel.to_string())
                .or_insert_with(|| broadcast::channel(DEFAULT_CHANNEL_CAPACITY).0);
        }
        let channels = self.payload_channels.read().await;
        if let Some(sender) = channels.get(channel) {
            // Ignore send errors (no active receivers is fine — the
            // payload is dropped on the floor in that case).
            let _ = sender.send(payload);
        }
    }

    /// Number of active payload subscribers for a given channel.
    pub async fn payload_subscriber_count(&self, channel: &str) -> usize {
        self.payload_channels
            .read()
            .await
            .get(channel)
            .map(|s| s.receiver_count())
            .unwrap_or(0)
    }

    /// Subscribe to change notifications for a specific table.
    /// Returns a `broadcast::Receiver` that fires whenever the table is mutated.
    pub async fn subscribe(&self, table: &str) -> broadcast::Receiver<()> {
        {
            let channels = self.channels.read().await;
            if let Some(sender) = channels.get(table) {
                return sender.subscribe();
            }
        }

        let mut channels = self.channels.write().await;
        let sender = channels
            .entry(table.to_string())
            .or_insert_with(|| broadcast::channel(DEFAULT_CHANNEL_CAPACITY).0);
        sender.subscribe()
    }

    /// Notify all subscribers that a single table has been mutated.
    pub async fn notify(&self, table: &str) {
        let channels = self.channels.read().await;
        if let Some(sender) = channels.get(table) {
            let count = sender.receiver_count();
            tracing::debug!(
                table = table,
                subscribers = count,
                "subscription notification fired"
            );
            // Ignore send errors (no active receivers is fine)
            let _ = sender.send(());
        }
    }

    /// Notify all subscribers for multiple tables at once.
    /// Typically called after a `@mutation` commits.
    pub async fn notify_tables(&self, tables: &[&str]) {
        let channels = self.channels.read().await;
        for table in tables {
            if let Some(sender) = channels.get(*table) {
                let _ = sender.send(());
            }
        }
    }

    /// Subscribe to change notifications for multiple tables.
    /// Returns receivers for each table.
    pub async fn subscribe_tables(&self, tables: &[&str]) -> Vec<broadcast::Receiver<()>> {
        let mut out = Vec::with_capacity(tables.len());
        for t in tables {
            out.push(self.subscribe(t).await);
        }
        out
    }

    /// Number of active subscribers for a given table.
    pub async fn subscriber_count(&self, table: &str) -> usize {
        self.channels
            .read()
            .await
            .get(table)
            .map(|s| s.receiver_count())
            .unwrap_or(0)
    }

    /// Notify all subscribers for all tracked tables.
    pub async fn notify_all(&self) {
        let channels = self.channels.read().await;
        for sender in channels.values() {
            let _ = sender.send(());
        }
    }

    /// Remove all subscription channels (for graceful shutdown).
    pub async fn unsubscribe_all(&self) {
        self.channels.write().await.clear();
        self.replay_buffers.write().await.clear();
    }

    /// Publish a sequenced event for `Stream[T]` SSE endpoints with `Last-Event-ID` replay (`STREAM-01`).
    pub async fn publish_sequenced(&self, channel: &str, data: String) -> u64 {
        let id = self.next_event_id.fetch_add(1, Ordering::Relaxed);
        let ev = SequencedStreamEvent {
            id,
            channel: channel.to_string(),
            data: data.clone(),
        };
        {
            let mut buffers = self.replay_buffers.write().await;
            let ring = buffers
                .entry(channel.to_string())
                .or_insert_with(|| VecDeque::with_capacity(STREAM_REPLAY_CAPACITY));
            if ring.len() >= STREAM_REPLAY_CAPACITY {
                ring.pop_front();
            }
            ring.push_back(ev);
        }
        self.notify_payload(channel, data).await;
        id
    }

    /// Return buffered events with `id > last_event_id` for `Last-Event-ID` SSE reconnection (`STREAM-01`).
    pub async fn replay_since(
        &self,
        channel: &str,
        last_event_id: u64,
    ) -> Vec<SequencedStreamEvent> {
        let buffers = self.replay_buffers.read().await;
        buffers
            .get(channel)
            .map(|ring| {
                ring.iter()
                    .filter(|ev| ev.id > last_event_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B7: payload-bearing broadcast through SubscriptionManager — the
    /// channel must round-trip a real `String` to every subscriber.
    #[tokio::test]
    async fn payload_round_trips_to_subscribers() {
        let mgr = SubscriptionManager::new();
        let mut rx_a = mgr.subscribe_payload("ChatRoom").await;
        let mut rx_b = mgr.subscribe_payload("ChatRoom").await;
        assert_eq!(mgr.payload_subscriber_count("ChatRoom").await, 2);
        mgr.notify_payload("ChatRoom", "hello".to_string()).await;
        assert_eq!(rx_a.recv().await.unwrap(), "hello");
        assert_eq!(rx_b.recv().await.unwrap(), "hello");
    }

    /// Subscribers attached AFTER an early notify must still receive
    /// subsequent payloads — the channel is buffered, not edge-triggered.
    #[tokio::test]
    async fn late_subscriber_receives_subsequent_payloads() {
        let mgr = SubscriptionManager::new();
        mgr.notify_payload("ChatRoom", "early".to_string()).await;
        let mut rx = mgr.subscribe_payload("ChatRoom").await;
        mgr.notify_payload("ChatRoom", "late".to_string()).await;
        assert_eq!(rx.recv().await.unwrap(), "late");
    }

    /// Signal channel and payload channel use independent maps — sending
    /// on one MUST NOT spill into the other.
    #[tokio::test]
    async fn signal_and_payload_channels_are_independent() {
        let mgr = SubscriptionManager::new();
        let mut sig_rx = mgr.subscribe("ChatRoom").await;
        let mut payload_rx = mgr.subscribe_payload("ChatRoom").await;
        mgr.notify_payload("ChatRoom", "payload".to_string()).await;
        assert_eq!(payload_rx.recv().await.unwrap(), "payload");
        // Signal channel must NOT have fired.
        assert!(
            sig_rx.try_recv().is_err(),
            "signal channel must be independent"
        );
    }

    #[tokio::test]
    async fn test_subscribe_and_notify() {
        let mgr = SubscriptionManager::new();
        let mut rx = mgr.subscribe("tasks").await;

        mgr.notify("tasks").await;

        let result = tokio::time::timeout(vox_config::timeouts::D_100MS, rx.recv()).await;

        assert!(result.is_ok(), "should receive notification");
    }

    #[tokio::test]
    async fn test_no_notification_for_other_table() {
        let mgr = SubscriptionManager::new();
        let mut rx = mgr.subscribe("tasks").await;

        mgr.notify("users").await; // different table

        let result = tokio::time::timeout(vox_config::timeouts::D_50MS, rx.recv()).await;

        assert!(
            result.is_err(),
            "should NOT receive notification for unrelated table"
        );
    }

    #[tokio::test]
    async fn test_notify_tables_multiple() {
        let mgr = SubscriptionManager::new();
        let mut rx_tasks = mgr.subscribe("tasks").await;
        let mut rx_users = mgr.subscribe("users").await;

        mgr.notify_tables(&["tasks", "users"]).await;

        assert!(rx_tasks.recv().await.is_ok());
        assert!(rx_users.recv().await.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let mgr = SubscriptionManager::new();
        let mut rx1 = mgr.subscribe("tasks").await;
        let mut rx2 = mgr.subscribe("tasks").await;

        mgr.notify("tasks").await;

        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }

    #[tokio::test]
    async fn sequenced_stream_replays_after_last_event_id() {
        let mgr = SubscriptionManager::new();
        let id1 = mgr
            .publish_sequenced("job_progress", "{\"pct\":10}".into())
            .await;
        let id2 = mgr
            .publish_sequenced("job_progress", "{\"pct\":50}".into())
            .await;
        let id3 = mgr
            .publish_sequenced("job_progress", "{\"pct\":100}".into())
            .await;
        assert!(id1 < id2 && id2 < id3);

        let replayed = mgr.replay_since("job_progress", id1).await;
        assert_eq!(replayed.len(), 2);
        assert_eq!(replayed[0].id, id2);
        assert_eq!(replayed[0].data, "{\"pct\":50}");
        assert_eq!(replayed[1].id, id3);
        assert_eq!(replayed[1].data, "{\"pct\":100}");
    }
}
