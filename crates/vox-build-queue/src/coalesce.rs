//! Windows-safe leader-follower flight coalescing under `<broker_home>/inflight/flights/`.
//! Uses 0-byte `<key>.lock` files and unlocked per-flight `<key>.<run_id>.{stdout,stderr,exit}` siblings.

use anyhow::Result;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::metrics::now_ms;
pub use crate::mtime::latest_source_mtime_ms;

const FOLLOWER_POLL_MS: u64 = 50;
const RECEIPT_FRESH_WINDOW_MS: u128 = 30_000;
const FLIGHT_ARTIFACT_TTL_MS: u128 = 60 * 60 * 1000;

static FLIGHT_NONCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderMeta {
    pub run_id: String,
    pub started_ms: u128,
    pub subcmd: String,
    pub target_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderExit {
    pub run_id: String,
    pub started_ms: u128,
    pub finished_ms: u128,
    pub exit_code: i32,
    pub timed_out: bool,
}

pub struct LeaderGuard {
    lock_file: Option<std::fs::File>,
    meta: LeaderMeta,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    exit_path: PathBuf,
}

impl LeaderGuard {
    pub fn stream_paths(&self) -> (&Path, &Path) {
        (&self.stdout_path, &self.stderr_path)
    }

    pub fn finish(mut self, exit_code: i32, timed_out: bool) {
        let receipt = LeaderExit {
            run_id: self.meta.run_id.clone(),
            started_ms: self.meta.started_ms,
            finished_ms: now_ms(),
            exit_code,
            timed_out,
        };
        let _ = write_json_atomic(&self.exit_path, &receipt);
        if let Some(f) = self.lock_file.take() {
            let _ = FileExt::unlock(&f);
        }
    }
}

impl Drop for LeaderGuard {
    fn drop(&mut self) {
        if let Some(f) = self.lock_file.take() {
            let _ = FileExt::unlock(&f);
        }
    }
}

fn flights_dir(root: &Path) -> PathBuf {
    root.join("inflight").join("flights")
}

fn flight_artifact(dir: &Path, key_hash: &str, run_id: &str, ext: &str) -> PathBuf {
    dir.join(format!("{key_hash}.{run_id}.{ext}"))
}

fn write_json_atomic<T: Serialize>(dest: &Path, val: &T) -> Result<()> {
    let seq = FLIGHT_NONCE.fetch_add(1, Ordering::Relaxed);
    let tmp = dest.with_extension(format!("tmp.{}.{}.{seq}", std::process::id(), now_ms()));
    std::fs::write(&tmp, serde_json::to_vec(val)?)?;
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

/// Non-exclusively probe whether an active leader holds `<key>.lock`.
/// Uses `try_lock_shared()` so concurrent followers never mistake each other for a leader.
fn is_lock_held(lock_path: &Path) -> bool {
    let Ok(f) = std::fs::OpenOptions::new().read(true).open(lock_path) else {
        return false;
    };
    match f.try_lock_shared() {
        Ok(()) => {
            let _ = FileExt::unlock(&f);
            false
        }
        Err(_) => true,
    }
}

/// Best-effort opportunistic reaper for unlocked flight artifacts older than `ttl_ms`.
fn reap_stale_flights(dir: &Path, exclude_key: &str, ttl_ms: u128) {
    let (now, Ok(rd)) = (now_ms(), std::fs::read_dir(dir)) else {
        return;
    };
    for entry in rd.flatten() {
        if !entry.file_type().is_ok_and(|ft| ft.is_file()) {
            continue;
        }
        let fname = entry.file_name();
        let key_prefix = fname
            .to_str()
            .and_then(|s| s.split('.').next())
            .unwrap_or("");
        if key_prefix.is_empty() || key_prefix == exclude_key {
            continue;
        }
        let p = entry.path();
        let mtime = crate::mtime::file_mtime_ms(&p);
        let key_lock = dir.join(format!("{key_prefix}.lock"));
        if mtime > 0
            && now.saturating_sub(mtime) > ttl_ms
            && !(key_lock.is_file() && is_lock_held(&key_lock))
        {
            let _ = std::fs::remove_file(&p);
        }
    }
}

fn drain_from(path: &Path, offset: u64, on_chunk: &mut impl FnMut(&[u8])) -> u64 {
    let Ok(mut f) = std::fs::File::open(path) else {
        return offset;
    };
    if f.seek(SeekFrom::Start(offset)).is_err() {
        return offset;
    }
    let (mut cur, mut buf) = (offset, [0u8; 8192]);
    while let Ok(n) = f.read(&mut buf) {
        if n == 0 {
            break;
        }
        on_chunk(&buf[..n]);
        cur += n as u64;
    }
    cur
}

/// Whether `(sub, args, env)` is eligible for leader-follower coalescing.
pub fn is_coalesceable(sub: &str, args: &[String], env: &BTreeMap<String, String>) -> bool {
    if env
        .get("VOX_BROKER_COALESCE")
        .is_some_and(|v| v.trim() == "0" || v.trim().eq_ignore_ascii_case("false"))
    {
        return false;
    }
    if args
        .iter()
        .take_while(|a| a.as_str() != "--")
        .any(|a| a.starts_with("--message-format"))
    {
        return false;
    }
    match sub {
        "check" | "build" | "clippy" => true,
        "test" => {
            let snap = ["INSTA_UPDATE", "UPDATE_EXPECT", "SNAPSHOT_UPDATE"]
                .iter()
                .any(|k| {
                    env.get(*k).is_some_and(|v| {
                        let t = v.trim();
                        !t.is_empty() && t != "0" && !t.eq_ignore_ascii_case("false")
                    })
                });
            !snap
                && !env
                    .get("TRYBUILD")
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("overwrite"))
        }
        "run" => crate::resolve::build_args_for_run(args).is_some(),
        _ => false,
    }
}

fn receipt_is_fresh(exit: &LeaderExit, expected_run_id: &str, min_started_ms: u128) -> bool {
    exit.run_id == expected_run_id
        && !exit.timed_out
        && exit.started_ms >= min_started_ms
        && now_ms().saturating_sub(exit.finished_ms) <= RECEIPT_FRESH_WINDOW_MS
}

/// Attach to an active in-flight leader for `key_hash` if its `started_ms >= min_started_ms`.
pub fn try_follow_inflight(
    root: &Path,
    key_hash: &str,
    min_started_ms: u128,
    mut on_stdout: impl FnMut(&[u8]),
    mut on_stderr: impl FnMut(&[u8]),
) -> Option<(i32, u64)> {
    let dir = flights_dir(root);
    let lock_path = dir.join(format!("{key_hash}.lock"));
    if !lock_path.is_file() || !is_lock_held(&lock_path) {
        return None;
    }
    let meta_path = dir.join(format!("{key_hash}.meta"));
    let mut meta: Option<LeaderMeta> = None;
    for _ in 0..4 {
        if let Ok(raw) = std::fs::read_to_string(&meta_path)
            && let Ok(m) = serde_json::from_str::<LeaderMeta>(&raw)
        {
            let prior_exit = flight_artifact(&dir, key_hash, &m.run_id, "exit");
            if !(prior_exit.is_file() && is_lock_held(&lock_path)) {
                meta = Some(m);
                break;
            }
        }
        if !is_lock_held(&lock_path) {
            break;
        }
        std::thread::sleep(Duration::from_millis(FOLLOWER_POLL_MS));
    }
    let meta = meta.filter(|m| m.started_ms >= min_started_ms)?;

    let stdout_path = flight_artifact(&dir, key_hash, &meta.run_id, "stdout");
    let stderr_path = flight_artifact(&dir, key_hash, &meta.run_id, "stderr");
    let exit_path = flight_artifact(&dir, key_hash, &meta.run_id, "exit");
    let t0 = Instant::now();
    let (mut out_off, mut err_off) = (0u64, 0u64);
    loop {
        let exit_ready = exit_path.is_file();
        let held = is_lock_held(&lock_path);
        let superseded = std::fs::read_to_string(&meta_path)
            .ok()
            .and_then(|s| serde_json::from_str::<LeaderMeta>(&s).ok())
            .is_some_and(|cur| cur.run_id != meta.run_id);
        out_off = drain_from(&stdout_path, out_off, &mut on_stdout);
        err_off = drain_from(&stderr_path, err_off, &mut on_stderr);
        if exit_ready || !held || superseded {
            let _ = drain_from(&stdout_path, out_off, &mut on_stdout);
            let _ = drain_from(&stderr_path, err_off, &mut on_stderr);
            break;
        }
        std::thread::sleep(Duration::from_millis(FOLLOWER_POLL_MS));
    }

    let exit_raw = std::fs::read_to_string(&exit_path).ok()?;
    let exit: LeaderExit = serde_json::from_str(&exit_raw).ok()?;
    if !receipt_is_fresh(&exit, &meta.run_id, min_started_ms) {
        return None;
    }
    Some((exit.exit_code, t0.elapsed().as_millis() as u64))
}

/// Replay a recently completed leader that started at/after `ticket_ts_ms` and `min_started_ms`.
pub fn try_replay_recent_leader(
    root: &Path,
    key_hash: &str,
    ticket_ts_ms: u128,
    min_started_ms: u128,
    mut on_stdout: impl FnMut(&[u8]),
    mut on_stderr: impl FnMut(&[u8]),
) -> Option<i32> {
    let dir = flights_dir(root);
    let lock_path = dir.join(format!("{key_hash}.lock"));
    if lock_path.is_file() && is_lock_held(&lock_path) {
        return None;
    }
    let meta_raw = std::fs::read_to_string(dir.join(format!("{key_hash}.meta"))).ok()?;
    let meta: LeaderMeta = serde_json::from_str(&meta_raw).ok()?;
    let exit_raw =
        std::fs::read_to_string(flight_artifact(&dir, key_hash, &meta.run_id, "exit")).ok()?;
    let exit: LeaderExit = serde_json::from_str(&exit_raw).ok()?;
    let floor_ms = ticket_ts_ms.max(min_started_ms);
    if !receipt_is_fresh(&exit, &meta.run_id, floor_ms) {
        return None;
    }
    let _ = drain_from(
        &flight_artifact(&dir, key_hash, &meta.run_id, "stdout"),
        0,
        &mut on_stdout,
    );
    let _ = drain_from(
        &flight_artifact(&dir, key_hash, &meta.run_id, "stderr"),
        0,
        &mut on_stderr,
    );
    Some(exit.exit_code)
}

/// Claim leader role for `key_hash` by locking a 0-byte `<key>.lock` file.
pub fn claim_leader(
    root: &Path,
    key_hash: &str,
    subcmd: &str,
    target_dir: &Path,
) -> Result<Option<LeaderGuard>> {
    let dir = flights_dir(root);
    std::fs::create_dir_all(&dir)?;
    reap_stale_flights(&dir, key_hash, FLIGHT_ARTIFACT_TTL_MS);

    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(dir.join(format!("{key_hash}.lock")))?;
    let locked = (0..5).any(|attempt| {
        if lock_file.try_lock_exclusive().is_ok() {
            return true;
        }
        if attempt < 4 {
            std::thread::sleep(Duration::from_millis(5));
        }
        false
    });
    if !locked {
        return Ok(None);
    }

    let meta_path = dir.join(format!("{key_hash}.meta"));
    let _ = std::fs::remove_file(&meta_path);

    let started_ms = now_ms();
    let seq = FLIGHT_NONCE.fetch_add(1, Ordering::Relaxed);
    let run_id = format!("{}-{started_ms}-{seq}", std::process::id());
    let stdout_path = flight_artifact(&dir, key_hash, &run_id, "stdout");
    let stderr_path = flight_artifact(&dir, key_hash, &run_id, "stderr");
    let exit_path = flight_artifact(&dir, key_hash, &run_id, "exit");
    std::fs::File::create(&stdout_path)?;
    std::fs::File::create(&stderr_path)?;

    let meta = LeaderMeta {
        run_id,
        started_ms,
        subcmd: subcmd.to_string(),
        target_dir: target_dir.display().to_string(),
    };
    write_json_atomic(&meta_path, &meta)?;

    Ok(Some(LeaderGuard {
        lock_file: Some(lock_file),
        meta,
        stdout_path,
        stderr_path,
        exit_path,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sv(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn coalesce_eligibility_rules() {
        let e = BTreeMap::new();
        for a in [
            &["check", "-p", "foo"][..],
            &["build", "--workspace"][..],
            &["clippy", "-p", "foo"][..],
            &["test", "-p", "foo"][..],
            &["run", "-p", "vox-cli", "--", "version"][..],
        ] {
            assert!(is_coalesceable(a[0], &sv(a), &e));
        }
        for a in [
            &["bench"][..],
            &["fmt"][..],
            &["build", "--message-format=json"][..],
            &["clippy", "--message-format", "short"][..],
        ] {
            assert!(!is_coalesceable(a[0], &sv(a), &e));
        }
        let disabled = BTreeMap::from([("VOX_BROKER_COALESCE".into(), "0".into())]);
        assert!(!is_coalesceable("check", &sv(&["check"]), &disabled));
        for (k, v) in [
            ("INSTA_UPDATE", "always"),
            ("UPDATE_EXPECT", "1"),
            ("SNAPSHOT_UPDATE", "1"),
            ("TRYBUILD", "overwrite"),
        ] {
            let env = BTreeMap::from([(k.into(), v.into())]);
            assert!(!is_coalesceable("test", &sv(&["test"]), &env));
            assert!(is_coalesceable("check", &sv(&["check"]), &env));
        }
    }

    #[test]
    fn leader_follower_streams_with_zero_byte_lock_and_recent_replay() {
        let tmp = tempfile::tempdir().unwrap();
        let (root, key) = (tmp.path(), "abcd1234ef567890");
        let leader = claim_leader(root, key, "check", Path::new("/t"))
            .unwrap()
            .unwrap();
        assert!(
            claim_leader(root, key, "check", Path::new("/t"))
                .unwrap()
                .is_none()
        );

        let (out_p, err_p) = leader.stream_paths();
        let (out_path, err_path) = (out_p.to_path_buf(), err_p.to_path_buf());
        let lock_path = flights_dir(root).join(format!("{key}.lock"));
        assert_eq!(std::fs::metadata(&lock_path).unwrap().len(), 0);

        let rc = root.to_path_buf();
        let follower = std::thread::spawn(move || {
            let (mut ob, mut eb) = (Vec::<u8>::new(), Vec::<u8>::new());
            let r = try_follow_inflight(&rc, key, 0, |b| ob.extend(b), |b| eb.extend(b));
            (r, ob, eb)
        });

        std::thread::sleep(Duration::from_millis(60));
        std::fs::write(&out_path, b"hello stdout\n").unwrap();
        std::fs::write(&err_path, b"hello stderr\n").unwrap();
        std::thread::sleep(Duration::from_millis(60));
        leader.finish(0, false);

        let (res, out_buf, err_buf) = follower.join().unwrap();
        assert_eq!(res.unwrap().0, 0);
        assert_eq!(out_buf, b"hello stdout\n");
        assert_eq!(err_buf, b"hello stderr\n");
        assert_eq!(std::fs::metadata(&lock_path).unwrap().len(), 0);
        assert!(!is_lock_held(&lock_path));

        let (mut ro, mut re) = (Vec::<u8>::new(), Vec::<u8>::new());
        assert_eq!(
            try_replay_recent_leader(root, key, 0, 0, |b| ro.extend(b), |b| re.extend(b)),
            Some(0)
        );
        assert!(ro == b"hello stdout\n" && re == b"hello stderr\n");
        assert!(
            try_replay_recent_leader(root, key, 0, now_ms() + 60_000, |_| {}, |_| {}).is_none()
        );
    }

    #[test]
    fn shared_probe_back_to_back_isolation_and_ttl_reaping() {
        let tmp = tempfile::tempdir().unwrap();
        let (root, key, tgt) = (tmp.path(), "runisolation0001", Path::new("/t"));

        let l1 = claim_leader(root, key, "check", tgt).unwrap().unwrap();
        let r1_out = l1.stream_paths().0.to_path_buf();
        std::fs::write(&r1_out, b"run1\n").unwrap();
        l1.finish(0, false);

        let l2 = claim_leader(root, key, "check", tgt).unwrap().unwrap();
        let r2_out = l2.stream_paths().0.to_path_buf();
        assert_ne!(r1_out, r2_out);
        std::fs::write(&r2_out, b"run2\n").unwrap();
        l2.finish(0, false);

        assert_eq!(std::fs::read(&r1_out).unwrap(), b"run1\n");
        let mut replay_out = Vec::<u8>::new();
        assert_eq!(
            try_replay_recent_leader(root, key, 0, 0, |b| replay_out.extend(b), |_| {}),
            Some(0)
        );
        assert_eq!(replay_out, b"run2\n");

        //Shared lock held by observer does NOT make is_lock_held report exclusive leader
        let lock_path = flights_dir(root).join(format!("{key}.lock"));
        let reader = std::fs::File::open(&lock_path).unwrap();
        reader.try_lock_shared().unwrap();
        assert!(!is_lock_held(&lock_path));
        FileExt::unlock(&reader).unwrap();

        // Stale unlocked flight artifacts are reaped; timed-out or expired receipts are rejected
        let dir = flights_dir(root);
        let stale_meta = dir.join("stale00000000001.meta");
        std::fs::write(&stale_meta, "{}").unwrap();
        std::thread::sleep(Duration::from_millis(30));
        reap_stale_flights(&dir, key, 10);
        assert!(!stale_meta.exists());

        let l_to = claim_leader(root, "timeoutkey000001", "test", Path::new("/t"))
            .unwrap()
            .unwrap();
        l_to.finish(124, true);
        assert!(try_replay_recent_leader(root, "timeoutkey000001", 0, 0, |_| {}, |_| {}).is_none());
        let old_exit = LeaderExit {
            run_id: "r-old".into(),
            started_ms: 100,
            finished_ms: 200,
            exit_code: 0,
            timed_out: false,
        };
        assert!(!receipt_is_fresh(&old_exit, "r-old", 0));
    }
}
