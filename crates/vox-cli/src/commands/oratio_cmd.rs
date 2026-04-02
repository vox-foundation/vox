//! CLI surface for **`vox oratio`** (speech-to-text).

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum CorrectionProfileCli {
    Conservative,
    Balanced,
    Aggressive,
}

impl From<CorrectionProfileCli> for vox_oratio::refine::OratioCorrectionProfile {
    fn from(value: CorrectionProfileCli) -> Self {
        match value {
            CorrectionProfileCli::Conservative => Self::Conservative,
            CorrectionProfileCli::Balanced => Self::Balanced,
            CorrectionProfileCli::Aggressive => Self::Aggressive,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum RouteModeCli {
    None,
    Tool,
    Chat,
    Orchestrator,
}

fn wait_for_enter_or_timeout(timeout_ms: u64, heartbeat_ms: u64) -> Result<bool> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
        let _ = tx.send(());
    });

    let timeout = Duration::from_millis(timeout_ms);
    let hb = Duration::from_millis(heartbeat_ms.max(500));
    let start = Instant::now();
    loop {
        if rx.recv_timeout(hb).is_ok() {
            return Ok(true);
        }
        let elapsed = start.elapsed();
        if elapsed >= timeout {
            return Ok(false);
        }
        let remaining_ms = timeout.saturating_sub(elapsed).as_millis();
        tracing::info!(
            target: "vox_oratio_cli",
            remaining_ms,
            "Waiting for Enter; session heartbeat"
        );
    }
}

fn append_asr_refine_pair(
    path: &std::path::Path,
    noisy_text: &str,
    corrected_text: &str,
) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid asr_refine output path: {}", path.display()))?;
    std::fs::create_dir_all(parent)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    use std::io::Write;
    writeln!(
        f,
        "{}",
        serde_json::to_string(&serde_json::json!({
            "noisy_text": noisy_text,
            "corrected_text": corrected_text,
        }))?
    )?;
    Ok(())
}

impl From<RouteModeCli> for vox_oratio::RouteMode {
    fn from(value: RouteModeCli) -> Self {
        match value {
            RouteModeCli::None => Self::None,
            RouteModeCli::Tool => Self::Tool,
            RouteModeCli::Chat => Self::Chat,
            RouteModeCli::Orchestrator => Self::Orchestrator,
        }
    }
}

/// Subcommands for Oratio (STT / transcripts).
#[derive(Parser, Debug)]
pub enum OratioAction {
    /// Transcribe a file to text (native STT when enabled; `.txt`/`.md` fixtures always)
    Transcribe {
        /// Audio or transcript fixture path
        path: PathBuf,
        /// Print JSON instead of plain text
        #[arg(long, default_value = "false")]
        json: bool,
        /// Emit refined text when available (default: yes)
        #[arg(long, default_value = "true")]
        refined: bool,
    },
    /// Sessionized transcript workflow: Enter-to-confirm or timeout, then optional route loop.
    Listen {
        /// Audio sample path to transcribe
        path: PathBuf,
        /// Wait this long for Enter before timing out (ms)
        #[arg(long, default_value_t = 12_000)]
        timeout_ms: u64,
        /// Hard cap for whole session duration (ms)
        #[arg(long, default_value_t = 120_000)]
        max_ms: u64,
        /// Stricter cap for transcription + refine only (ms); 0 = use runtime / max_ms
        #[arg(long, default_value_t = 0)]
        inference_deadline_ms: u64,
        /// Optional language hint
        #[arg(long)]
        language: Option<String>,
        /// Correction strictness profile
        #[arg(long, value_enum, default_value_t = CorrectionProfileCli::Balanced)]
        profile: CorrectionProfileCli,
        /// Session route mode after transcript
        #[arg(long, value_enum, default_value_t = RouteModeCli::None)]
        route: RouteModeCli,
        /// Print JSON instead of plain text
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Emit parser payload debug logs
        #[arg(long, default_value_t = false)]
        debug_parser_payload: bool,
        /// Require explicit Enter confirmation before route execution
        #[arg(long, default_value_t = true)]
        confirm_route: bool,
        /// Optional JSONL output path for `asr_refine` pair collection
        #[arg(long)]
        emit_asr_refine: Option<PathBuf>,
    },
    /// Record the default microphone to a WAV file, then run native STT (requires `--features oratio-mic`).
    #[cfg(feature = "oratio-mic")]
    RecordTranscribe {
        /// Seconds of audio to capture (default ~8s; max 300).
        #[arg(long, default_value_t = 8.0)]
        seconds: f32,
        /// Save the WAV here; default is a temp file under the system temp directory.
        #[arg(long)]
        output_wav: Option<PathBuf>,
        /// Print JSON instead of plain text
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Emit refined transcript text when available
        #[arg(long, default_value_t = true)]
        refined: bool,
    },
    /// Diagnose current Oratio runtime and configuration health.
    Doctor,
    /// Show which Oratio backends and passthrough modes are available
    Status,
}

/// Run **`vox oratio …`**.
pub fn run(action: OratioAction, global_json: bool) -> Result<()> {
    let runtime = vox_oratio::resolved_runtime_config();
    match action {
        OratioAction::Transcribe {
            path,
            json,
            refined,
        } => {
            let use_json = json || global_json;
            let t = vox_oratio::transcribe_path(&path)?;
            let text = if refined {
                t.display_text().to_string()
            } else {
                t.raw_text.clone()
            };
            if use_json {
                let payload = serde_json::json!({
                    "path": path,
                    "raw_text": t.raw_text,
                    "refined_text": t.refined_text,
                    "text": text,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("{text}");
            }
            Ok(())
        }
        #[cfg(feature = "oratio-mic")]
        OratioAction::RecordTranscribe {
            seconds,
            output_wav,
            json,
            refined,
        } => {
            let use_json = json || global_json;
            let (wav_path, delete_after) = match output_wav {
                Some(p) => (p, false),
                None => (
                    std::env::temp_dir()
                        .join(format!("vox_oratio_mic_{}.wav", uuid::Uuid::new_v4())),
                    true,
                ),
            };
            crate::commands::oratio_mic::record_default_input_wav(&wav_path, seconds)?;
            let t = vox_oratio::transcribe_path(&wav_path)?;
            let text = if refined {
                t.display_text().to_string()
            } else {
                t.raw_text.clone()
            };
            if delete_after {
                let _ = std::fs::remove_file(&wav_path);
            }
            if use_json {
                let payload = serde_json::json!({
                    "path": wav_path,
                    "raw_text": t.raw_text,
                    "refined_text": t.refined_text,
                    "text": text,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("{text}");
            }
            Ok(())
        }
        OratioAction::Listen {
            path,
            timeout_ms,
            max_ms,
            inference_deadline_ms,
            language,
            profile,
            route,
            json,
            debug_parser_payload,
            confirm_route,
            emit_asr_refine,
        } => {
            let use_json = json || global_json;
            let heartbeat_ms = runtime.session_timing.heartbeat_ms.max(500);
            println!(
                "Provide audio sample at {}. Press Enter to run transcription, waiting up to {} ms...",
                path.display(),
                timeout_ms
            );
            let confirmed = wait_for_enter_or_timeout(timeout_ms, heartbeat_ms)?;
            if !confirmed {
                anyhow::bail!("oratio_capture_timeout: no Enter received within {timeout_ms} ms");
            }
            let session = vox_oratio::transcribe_path_session_with_runtime(
                &path,
                &vox_oratio::OratioSessionConfig {
                    timeout_ms,
                    max_duration_ms: max_ms,
                    inference_deadline_ms: (inference_deadline_ms > 0)
                        .then_some(inference_deadline_ms),
                    language_hint: language,
                    correction_profile: profile.into(),
                    debug_parser_payload,
                    heartbeat_ms,
                    session_id: None,
                },
                runtime,
            )?;
            if let Some(out_path) = emit_asr_refine {
                append_asr_refine_pair(&out_path, &session.raw_text, &session.refined_text)?;
            }
            if confirm_route {
                println!(
                    "Transcript preview:\n{}\nPress Enter to execute route, waiting up to {} ms...",
                    session.text, timeout_ms
                );
                let route_confirmed = wait_for_enter_or_timeout(timeout_ms, heartbeat_ms)?;
                if !route_confirmed {
                    anyhow::bail!(
                        "oratio_capture_timeout: route confirm gate timed out after {timeout_ms} ms"
                    );
                }
            }
            let route_payload = vox_oratio::route_transcript_with_options(
                route.into(),
                &session.session_id,
                &session.text,
                session.confidence,
                runtime,
            );
            if use_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "session": session,
                        "route": route_payload,
                    }))?
                );
            } else {
                println!("{}", session.text);
                println!("{}", serde_json::to_string_pretty(&route_payload.payload)?);
            }
            Ok(())
        }
        OratioAction::Doctor => {
            let mut checks = Vec::new();
            checks.push(serde_json::json!({
                "name": "status_summary",
                "ok": true,
                "value": vox_oratio::transcript_status(),
            }));
            let candle = vox_oratio::candle_backend_status_json();
            checks.push(serde_json::json!({
                "name": "candle_backend",
                "ok": true,
                "value": candle,
            }));
            checks.push(serde_json::json!({
                "name": "runtime_config",
                "ok": true,
                "value": vox_oratio::runtime_config_diagnostic_json(runtime),
            }));
            let payload = serde_json::json!({
                "checks": checks,
                "result": "ok"
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
            Ok(())
        }
        OratioAction::Status => {
            println!("{}", vox_oratio::transcript_status());
            println!(
                "{}",
                serde_json::to_string_pretty(&vox_oratio::candle_backend_status_json())?
            );
            Ok(())
        }
    }
}
