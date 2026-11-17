//! `#seq` Shared Sequence Board, `0 tok/s` parked actors, and wakeup routing for [`BulletinBoard`].

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use tokio::sync::Notify;
use vox_orchestrator_types::concilium::{
    ComputeTier, ConciliumAgentLane, ConciliumSnapshotDto, ConciliumSpanBar, ConciliumSpanKind,
    ConciliumWakeupArc, SequenceBoardEntry, SequenceBoardFilter, SwarmWorkerStatus,
    SwarmWorkspaceMode,
};

use super::BulletinBoard;
use crate::types::now_unix_ms;

pub(super) const MAX_SEQUENCE_BOARD_ENTRIES: usize = 512;
const MAX_WAKEUP_ARCS: usize = 256;
pub(super) const MAX_LANE_SPANS: usize = 64;

#[derive(Default)]
pub(super) struct BulletinConciliumInner {
    pub(super) seq_gen: AtomicU64,
    pub(super) board: RwLock<VecDeque<SequenceBoardEntry>>,
    pub(super) lanes: RwLock<BTreeMap<String, ConciliumAgentLane>>,
    pub(super) parked_channels: RwLock<HashMap<String, String>>,
    pub(super) doorbells: RwLock<HashMap<String, Arc<Notify>>>,
    pub(super) wakeup_arcs: RwLock<VecDeque<ConciliumWakeupArc>>,
}

pub(super) fn default_lane(id: &str, roster: &str) -> ConciliumAgentLane {
    ConciliumAgentLane {
        agent_id: id.into(),
        roster_name: roster.into(),
        role: "worker".into(),
        status: SwarmWorkerStatus::Working,
        compute_tier: ComputeTier::CloudBandit,
        node_id: "cloud-bandit-router".into(),
        model_id: "claude-sonnet-4-6".into(),
        workspace_mode: SwarmWorkspaceMode::Inherit,
        file_lock_prefix: None,
        parked_since_seq: None,
        tok_per_sec: 36.0,
        used_tokens: 0,
        max_tokens: 200_000,
        kv_cache_hit_pct: None,
        current_action: String::new(),
        spans: Vec::new(),
    }
}

pub(super) fn push_lane_span(lane: &mut ConciliumAgentLane, bar: ConciliumSpanBar) {
    if lane.spans.len() >= MAX_LANE_SPANS {
        let excess = lane.spans.len() + 1 - MAX_LANE_SPANS;
        lane.spans.drain(..excess);
    }
    lane.spans.push(bar);
}

#[allow(clippy::too_many_arguments)]
fn span_bar(
    span_id: String,
    traceparent: String,
    kind: ConciliumSpanKind,
    label: String,
    detail: String,
    seq: u64,
    duration_ms: u64,
    file_lock_prefix: Option<String>,
    woken_by_seq: Option<u64>,
) -> ConciliumSpanBar {
    ConciliumSpanBar {
        span_id,
        traceparent,
        kind,
        label,
        detail: Some(detail),
        start_offset_ms: seq.saturating_mul(250),
        duration_ms,
        file_lock_prefix,
        woken_by_seq,
    }
}

impl BulletinBoard {
    /// Phase 454 (CONC-02): Park a worker at `0 tok/s` on `channel` and record its `#seq` watermark.
    pub fn park_worker(&self, agent_id: &str, roster_name: &str, channel: Option<&str>) -> u64 {
        let parked_since = self.head_seq();
        let key = agent_id.trim().trim_start_matches('@').to_string();
        let r_trim = roster_name.trim().trim_start_matches('@');
        let roster = if r_trim.is_empty() {
            key.clone()
        } else {
            r_trim.to_string()
        };
        let norm_ch = Self::normalize_channel(channel.unwrap_or("#main"));
        let _ = self.doorbell_for(&key);
        self.concilium
            .parked_channels
            .write()
            .insert(key.clone(), norm_ch.clone());
        let mut lanes = self.concilium.lanes.write();
        let sid = format!("{:016x}", now_unix_ms() ^ parked_since.wrapping_add(0xA5));
        let tp = format!("00-{:032x}-{sid}-01", now_unix_ms());
        let lane = lanes
            .entry(key.clone())
            .or_insert_with(|| default_lane(&key, &roster));
        if r_trim != key || lane.roster_name.is_empty() {
            lane.roster_name = roster;
        }
        lane.status = SwarmWorkerStatus::Parked;
        lane.tok_per_sec = 0.0;
        lane.parked_since_seq = Some(parked_since);
        lane.current_action = format!("parked on {norm_ch} since #seq-{parked_since} (0 tok/s)");
        let lock = lane.file_lock_prefix.clone();
        push_lane_span(
            lane,
            span_bar(
                sid,
                tp,
                ConciliumSpanKind::Parked,
                format!("0 tok/s · #seq-{parked_since}"),
                norm_ch,
                parked_since,
                500,
                lock,
                None,
            ),
        );
        parked_since
    }

    /// Phase 454 (CONC-02): Append an entry onto the `#seq` board and wake matching parked workers.
    #[allow(clippy::too_many_arguments)]
    pub fn post_board_entry(
        &self,
        channel: &str,
        sender: &str,
        body: &str,
        compute_tier: ComputeTier,
        workspace_mode: SwarmWorkspaceMode,
        file_lock_prefix: Option<String>,
        traceparent: Option<String>,
        receipt_badge: Option<String>,
        is_watchdog_alert: bool,
    ) -> SequenceBoardEntry {
        let seq = self.concilium.seq_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let norm_channel = Self::normalize_channel(channel);
        let timestamp_ms = now_unix_ms();
        let span_id = format!(
            "{:016x}",
            timestamp_ms.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ seq
        );
        let traceparent = traceparent
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                format!(
                    "00-{:016x}{:016x}-{span_id}-01",
                    timestamp_ms,
                    seq.wrapping_mul(0x517C_C1B7_2722_0A95)
                )
            });
        let mentions = Self::extract_mentions(body);
        let sender_key = sender.trim().trim_start_matches('@').to_string();
        let parked_since_seq = self
            .concilium
            .lanes
            .read()
            .get(&sender_key)
            .and_then(|l| l.parked_since_seq);
        let entry = SequenceBoardEntry {
            seq,
            channel: norm_channel,
            sender: sender_key,
            body: body.to_string(),
            mentions: mentions.clone(),
            timestamp_ms,
            traceparent: traceparent.clone(),
            span_id,
            compute_tier,
            file_lock_prefix,
            workspace_mode,
            parked_since_seq,
            receipt_badge,
            is_watchdog_alert: is_watchdog_alert.then_some(true),
        };
        {
            let mut board = self.concilium.board.write();
            if board.len() >= MAX_SEQUENCE_BOARD_ENTRIES {
                board.pop_front();
            }
            board.push_back(entry.clone());
        }
        if !mentions.is_empty() {
            self.dispatch_mention_wakeups(seq, timestamp_ms, &traceparent, &mentions);
        }
        entry
    }

    fn dispatch_mention_wakeups(&self, seq: u64, ts_ms: u64, tp: &str, mentions: &[String]) {
        let mut parked_chs = self.concilium.parked_channels.write();
        let mut lanes = self.concilium.lanes.write();
        let mut arcs = self.concilium.wakeup_arcs.write();
        let mut woken = Vec::new();
        for (key, lane) in lanes.iter_mut() {
            let parked_ch = parked_chs.get(key).map(String::as_str);
            let hit = mentions.iter().find(|m| {
                let raw = m.trim_start_matches('@');
                raw.eq_ignore_ascii_case("all")
                    || raw.eq_ignore_ascii_case(&lane.roster_name)
                    || raw.eq_ignore_ascii_case(&lane.agent_id)
                    || (raw.starts_with('#')
                        && parked_ch.is_some_and(|w_ch| Self::normalize_channel(raw) == w_ch))
            });
            if let Some(m) = hit {
                let w_span = format!("{:016x}", seq.wrapping_mul(0xBF58_476D) ^ ts_ms);
                lane.status = SwarmWorkerStatus::Working;
                lane.parked_since_seq = None;
                lane.tok_per_sec = 42.0;
                lane.current_action = format!("woken by #seq-{seq} ({m})");
                let lock = lane.file_lock_prefix.clone();
                push_lane_span(
                    lane,
                    span_bar(
                        w_span.clone(),
                        tp.into(),
                        ConciliumSpanKind::Thinking,
                        format!("woken by #seq-{seq}"),
                        m.clone(),
                        seq,
                        400,
                        lock,
                        Some(seq),
                    ),
                );
                if arcs.len() >= MAX_WAKEUP_ARCS {
                    arcs.pop_front();
                }
                arcs.push_back(ConciliumWakeupArc {
                    from_seq: seq,
                    target_agent: lane.roster_name.clone(),
                    target_span_id: w_span,
                    mention: m.clone(),
                });
                woken.push(key.clone());
            }
        }
        for key in &woken {
            parked_chs.remove(key);
        }
        drop((parked_chs, lanes, arcs));
        for (d_key, notify) in self.concilium.doorbells.read().iter() {
            if woken.iter().any(|k| k.eq_ignore_ascii_case(d_key))
                || mentions.iter().any(|m| {
                    let r = m.trim_start_matches('@');
                    r.eq_ignore_ascii_case("all") || r.eq_ignore_ascii_case(d_key)
                })
            {
                notify.notify_one();
            }
        }
    }

    /// Phase 454 (CONC-02): Read `#seq` board entries matching `filter` in ascending sequence order.
    #[must_use]
    pub fn read_board(&self, filter: &SequenceBoardFilter) -> Vec<SequenceBoardEntry> {
        let board = self.concilium.board.read();
        let ch_filter = filter
            .channel
            .as_deref()
            .map(str::trim)
            .filter(|s| {
                !s.is_empty() && !s.eq_ignore_ascii_case("#all") && !s.eq_ignore_ascii_case("all")
            })
            .map(Self::normalize_channel);
        let limit = filter
            .limit
            .unwrap_or(50)
            .clamp(1, MAX_SEQUENCE_BOARD_ENTRIES);
        let mut matched: Vec<SequenceBoardEntry> = board
            .iter()
            .filter(|e| {
                if let Some(ref ch) = ch_filter
                    && !e.channel.eq_ignore_ascii_case(ch)
                {
                    return false;
                }
                if let Some(ref seqs) = filter.seqs
                    && !seqs.is_empty()
                {
                    return seqs.contains(&e.seq);
                }
                filter.since_seq.is_none_or(|since| e.seq > since)
            })
            .cloned()
            .collect();
        if filter.since_seq.is_none() && filter.seqs.as_ref().is_none_or(Vec::is_empty) {
            if matched.len() > limit {
                matched.drain(..matched.len() - limit);
            }
        } else {
            matched.truncate(limit);
        }
        matched
    }

    /// Phase 454 (CONC-02): Snapshot all Concilium swimlane rows and `@mention` wakeup arcs.
    #[must_use]
    pub fn lanes_and_arcs(&self) -> (Vec<ConciliumAgentLane>, Vec<ConciliumWakeupArc>) {
        (
            self.concilium.lanes.read().values().cloned().collect(),
            self.concilium.wakeup_arcs.read().iter().cloned().collect(),
        )
    }

    /// Phase 454 (CONC-03): Assemble the current `ConciliumSnapshotDto` from the in-memory board.
    #[must_use]
    pub fn concilium_snapshot(&self) -> ConciliumSnapshotDto {
        let (lanes, wakeup_arcs) = self.lanes_and_arcs();
        let head_seq = self.head_seq();
        let active_workers = lanes
            .iter()
            .filter(|l| l.status == SwarmWorkerStatus::Working)
            .count();
        let parked_workers = lanes
            .iter()
            .filter(|l| l.status == SwarmWorkerStatus::Parked)
            .count();
        let active_locks = lanes
            .iter()
            .filter(|l| l.file_lock_prefix.is_some())
            .count();
        let board_tail = self.read_board(&SequenceBoardFilter::default());
        let now_playing_ticker = format!(
            "Head #seq-{head_seq} · {active_workers} active · {parked_workers} parked (0 tok/s)"
        );
        ConciliumSnapshotDto {
            head_seq,
            window_duration_ms: 10_000,
            active_workers,
            parked_workers,
            active_locks,
            now_playing_ticker,
            lanes,
            wakeup_arcs,
            board_tail,
        }
    }
}

#[cfg(test)]
#[path = "concilium_board_tests.rs"]
mod tests;
