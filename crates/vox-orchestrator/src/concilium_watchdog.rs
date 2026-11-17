//! Deterministic Zero-Token Rust Watchdog for Vox Concilium (`CONC-04`).
//!
//! Evaluates `CircuitBreakerState` (5-signal doom-loop detector), lease TTL staleness,
//! `AttentionBudget` exhaustion, and `FileAffinityMap` lock contention completely in Rust
//! at `0` LLM token cost. Appends structured `#seq` system alerts onto `BulletinBoard` (`#ops`)
//! and emits `@monitor` ONLY when deterministic recovery thresholds trip.

use serde::{Deserialize, Serialize};
use vox_orchestrator_types::concilium::{
    ComputeTier, FileLockCellDto, SequenceBoardEntry, SwarmWorkspaceMode,
};

use crate::attention::AttentionBudget;
use crate::bulletin::BulletinBoard;
use crate::circuit_breaker::{
    AlarmTier, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState, TripReason,
};

/// Canonical sender name for zero-token Rust watchdog alerts.
pub const WATCHDOG_SENDER: &str = "concilium-watchdog";
/// Canonical `#channel` where watchdog telemetry and recovery alerts land.
pub const WATCHDOG_CHANNEL: &str = "#ops";

/// Severity classification produced by a single watchdog evaluation tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchdogSeverity {
    Healthy,
    Caution,
    Warning,
    Tripped,
}

/// Per-worker telemetry sample inspected by the deterministic Rust watchdog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogWorkerSample {
    pub agent_id: String,
    pub roster_name: String,
    pub breaker_state: CircuitBreakerState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_remaining_ms: Option<u64>,
    pub compute_tier: ComputeTier,
    pub workspace_mode: SwarmWorkspaceMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_lock_prefix: Option<String>,
}

/// Complete input bundle for a deterministic watchdog tick (`CONC-04`).
#[derive(Debug, Clone, Default)]
pub struct WatchdogTickInput {
    pub breaker_config: CircuitBreakerConfig,
    pub workers: Vec<WatchdogWorkerSample>,
    pub attention_budget: Option<AttentionBudget>,
    pub lock_cells: Vec<FileLockCellDto>,
    /// Minimum waiter count on an exclusive lock to trigger `@monitor` recovery (defaults to `1`).
    pub lock_contention_trip_waiters: usize,
}

/// Structured evaluation report from [`evaluate_watchdog_pure`] / [`evaluate_watchdog_tick`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchdogAlertReport {
    pub severity: WatchdogSeverity,
    pub trip_reason: Option<TripReason>,
    pub alarm_tier: AlarmTier,
    pub wake_monitor: bool,
    pub primary_agent: Option<String>,
    pub file_lock_prefix: Option<String>,
    pub receipt_badge: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub posted_entry: Option<SequenceBoardEntry>,
}

/// Phase 454 (`CONC-04`): Pure, zero-I/O evaluation of swarm health signals.
///
/// Returns `None` when all signals are healthy (`AlarmTier::None`, no expired leases,
/// no contended exclusive locks, and attention budget not exhausted).
#[must_use]
pub fn evaluate_watchdog_pure(input: &WatchdogTickInput) -> Option<WatchdogAlertReport> {
    let breaker = CircuitBreaker::new(input.breaker_config.clone());
    let waiter_threshold = input.lock_contention_trip_waiters.max(1);

    // 1. Check hard circuit-breaker trip or expired execution lease (`lease_remaining_ms == Some(0)`).
    for worker in &input.workers {
        let display_name = {
            let trimmed = worker.roster_name.trim().trim_start_matches('@');
            if trimmed.is_empty() {
                worker.agent_id.trim().trim_start_matches('@')
            } else {
                trimmed
            }
        };
        if let Some(reason) = breaker.should_trip(&worker.breaker_state) {
            let escalate = breaker.should_escalate(&worker.breaker_state);
            let badge = if escalate {
                format!("TRIP ESCALATE · {reason}")
            } else {
                format!("TRIP · {reason}")
            };
            let body = format!(
                "@monitor [watchdog:trip] worker @{display_name} tripped circuit breaker ({reason}; replan_attempts={})",
                worker.breaker_state.replan_attempts,
            );
            return Some(WatchdogAlertReport {
                severity: WatchdogSeverity::Tripped,
                trip_reason: Some(reason),
                alarm_tier: AlarmTier::Warning,
                wake_monitor: true,
                primary_agent: Some(worker.roster_name.clone()),
                file_lock_prefix: worker.file_lock_prefix.clone(),
                receipt_badge: badge,
                body,
                posted_entry: None,
            });
        }

        if worker.lease_remaining_ms == Some(0) {
            let body = format!(
                "@monitor [watchdog:lease-expired] worker @{display_name} lease TTL expired (0ms remaining)",
            );
            return Some(WatchdogAlertReport {
                severity: WatchdogSeverity::Tripped,
                trip_reason: Some(TripReason::NoProgress),
                alarm_tier: AlarmTier::Warning,
                wake_monitor: true,
                primary_agent: Some(worker.roster_name.clone()),
                file_lock_prefix: worker.file_lock_prefix.clone(),
                receipt_badge: "LEASE EXPIRED".to_string(),
                body,
                posted_entry: None,
            });
        }
    }

    // 2. Check exclusive `FileAffinityMap` lock contention (`contended == true` or waiters >= threshold).
    for cell in &input.lock_cells {
        if cell.exclusive && (cell.contended || cell.waiters.len() >= waiter_threshold) {
            let holder = cell.holder_agent.trim().trim_start_matches('@');
            let body = if cell.waiters.is_empty() {
                format!(
                    "@monitor [watchdog:lock-stall] exclusive deadlock contention on {} held by @{holder}",
                    cell.prefix,
                )
            } else {
                format!(
                    "@monitor [watchdog:lock-stall] exclusive lock on {} held by @{holder} blocking {} waiter(s): {}",
                    cell.prefix,
                    cell.waiters.len(),
                    cell.waiters.join(", "),
                )
            };
            let badge = if cell.waiters.is_empty() {
                "LOCK DEADLOCK".to_string()
            } else {
                format!("LOCK CONTENTION ({})", cell.waiters.len())
            };
            return Some(WatchdogAlertReport {
                severity: WatchdogSeverity::Tripped,
                trip_reason: None,
                alarm_tier: AlarmTier::Warning,
                wake_monitor: true,
                primary_agent: Some(cell.holder_agent.clone()),
                file_lock_prefix: Some(cell.prefix.clone()),
                receipt_badge: badge,
                body,
                posted_entry: None,
            });
        }
    }

    // 3. Sub-trip graduated alarms (Caution / Warning) or pilot AttentionBudget exhaustion:
    //    log zero-token `#ops` telemetry WITHOUT mentioning `@monitor` (`0 tok/s` preserved).
    let mut highest_tier = AlarmTier::None;
    let mut highest_worker: Option<&WatchdogWorkerSample> = None;
    for worker in &input.workers {
        let tier = breaker.check_tier(&worker.breaker_state);
        if tier > highest_tier {
            highest_tier = tier;
            highest_worker = Some(worker);
        }
    }

    if let Some(worker) = highest_worker {
        let severity = match highest_tier {
            AlarmTier::Warning => WatchdogSeverity::Warning,
            AlarmTier::Caution => WatchdogSeverity::Caution,
            AlarmTier::None => WatchdogSeverity::Healthy,
        };
        let badge = match highest_tier {
            AlarmTier::Warning => "WATCHDOG WARNING",
            _ => "WATCHDOG CAUTION",
        };
        let body = format!(
            "[watchdog:{:?}] worker {} elevated (no_progress={}, same_error={}, tool_thrash={}) — 0 tok/s telemetry",
            severity,
            worker.roster_name.trim_start_matches('@'),
            worker.breaker_state.no_progress_loops,
            worker.breaker_state.same_error_loops,
            worker.breaker_state.tool_thrash_count,
        );
        return Some(WatchdogAlertReport {
            severity,
            trip_reason: None,
            alarm_tier: highest_tier,
            wake_monitor: false,
            primary_agent: Some(worker.roster_name.clone()),
            file_lock_prefix: worker.file_lock_prefix.clone(),
            receipt_badge: badge.to_string(),
            body,
            posted_entry: None,
        });
    }

    if let Some(ref budget) = input.attention_budget
        && budget.exhausted()
    {
        let body = format!(
            "[watchdog:attention-budget] pilot attention budget exhausted ({}ms / {}ms; focus={:?}) — 0 tok/s telemetry",
            budget.spent_ms,
            budget.max_attention_ms,
            budget.focus_depth(),
        );
        return Some(WatchdogAlertReport {
            severity: WatchdogSeverity::Caution,
            trip_reason: None,
            alarm_tier: AlarmTier::Caution,
            wake_monitor: false,
            primary_agent: None,
            file_lock_prefix: None,
            receipt_badge: "ATTENTION EXHAUSTED".to_string(),
            body,
            posted_entry: None,
        });
    }

    None
}

/// Phase 454 (`CONC-04`): Evaluate swarm health and append a zero-token `#seq` alert onto `board`
/// when non-healthy.
pub fn evaluate_watchdog_tick(
    board: &BulletinBoard,
    input: &WatchdogTickInput,
) -> Option<WatchdogAlertReport> {
    let mut report = evaluate_watchdog_pure(input)?;
    let entry = board.post_board_entry(
        WATCHDOG_CHANNEL,
        WATCHDOG_SENDER,
        &report.body,
        ComputeTier::InlineCpu,
        SwarmWorkspaceMode::Inherit,
        report.file_lock_prefix.clone(),
        None,
        Some(report.receipt_badge.clone()),
        true,
    );
    report.posted_entry = Some(entry);
    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_worker(name: &str, state: CircuitBreakerState) -> WatchdogWorkerSample {
        WatchdogWorkerSample {
            agent_id: name.to_string(),
            roster_name: name.to_string(),
            breaker_state: state,
            lease_remaining_ms: Some(30_000),
            compute_tier: ComputeTier::LocalMensGpu,
            workspace_mode: SwarmWorkspaceMode::Share,
            file_lock_prefix: Some("crates/vox-orchestrator/".to_string()),
        }
    }

    #[tokio::test]
    async fn caution_alert_posts_zero_token_telemetry_without_waking_monitor() {
        let board = BulletinBoard::new(16);
        let monitor_doorbell = board.doorbell_for("monitor");
        let _ = board.park_worker("monitor", "monitor", Some("#ops"));

        let input = WatchdogTickInput {
            breaker_config: CircuitBreakerConfig::default(),
            workers: vec![sample_worker(
                "parser",
                CircuitBreakerState {
                    no_progress_loops: 1, // Caution threshold
                    ..Default::default()
                },
            )],
            attention_budget: None,
            lock_cells: Vec::new(),
            lock_contention_trip_waiters: 1,
        };

        let report = evaluate_watchdog_tick(&board, &input).expect("caution alert expected");
        assert_eq!(report.severity, WatchdogSeverity::Caution);
        assert!(!report.wake_monitor);
        assert!(!report.body.contains("@monitor"));
        let entry = report.posted_entry.expect("posted entry");
        assert_eq!(entry.channel, "#ops");
        assert_eq!(entry.compute_tier, ComputeTier::InlineCpu);
        assert_eq!(entry.is_watchdog_alert, Some(true));
        assert!(entry.mentions.is_empty());

        // Monitor must remain parked at 0 tok/s.
        let snap = board.concilium_snapshot();
        assert_eq!(snap.parked_workers, 1);
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(20),
                monitor_doorbell.notified()
            )
            .await
            .is_err(),
            "caution telemetry must NOT wake @monitor"
        );
    }

    #[tokio::test]
    async fn circuit_trip_and_lock_stall_wake_monitor_via_seq_doorbell() {
        let board = BulletinBoard::new(16);
        let monitor_doorbell = board.doorbell_for("monitor");
        let _ = board.park_worker("monitor", "monitor", Some("#ops"));

        let trip_input = WatchdogTickInput {
            breaker_config: CircuitBreakerConfig::default(),
            workers: vec![sample_worker(
                "parser",
                CircuitBreakerState {
                    no_progress_loops: 3, // Trip threshold
                    ..Default::default()
                },
            )],
            attention_budget: None,
            lock_cells: Vec::new(),
            lock_contention_trip_waiters: 1,
        };

        let report = evaluate_watchdog_tick(&board, &trip_input).expect("trip alert expected");
        assert_eq!(report.severity, WatchdogSeverity::Tripped);
        assert_eq!(report.trip_reason, Some(TripReason::NoProgress));
        assert!(report.wake_monitor);
        assert!(report.body.contains("@monitor"));

        tokio::time::timeout(vox_config::timeouts::D_100MS, monitor_doorbell.notified())
            .await
            .expect("trip alert must wake @monitor via doorbell");

        // Re-park monitor and test exclusive lock contention stall wakeup.
        let _ = board.park_worker("monitor", "monitor", Some("#ops"));
        let lock_input = WatchdogTickInput {
            breaker_config: CircuitBreakerConfig::default(),
            workers: Vec::new(),
            attention_budget: None,
            lock_cells: vec![FileLockCellDto {
                prefix: "crates/vox-db/src/store/".to_string(),
                holder_agent: "writer-a".to_string(),
                exclusive: true,
                compute_tier: ComputeTier::LocalMensGpu,
                workspace_mode: SwarmWorkspaceMode::Share,
                waiters: vec!["writer-b".to_string()],
                contended: true,
                acquired_seq: 1,
            }],
            lock_contention_trip_waiters: 1,
        };

        let lock_report = evaluate_watchdog_tick(&board, &lock_input).expect("lock stall alert");
        assert!(lock_report.wake_monitor);
        assert!(lock_report.body.contains("@monitor"));
        tokio::time::timeout(vox_config::timeouts::D_100MS, monitor_doorbell.notified())
            .await
            .expect("lock contention alert must wake @monitor");
    }

    #[test]
    fn deadlock_contended_cell_without_waiters_wakes_monitor() {
        let input = WatchdogTickInput {
            lock_cells: vec![FileLockCellDto {
                prefix: "crates/vox-orchestrator/src/".to_string(),
                holder_agent: "writer-a".to_string(),
                exclusive: true,
                compute_tier: ComputeTier::CloudBandit,
                workspace_mode: SwarmWorkspaceMode::Share,
                waiters: Vec::new(),
                contended: true,
                acquired_seq: 4,
            }],
            lock_contention_trip_waiters: 1,
            ..Default::default()
        };
        let report = evaluate_watchdog_pure(&input).expect("deadlock alert expected");
        assert!(report.wake_monitor);
        assert_eq!(report.receipt_badge, "LOCK DEADLOCK");
        assert!(report.body.contains("@monitor"));
    }
}
