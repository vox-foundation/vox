//! Build metrics record + summary.
//!
//! Each cargo invocation through the broker appends one JSON line. `summarize`
//! powers `vox build-broker stats`, which yields the go/no-go verdict for the
//! deferred coalescing daemon (Layer 1b).

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricRecord {
    pub ts_ms: u128,
    pub worktree: String,
    pub subcmd: String,
    pub queue_wait_ms: u64,
    pub ran_ms: u64,
    pub argv_hash: u64,
    pub env_hash: u64,
    pub would_coalesce: bool,
    #[serde(default)]
    pub target_wait_ms: u64,
    #[serde(default)]
    pub slot_wait_ms: u64,
    #[serde(default)]
    pub compile_ms: u64,
    #[serde(default)]
    pub coalesced_hit: bool,
    #[serde(default)]
    pub timed_out: bool,
}

/// Wall-clock milliseconds since the Unix epoch (0 if the clock is before it).
pub fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Append one JSON line to the metrics file (created with parents if absent).
pub fn append(path: &Path, rec: &MetricRecord) -> anyhow::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{}", serde_json::to_string(rec)?)?;
    Ok(())
}

#[derive(Debug, PartialEq)]
pub struct Summary {
    pub count: usize,
    pub p50_wait_ms: u64,
    pub p95_wait_ms: u64,
    pub p95_target_wait_ms: u64,
    pub p95_slot_wait_ms: u64,
    pub p50_compile_ms: u64,
    pub coalesce_rate: f64,
    pub coalesced_hits: usize,
    pub timeouts: usize,
}

impl Summary {
    /// Go/no-go for the deferred coalescing daemon (Layer 1b): worth building
    /// only if a material share of invocations had a coalescing opportunity AND
    /// queue waits actually occurred.
    pub fn daemon_recommended(&self) -> bool {
        self.coalesce_rate >= 0.10 && self.p50_wait_ms > 0
    }

    /// One-line human summary for `vox build-broker stats`.
    pub fn render(&self) -> String {
        let verdict = if self.daemon_recommended() {
            "BUILD-DAEMON: recommended (coalesce>=10% and queue waits present)"
        } else {
            "BUILD-DAEMON: not needed (daemonless shim sufficient)"
        };
        format!(
            "builds={} p50_wait={}ms p95_wait={}ms (target_p95={}ms slot_p95={}ms compile_p50={}ms) coalesce_rate={:.1}% coalesced={} timeouts={} -> {}",
            self.count,
            self.p50_wait_ms,
            self.p95_wait_ms,
            self.p95_target_wait_ms,
            self.p95_slot_wait_ms,
            self.p50_compile_ms,
            self.coalesce_rate * 100.0,
            self.coalesced_hits,
            self.timeouts,
            verdict
        )
    }
}

/// Read all valid `MetricRecord` lines from a `metrics.jsonl` file, skipping
/// missing files or malformed lines.
pub fn read_records(path: &Path) -> Vec<MetricRecord> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Read a metrics.jsonl file and summarize. Malformed lines are skipped so a
/// partially-written tail never breaks `stats`.
pub fn summarize(path: &Path) -> anyhow::Result<Summary> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    summarize_str(&text)
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[((sorted.len() as f64 - 1.0) * p).round() as usize]
}

/// Summarize already-loaded JSONL content (used when aggregating across dirs).
pub fn summarize_str(text: &str) -> anyhow::Result<Summary> {
    let recs: Vec<MetricRecord> = text
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let count = recs.len();
    if count == 0 {
        return Ok(Summary {
            count: 0,
            p50_wait_ms: 0,
            p95_wait_ms: 0,
            p95_target_wait_ms: 0,
            p95_slot_wait_ms: 0,
            p50_compile_ms: 0,
            coalesce_rate: 0.0,
            coalesced_hits: 0,
            timeouts: 0,
        });
    }
    let mut waits: Vec<u64> = recs.iter().map(|r| r.queue_wait_ms).collect();
    waits.sort_unstable();

    let mut target_waits: Vec<u64> = recs.iter().map(|r| r.target_wait_ms).collect();
    target_waits.sort_unstable();

    let mut slot_waits: Vec<u64> = recs.iter().map(|r| r.slot_wait_ms).collect();
    slot_waits.sort_unstable();

    let mut compiles: Vec<u64> = recs
        .iter()
        .map(|r| {
            if r.compile_ms == 0 && !r.coalesced_hit && r.ran_ms > 0 {
                r.ran_ms
            } else {
                r.compile_ms
            }
        })
        .collect();
    compiles.sort_unstable();

    let coalesce = recs.iter().filter(|r| r.would_coalesce).count() as f64 / count as f64;
    let coalesced_hits = recs.iter().filter(|r| r.coalesced_hit).count();
    let timeouts = recs.iter().filter(|r| r.timed_out).count();

    Ok(Summary {
        count,
        p50_wait_ms: percentile(&waits, 0.50),
        p95_wait_ms: percentile(&waits, 0.95),
        p95_target_wait_ms: percentile(&target_waits, 0.95),
        p95_slot_wait_ms: percentile(&slot_waits, 0.95),
        p50_compile_ms: percentile(&compiles, 0.50),
        coalesce_rate: coalesce,
        coalesced_hits,
        timeouts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(wait: u64, coalesce: bool) -> MetricRecord {
        MetricRecord {
            ts_ms: 0,
            worktree: "wt".into(),
            subcmd: "test".into(),
            queue_wait_ms: wait,
            ran_ms: 100,
            argv_hash: 1,
            env_hash: 2,
            would_coalesce: coalesce,
            target_wait_ms: wait,
            slot_wait_ms: 0,
            compile_ms: 100,
            coalesced_hit: false,
            timed_out: false,
        }
    }

    #[test]
    fn append_then_summarize() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("metrics.jsonl");
        for (w, c) in [(0, false), (100, true), (200, false), (300, false)] {
            append(&p, &rec(w, c)).unwrap();
        }
        let s = summarize(&p).unwrap();
        assert_eq!(s.count, 4);
        assert_eq!(s.p50_wait_ms, 200); // round((4-1)*0.5)=2 -> sorted[2]=200
        assert!((s.coalesce_rate - 0.25).abs() < 1e-9);
    }

    #[test]
    fn summarize_missing_file_is_empty() {
        let s = summarize(Path::new("does-not-exist.jsonl")).unwrap();
        assert_eq!(s.count, 0);
    }

    #[test]
    fn verdict_not_needed_without_coalesce_or_waits() {
        let s = Summary {
            count: 10,
            p50_wait_ms: 0,
            p95_wait_ms: 0,
            p95_target_wait_ms: 0,
            p95_slot_wait_ms: 0,
            p50_compile_ms: 0,
            coalesce_rate: 0.0,
            coalesced_hits: 0,
            timeouts: 0,
        };
        assert!(!s.daemon_recommended());
        assert!(s.render().contains("not needed"));
    }

    #[test]
    fn verdict_recommended_with_coalesce_and_waits() {
        let s = Summary {
            count: 10,
            p50_wait_ms: 50,
            p95_wait_ms: 200,
            p95_target_wait_ms: 180,
            p95_slot_wait_ms: 20,
            p50_compile_ms: 100,
            coalesce_rate: 0.2,
            coalesced_hits: 2,
            timeouts: 1,
        };
        assert!(s.daemon_recommended());
        assert!(s.render().contains("recommended"));
    }

    #[test]
    fn malformed_lines_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("m.jsonl");
        std::fs::write(&p, "not json\n").unwrap();
        append(&p, &rec(5, false)).unwrap();
        assert_eq!(summarize(&p).unwrap().count, 1);
    }

    #[test]
    fn legacy_8_field_jsonl_deserializes_and_renders_split_summary() {
        let raw = concat!(
            r#"{"ts_ms":1,"worktree":"w","subcmd":"check","queue_wait_ms":0,"ran_ms":420,"argv_hash":1,"env_hash":2,"would_coalesce":false}"#,
            "\n",
            r#"{"ts_ms":2,"worktree":"w","subcmd":"check","queue_wait_ms":300,"ran_ms":0,"argv_hash":1,"env_hash":2,"would_coalesce":true,"target_wait_ms":250,"slot_wait_ms":50,"compile_ms":0,"coalesced_hit":true,"timed_out":false}"#,
            "\n",
            r#"{"ts_ms":3,"worktree":"w","subcmd":"test","queue_wait_ms":100,"ran_ms":900,"argv_hash":3,"env_hash":2,"would_coalesce":false,"target_wait_ms":80,"slot_wait_ms":20,"compile_ms":900,"coalesced_hit":false,"timed_out":true}"#,
            "\n"
        );
        let s = summarize_str(raw).unwrap();
        assert_eq!(s.count, 3);
        assert_eq!(s.p95_target_wait_ms, 250);
        assert_eq!(s.p95_slot_wait_ms, 50);
        assert_eq!(s.p50_compile_ms, 420); // sorted [0, 420, 900] -> 420 from legacy ran_ms fallback
        assert_eq!(s.coalesced_hits, 1);
        assert_eq!(s.timeouts, 1);
        let rendered = s.render();
        assert!(
            rendered.contains(
                "(target_p95=250ms slot_p95=50ms compile_p50=420ms) coalesce_rate=33.3% coalesced=1 timeouts=1 ->"
            ),
            "got: {rendered}"
        );
    }
}
