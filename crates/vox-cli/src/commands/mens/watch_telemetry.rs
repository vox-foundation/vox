//! `vox mens watch-telemetry` — periodic tail of stderr + JSONL training events (replaces `scripts/telemetry_watch.ps1`).

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

        if err_log.is_file() {
            if let Ok(mut f) = std::fs::File::open(&err_log) {
                if let Ok(len) = f.metadata().map(|m| m.len()) {
                    if len > off_err {
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
                }
            }
        }

        if telemetry.is_file() {
            if let Ok(mut f) = std::fs::File::open(&telemetry) {
                if let Ok(len) = f.metadata().map(|m| m.len()) {
                    if len > off_tel {
                        let n = len.saturating_sub(off_tel);
                        if n > 0 {
                            f.seek(SeekFrom::Start(off_tel)).ok();
                            let mut buf = Vec::with_capacity(n as usize);
                            if (&mut f).take(n).read_to_end(&mut buf).is_ok() {
                                off_tel = len;
                                let s = String::from_utf8_lossy(&buf);
                                for line in s.lines() {
                                    if !line.contains("\"event\":\"train")
                                        && !line.contains("\"event\": \"train")
                                    {
                                        continue;
                                    }
                                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                                        if !table_header {
                                            println!(
                                                "\n {:>6} {:>5} {:>10} {:>8} {:>6}",
                                                "step", "ep", "loss", "tok/s", "eta"
                                            );
                                            println!("{}", "-".repeat(52));
                                            table_header = true;
                                        }
                                        let step = v
                                            .get("step")
                                            .or_else(|| v.get("global_step"))
                                            .and_then(|x| x.as_u64())
                                            .unwrap_or(0);
                                        let epoch =
                                            v.get("epoch").and_then(|x| x.as_f64()).unwrap_or(0.0);
                                        let loss =
                                            v.get("loss").and_then(|x| x.as_f64()).unwrap_or(0.0);
                                        let tps = v
                                            .get("tokens_per_sec")
                                            .and_then(|x| x.as_f64())
                                            .map(|x| format!("{x:.1}"))
                                            .unwrap_or_else(|| "—".to_string());
                                        let eta = v
                                            .get("eta_sec")
                                            .and_then(|x| x.as_f64())
                                            .map(format_eta)
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
