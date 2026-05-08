//! `vox mens watch-telemetry` — periodic tail of stderr + JSONL training events (replaces `scripts/telemetry_watch.ps1`).
//!
//! JSONL lines follow Populi `telemetry::append`: `{ "ts_ms", "event", "payload" }` (see
//! `crates/vox-populi/src/mens/tensor/telemetry.rs` + `telemetry_schema.rs`).
//! CI guards (`vox ci data-ssot-guards`) assert this CLI still parses keys such as `eta_seconds_remaining`
//! and `steps_per_sec_ema` from `telemetry_schema` payloads — keep them in sync when renaming fields.

use regex::Regex;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::Duration;

/// Runs until Ctrl+C (process exit). Diverges on normal paths.
pub fn run(telemetry: PathBuf, err_log: PathBuf, interval_ms: u64) -> ! {
    let mut off_err: u64 = 0;
    let mut off_tel: u64 = 0;
    let mut table_header = false;
    let ansi = Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("ansi regex");

    eprintln!(
        "Mens watch-telemetry (every {}ms) — Ctrl+C to stop",
        interval_ms
    );
    eprintln!("  stderr: {}", err_log.display());
    eprintln!("  telemetry: {}", telemetry.display());

    loop {
        std::thread::sleep(Duration::from_millis(interval_ms));

        if err_log.is_file()
            && let Ok(mut f) = std::fs::File::open(&err_log)
            && let Ok(len) = f.metadata().map(|m| m.len())
            && len > off_err
        {
            let n = len.saturating_sub(off_err);
            if n > 0 {
                f.seek(SeekFrom::Start(off_err)).ok();
                let mut buf = Vec::with_capacity(n as usize);
                if (&mut f).take(n).read_to_end(&mut buf).is_ok() {
                    off_err = len;
                    let s = String::from_utf8_lossy(&buf);
                    for line in s.lines() {
                        let clean = ansi.replace_all(line, "");
                        let t = clean.trim();
                        if t.is_empty() {
                            continue;
                        }
                        if let Some(msg) = summarize_err_line(t) {
                            println!("[stderr] {msg}");
                        }
                    }
                }
            }
        }

        if telemetry.is_file()
            && let Ok(mut f) = std::fs::File::open(&telemetry)
            && let Ok(len) = f.metadata().map(|m| m.len())
            && len > off_tel
        {
            let n = len.saturating_sub(off_tel);
            if n > 0 {
                f.seek(SeekFrom::Start(off_tel)).ok();
                let mut buf = Vec::with_capacity(n as usize);
                if (&mut f).take(n).read_to_end(&mut buf).is_ok() {
                    off_tel = len;
                    let s = String::from_utf8_lossy(&buf);
                    for line in s.lines() {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                            let event = v.get("event").and_then(|x| x.as_str()).unwrap_or("");
                            if !matches!(
                                event,
                                "train_start" | "step" | "train_complete" | "gpu_fallback"
                            ) {
                                continue;
                            }
                            // Progress table: only stepping events carry the numeric payload.
                            if event != "step" {
                                println!("[telemetry] {event}");
                                continue;
                            }
                            let payload = v.get("payload").cloned().unwrap_or_else(|| v.clone());
                            if !table_header {
                                println!(
                                    "\n {:>6} {:>5} {:>10} {:>8} {:>6}",
                                    "step", "ep", "loss", "stp/s", "eta"
                                );
                                println!("{}", "-".repeat(52));
                                table_header = true;
                            }
                            let step = payload.get("step").and_then(|x| x.as_u64()).unwrap_or(0);
                            let epoch =
                                payload.get("epoch").and_then(|x| x.as_f64()).unwrap_or(0.0);
                            let loss = payload.get("loss").and_then(|x| x.as_f64()).unwrap_or(0.0);
                            let tps = payload
                                .get("steps_per_sec_ema")
                                .and_then(|x| x.as_f64())
                                .map(|x| format!("{x:.2}"))
                                .unwrap_or_else(|| "—".to_string());
                            let eta_sec = payload.get("eta_seconds_remaining").and_then(|x| {
                                x.as_u64()
                                    .or_else(|| x.as_f64().map(|f| f.max(0.0).round() as u64))
                            });
                            let eta = eta_sec
                                .map(|s| format_eta(s as f64))
                                .unwrap_or_else(|| "…".to_string());
                            println!(
                                " {:>6} {:>5.1} {:>10.4} {:>8} {:>6}",
                                step, epoch, loss, tps, eta
                            );
                        }
                    }
                }
            }
        }

        if !err_log.exists() && !telemetry.exists() {
            print!("\r[waiting for log files…] ");
            let _ = std::io::stdout().flush();
        }
    }
}

fn summarize_err_line(s: &str) -> Option<String> {
    if s.contains("QLoRA preflight OK") {
        return Some("preflight OK".into());
    }
    if s.contains("GPU") && s.contains("CPU fallback") {
        return Some("GPU fell back to CPU".into());
    }
    if s.contains("Error:") || s.contains("panicked at") {
        return Some(s.chars().take(120).collect());
    }
    if s.contains("no training rows") {
        return Some("empty dataset".into());
    }
    if s.contains("[Epoch ") {
        return Some(s.chars().take(100).collect());
    }
    if s.contains("Finished") && s.contains("release") {
        return Some("cargo: finished release build".into());
    }
    if s.contains("Compiling ") {
        return Some(s.chars().take(80).collect());
    }
    None
}

fn format_eta(sec: f64) -> String {
    if sec >= 3600.0 {
        format!("{:.1}h", sec / 3600.0)
    } else if sec >= 60.0 {
        format!("{:.0}m", sec / 60.0)
    } else {
        format!("{:.0}s", sec)
    }
}
