//! Local JSON file queue for optional telemetry upload (`vox telemetry …`).
//!
//! Previously a re-export of `vox_spool::queue`; inlined here when `vox-spool` was
//! deleted (scheduled removal noted in 0.6). See ADR 023 and
//! `docs/src/architecture/telemetry-remote-sink-spec.md`.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use uuid::Uuid;

/// Default cap on total pending-spool size before `prune` starts deleting
/// files (oldest first). Fixed rather than configurable — bump if a real
/// deployment needs a different ceiling.
// ponytail: fixed cap, no env knob; add VOX_TELEMETRY_SPOOL_MAX_BYTES if ever needed.
pub const DEFAULT_MAX_SPOOL_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB
/// Default cap on pending-file age; anything older is pruned regardless of size.
pub const DEFAULT_MAX_SPOOL_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60); // 7 days

fn default_spool_root() -> PathBuf {
    if let Ok(p) = std::env::var("VOX_TELEMETRY_SPOOL_DIR") {
        let t = p.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    // Anchored to the REPOSITORY ROOT, never to the current directory.
    //
    // This used to be `current_dir().join(".vox")`. Telemetry initializes before
    // command dispatch (see lib.rs), so every `vox` invocation from a
    // subdirectory created a fresh `.vox/telemetry-upload-queue/` right there.
    // Two strays were found in a single afternoon, in different directories, and
    // a stray `.vox` under `crates/` has broken cargo before now because the root
    // manifest globs `crates/*`.
    vox_config::paths::repo_dot_vox_dir().join("telemetry-upload-queue")
}

/// Resolved spool directory.
#[must_use]
pub fn spool_root() -> PathBuf {
    default_spool_root()
}

fn pending_dir(root: &Path) -> PathBuf {
    root.join("pending")
}

/// Ensure `pending/` exists under `root`.
pub fn ensure_spool(root: &Path) -> Result<PathBuf> {
    let p = pending_dir(root);
    fs::create_dir_all(&p).with_context(|| format!("create {}", p.display()))?;
    Ok(p)
}

/// Append one JSON value as a new pending file, then enforce the spool's
/// size/age caps so the queue can't grow without bound when nothing is
/// draining it via `vox telemetry upload` (see `prune`).
pub fn enqueue(root: &Path, value: &impl Serialize) -> Result<PathBuf> {
    let pending = ensure_spool(root)?;
    let id = Uuid::new_v4();
    let path = pending.join(format!("{id}.json"));
    let body = serde_json::to_vec_pretty(value).context("serialize telemetry payload")?;
    fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    let _ = prune(root, DEFAULT_MAX_SPOOL_BYTES, DEFAULT_MAX_SPOOL_AGE);
    Ok(path)
}

/// Delete pending files older than `max_age`, then — if the remaining files
/// still total more than `max_bytes` — delete oldest-first until under the
/// cap. Best-effort: I/O errors on individual files are skipped, not fatal.
/// Returns `(files_removed, bytes_removed)`.
pub fn prune(root: &Path, max_bytes: u64, max_age: Duration) -> Result<(usize, u64)> {
    let pending = pending_dir(root);
    if !pending.is_dir() {
        return Ok((0, 0));
    }
    let now = SystemTime::now();
    let mut removed_files = 0usize;
    let mut removed_bytes = 0u64;

    // Collect (path, mtime, size), evicting age-expired files immediately.
    let mut remaining: Vec<(PathBuf, SystemTime, u64)> = Vec::new();
    for path in list_pending(root)? {
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let mtime = meta.modified().unwrap_or(now);
        let size = meta.len();
        let age = now.duration_since(mtime).unwrap_or_default();
        if age > max_age {
            if fs::remove_file(&path).is_ok() {
                removed_files += 1;
                removed_bytes += size;
            }
            continue;
        }
        remaining.push((path, mtime, size));
    }

    // Oldest-first eviction until total size is back under the cap.
    remaining.sort_by_key(|(_, mtime, _)| *mtime);
    let mut total: u64 = remaining.iter().map(|(_, _, size)| size).sum();
    for (path, _, size) in remaining {
        if total <= max_bytes {
            break;
        }
        if fs::remove_file(&path).is_ok() {
            removed_files += 1;
            removed_bytes += size;
            total = total.saturating_sub(size);
        }
    }

    Ok((removed_files, removed_bytes))
}

/// Sorted pending file paths (lexicographic by name).
pub fn list_pending(root: &Path) -> Result<Vec<PathBuf>> {
    let pending = pending_dir(root);
    if !pending.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = fs::read_dir(&pending)
        .with_context(|| format!("read_dir {}", pending.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    Ok(paths)
}

/// Read JSON from a pending file.
pub fn read_payload(path: &Path) -> Result<serde_json::Value> {
    let mut f = fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut s = String::new();
    f.read_to_string(&mut s)
        .with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&s).with_context(|| format!("parse JSON {}", path.display()))
}

/// Delete a pending file after successful upload.
pub fn ack(path: &Path) -> Result<()> {
    fs::remove_file(path).with_context(|| format!("remove {}", path.display()))
}

/// Count pending files.
#[must_use]
pub fn pending_count(root: &Path) -> usize {
    list_pending(root).map(|v| v.len()).unwrap_or(0)
}

/// Export all pending payloads as JSON Lines to `w` (does not delete).
pub fn export_jsonl(root: &Path, w: &mut dyn std::io::Write) -> Result<usize> {
    let paths = list_pending(root)?;
    let mut n = 0usize;
    for p in paths {
        let v = read_payload(&p)?;
        let line = serde_json::to_string(&v).context("serialize json line")?;
        writeln!(w, "{line}").context("write json line")?;
        n += 1;
    }
    Ok(n)
}

/// Render up to `limit` pending spool files as a batched OTLP/HTTP JSON preview to `w`
/// without making network calls or deleting files (`TELEM-04`).
pub fn preview_pending(root: &Path, limit: usize, w: &mut dyn std::io::Write) -> Result<usize> {
    let paths = list_pending(root)?;
    let mut envelopes = Vec::new();
    for p in paths.into_iter().take(limit.max(1)) {
        envelopes.push(read_payload(&p)?);
    }
    let count = envelopes.len();
    let merged = vox_telemetry_otlp::otlp_json::merge_otlp_log_batch(&envelopes);
    let pretty = serde_json::to_string_pretty(&merged).context("serialize OTLP preview")?;
    writeln!(w, "{pretty}").context("write OTLP preview")?;
    Ok(count)
}

/// POST pending JSON payloads in batches of up to 100 records per request to `url` with optional
/// `Authorization: Bearer …`. Enforces `vox_telemetry::config::is_remote_allowed()` (`TELEM-02`, `TELEM-03`).
pub async fn upload_pending(
    root: &Path,
    url: &str,
    bearer: Option<&str>,
    dry_run: bool,
) -> Result<(usize, usize)> {
    if !dry_run && !vox_telemetry::config::is_remote_allowed() {
        return Err(anyhow!(
            "remote telemetry upload is disabled (consent not granted or VOX_TELEMETRY=off; run `vox telemetry consent grant`)"
        ));
    }
    if url.trim().is_empty() {
        return Err(anyhow!("telemetry upload URL is empty"));
    }
    let client = vox_http_client::client_builder()
        .build()
        .context("build HTTP client")?;
    let paths = list_pending(root)?;
    let mut ok = 0usize;
    let mut fail = 0usize;
    for chunk in paths.chunks(100) {
        let mut batch_paths = Vec::with_capacity(chunk.len());
        let mut batch_payloads = Vec::with_capacity(chunk.len());
        for p in chunk {
            match read_payload(p) {
                Ok(v) => {
                    batch_paths.push(p.clone());
                    batch_payloads.push(v);
                }
                Err(_) => fail += 1,
            }
        }
        if batch_payloads.is_empty() {
            continue;
        }
        let merged = vox_telemetry_otlp::otlp_json::merge_otlp_log_batch(&batch_payloads);
        let raw = serde_json::to_string(&merged).context("serialize batched OTLP body")?;
        if dry_run {
            eprintln!(
                "[dry-run] would POST {} record(s) ({} bytes) to {url}",
                batch_paths.len(),
                raw.len()
            );
            ok += batch_paths.len();
            continue;
        }
        // Captured before `raw` is moved into the request body: the gamify
        // event below reports it after the POST completes.
        #[cfg(feature = "vox-gamify")]
        let raw_len = raw.len();
        let mut req = client.post(url).body(raw).header(
            reqwest::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        );
        if let Some(t) = bearer.filter(|s| !s.trim().is_empty()) {
            req = req.header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", t.trim()),
            );
        }
        let resp = req
            .send()
            .await
            .with_context(|| format!("POST batch of {} files", batch_paths.len()))?;
        let status = resp.status();
        if status.is_success() {
            for p in &batch_paths {
                ack(p)?;
                ok += 1;
            }

            #[cfg(feature = "vox-gamify")]
            {
                if let Ok(db) = vox_db::Codex::connect_default().await {
                    let ev = serde_json::json!({
                        "type": "telemetry_shared",
                        "source": "vox-telemetry",
                        "payload": { "bytes_shared": raw_len },
                    });
                    if let Err(e) = vox_gamify::event_router::route_event_auto_user(&db, &ev).await
                    {
                        tracing::debug!(error = %e, "failed to route telemetry_shared event");
                    }
                }
            }
        } else {
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!(
                status = %status,
                batch_len = batch_paths.len(),
                body = %text.chars().take(200).collect::<String>(),
                "telemetry batch upload rejected"
            );
            fail += batch_paths.len();
        }
    }
    Ok((ok, fail))
}

#[cfg(test)]
mod prune_tests {
    use super::*;
    use std::fs::FileTimes;

    fn write_pending_file(root: &Path, name: &str, size: usize, age: Duration) -> PathBuf {
        let pending = ensure_spool(root).unwrap();
        let path = pending.join(name);
        fs::write(&path, vec![b'x'; size]).unwrap();
        let mtime = SystemTime::now() - age;
        let times = FileTimes::new().set_modified(mtime);
        let f = fs::OpenOptions::new().write(true).open(&path).unwrap();
        f.set_times(times).unwrap();
        path
    }

    /// Regression: before the fix, nothing ever pruned the spool — it grew
    /// without bound (8,605 files / 34 MB observed in the wild).
    #[test]
    fn prune_evicts_oldest_files_over_size_cap() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // Three 100-byte files (total 300); cap at 150 bytes must evict
        // oldest-first until at or under the cap: removing just "a" leaves
        // 200 (still over), so "b" goes too, leaving only "c" at 100.
        // drift-allow(timeout-literal,duplicate-numeric-literal): test fixture duration, not an HTTP request timeout; a shared constant needs a vox-config edge
        write_pending_file(root, "a.json", 100, Duration::from_secs(300));
        write_pending_file(root, "b.json", 100, Duration::from_secs(200));
        write_pending_file(root, "c.json", 100, Duration::from_secs(100));

        // drift-allow(timeout-literal,duplicate-numeric-literal): test fixture duration, not an HTTP request timeout; a shared constant needs a vox-config edge
        let (removed_files, removed_bytes) = prune(root, 150, Duration::from_secs(3600)).unwrap();

        assert_eq!(
            removed_files, 2,
            "should evict oldest-first until under the cap"
        );
        assert_eq!(removed_bytes, 200);
        let remaining: Vec<_> = list_pending(root)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_owned())
            .collect();
        assert!(!remaining.contains(&std::ffi::OsString::from("a.json")));
        assert!(!remaining.contains(&std::ffi::OsString::from("b.json")));
        assert!(remaining.contains(&std::ffi::OsString::from("c.json")));
    }

    #[test]
    fn prune_evicts_files_older_than_max_age_regardless_of_size() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write_pending_file(root, "old.json", 1, Duration::from_secs(10 * 86400));
        // drift-allow(timeout-literal,duplicate-numeric-literal): test fixture duration, not an HTTP request timeout; a shared constant needs a vox-config edge
        write_pending_file(root, "new.json", 1, Duration::from_secs(1));

        let (removed_files, _) = prune(root, u64::MAX, Duration::from_secs(7 * 86400)).unwrap();

        assert_eq!(removed_files, 1);
        assert_eq!(pending_count(root), 1);
    }

    #[test]
    fn prune_is_noop_under_both_caps() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // drift-allow(timeout-literal,duplicate-numeric-literal): test fixture duration, not an HTTP request timeout; a shared constant needs a vox-config edge
        write_pending_file(root, "a.json", 10, Duration::from_secs(1));

        let (removed_files, removed_bytes) =
            prune(root, DEFAULT_MAX_SPOOL_BYTES, DEFAULT_MAX_SPOOL_AGE).unwrap();

        assert_eq!(removed_files, 0);
        assert_eq!(removed_bytes, 0);
        assert_eq!(pending_count(root), 1);
    }

    /// `enqueue` must self-bound the spool: a pre-existing file older than
    /// `DEFAULT_MAX_SPOOL_AGE` gets evicted as a side effect of the next
    /// write, with no separate "vox telemetry prune" step required.
    #[test]
    fn enqueue_self_prunes_age_expired_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write_pending_file(
            root,
            "old.json",
            5,
            // drift-allow(timeout-literal,duplicate-numeric-literal): test fixture duration, not an HTTP request timeout; a shared constant needs a vox-config edge
            DEFAULT_MAX_SPOOL_AGE + Duration::from_secs(60),
        );

        enqueue(root, &serde_json::json!({"k": "v"})).unwrap();

        let remaining: Vec<_> = list_pending(root)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_owned())
            .collect();
        assert!(
            !remaining.contains(&std::ffi::OsString::from("old.json")),
            "enqueue should have pruned the age-expired file"
        );
        assert_eq!(remaining.len(), 1, "only the freshly-enqueued file remains");
    }

    #[test]
    fn preview_pending_renders_batched_otlp_envelope_without_deleting_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        enqueue(
            root,
            &serde_json::json!({
                "resourceLogs": [{
                    "resource": { "attributes": [{ "key": "vox.install_id", "value": { "stringValue": "inst-1" } }] },
                    "scopeLogs": [{ "scope": { "name": "vox-telemetry-otlp" }, "logRecords": [{ "body": { "stringValue": "vox.command" } }] }]
                }]
            }),
        )
        .unwrap();
        enqueue(
            root,
            &serde_json::json!({
                "resourceLogs": [{
                    "resource": { "attributes": [{ "key": "vox.install_id", "value": { "stringValue": "inst-1" } }] },
                    "scopeLogs": [{ "scope": { "name": "vox-telemetry-otlp" }, "logRecords": [{ "body": { "stringValue": "vox.model_call" } }] }]
                }]
            }),
        )
        .unwrap();

        let mut buf = Vec::new();
        let count = preview_pending(root, 10, &mut buf).expect("preview_pending");
        assert_eq!(count, 2);
        assert_eq!(
            pending_count(root),
            2,
            "preview must not delete pending files"
        );
        let parsed: serde_json::Value = serde_json::from_slice(&buf).expect("valid JSON");
        let records = parsed
            .pointer("/resourceLogs/0/scopeLogs/0/logRecords")
            .and_then(|v| v.as_array())
            .expect("logRecords");
        assert_eq!(records.len(), 2);
    }
}
