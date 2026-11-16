//! Event-driven SCIENTIA WebSocket/SSE feed, bounded telemetry batcher, and replay ring.
//!
//! Replaces 5s `VoxDb::connect_default()` polling with:
//! - A bounded `mpsc(512)` [`TelemetryBatcher`] reusing the shared `ServerState.db` handle
//! - A bounded [`REPLAY_RING_MAX_LEN`] (`1024`) ring buffer of [`SequencedResearchEvent`]s
//!   for resumable `?from_seq=` SSE and `"research.session.delta"` WS replay with
//!   explicit `resync_required` signaling when a cursor predates the retained window.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::{Mutex, RwLock};
use serde_json::Value;
use tokio::sync::{Notify, broadcast, mpsc};
use vox_research_events::{ResearchEvent, ResearchEventSequencer, SequencedResearchEvent};

use crate::server_state::ServerState;

/// Canonical topic name for scientia queue changes.
pub(crate) const SCIENTIA_QUEUE_CHANGED: &str = "scientia.queue.changed";

/// Canonical topic name for newly-surfaced discovery candidates (inbox rows).
pub(crate) const SCIENTIA_DISCOVERY_SURFACED: &str = "scientia.discovery.surfaced";

/// Canonical topic name for sequenced live research session deltas.
pub(crate) const RESEARCH_SESSION_DELTA: &str = "research.session.delta";

/// Max inbox rows fetched (and broadcast) per refresh.
pub(crate) const DISCOVERY_BATCH_LIMIT: i64 = 64;

/// Quiet fallback interval for shared-DB queue/discovery snapshots when no event woke the loop.
pub(crate) const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Capacity of the topic broadcast channel.
pub(crate) const TOPIC_CHANNEL_CAPACITY: usize = 64;

/// Capacity of the research delta broadcast channel.
pub(crate) const DELTA_CHANNEL_CAPACITY: usize = 256;

/// Bounded capacity of the telemetry batcher `mpsc` channel.
pub(crate) const TELEMETRY_CHANNEL_CAPACITY: usize = 512;

/// Flush telemetry batch eagerly when buffered items reach this count.
pub(crate) const TELEMETRY_BATCH_SIZE: usize = 32;

/// Maximum interval before flushing a non-empty telemetry batch.
pub(crate) const TELEMETRY_FLUSH_INTERVAL: Duration = Duration::from_millis(250);

/// Maximum number of sequenced events retained in the replay ring buffer.
pub(crate) const REPLAY_RING_MAX_LEN: usize = 1024;

/// A topic-tagged payload broadcast to all WS connections; each connection
/// forwards it only if it has subscribed to `topic`.
#[derive(Debug, Clone)]
pub(crate) struct TopicMessage {
    pub topic: String,
    pub data: Value,
}

/// Result of querying the bounded replay ring for `seq > from_seq`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ReplayWindow {
    Events(Vec<SequencedResearchEvent>),
    ResyncRequired {
        oldest_available_seq: u64,
        latest_seq: u64,
    },
}

/// One buffered telemetry record destined for `agent_telemetry_flat`.
#[derive(Debug, Clone)]
pub(crate) struct TelemetryBatchItem {
    pub agent_id: String,
    pub session_id: String,
    pub repository_id: String,
    pub event_kind: String,
    pub provider: Option<String>,
    pub payload_json: Option<String>,
}

impl TelemetryBatchItem {
    pub(crate) fn from_research_event(event: &ResearchEvent, repository_id: &str) -> Self {
        let session_id = research_event_session_id(event).to_string();
        let event_kind = format!("{:?}", event.kind());
        let provider = match event {
            ResearchEvent::TelemetryObservation { provider, .. }
            | ResearchEvent::AggregateComputed { provider, .. } => Some(provider.clone()),
            ResearchEvent::SubqueryDispatched { provider_lane, .. } => Some(provider_lane.clone()),
            _ => None,
        };
        let payload_json = serde_json::to_string(event).ok();
        Self {
            agent_id: "scientia-feed".to_string(),
            session_id: if session_id.is_empty() {
                "global".to_string()
            } else {
                session_id
            },
            repository_id: repository_id.to_string(),
            event_kind,
            provider,
            payload_json,
        }
    }
}

/// Bounded `mpsc(512)` telemetry batcher that flushes to a shared `VoxDb` without per-event spawns.
#[derive(Clone)]
pub(crate) struct TelemetryBatcher {
    tx: mpsc::Sender<TelemetryBatchItem>,
    dropped: Arc<AtomicU64>,
    flushed: Arc<AtomicU64>,
}

impl TelemetryBatcher {
    pub(crate) fn new_with_receiver() -> (Self, mpsc::Receiver<TelemetryBatchItem>) {
        let (tx, rx) = mpsc::channel(TELEMETRY_CHANNEL_CAPACITY);
        (
            Self {
                tx,
                dropped: Arc::new(AtomicU64::new(0)),
                flushed: Arc::new(AtomicU64::new(0)),
            },
            rx,
        )
    }

    pub(crate) fn try_send(&self, item: TelemetryBatchItem) -> bool {
        match self.tx.try_send(item) {
            Ok(()) => true,
            Err(_) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    pub(crate) fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub(crate) fn flushed_count(&self) -> u64 {
        self.flushed.load(Ordering::Relaxed)
    }

    pub(crate) async fn run_flush_loop(
        mut rx: mpsc::Receiver<TelemetryBatchItem>,
        db: Option<Arc<vox_db::VoxDb>>,
        flushed: Arc<AtomicU64>,
    ) {
        let mut batch: Vec<TelemetryBatchItem> = Vec::with_capacity(TELEMETRY_BATCH_SIZE);
        let mut ticker = tokio::time::interval(TELEMETRY_FLUSH_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                maybe_item = rx.recv() => {
                    match maybe_item {
                        Some(item) => {
                            batch.push(item);
                            while batch.len() < TELEMETRY_BATCH_SIZE {
                                match rx.try_recv() {
                                    Ok(next) => batch.push(next),
                                    Err(_) => break,
                                }
                            }
                            if batch.len() >= TELEMETRY_BATCH_SIZE {
                                flush_telemetry_batch(db.as_deref(), &mut batch, &flushed).await;
                            }
                        }
                        None => {
                            if !batch.is_empty() {
                                flush_telemetry_batch(db.as_deref(), &mut batch, &flushed).await;
                            }
                            break;
                        }
                    }
                }
                _ = ticker.tick() => {
                    if !batch.is_empty() {
                        flush_telemetry_batch(db.as_deref(), &mut batch, &flushed).await;
                    }
                }
            }
        }
    }
}

async fn flush_telemetry_batch(
    db: Option<&vox_db::VoxDb>,
    batch: &mut Vec<TelemetryBatchItem>,
    flushed: &AtomicU64,
) {
    let items: Vec<TelemetryBatchItem> = std::mem::take(batch);
    let Some(db) = db else {
        return;
    };
    for item in &items {
        let res = db
            .insert_telemetry_flat_raw(
                &item.agent_id,
                &item.session_id,
                None,
                &item.repository_id,
                &item.event_kind,
                None,
                None,
                item.provider.as_deref(),
                None,
                None,
                None,
                None,
                item.payload_json.as_deref(),
            )
            .await;
        if res.is_ok() {
            flushed.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Shared research-event sequencer, bounded replay ring, delta broadcast, and telemetry batcher.
pub(crate) struct ResearchFeedHub {
    sequencer: ResearchEventSequencer,
    ring: RwLock<VecDeque<SequencedResearchEvent>>,
    ring_cap: usize,
    delta_tx: broadcast::Sender<SequencedResearchEvent>,
    topic_tx: broadcast::Sender<TopicMessage>,
    telemetry: TelemetryBatcher,
    telemetry_rx: Mutex<Option<mpsc::Receiver<TelemetryBatchItem>>>,
    queue_notify: Arc<Notify>,
    repository_id: String,
    bridge_started: AtomicBool,
}

impl ResearchFeedHub {
    pub(crate) fn new(topic_tx: broadcast::Sender<TopicMessage>, repository_id: String) -> Self {
        Self::with_ring_capacity(topic_tx, repository_id, REPLAY_RING_MAX_LEN)
    }

    pub(crate) fn with_ring_capacity(
        topic_tx: broadcast::Sender<TopicMessage>,
        repository_id: String,
        ring_cap: usize,
    ) -> Self {
        let (delta_tx, _) = broadcast::channel(DELTA_CHANNEL_CAPACITY);
        let (telemetry, telemetry_rx) = TelemetryBatcher::new_with_receiver();
        Self {
            sequencer: ResearchEventSequencer::new(),
            ring: RwLock::new(VecDeque::with_capacity(ring_cap.min(1024))),
            ring_cap: ring_cap.max(1),
            delta_tx,
            topic_tx,
            telemetry,
            telemetry_rx: Mutex::new(Some(telemetry_rx)),
            queue_notify: Arc::new(Notify::new()),
            repository_id,
            bridge_started: AtomicBool::new(false),
        }
    }

    pub(crate) fn subscribe_deltas(&self) -> broadcast::Receiver<SequencedResearchEvent> {
        self.delta_tx.subscribe()
    }

    pub(crate) fn oldest_seq(&self) -> Option<u64> {
        self.ring.read().front().map(|e| e.seq)
    }

    pub(crate) fn latest_seq(&self) -> u64 {
        self.sequencer.current_watermark()
    }

    pub(crate) fn telemetry(&self) -> &TelemetryBatcher {
        &self.telemetry
    }

    pub(crate) fn publish_research_event(
        &self,
        event: ResearchEvent,
        recorded_at_ms: i64,
    ) -> SequencedResearchEvent {
        let raw_session = research_event_session_id(&event);
        let session_id = if raw_session.is_empty() {
            "global".to_string()
        } else {
            raw_session.to_string()
        };
        let sequenced = self
            .sequencer
            .next_event(session_id, recorded_at_ms, event.clone());

        {
            let mut ring = self.ring.write();
            while ring.len() >= self.ring_cap {
                ring.pop_front();
            }
            ring.push_back(sequenced.clone());
        }

        let _ = self.delta_tx.send(sequenced.clone());
        let _ = self.topic_tx.send(TopicMessage {
            topic: RESEARCH_SESSION_DELTA.to_string(),
            data: serde_json::to_value(&sequenced).unwrap_or(Value::Null),
        });

        let item = TelemetryBatchItem::from_research_event(&event, &self.repository_id);
        let _ = self.telemetry.try_send(item);

        if matches!(
            event,
            ResearchEvent::FindingCandidateProposed { .. }
                | ResearchEvent::FindingApproved { .. }
                | ResearchEvent::PublicationAttempted { .. }
                | ResearchEvent::PublicationSucceeded { .. }
                | ResearchEvent::PublicationFailed { .. }
        ) {
            self.queue_notify.notify_one();
        }

        sequenced
    }

    /// Replay events for `session_id` with `seq > from_seq`, or signal `ResyncRequired`
    /// when `from_seq` precedes the oldest retained event (`oldest_seq - 1`) or exceeds `latest_seq`.
    pub(crate) fn replay_since(&self, session_id: &str, from_seq: Option<u64>) -> ReplayWindow {
        let ring = self.ring.read();
        let latest_seq = self.sequencer.current_watermark();
        let oldest_seq = ring.front().map(|e| e.seq);

        if let Some(cursor) = from_seq {
            if cursor > latest_seq {
                return ReplayWindow::ResyncRequired {
                    oldest_available_seq: oldest_seq.unwrap_or(latest_seq),
                    latest_seq,
                };
            }
            if let Some(oldest) = oldest_seq
                && oldest > 1
                && cursor < oldest.saturating_sub(1)
            {
                return ReplayWindow::ResyncRequired {
                    oldest_available_seq: oldest,
                    latest_seq,
                };
            }
        }

        let min_seq = from_seq.unwrap_or(0);
        let events = ring
            .iter()
            .filter(|ev| ev.seq > min_seq && (session_id.is_empty() || ev.session_id == session_id))
            .cloned()
            .collect();
        ReplayWindow::Events(events)
    }

    /// Lazily start background research bridge + telemetry batcher once inside a Tokio runtime.
    pub(crate) fn ensure_bridge_started(self: &Arc<Self>, server_state: &ServerState) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        if self
            .bridge_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            spawn_feed_workers_inner(server_state, Arc::clone(self));
        }
    }
}

/// Returns true when a message on `topic` should be forwarded to a connection
/// whose subscription set is `subscribed`.
pub(crate) fn should_forward(subscribed: &HashSet<String>, topic: &str) -> bool {
    subscribed.contains(topic)
}

/// Extract the primary session or correlation identifier for a [`ResearchEvent`].
#[must_use]
pub(crate) fn research_event_session_id(event: &ResearchEvent) -> &str {
    match event {
        ResearchEvent::TelemetryObservation { session_id, .. }
        | ResearchEvent::AggregateComputed { session_id, .. }
        | ResearchEvent::ClaimExtracted { session_id, .. }
        | ResearchEvent::ClaimVerified { session_id, .. }
        | ResearchEvent::NoveltyAssessed { session_id, .. }
        | ResearchEvent::FindingCandidateProposed { session_id, .. }
        | ResearchEvent::FindingApproved { session_id, .. }
        | ResearchEvent::WaveStarted { session_id, .. }
        | ResearchEvent::WaveCompleted { session_id, .. }
        | ResearchEvent::SubqueryDispatched { session_id, .. }
        | ResearchEvent::ContinuumEscalated { session_id, .. }
        | ResearchEvent::EpistemicDagDelta { session_id, .. } => session_id.as_str(),
        ResearchEvent::PreregistrationSubmitted { prereg_id, .. } => prereg_id.as_str(),
        ResearchEvent::CampaignStarted { campaign_id, .. }
        | ResearchEvent::CampaignAborted { campaign_id, .. } => campaign_id.as_str(),
        ResearchEvent::PublicationAttempted { manifest_id, .. }
        | ResearchEvent::PublicationSucceeded { manifest_id, .. }
        | ResearchEvent::PublicationFailed { manifest_id, .. }
        | ResearchEvent::RightOfReplyWindowOpened { manifest_id, .. }
        | ResearchEvent::RightOfReplyCleared { manifest_id, .. } => manifest_id.as_str(),
        ResearchEvent::RetractionEmitted { target_doi, .. } => target_doi.as_str(),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Spawn the background telemetry batcher and event-driven scientia feed workers,
/// reusing `server_state.db` (`Option<Arc<VoxDb>>`) with zero `VoxDb::connect_default()` calls.
pub(crate) fn spawn_scientia_feed_workers(server_state: &ServerState, hub: Arc<ResearchFeedHub>) {
    hub.ensure_bridge_started(server_state);
}

fn spawn_feed_workers_inner(server_state: &ServerState, hub: Arc<ResearchFeedHub>) {
    let db = server_state.db.clone();
    if let Some(rx) = hub.telemetry_rx.lock().take() {
        let flushed = Arc::clone(&hub.telemetry.flushed);
        let flush_db = db.clone();
        tokio::spawn(async move {
            TelemetryBatcher::run_flush_loop(rx, flush_db, flushed).await;
        });
    }

    let mut research_rx = server_state.research_events.subscribe();
    let topic_tx = hub.topic_tx.clone();
    let notify = Arc::clone(&hub.queue_notify);
    tokio::spawn(async move {
        let mut last_serialized: Option<String> = None;
        let mut last_max_inbox_id: i64 = match db.as_ref() {
            Some(shared_db) => shared_db.max_discovery_inbox_id().await.unwrap_or(0),
            None => 0,
        };
        let mut ticker = tokio::time::interval(POLL_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                recv_res = research_rx.recv() => {
                    match recv_res {
                        Ok(event) => {
                            hub.publish_research_event(event, now_ms());
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!(skipped, "scientia research_events receiver lagged");
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = notify.notified() => {
                    refresh_shared_db_topics(
                        db.as_deref(),
                        &topic_tx,
                        &mut last_max_inbox_id,
                        &mut last_serialized,
                    ).await;
                }
                _ = ticker.tick() => {
                    refresh_shared_db_topics(
                        db.as_deref(),
                        &topic_tx,
                        &mut last_max_inbox_id,
                        &mut last_serialized,
                    ).await;
                }
            }
        }
    });
}

async fn refresh_shared_db_topics(
    db: Option<&vox_db::VoxDb>,
    sender: &broadcast::Sender<TopicMessage>,
    last_max_inbox_id: &mut i64,
    last_serialized: &mut Option<String>,
) {
    let Some(db) = db else {
        return;
    };
    poll_discovery_surfaced(db, sender, last_max_inbox_id).await;
    match vox_scientia::review_flow::assemble_scientia_queue_from_db(db).await {
        Ok(snapshot) => {
            if let Ok(value) = serde_json::to_value(&snapshot) {
                let serialized = value.to_string();
                if last_serialized.as_deref() != Some(serialized.as_str()) {
                    *last_serialized = Some(serialized);
                    let _ = sender.send(TopicMessage {
                        topic: SCIENTIA_QUEUE_CHANGED.to_string(),
                        data: value,
                    });
                }
            }
        }
        Err(e) => {
            tracing::debug!("scientia queue refresh failed: {e}");
        }
    }
}

/// JSON shape broadcast for one inbox row on `scientia.discovery.surfaced`.
fn discovery_row_to_json(row: &vox_db::DiscoveryInboxRow) -> Value {
    serde_json::json!({
        "id": row.id,
        "publication_id": row.publication_id,
        "surfaced_at_ms": row.surfaced_at_ms,
        "intake_tier": row.intake_tier,
        "signal_codes": row.signal_codes,
        "acknowledged_at_ms": row.acknowledged_at_ms,
    })
}

async fn poll_discovery_surfaced(
    db: &vox_db::VoxDb,
    sender: &broadcast::Sender<TopicMessage>,
    last_max_inbox_id: &mut i64,
) {
    let current_max = match db.max_discovery_inbox_id().await {
        Ok(m) => m,
        Err(e) => {
            tracing::debug!("scientia discovery refresh: max id failed: {e}");
            return;
        }
    };
    if current_max <= *last_max_inbox_id {
        return;
    }
    let rows = match db
        .discoveries_since(*last_max_inbox_id, DISCOVERY_BATCH_LIMIT)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::debug!("scientia discovery refresh: discoveries_since failed: {e}");
            return;
        }
    };
    if rows.is_empty() {
        *last_max_inbox_id = current_max;
        return;
    }
    let data = Value::Array(rows.iter().map(discovery_row_to_json).collect());
    let new_max = rows.last().map_or(current_max, |r| r.id);
    let _ = sender.send(TopicMessage {
        topic: SCIENTIA_DISCOVERY_SURFACED.to_string(),
        data,
    });
    *last_max_inbox_id = new_max;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_forward_only_for_subscribed_topics() {
        let mut subs = HashSet::new();
        assert!(!should_forward(&subs, SCIENTIA_QUEUE_CHANGED));
        subs.insert(SCIENTIA_QUEUE_CHANGED.to_string());
        assert!(should_forward(&subs, SCIENTIA_QUEUE_CHANGED));
        assert!(!should_forward(&subs, "some.other.topic"));
    }

    #[test]
    fn discovery_surfaced_topic_is_independent() {
        let mut subs = HashSet::new();
        subs.insert(SCIENTIA_DISCOVERY_SURFACED.to_string());
        assert!(should_forward(&subs, SCIENTIA_DISCOVERY_SURFACED));
        assert!(!should_forward(&subs, SCIENTIA_QUEUE_CHANGED));
        assert_ne!(SCIENTIA_DISCOVERY_SURFACED, SCIENTIA_QUEUE_CHANGED);
    }

    #[tokio::test]
    async fn discovery_poller_broadcasts_new_inbox_rows_once() {
        use vox_db::{DbConfig, VoxDb};

        let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
        let (tx, mut rx) = broadcast::channel::<TopicMessage>(TOPIC_CHANNEL_CAPACITY);

        let mut last_max = db.max_discovery_inbox_id().await.expect("max");
        assert_eq!(last_max, 0);

        let id = db
            .insert_discovery_inbox(
                "commit-feed",
                1_234,
                "review_suggested",
                r#"["perf_claim"]"#,
            )
            .await
            .expect("insert");

        poll_discovery_surfaced(&db, &tx, &mut last_max).await;
        assert_eq!(last_max, id);

        let msg = rx.recv().await.expect("subscriber receives message");
        assert_eq!(msg.topic, SCIENTIA_DISCOVERY_SURFACED);
        let arr = msg.data.as_array().expect("payload is an array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["publication_id"], "commit-feed");
        assert_eq!(arr[0]["id"], id);

        poll_discovery_surfaced(&db, &tx, &mut last_max).await;
        assert!(rx.try_recv().is_err(), "no new rows -> no broadcast");
    }

    #[tokio::test]
    async fn subscriber_receives_matching_topic_message() {
        let (tx, _rx0) = broadcast::channel::<TopicMessage>(TOPIC_CHANNEL_CAPACITY);
        let mut rx = tx.subscribe();
        tx.send(TopicMessage {
            topic: SCIENTIA_QUEUE_CHANGED.to_string(),
            data: serde_json::json!({ "depth": 3 }),
        })
        .expect("send should succeed with a live subscriber");
        let msg = rx.recv().await.expect("subscriber should receive message");

        let mut subs = HashSet::new();
        subs.insert(SCIENTIA_QUEUE_CHANGED.to_string());
        assert!(should_forward(&subs, &msg.topic));
        assert_eq!(msg.data, serde_json::json!({ "depth": 3 }));

        let empty = HashSet::new();
        assert!(!should_forward(&empty, &msg.topic));
    }

    #[tokio::test]
    async fn telemetry_batcher_bounds_at_512_and_flushes_to_shared_db() {
        let db = Arc::new(
            vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
                .await
                .expect("open db"),
        );
        let (batcher, rx) = TelemetryBatcher::new_with_receiver();

        // Fill up to 512 without draining, then verify overflow drops and increments counter.
        for i in 0..TELEMETRY_CHANNEL_CAPACITY {
            let evt = ResearchEvent::WaveStarted {
                session_id: format!("s-{i}"),
                wave: 1,
                max_waves: 3,
                tier: "deep".into(),
                subquery_count: 2,
            };
            assert!(
                batcher.try_send(TelemetryBatchItem::from_research_event(&evt, "repo-1")),
                "queue capacity 512 should accept item {i}"
            );
        }
        let overflow_evt = ResearchEvent::WaveCompleted {
            session_id: "s-overflow".into(),
            wave: 1,
            new_sources: 1,
            verified_claims: 1,
            stability_score: 0.9,
        };
        assert!(!batcher.try_send(TelemetryBatchItem::from_research_event(
            &overflow_evt,
            "repo-1"
        )));
        assert_eq!(batcher.dropped_count(), 1);

        let flushed = Arc::clone(&batcher.flushed);
        let flushed_check = Arc::clone(&batcher.flushed);
        let db_clone = Arc::clone(&db);
        let worker = tokio::spawn(async move {
            TelemetryBatcher::run_flush_loop(rx, Some(db_clone), flushed).await;
        });
        drop(batcher);
        worker.await.unwrap();

        assert_eq!(
            flushed_check.load(Ordering::Relaxed),
            TELEMETRY_CHANNEL_CAPACITY as u64,
            "all 512 queued items must flush on channel close"
        );
        let rows = db
            .query_all(
                "SELECT COUNT(*) FROM agent_telemetry_flat WHERE agent_id = 'scientia-feed'",
                (),
            )
            .await
            .unwrap();
        let count: i64 = rows[0].get(0).unwrap();
        assert_eq!(count, TELEMETRY_CHANNEL_CAPACITY as i64);
    }

    #[test]
    fn replay_since_returns_deltas_and_emits_resync_required_when_behind_ring() {
        let (topic_tx, _) = broadcast::channel(TOPIC_CHANNEL_CAPACITY);
        let hub = ResearchFeedHub::with_ring_capacity(topic_tx, "repo-1".into(), 4);

        for i in 1..=6u8 {
            let evt = ResearchEvent::WaveStarted {
                session_id: if i % 2 == 0 { "s-even" } else { "s-odd" }.into(),
                wave: i,
                max_waves: 5,
                tier: "deep".into(),
                subquery_count: 2,
            };
            hub.publish_research_event(evt, 1_000 + i as i64);
        }

        // Ring capacity is 4, so seqs 3..=6 are retained (oldest_available_seq = 3, latest_seq = 6).
        assert_eq!(hub.oldest_seq(), Some(3));
        assert_eq!(hub.latest_seq(), 6);

        // Cursor 1 predates oldest_seq - 1 (2) -> resync_required.
        assert_eq!(
            hub.replay_since("s-even", Some(1)),
            ReplayWindow::ResyncRequired {
                oldest_available_seq: 3,
                latest_seq: 6,
            }
        );

        // Cursor 2 equals oldest_seq - 1 -> exact contiguous replay (seqs 4, 6 for "s-even").
        match hub.replay_since("s-even", Some(2)) {
            ReplayWindow::Events(evts) => {
                let seqs: Vec<u64> = evts.iter().map(|e| e.seq).collect();
                assert_eq!(seqs, vec![4, 6]);
            }
            other => panic!("expected Events, got {other:?}"),
        }

        // Cursor ahead of watermark (99 > 6) -> resync_required.
        assert_eq!(
            hub.replay_since("s-even", Some(99)),
            ReplayWindow::ResyncRequired {
                oldest_available_seq: 3,
                latest_seq: 6,
            }
        );
    }
}
