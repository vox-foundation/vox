//! `train-record.json`: the D-08 record every local `vox mens train` run leaves
//! in its output directory, success or failure. The status comes only from
//! evidence this run wrote: a `run_summary.json` modified after the run started
//! and telemetry rows stamped after it, so a reused directory cannot turn a
//! failed run into a trained one.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use vox_populi::mens::tensor::size_select::SizePick;

/// Everything the record states that the run directory does not.
pub(crate) struct RecordInput {
    pub output_dir: PathBuf,
    pub started: SystemTime,
    pub total_wall_secs: f64,
    pub requested_model: Option<String>,
    pub pin: Option<String>,
    pub size: Option<String>,
    pub preset: Option<String>,
    pub explicit: Option<bool>,
    pub epochs: Option<usize>,
    pub seq_len: Option<usize>,
    pub max_runtime_secs: Option<u64>,
    pub host: String,
    pub error: Option<String>,
}

impl RecordInput {
    /// The facts fixed before `train::run_train` starts, stamped with its start.
    pub(crate) fn start(epochs: Option<usize>, seq_len: Option<usize>, cap: Option<u64>) -> Self {
        Self {
            output_dir: PathBuf::new(),
            started: SystemTime::now(),
            total_wall_secs: 0.0,
            requested_model: None,
            pin: None,
            size: None,
            preset: None,
            explicit: None,
            epochs,
            seq_len,
            max_runtime_secs: cap,
            host: String::new(),
            error: None,
        }
    }

    /// Complete the input once the run returned: the final size pick when it
    /// returned one, else only what was requested, plus the error text.
    pub(crate) fn finish(
        self,
        output_dir: &Path,
        model: &Option<String>,
        cli_preset: &Option<String>,
        res: &anyhow::Result<Option<SizePick>>,
    ) -> Self {
        let pick = res.as_ref().ok().and_then(Option::as_ref);
        Self {
            output_dir: output_dir.to_path_buf(),
            total_wall_secs: self.started.elapsed().map_or(0.0, |d| d.as_secs_f64()),
            requested_model: model.clone(),
            pin: pick.map(|p| p.rung.hf_id.clone()),
            size: pick.map(|p| p.rung.size.clone()),
            preset: super::train_arm::trained_preset(cli_preset.clone(), pick),
            explicit: pick.map(|p| p.explicit),
            host: vox_populi::mens::probe_gpu().model_name,
            error: res.as_ref().err().map(|e| format!("{e:#}")),
            ..self
        }
    }
}

fn unix_ms(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// `run_summary.json` when this run wrote it (modified at or after `started`).
fn fresh_summary(dir: &Path, started: SystemTime) -> Option<Value> {
    let path = dir.join("run_summary.json");
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok()?;
    if modified < started {
        return None;
    }
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Highest step peak or OOM observation in telemetry rows stamped at or after
/// `started_ms`; rows without a timestamp count as older.
fn telemetry_peak_bytes(dir: &Path, started_ms: i64) -> Option<u64> {
    let text = std::fs::read_to_string(dir.join("telemetry.jsonl")).ok()?;
    let mut peak = None;
    for row in text.lines().flat_map(serde_json::from_str::<Value>) {
        let ts = row["ts_ms"].as_i64().unwrap_or(i64::MIN);
        if ts < started_ms {
            continue;
        }
        for key in ["peak_metal_allocated_bytes", "observed_bytes"] {
            peak = peak.max(row["payload"][key].as_u64().filter(|&b| b > 0));
        }
    }
    peak
}

const NO_SUMMARY: &str = "no run_summary.json from this run (paused or interrupted)";

fn record_status(train_ok: bool, summary: Option<&Value>) -> &'static str {
    match summary {
        Some(s) if train_ok && s["capped"] == true => "capped",
        Some(_) if train_ok => "trained",
        _ => "not_trained",
    }
}

pub(crate) fn build_train_record(input: &RecordInput, train_ok: bool) -> Value {
    let dir = &input.output_dir;
    let summary = fresh_summary(dir, input.started);
    let status = record_status(train_ok, summary.as_ref());
    let reason = match (&input.error, &summary) {
        (Some(e), _) => Value::from(e.as_str()),
        (None, None) if train_ok => NO_SUMMARY.into(),
        _ => Value::Null,
    };
    let s = summary.unwrap_or(Value::Null);
    let (peak, source) = match s["peak_metal_allocated_bytes"].as_u64().filter(|&b| b > 0) {
        Some(b) => (Some(b), Some("run_summary")),
        None => {
            let t = telemetry_peak_bytes(dir, unix_ms(input.started));
            (t, t.map(|_| "telemetry"))
        }
    };
    let adapter = dir.join("candle_qlora_adapter.safetensors");
    let adapter =
        (status != "not_trained" && adapter.is_file()).then(|| adapter.display().to_string());
    serde_json::json!({
        "schema": "vox_mens_train_record_v1",
        "status": status,
        "reason": reason,
        "requested_model": input.requested_model,
        "pin": input.pin,
        "size": input.size,
        "preset": input.preset,
        "explicit": input.explicit,
        "epochs": input.epochs,
        "seq_len": input.seq_len,
        "max_runtime_secs": input.max_runtime_secs,
        "optimizer_steps": s["optimizer_step_count"],
        "micro_steps": s["global_step"],
        "final_loss_ema": s["final_loss_ema"],
        "avg_train_loss": s["avg_train_loss"],
        "peak_mb": peak.map(|b| b.div_ceil(1 << 20)),
        "peak_source": source,
        "train_wall_secs": s["wall_secs"],
        "total_wall_secs": input.total_wall_secs,
        "truncated_rows": s["truncated_rows"],
        "prompt_trimmed_rows": s["prompt_trimmed_rows"],
        "adapter": adapter,
        "host": input.host,
        "recorded_at": chrono::Utc::now().to_rfc3339(),
    })
}

pub(crate) fn write_train_record(input: &RecordInput, train_ok: bool) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(&input.output_dir)?;
    let path = input.output_dir.join("train-record.json");
    let record = build_train_record(input, train_ok);
    std::fs::write(&path, serde_json::to_string_pretty(&record)?)?;
    Ok(path)
}

/// Print where the record went; a failed write only warns and never changes
/// the run's result.
pub(crate) fn report_record(res: anyhow::Result<PathBuf>) {
    use owo_colors::OwoColorize;
    match res {
        Ok(path) => eprintln!("  {} Wrote train record {}", "✓".green(), path.display()),
        Err(e) => eprintln!("  {} No train-record.json: {e:#}", "⚠️".yellow()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    fn input(dir: &Path, started: SystemTime, error: Option<&str>) -> RecordInput {
        let mut i = RecordInput::start(None, None, None);
        (i.output_dir, i.started, i.error) = (dir.to_path_buf(), started, error.map(Into::into));
        i
    }

    fn write_summary(dir: &Path, capped: bool) {
        let s = json!({
            "capped": capped, "optimizer_step_count": 12, "global_step": 96,
            "final_loss_ema": 1.25, "peak_metal_allocated_bytes": 1_073_741_825u64,
            "wall_secs": 300.0,
        });
        std::fs::write(dir.join("run_summary.json"), s.to_string()).unwrap();
    }

    /// Append one telemetry row after an unparsable line (which must be skipped).
    fn telemetry(dir: &Path, ts_ms: i64, key: &str, gib: u64) {
        let p = dir.join("telemetry.jsonl");
        let old = std::fs::read_to_string(&p).unwrap_or_default();
        let row = json!({"ts_ms": ts_ms, "event": "x", "payload": {key: gib << 30}});
        std::fs::write(&p, format!("{old}not json\n{row}\n")).unwrap();
    }

    #[test]
    fn fresh_summary_marks_trained_with_steps_loss_and_peak() {
        let dir = tempfile::tempdir().unwrap();
        let started = SystemTime::now() - Duration::from_secs(5);
        write_summary(dir.path(), false);
        let r = build_train_record(&input(dir.path(), started, None), true);
        assert_eq!(r["schema"], "vox_mens_train_record_v1");
        assert_eq!(r["status"], "trained");
        assert_eq!(r["optimizer_steps"], 12);
        assert_eq!(r["micro_steps"], 96);
        assert_eq!(r["final_loss_ema"], 1.25);
        assert_eq!(r["peak_mb"], 1025);
        assert_eq!(r["peak_source"], "run_summary");
        assert_eq!(r["train_wall_secs"], 300.0);
        assert!(r["reason"].is_null());
    }

    #[test]
    fn capped_summary_marks_capped() {
        let dir = tempfile::tempdir().unwrap();
        let started = SystemTime::now() - Duration::from_secs(5);
        write_summary(dir.path(), true);
        let r = build_train_record(&input(dir.path(), started, None), true);
        assert_eq!(r["status"], "capped");
    }

    #[test]
    fn failed_run_is_not_trained_with_error_and_telemetry_peak() {
        let dir = tempfile::tempdir().unwrap();
        let started = SystemTime::now() - Duration::from_secs(5);
        let ts = unix_ms(started) + 1000;
        telemetry(dir.path(), ts, "peak_metal_allocated_bytes", 2);
        telemetry(dir.path(), ts + 1, "observed_bytes", 3);
        let r = build_train_record(&input(dir.path(), started, Some("Metal OOM")), false);
        assert_eq!(r["status"], "not_trained");
        assert_eq!(r["peak_mb"], 3072);
        assert_eq!(r["peak_source"], "telemetry");
        assert_eq!(r["reason"], "Metal OOM");
        assert!(r["adapter"].is_null());
    }

    #[test]
    fn ok_without_summary_is_not_trained() {
        let dir = tempfile::tempdir().unwrap();
        let started = SystemTime::now() - Duration::from_secs(5);
        let r = build_train_record(&input(dir.path(), started, None), true);
        assert_eq!(r["status"], "not_trained");
        assert!(r["reason"].as_str().unwrap().contains("run_summary.json"));
    }

    #[test]
    fn stale_summary_from_an_earlier_run_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        write_summary(dir.path(), false);
        let started = SystemTime::now() + Duration::from_secs(10);
        let r = build_train_record(&input(dir.path(), started, None), true);
        assert_eq!(r["status"], "not_trained");
        assert!(r["optimizer_steps"].is_null());
    }

    #[test]
    fn stale_telemetry_rows_do_not_count() {
        let dir = tempfile::tempdir().unwrap();
        let started = SystemTime::now();
        telemetry(dir.path(), unix_ms(started) - 60_000, "observed_bytes", 3);
        let r = build_train_record(&input(dir.path(), started, Some("boom")), false);
        assert_eq!(r["status"], "not_trained");
        assert!(r["peak_mb"].is_null());
        assert!(r["peak_source"].is_null());
    }

    #[test]
    fn write_puts_the_record_in_the_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("run");
        let path = write_train_record(&input(&out, SystemTime::now(), Some("x")), false).unwrap();
        assert_eq!(path, out.join("train-record.json"));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(v["status"], "not_trained");
    }
}
