//! IPC Dispatcher for the thin Vox CLI.
//!
//! Spawns a daemon binary (`vox-compilerd` or `vox-dei-d`), sends a single
//! `DispatchRequest`, then streams `DispatchResponse` events to the terminal
//! until the daemon emits `Done` or its stdout closes.

use crate::dispatch_protocol::{DispatchPayload, DispatchRequest, DispatchResponse};
use crate::process_supervision::{resolve_managed_binary_path, terminate_process_tree};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

/// Leading text for daemon spawn failures from [`call_daemon`] and [`call_daemon_streaming`].
/// [`crate::dei_daemon`] matches on this (via error display) to append install hints for `vox-dei-d`.
pub const DAEMON_SPAWN_FAILED_PREFIX: &str = "Failed to spawn daemon";

/// Spawn `daemon` (resolved from `$PATH` or sibling to current exe), send a
/// single JSON request, stream all response events to the terminal, and return
/// when the daemon emits `Done` or its stdout closes.
///
/// Exits with an error if the daemon reports a non-zero exit code or an `Error`
/// payload before `Done`.
///
/// Use [`call_daemon_streaming`] for long-lived commands (`dev`) that don't
/// send `Done` until Ctrl+C.
pub async fn call_daemon(
    daemon: &str,
    method: &str,
    params: serde_json::Value,
    auto_open: bool,
) -> anyhow::Result<serde_json::Value> {
    // Resolve the daemon path: prefer a sibling binary first (installed alongside vox),
    // then fall back to PATH lookup so dev builds with `cargo run` still work.
    let daemon_path = resolve_managed_binary_path(daemon);

    let mut child = Command::new(&daemon_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit()) // raw stderr (tracing logs) pass through immediately
        .spawn()
        .map_err(|e| {
            anyhow::anyhow!(
                "{} '{}': {}",
                DAEMON_SPAWN_FAILED_PREFIX,
                daemon_path.display(),
                e
            )
        })?;

    let mut stdin = child.stdin.take().expect("stdin was piped");
    let stdout = child.stdout.take().expect("stdout was piped");

    let req = DispatchRequest {
        id: uuid::Uuid::new_v4().to_string(),
        method: method.into(),
        params,
    };
    let json = serde_json::to_string(&req)? + "\n";
    stdin.write_all(json.as_bytes()).await?;
    stdin.flush().await?;
    // Drop stdin so the daemon sees EOF and knows the request stream has ended
    drop(stdin);

    let mut reader = BufReader::new(stdout).lines();
    let mut final_result: serde_json::Value = serde_json::Value::Null;
    let mut had_error: Option<String> = None;
    let mut exit_code = 0i32;

    while let Ok(Some(line)) = reader.next_line().await {
        // Try to parse as a structured DispatchResponse
        match serde_json::from_str::<DispatchResponse>(&line) {
            Ok(resp) => match resp.payload {
                DispatchPayload::Log { level, msg } => match level.as_str() {
                    "error" | "warn" => eprintln!("[{}] {}", level.to_uppercase(), msg),
                    _ => println!("[{}] {}", level.to_uppercase(), msg),
                },
                DispatchPayload::Diag {
                    severity,
                    message,
                    file,
                    line,
                    col,
                } => {
                    eprintln!(
                        "{}: {}:{}:{} — {}",
                        severity.to_uppercase(),
                        file,
                        line,
                        col,
                        message
                    );
                }
                DispatchPayload::Artifact { path } => {
                    println!("✓ artifact: {}", path);
                }
                DispatchPayload::Progress { percent, status } => {
                    println!("[{:.0}%] {}", percent, status);
                }
                DispatchPayload::Chunk { text } => {
                    use std::io::Write;
                    print!("{}", text);
                    let _ = std::io::stdout().flush();
                }
                DispatchPayload::Result { value } => {
                    final_result = value;
                }
                DispatchPayload::Error { message, code } => {
                    had_error = Some(format!("Daemon error (code {}): {}", code, message));
                }
                DispatchPayload::Done { exit } => {
                    exit_code = exit;
                    break; // Stop reading — daemon is finished
                }
            },
            Err(_) => {
                emit_unstructured_daemon_line(&line, auto_open, false).await;
            }
        }
    }

    // Wait for daemon process to exit cleanly
    let _ = child.wait().await;

    if let Some(err) = had_error {
        anyhow::bail!("{}", err);
    }
    if exit_code != 0 {
        anyhow::bail!("Daemon '{}' exited with code {}", daemon, exit_code);
    }

    Ok(final_result)
}

/// Like [`call_daemon`] but for long-lived commands that talk to `vox-compilerd` until Ctrl+C.
///
/// Used by [`crate::commands::dev`] (`vox dev`). The legacy `commands/runtime/dev` module
/// re-exports the same [`crate::commands::dev::run`] when that tree is wired in.
///
/// Differences from `call_daemon`:
/// - Does **not** bail if the daemon is killed by a signal (SIGINT/SIGTERM).
/// - Does **not** require a `Done` payload — clean exit on EOF.
/// - Forwards `Ctrl+C` tokio signal to the child process before waiting.
pub async fn call_daemon_streaming(
    daemon: &str,
    method: &str,
    params: serde_json::Value,
    auto_open: bool,
) -> anyhow::Result<()> {
    let daemon_path = resolve_managed_binary_path(daemon);

    let mut child = Command::new(&daemon_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| {
            anyhow::anyhow!(
                "{} '{}': {}",
                DAEMON_SPAWN_FAILED_PREFIX,
                daemon_path.display(),
                e
            )
        })?;

    let mut stdin = child.stdin.take().expect("stdin was piped");
    let stdout = child.stdout.take().expect("stdout was piped");

    let req = DispatchRequest {
        id: uuid::Uuid::new_v4().to_string(),
        method: method.into(),
        params,
    };
    let json = serde_json::to_string(&req)? + "\n";
    stdin.write_all(json.as_bytes()).await?;
    stdin.flush().await?;
    drop(stdin);

    // Spawn a Ctrl+C listener that kills the child when the user interrupts
    let child_id = child.id();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok()
            && let Some(pid) = child_id
        {
            // Best-effort: send SIGINT/SIGKILL to the child
            let _ = terminate_process_tree(pid);
        }
    });

    let mut reader = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        match serde_json::from_str::<DispatchResponse>(&line) {
            Ok(resp) => match resp.payload {
                DispatchPayload::Log { level, msg } => match level.as_str() {
                    "error" | "warn" => eprintln!("[{}] {}", level.to_uppercase(), msg),
                    _ => println!("[{}] {}", level.to_uppercase(), msg),
                },
                DispatchPayload::Diag {
                    severity,
                    message,
                    file,
                    line,
                    col,
                } => {
                    eprintln!(
                        "{}: {}:{}:{} — {}",
                        severity.to_uppercase(),
                        file,
                        line,
                        col,
                        message
                    );
                }
                DispatchPayload::Progress { percent, status } => {
                    println!("[{:.0}%] {}", percent, status);
                }
                DispatchPayload::Artifact { path } => {
                    println!("✓ {}", path);
                }
                DispatchPayload::Error { message, code } => {
                    eprintln!("[ERROR {}] {}", code, message);
                }
                DispatchPayload::Done { .. } => break, // dev finished
                DispatchPayload::Chunk { text } => {
                    use std::io::Write;
                    print!("{}", text);
                    let _ = std::io::stdout().flush();
                }
                DispatchPayload::Result { .. } => {}
            },
            Err(_) => {
                emit_unstructured_daemon_line(&line, auto_open, true).await;
            }
        }
    }

    // Wait for the child — ignore signal-killed status (non-zero)
    let _ = child.wait().await;
    Ok(())
}

async fn emit_unstructured_daemon_line(line: &str, auto_open: bool, app_launched_banner: bool) {
    if let Some(pos) = line.find("[VOX_DASHBOARD_READY: ") {
        if auto_open {
            let url = &line[pos + 22..];
            if let Some(end) = url.find(']') {
                vox_cli_core::fs_utils::open_browser(url[..end].trim()).await;
            }
        }
        println!("\n  ↳  Dashboard ready: {}", line.trim());
    } else if line.contains("http://") || line.contains("App Launched at") {
        if auto_open {
            if let Some(pos) = line.find("http://") {
                let rest = &line[pos..];
                let extracted_url = rest.split_whitespace().next().unwrap_or(rest);
                vox_cli_core::fs_utils::open_browser(extracted_url.trim_end_matches(']')).await;
            }
        }
        if app_launched_banner {
            println!("\n  ↳  App launched: {}", line.trim());
        } else {
            println!("{}", line);
        }
    } else {
        println!("{}", line);
    }
}
