//! Global (cross-worktree) build coordination.
//!
//! The per-worktree [`crate::queue`] only orders builds that share one target.
//! With many worktrees and agents on one machine the real problems are
//! machine-wide: too many concurrent `cargo` builds saturate CPU/RAM/IO and pile
//! up on cargo's global package-cache lock. This module adds a **machine-wide
//! concurrency cap** (an N-slot cross-process semaphore) plus a single global
//! log, all stored OUTSIDE any repo (`~/.vox/build-broker`) so concurrent agents'
//! git operations can't wipe it.

use anyhow::Result;
use fs2::FileExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::metrics::now_ms;

/// How long a build waiting for admission sleeps between slot/load re-checks.
const ADMISSION_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Root dir for global broker state. Outside any repo → survives git clean/checkout.
/// Overridable with `VOX_BROKER_HOME` (used by tests).
pub fn global_root() -> PathBuf {
    if let Some(d) = std::env::var_os("VOX_BROKER_HOME") {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".vox").join("build-broker")
}

/// Max concurrent cargo builds machine-wide. `VOX_BROKER_MAX_CONCURRENT` overrides;
/// otherwise the machine's available parallelism (floored at 1).
pub fn max_concurrent() -> usize {
    let raw = std::env::var("VOX_BROKER_MAX_CONCURRENT").ok();
    let par = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    max_concurrent_from(raw.as_deref(), par)
}

/// Pure core of [`max_concurrent`], split out so it's testable without mutating
/// the process environment (which would require `unsafe` under edition 2024).
/// When unset, falls back to `parallelism.max(1)`.
pub fn max_concurrent_from(raw: Option<&str>, parallelism: usize) -> usize {
    if let Some(n) = raw
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n >= 1)
    {
        return n;
    }
    parallelism.max(1)
}

/// Slots reserved for a build domain the filesystem semaphore cannot see (a
/// containerised CI runner sharing this host's CPU but not its mount
/// namespace). `VOX_BROKER_RESERVED_SLOTS` overrides; unset, unparseable, or
/// negative is treated as 0 reserved slots.
pub fn reserved_slots() -> usize {
    let raw = std::env::var("VOX_BROKER_RESERVED_SLOTS").ok();
    reserved_slots_from(raw.as_deref())
}

/// Pure core of [`reserved_slots`], split out for the same reason as
/// [`max_concurrent_from`].
pub fn reserved_slots_from(raw: Option<&str>) -> usize {
    raw.and_then(|v| v.parse::<usize>().ok()).unwrap_or(0)
}

/// The effective machine-wide concurrency cap: [`max_concurrent`] reduced by
/// [`reserved_slots`], floored at 1 (never 0 — `acquire_slot` loops until a
/// slot frees, and a cap of 0 slots never frees one).
pub fn effective_max_concurrent() -> usize {
    let max_raw = std::env::var("VOX_BROKER_MAX_CONCURRENT").ok();
    let reserved_raw = std::env::var("VOX_BROKER_RESERVED_SLOTS").ok();
    let par = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    effective_max_concurrent_from(max_raw.as_deref(), reserved_raw.as_deref(), par)
}

/// Pure core of [`effective_max_concurrent`]. The reservation is applied
/// *after* `VOX_BROKER_MAX_CONCURRENT` — an explicit override is still
/// subject to it, because the reserved slots are physically in use by a
/// kernel this semaphore can't see regardless of what the override says.
pub fn effective_max_concurrent_from(
    max_raw: Option<&str>,
    reserved_raw: Option<&str>,
    parallelism: usize,
) -> usize {
    let base = max_concurrent_from(max_raw, parallelism);
    let reserved = reserved_slots_from(reserved_raw);
    base.saturating_sub(reserved).max(1)
}

/// Whether a build may start now. The first build always runs. Another runs only while the
/// 1-minute load average is below the machine's parallelism (idle cores exist), and never past
/// `cap` slots. Without a load reading (non-unix), builds run one at a time.
pub fn should_admit(load_1m: Option<f64>, parallelism: usize, held: usize, cap: usize) -> bool {
    if held == 0 {
        return true;
    }
    if held >= cap {
        return false;
    }
    match load_1m {
        Some(load) => load < parallelism as f64,
        None => false,
    }
}

/// A held global slot; dropping it (closing the file handle) frees the slot.
pub struct Slot {
    _f: std::fs::File,
}

/// Try to grab one free slot without blocking. Returns `(slot, busy_count)` where
/// `busy_count` is how many of the N slots were already taken, or `None` if all
/// N are busy.
pub fn try_acquire_slot(root: &Path, n: usize) -> Result<Option<(Slot, usize)>> {
    let slots_dir = root.join("slots");
    std::fs::create_dir_all(&slots_dir)?;
    let mut busy = 0;
    for i in 0..n.max(1) {
        let f = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(slots_dir.join(format!("slot_{i}")))?;
        match f.try_lock_exclusive() {
            Ok(()) => return Ok(Some((Slot { _f: f }, busy))),
            Err(_) => busy += 1,
        }
    }
    Ok(None)
}

/// Acquire one of N slots, polling until one frees. Returns the held slot, the
/// time spent waiting (ms), and how many slots were busy when we got in.
pub fn acquire_slot(root: &Path, n: usize) -> Result<(Slot, u64, usize)> {
    let start = now_ms();
    loop {
        if let Some((slot, busy)) = try_acquire_slot(root, n)? {
            let waited = now_ms().saturating_sub(start) as u64;
            return Ok((slot, waited, busy));
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

/// Count how many of the N slots are currently held, WITHOUT disturbing them:
/// each slot is probed with a non-blocking lock attempt and immediately
/// released if acquired (never held past the probe). Unlike
/// [`try_acquire_slot`] this never returns a slot to the caller, so calling it
/// (e.g. from a read-only status viewer) can't itself perturb the count it's
/// trying to measure.
///
/// This is inherently a **sample, not a snapshot**: another process may take
/// or release a slot between this probing two different slot files, so the
/// count can be stale the instant it's returned.
pub fn probe_busy_slots(root: &Path, n: usize) -> Result<usize> {
    let slots_dir = root.join("slots");
    if !slots_dir.is_dir() {
        return Ok(0);
    }
    let mut busy = 0;
    for i in 0..n.max(1) {
        let path = slots_dir.join(format!("slot_{i}"));
        if !path.is_file() {
            continue; // never claimed yet -> free
        }
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)?;
        match f.try_lock_exclusive() {
            Ok(()) => {
                let _ = f.unlock();
            }
            Err(_) => busy += 1,
        }
    }
    Ok(busy)
}

/// 1-minute load average, or `None` where the OS does not report one.
#[allow(unsafe_code)]
pub fn load_average_1m() -> Option<f64> {
    #[cfg(unix)]
    {
        let mut v = [0f64; 3];
        // SAFETY: getloadavg writes at most `nelem` doubles into the provided buffer.
        let n = unsafe { libc::getloadavg(v.as_mut_ptr(), 3) };
        (n >= 1).then_some(v[0])
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Like [`acquire_slot`], but a free slot is taken only when [`should_admit`] agrees.
pub fn acquire_slot_adaptive(root: &Path, cap: usize) -> Result<(Slot, u64, usize)> {
    let start = now_ms();
    let parallelism = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    loop {
        let held = probe_busy_slots(root, cap)?;
        if should_admit(load_average_1m(), parallelism, held, cap)
            && let Some((slot, busy)) = try_acquire_slot(root, cap)?
        {
            return Ok((slot, now_ms().saturating_sub(start) as u64, busy));
        }
        std::thread::sleep(ADMISSION_POLL_INTERVAL);
    }
}

/// Marker for an in-flight build identity; holds an exclusive lock on its entry for
/// the build's lifetime. Dropping it unlocks and removes the marker.
pub struct Inflight {
    _f: Option<std::fs::File>,
    path: PathBuf,
}

impl Drop for Inflight {
    fn drop(&mut self) {
        if let Some(f) = self._f.take() {
            let _ = f.unlock();
            drop(f);
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Register this build's identity (`key`) and report whether another in-flight
/// build already shares it (a true coalescing opportunity, cross-worktree).
/// Holds an exclusive lock on its entry for the build's lifetime, and removes any
/// stale entry whose lock can be acquired (its owner is gone).
pub fn register_inflight(root: &Path, key: &str) -> Result<(Inflight, bool)> {
    let dir = root.join("inflight");
    std::fs::create_dir_all(&dir)?;
    let mut coalesce = false;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            let Ok(f) = std::fs::OpenOptions::new().read(true).write(true).open(&p) else {
                if std::fs::read_to_string(&p)
                    .map(|s| s.trim() == key)
                    .unwrap_or(false)
                {
                    coalesce = true;
                }
                continue;
            };

            match f.try_lock_exclusive() {
                Ok(()) => {
                    // Lock acquired -> owner is gone -> reap stale entry.
                    let _ = f.unlock();
                    drop(f);
                    let _ = std::fs::remove_file(&p);
                }
                Err(_) => {
                    // Entry is locked by an active process -> still in-flight.
                    drop(f);
                    if std::fs::read_to_string(&p)
                        .map(|s| s.trim() == key)
                        .unwrap_or(false)
                    {
                        coalesce = true;
                    }
                }
            }
        }
    }
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mine = dir.join(format!("{}-{}-{}", std::process::id(), now_ms(), seq));
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&mine)?;
    f.try_lock_exclusive()?;
    use std::io::Write;
    f.write_all(key.as_bytes())?;
    f.flush()?;
    Ok((
        Inflight {
            _f: Some(f),
            path: mine,
        },
        coalesce,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semaphore_caps_concurrency() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // N=2: take both slots, the 3rd try must fail until one is released.
        let (s1, b1) = try_acquire_slot(root, 2).unwrap().unwrap();
        assert_eq!(b1, 0);
        let (_s2, b2) = try_acquire_slot(root, 2).unwrap().unwrap();
        assert_eq!(b2, 1);
        assert!(
            try_acquire_slot(root, 2).unwrap().is_none(),
            "3rd must be blocked"
        );
        drop(s1);
        assert!(
            try_acquire_slot(root, 2).unwrap().is_some(),
            "freed slot reusable"
        );
    }

    #[test]
    fn semaphore_caps_concurrency_with_reservation() {
        // Same shape as `semaphore_caps_concurrency`, but exercising the
        // scenario the reservation exists for: an effective cap of 1 (base 3,
        // reserved 2) must serialize -- hold a slot, prove the second
        // acquisition is blocked, release, and prove it succeeds again. This
        // is the deliberate proof the brief asks for in place of a timing race.
        let n = effective_max_concurrent_from(Some("3"), Some("2"), 24);
        assert_eq!(n, 1);
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let (s1, b1) = try_acquire_slot(root, n).unwrap().unwrap();
        assert_eq!(b1, 0);
        assert!(
            try_acquire_slot(root, n).unwrap().is_none(),
            "reserved-down cap of 1 must block a second acquisition"
        );
        drop(s1);
        assert!(
            try_acquire_slot(root, n).unwrap().is_some(),
            "freed slot reusable"
        );
    }

    #[test]
    fn probe_busy_slots_never_holds_what_it_counts() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        assert_eq!(probe_busy_slots(root, 2).unwrap(), 0, "nothing held yet");

        let (_s1, _) = try_acquire_slot(root, 2).unwrap().unwrap();
        assert_eq!(probe_busy_slots(root, 2).unwrap(), 1);

        // The probe must not itself hold the free slot: a real acquire must
        // still succeed right after probing.
        let (_s2, busy) = try_acquire_slot(root, 2).unwrap().unwrap();
        assert_eq!(busy, 1);
        assert_eq!(probe_busy_slots(root, 2).unwrap(), 2);
    }

    #[test]
    fn probe_busy_slots_on_missing_root_is_zero() {
        let tmp = tempfile::tempdir().unwrap();
        // Root exists but `slots/` was never created -- broker never ran.
        assert_eq!(probe_busy_slots(tmp.path(), 4).unwrap(), 0);
    }

    #[test]
    fn reserved_slots_logic() {
        assert_eq!(reserved_slots_from(None), 0);
        assert_eq!(reserved_slots_from(Some("0")), 0);
        assert_eq!(reserved_slots_from(Some("3")), 3);
        assert_eq!(reserved_slots_from(Some("-1")), 0, "negative -> ignored");
        assert_eq!(reserved_slots_from(Some("xx")), 0, "unparseable -> ignored");
    }

    #[test]
    fn effective_max_concurrent_logic() {
        // No reservation: falls through to the base cap unchanged.
        assert_eq!(effective_max_concurrent_from(None, None, 24), 24);
        assert_eq!(effective_max_concurrent_from(None, Some("0"), 24), 24);
        // Normal reservation: base 24, reserve 3 -> 21.
        assert_eq!(effective_max_concurrent_from(None, Some("3"), 24), 21);
        // Reservation exceeding the base cap floors at 1, never 0.
        assert_eq!(effective_max_concurrent_from(None, Some("99"), 24), 1);
        // Reservation applies AFTER an explicit override too.
        assert_eq!(effective_max_concurrent_from(Some("4"), Some("1"), 24), 3);
        assert_eq!(effective_max_concurrent_from(Some("4"), Some("10"), 24), 1);
        // Unparseable reservation is ignored (treated as 0).
        assert_eq!(
            effective_max_concurrent_from(Some("4"), Some("nope"), 24),
            4
        );
    }

    #[test]
    fn inflight_detects_same_key() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let (_a, c1) = register_inflight(root, "build|h1").unwrap();
        assert!(!c1); // first of its kind
        let (_b, c2) = register_inflight(root, "build|h1").unwrap();
        assert!(c2); // matches the in-flight one
        let (_d, c3) = register_inflight(root, "test|h2").unwrap();
        assert!(!c3); // different identity
    }

    #[test]
    fn inflight_marker_removed_on_drop() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        {
            let (_g, _) = register_inflight(root, "k").unwrap();
            let count = std::fs::read_dir(root.join("inflight")).unwrap().count();
            assert_eq!(count, 1);
        }
        let count = std::fs::read_dir(root.join("inflight")).unwrap().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn admits_the_first_build_regardless_of_load() {
        assert!(should_admit(Some(1000.0), 8, 0, 8));
    }

    #[test]
    fn admits_another_build_only_while_the_machine_has_idle_cores() {
        assert!(should_admit(Some(5.0), 8, 1, 8));
        assert!(!should_admit(Some(8.0), 8, 1, 8));
        assert!(!should_admit(Some(12.5), 8, 2, 8));
    }

    #[test]
    fn never_exceeds_the_slot_cap() {
        assert!(!should_admit(Some(0.0), 8, 8, 8));
    }

    #[test]
    fn without_a_load_reading_admits_one_build_at_a_time() {
        assert!(should_admit(None, 8, 0, 8));
        assert!(!should_admit(None, 8, 1, 8));
    }

    #[test]
    fn default_cap_follows_the_machine_with_no_literal_bounds() {
        assert_eq!(max_concurrent_from(None, 18), 18);
        assert_eq!(max_concurrent_from(None, 1), 1);
        assert_eq!(max_concurrent_from(Some("3"), 18), 3);
        // Invalid or zero overrides fall back to the machine default.
        assert_eq!(max_concurrent_from(Some("0"), 18), 18);
        assert_eq!(max_concurrent_from(Some("xx"), 18), 18);
    }

    #[test]
    fn stale_inflight_entry_is_reaped_while_locked_one_survives() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let inflight_dir = root.join("inflight");
        std::fs::create_dir_all(&inflight_dir).unwrap();

        // 1. Hold a live locked inflight entry.
        let (live, _) = register_inflight(root, "live_key").unwrap();
        assert!(live.path.exists());

        // 2. Create an unlocked entry (simulating a dead process that crashed).
        let stale = inflight_dir.join("stale_unlocked");
        std::fs::write(&stale, "stale_key").unwrap();
        assert!(stale.exists());

        // 3. Register a new entry; this must reap the unlocked stale file.
        let (_another, _) = register_inflight(root, "another_key").unwrap();

        assert!(!stale.exists(), "stale unlocked entry should be reaped");
        assert!(live.path.exists(), "locked entry must survive reaping");
    }
}
