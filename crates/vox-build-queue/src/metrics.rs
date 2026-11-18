//! Build metrics record + summary.
//!
//! Each cargo invocation through the broker appends one JSON line. `summarize`
//! powers `vox build-broker stats`, which yields the go/no-go verdict for the
//! deferred coalescing daemon (Layer 1b).

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
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
    #[serde(default)]
    pub user_cpu_ms: u64,
    #[serde(default)]
    pub sys_cpu_ms: u64,
    #[serde(default)]
    pub max_rss_kb: u64,
    #[serde(default)]
    pub page_faults_major: u64,
    #[serde(default)]
    pub page_faults_minor: u64,
    #[serde(default)]
    pub load_1m_x100: u64,
    #[serde(default)]
    pub effective_jobs: u32,
}

/// Cumulative child-tree resource usage snapshot from `getrusage(RUSAGE_CHILDREN)`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChildrenRusage {
    pub user_cpu_ms: u64,
    pub sys_cpu_ms: u64,
    pub max_rss_kb: u64,
    pub page_faults_major: u64,
    pub page_faults_minor: u64,
}

impl ChildrenRusage {
    /// Delta between a pre-spawn snapshot and a post-exit snapshot.
    /// `max_rss_kb` reports the post-exit high-water mark (monotonic per parent process).
    pub fn delta(&self, after: &Self) -> Self {
        Self {
            user_cpu_ms: after.user_cpu_ms.saturating_sub(self.user_cpu_ms),
            sys_cpu_ms: after.sys_cpu_ms.saturating_sub(self.sys_cpu_ms),
            max_rss_kb: after.max_rss_kb,
            page_faults_major: after
                .page_faults_major
                .saturating_sub(self.page_faults_major),
            page_faults_minor: after
                .page_faults_minor
                .saturating_sub(self.page_faults_minor),
        }
    }
}

/// Sample cumulative `RUSAGE_CHILDREN` metrics on Unix (zeros on non-Unix).
#[allow(unsafe_code)]
pub fn sample_children_rusage() -> ChildrenRusage {
    #[cfg(unix)]
    {
        // SAFETY: zero-initialized rusage struct passed to POSIX `getrusage(RUSAGE_CHILDREN, ...)`.
        let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut ru) };
        if rc != 0 {
            return ChildrenRusage::default();
        }
        let tv_ms = |tv: libc::timeval| -> u64 {
            (tv.tv_sec.max(0) as u64)
                .saturating_mul(1000)
                .saturating_add((tv.tv_usec.max(0) as u64) / 1000)
        };
        let raw_rss = ru.ru_maxrss.max(0) as u64;
        #[cfg(target_os = "macos")]
        let max_rss_kb = raw_rss / 1024;
        #[cfg(not(target_os = "macos"))]
        let max_rss_kb = raw_rss;
        ChildrenRusage {
            user_cpu_ms: tv_ms(ru.ru_utime),
            sys_cpu_ms: tv_ms(ru.ru_stime),
            max_rss_kb,
            page_faults_major: ru.ru_majflt.max(0) as u64,
            page_faults_minor: ru.ru_minflt.max(0) as u64,
        }
    }
    #[cfg(not(unix))]
    {
        ChildrenRusage::default()
    }
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

#[derive(Debug, Default, PartialEq)]
pub struct Summary {
    pub count: usize,
    pub p50_wait_ms: u64,
    pub p95_wait_ms: u64,
    pub p95_target_wait_ms: u64,
    pub p95_slot_wait_ms: u64,
    pub p50_compile_ms: u64,
    pub p50_cpu_ms: u64,
    pub mean_effective_parallelism_x100: u64,
    pub total_major_faults: u64,
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
        let eff_par = self.mean_effective_parallelism_x100 as f64 / 100.0;
        format!(
            "builds={} p50_wait={}ms p95_wait={}ms (target_p95={}ms slot_p95={}ms compile_p50={}ms) coalesce_rate={:.1}% coalesced={} timeouts={} -> {} [cpu_p50={}ms eff_par={:.2}x majflt={}]",
            self.count,
            self.p50_wait_ms,
            self.p95_wait_ms,
            self.p95_target_wait_ms,
            self.p95_slot_wait_ms,
            self.p50_compile_ms,
            self.coalesce_rate * 100.0,
            self.coalesced_hits,
            self.timeouts,
            verdict,
            self.p50_cpu_ms,
            eff_par,
            self.total_major_faults
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
        return Ok(Summary::default());
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

    let mut cpus: Vec<u64> = recs
        .iter()
        .map(|r| r.user_cpu_ms.saturating_add(r.sys_cpu_ms))
        .collect();
    cpus.sort_unstable();

    let mut par_sum: u128 = 0;
    let mut par_count: u128 = 0;
    for r in &recs {
        let wall = if r.compile_ms == 0 && !r.coalesced_hit && r.ran_ms > 0 {
            r.ran_ms
        } else {
            r.compile_ms
        };
        let cpu = r.user_cpu_ms.saturating_add(r.sys_cpu_ms);
        if wall > 0 && cpu > 0 {
            par_sum = par_sum.saturating_add(((cpu as u128) * 100) / (wall as u128));
            par_count += 1;
        }
    }
    let mean_effective_parallelism_x100 = par_sum.checked_div(par_count).unwrap_or(0) as u64;
    let total_major_faults = recs.iter().map(|r| r.page_faults_major).sum();

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
        p50_cpu_ms: percentile(&cpus, 0.50),
        mean_effective_parallelism_x100,
        total_major_faults,
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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

    #[test]
    fn contention_rusage_summary_calculates_cpu_parallelism_and_faults() {
        let mut r = rec(0, false);
        r.compile_ms = 1000;
        r.user_cpu_ms = 4200;
        r.sys_cpu_ms = 800;
        r.max_rss_kb = 128_000;
        r.page_faults_major = 12;
        r.page_faults_minor = 3400;
        r.load_1m_x100 = 1441;
        r.effective_jobs = 11;
        let line = serde_json::to_string(&r).unwrap();
        let s = summarize_str(&line).unwrap();
        assert_eq!(s.p50_cpu_ms, 5000);
        assert_eq!(s.mean_effective_parallelism_x100, 500);
        assert_eq!(s.total_major_faults, 12);
        let rendered = s.render();
        assert!(
            rendered.contains("cpu_p50=5000ms eff_par=5.00x majflt=12"),
            "got: {rendered}"
        );
    }
}
