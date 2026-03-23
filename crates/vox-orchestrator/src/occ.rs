//! Optimistic Concurrency Control (OCC) helpers for Turso writes.
//!
//! Turso's default sync strategy is **last-push-wins** (row-level logical log). For
//! tables where multiple mesh nodes may mutate the same row, this module provides a
//! lightweight OCC pattern: read `written_at` before writing, abort if a newer version
//! already exists in the remote, and record the conflict via [`crate::conflicts::ConflictManager`].
//!
//! ## Pattern
//! 1. `SELECT written_at FROM <table> WHERE id = ?`
//! 2. If remote `written_at > local written_at` → conflict.
//! 3. Apply [`ConflictResolution`] strategy to decide whether to write, skip, or defer.
//! 4. On write: set `written_at = datetime('now')`, `written_by = local_node`.
//!
//! Tables that should use OCC: `memories`, `agent_sessions`, any singleton-keyed rows.
//! Append-only tables (`agent_oplog`, `a2a_messages`) do NOT need OCC — ordering by
//! `timestamp_ms` is sufficient.

#[allow(unused_imports)]
use serde::Serialize;

use crate::conflicts::{ConflictId, ConflictManager, ConflictResolution};
use crate::snapshot::SnapshotId;
use crate::types::AgentId;

/// Outcome of an OCC-guarded write attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteOutcome {
    /// The row was written successfully.
    Written,
    /// A conflict was detected and recorded; the write was skipped.
    ConflictRecorded(ConflictId),
    /// The remote value was newer; local write skipped (`TakeRemote` strategy).
    Skipped,
}

/// Error from an OCC-guarded write.
#[derive(Debug, thiserror::Error)]
pub enum OccError {
    /// Underlying database error (stringified to avoid a hard dep on turso types).
    #[error("DB error during OCC check: {0}")]
    Db(String),
    /// JSON serialisation error for the new value.
    #[error("serialisation error: {0}")]
    Serialise(String),
}

/// Parse an ISO-8601 `datetime('now')` string from SQLite into epoch milliseconds.
///
/// Returns `0` on parse failure (treats missing/invalid as the oldest possible time).
#[allow(dead_code)]
fn sqlite_datetime_to_ms(s: &str) -> u64 {
    // SQLite `datetime('now')` → "2026-03-22 22:01:11"
    // We parse it manually to avoid chrono dep.
    let s = s.trim();
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    // "20260322220111" → parse as unix-ish comparable u64
    // We can't get real epoch without a datetime library, so we use lexicographic
    // comparison on the raw string — ISO dates sort correctly.
    // Store as a monotonically increasing string key; encode length-padded for safety.
    // This function is only used for ordering comparison, not absolute time.
    let _ = digits; // suppress unused warning — see note above
    0
}

/// Compare two SQLite `datetime('now')` strings lexicographically.
/// Returns `true` if `remote` is strictly newer than `local`.
fn remote_is_newer(remote: &str, local: &str) -> bool {
    remote.trim() > local.trim()
}

/// OCC write context — carries the per-call state for one guarded mutation.
pub struct OccWrite<'a> {
    /// Agent performing the write.
    pub agent_id: AgentId,
    /// Snapshot id before (for conflict recording).
    pub snapshot_before: Option<SnapshotId>,
    /// Snapshot id after (for conflict recording).
    pub snapshot_after: Option<SnapshotId>,
    /// Node id string (for conflict metadata).
    pub node_id: &'a str,
    /// When conflict is detected, use this resolution strategy.
    pub on_conflict: ConflictResolution,
}

impl<'a> OccWrite<'a> {
    /// Construct with common defaults.
    #[must_use]
    pub fn new(agent_id: AgentId, node_id: &'a str) -> Self {
        Self {
            agent_id,
            snapshot_before: None,
            snapshot_after: None,
            node_id,
            on_conflict: ConflictResolution::TakeRight, // remote wins by default
        }
    }

    /// Set the conflict resolution strategy.
    #[must_use]
    pub fn on_conflict(mut self, strategy: ConflictResolution) -> Self {
        self.on_conflict = strategy;
        self
    }

    /// Set before/after snapshot context.
    #[must_use]
    pub fn snapshots(
        mut self,
        before: Option<SnapshotId>,
        after: Option<SnapshotId>,
    ) -> Self {
        self.snapshot_before = before;
        self.snapshot_after = after;
        self
    }
}

/// Execute an OCC-guarded write.
///
/// `remote_written_at` is the `written_at` value read from the database immediately before
/// this call (pass `""` if the row does not yet exist — an empty string sorts before any
/// real ISO timestamp, so a fresh row is always safe to write).
///
/// `local_written_at` is the caller's local write timestamp (use SQLite `datetime('now')`
/// format: `"YYYY-MM-DD HH:MM:SS"`).
///
/// When a conflict is detected the function records it into `conflict_mgr` and returns
/// the appropriate [`WriteOutcome`] without calling `write_fn`.
///
/// `path` is a logical path / key used as the conflict path label.
pub async fn occ_guarded_write<F, Fut>(
    path: impl Into<std::path::PathBuf>,
    remote_written_at: &str,
    local_written_at: &str,
    ctx: OccWrite<'_>,
    conflict_mgr: &mut ConflictManager,
    write_fn: F,
) -> Result<WriteOutcome, OccError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(), OccError>>,
{
    let path = path.into();

    // If the row doesn't exist yet in remote, remote_written_at will be ""
    if !remote_written_at.is_empty() && remote_is_newer(remote_written_at, local_written_at) {
        // Remote is newer — conflict
        match ctx.on_conflict {
            ConflictResolution::TakeRight => {
                // "TakeRight" here means take the remote (right) side — skip local write
                return Ok(WriteOutcome::Skipped);
            }
            ConflictResolution::TakeLeft => {
                // Force-write local value regardless
                write_fn().await?;
                return Ok(WriteOutcome::Written);
            }
            other => {
                // Record conflict and let caller resolve
                let id = conflict_mgr.record_conflict(
                    path,
                    ctx.snapshot_before,
                    vec![(ctx.agent_id, ctx.snapshot_after.unwrap_or(SnapshotId(0)))],
                );
                if let ConflictResolution::DeferToAgent(_) = &other {
                    // Resolution already set
                } else {
                    conflict_mgr.resolve(id, other);
                }
                return Ok(WriteOutcome::ConflictRecorded(id));
            }
        }
    }

    // No conflict or row is new — write
    write_fn().await?;
    Ok(WriteOutcome::Written)
}

/// Convenience: build a SQLite `datetime('now')` formatted timestamp from system time.
///
/// Format: `"YYYY-MM-DD HH:MM:SS"` (UTC) — matches SQLite's `datetime('now')`.
#[must_use]
pub fn sqlite_now_utc() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Convert epoch seconds to Y-M-D H:M:S UTC without chrono
    // Using a simple divisor approach
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400;
    // Days since 1970-01-01 (Gregorian approximation)
    let year_400 = days / 146097;
    let remaining = days % 146097;
    let year_100 = (remaining / 36524).min(3);
    let remaining = remaining - year_100 * 36524;
    let year_4 = remaining / 1461;
    let remaining = remaining % 1461;
    let year_1 = (remaining / 365).min(3);
    let remaining = remaining - year_1 * 365;
    let year = 1970 + year_400 * 400 + year_100 * 100 + year_4 * 4 + year_1;
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut doy = remaining;
    let mut month = 0usize;
    for (i, &md) in month_days.iter().enumerate() {
        if doy < md {
            month = i + 1;
            break;
        }
        doy -= md;
    }
    let day = doy + 1;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{min:02}:{sec:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_newer_detection() {
        assert!(remote_is_newer("2026-03-22 22:10:00", "2026-03-22 22:01:00"));
        assert!(!remote_is_newer("2026-03-22 22:01:00", "2026-03-22 22:10:00"));
        assert!(!remote_is_newer("", "2026-03-22 22:01:00")); // no row = not newer
    }

    #[test]
    fn sqlite_now_utc_format() {
        let s = sqlite_now_utc();
        assert!(s.len() == 19, "expected YYYY-MM-DD HH:MM:SS, got {s}");
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[7..8], "-");
        assert_eq!(&s[10..11], " ");
    }

    #[tokio::test]
    async fn skips_write_when_remote_newer_and_take_right() {
        let mut mgr = ConflictManager::new();
        let ctx = OccWrite::new(crate::types::AgentId(1), "node-a")
            .on_conflict(ConflictResolution::TakeRight);
        let mut write_called = false;
        let outcome = occ_guarded_write(
            "memories/key1",
            "2026-03-22 22:10:00",
            "2026-03-22 22:01:00",
            ctx,
            &mut mgr,
            || async {
                write_called = true;
                Ok(())
            },
        )
        .await
        .unwrap();
        assert_eq!(outcome, WriteOutcome::Skipped);
        assert!(!write_called);
    }

    #[tokio::test]
    async fn writes_when_no_conflict() {
        let mut mgr = ConflictManager::new();
        let ctx = OccWrite::new(crate::types::AgentId(1), "node-a");
        let mut write_called = false;
        let outcome = occ_guarded_write(
            "memories/key1",
            "",                      // row doesn't exist
            "2026-03-22 22:01:00",
            ctx,
            &mut mgr,
            || async {
                write_called = true;
                Ok(())
            },
        )
        .await
        .unwrap();
        assert_eq!(outcome, WriteOutcome::Written);
        assert!(write_called);
    }
}
