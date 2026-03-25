//! Cargo lock / spawn telemetry (BL009). Replaces the removed `vox-build-lock` crate for CLI builds.
//!
//! When `VOX_LOCK_TELEMETRY=1`, events append as JSON lines under `~/.vox/lock-telemetry/events.jsonl`.
//!
//! Feature sets (`stub-check`/`codex` lock-report vs future script-execution hooks) do not reference every symbol.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

fn vox_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vox")
}

/// Directory containing `events.jsonl`.
#[must_use]
pub fn default_base() -> PathBuf {
    vox_home().join("lock-telemetry")
}

fn events_path() -> PathBuf {
    default_base().join("events.jsonl")
}

#[must_use]
pub fn is_telemetry_enabled() -> bool {
    std::env::var("VOX_LOCK_TELEMETRY").ok().as_deref() == Some("1")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEventLine {
    pub kind: String,
    #[serde(default)]
    pub wait_ms: Option<u64>,
    #[serde(default)]
    pub symptom: Option<String>,
}

/// Append one telemetry line (no-op when disabled).
///
/// Reserved for script runners that detect overlapping `cargo`/spawn contention; not yet wired from all paths.
#[allow(dead_code)]
pub fn record_proc_spawn_conflict(symptom: &str) {
    if !is_telemetry_enabled() {
        tracing::debug!(
            target: "vox_cli::lock",
            symptom = symptom,
            "spawn conflict (VOX_LOCK_TELEMETRY off)"
        );
        return;
    }
    let base = default_base();
    if std::fs::create_dir_all(&base).is_err() {
        return;
    }
    let line = serde_json::to_string(&LockEventLine {
        kind: "proc_spawn_conflict".into(),
        wait_ms: None,
        symptom: Some(symptom.to_string()),
    })
    .unwrap_or_else(|_| r#"{"kind":"proc_spawn_conflict"}"#.to_string());
    let path = events_path();
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{line}");
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LockReport {
    pub enabled: bool,
    pub count: usize,
    pub package_cache_wait_count: u64,
    pub build_dir_wait_count: u64,
    pub p50_ms: Option<u64>,
    pub p95_ms: Option<u64>,
}

#[derive(Debug, Default)]
struct Agg {
    waits: Vec<u64>,
    package_cache_wait_count: u64,
    build_dir_wait_count: u64,
}

impl Agg {
    fn feed(&mut self, ev: &LockEventLine) {
        match ev.kind.as_str() {
            "package_cache_wait" => {
                self.package_cache_wait_count += 1;
                if let Some(ms) = ev.wait_ms {
                    self.waits.push(ms);
                }
            }
            "build_dir_wait" | "proc_spawn_conflict" => {
                self.build_dir_wait_count += 1;
                if let Some(ms) = ev.wait_ms {
                    self.waits.push(ms);
                }
            }
            _ => {}
        }
    }

    fn report(self, enabled: bool, count: usize) -> LockReport {
        let mut w = self.waits;
        w.sort_unstable();
        let p50_ms = percentile(&w, 50);
        let p95_ms = percentile(&w, 95);
        LockReport {
            enabled,
            count,
            package_cache_wait_count: self.package_cache_wait_count,
            build_dir_wait_count: self.build_dir_wait_count,
            p50_ms,
            p95_ms,
        }
    }
}

fn percentile(sorted: &[u64], p: u8) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let idx = ((sorted.len() as f64 - 1.0) * (p as f64 / 100.0)).round() as usize;
    Some(sorted[idx.min(sorted.len() - 1)])
}

pub struct Metrics {
    inner: LockReport,
}

impl Metrics {
    #[must_use]
    pub fn report(&self) -> &LockReport {
        &self.inner
    }
}

/// Read up to `limit` recent JSON lines from `events.jsonl` under `base`.
pub fn aggregate_metrics(base: &Path, limit: u64) -> Metrics {
    let path = base.join("events.jsonl");
    let enabled = is_telemetry_enabled();
    let mut agg = Agg::default();
    let mut count = 0usize;

    if let Ok(data) = crate::commands::ci::bounded_read::read_utf8_path_capped(&path) {
        let lines: Vec<&str> = data.lines().filter(|l| !l.is_empty()).collect();
        let take = limit as usize;
        let start = lines.len().saturating_sub(take);
        for line in lines.into_iter().skip(start) {
            if let Ok(ev) = serde_json::from_str::<LockEventLine>(line) {
                agg.feed(&ev);
                count += 1;
            }
        }
    }

    Metrics {
        inner: agg.report(enabled, count),
    }
}
