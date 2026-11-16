//! Warp terminal helpers (`TERM-WARP-02..04`) and internal PTY command parsers.

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::Emitter;
use vox_terminal_core::history::DEFAULT_HISTORY_CAP;
use vox_terminal_core::{
    Attachment, BlockId, BlockStatus, CompletionEngine, HistoryEntry, HistoryStore,
    RunbookDescriptor, SessionEvent, SessionHost, TermCompletionItem, TermCompletionKind,
    TermCompletionRequest, TermControlMode, TermSecurityLevel, Utf8StreamDecoder,
};

use super::{
    PTY_EXIT_EVENT, PTY_OUTPUT_EVENT, TERM_EVENT_EVENT, TERM_OUTPUT_EVENT, TermEventPayload,
    TermOutputPayload, decode_stream_chunk,
};

#[derive(serde::Serialize, Clone)]
struct PtyChunk {
    tab_id: String,
    data: String,
}

#[derive(serde::Serialize, Clone)]
struct PtyExit {
    tab_id: String,
}

pub(super) fn spawn_pty_forwarder(
    app_handle: tauri::AppHandle,
    id: String,
    mut rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
) {
    tokio::spawn(async move {
        let mut decoder = Utf8StreamDecoder::new();
        while let Some(bytes) = rx.recv().await {
            let data = decode_stream_chunk(&mut decoder, &bytes);
            if !data.is_empty() {
                let _ = app_handle.emit(
                    PTY_OUTPUT_EVENT,
                    PtyChunk {
                        tab_id: id.clone(),
                        data,
                    },
                );
            }
        }
        let tail = decoder.flush_lossy();
        if !tail.is_empty() {
            let _ = app_handle.emit(
                PTY_OUTPUT_EVENT,
                PtyChunk {
                    tab_id: id.clone(),
                    data: tail,
                },
            );
        }
        let _ = app_handle.emit(PTY_EXIT_EVENT, PtyExit { tab_id: id });
    });
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Clone)]
pub struct PtyWarpState {
    pub history: Arc<HistoryStore>,
    pub completions: Arc<CompletionEngine>,
}

impl PtyWarpState {
    pub fn new_default() -> Self {
        let history_path = vox_config::paths::user_home_dir()
            .join(".vox")
            .join("terminal")
            .join("history.jsonl");
        let store = HistoryStore::open(history_path, DEFAULT_HISTORY_CAP)
            .unwrap_or_else(|_| HistoryStore::in_memory(DEFAULT_HISTORY_CAP));
        Self {
            history: Arc::new(store),
            completions: Arc::new(CompletionEngine::new()),
        }
    }

    #[cfg(test)]
    pub fn in_memory_for_tests(cap: usize) -> Self {
        Self {
            history: Arc::new(HistoryStore::in_memory(cap)),
            completions: Arc::new(CompletionEngine::new()),
        }
    }
}

pub(super) fn parse_security_level(raw: &str) -> Result<TermSecurityLevel, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "l1_read_only" | "1" | "l1" | "read_only" => Ok(TermSecurityLevel::L1ReadOnly),
        "l2_workspace_build" | "2" | "l2" | "workspace_build" | "safe_exec" => {
            Ok(TermSecurityLevel::L2WorkspaceBuild)
        }
        "l3_structured_default" | "3" | "l3" | "structured_default" | "workspace_write" => {
            Ok(TermSecurityLevel::L3StructuredDefault)
        }
        "l4_host_gated" | "4" | "l4" | "host_gated" | "system_exec" => {
            Ok(TermSecurityLevel::L4HostGated)
        }
        "l5_autopilot_floor" | "5" | "l5" | "autopilot_floor" => {
            Ok(TermSecurityLevel::L5AutopilotFloor)
        }
        _ => Err(format!("invalid terminal security level: {raw:?}")),
    }
}

pub(super) fn parse_control_mode(raw: &str) -> Result<TermControlMode, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "manual" => Ok(TermControlMode::Manual),
        "co_pilot" | "copilot" => Ok(TermControlMode::CoPilot),
        "autopilot" => Ok(TermControlMode::Autopilot),
        _ => Err(format!("invalid terminal control mode: {raw:?}")),
    }
}

pub(super) async fn promote_bg_on_host(
    host: &dyn SessionHost,
    session_id: &str,
    block_id: Option<u64>,
) -> Result<bool, String> {
    let target_block = match block_id {
        Some(id) => Some(BlockId(id)),
        None => {
            let att = host
                .attach(session_id, Some(u64::MAX))
                .await
                .map_err(|e| e.to_string())?;
            att.blocks
                .iter()
                .rev()
                .find(|b| matches!(b.status, BlockStatus::Running | BlockStatus::AwaitingInput))
                .map(|b| b.id)
        }
    };
    let Some(bid) = target_block else {
        return Ok(false);
    };
    host.promote_bg(session_id, bid)
        .await
        .map(|()| true)
        .map_err(|e| e.to_string())
}

pub(super) fn spawn_term_forwarder(
    app_handle: tauri::AppHandle,
    host: Arc<dyn SessionHost>,
    sid: String,
    mut att: Attachment,
    mut decoder: Utf8StreamDecoder,
    mut last_seq: u64,
) -> tokio::task::AbortHandle {
    tokio::spawn(async move {
        loop {
            let lagged = tokio::select! {
                chunk = att.output.recv() => match chunk {
                    Ok(c) => {
                        last_seq = c.seq;
                        let data = decode_stream_chunk(&mut decoder, &c.bytes);
                        if !data.is_empty() {
                            let _ = app_handle.emit(
                                TERM_OUTPUT_EVENT,
                                TermOutputPayload { session_id: sid.clone(), seq: c.seq, data },
                            );
                        }
                        false
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => true,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        let tail = decoder.flush_lossy();
                        if !tail.is_empty() {
                            let _ = app_handle.emit(
                                TERM_OUTPUT_EVENT,
                                TermOutputPayload { session_id: sid.clone(), seq: last_seq, data: tail },
                            );
                        }
                        break;
                    }
                },
                ev = att.events.recv() => match ev {
                    Ok(event) => {
                        if matches!(&event, SessionEvent::SnapshotResync { resync_required: true, .. }) {
                            decoder = Utf8StreamDecoder::new();
                        }
                        let _ = app_handle.emit(
                            TERM_EVENT_EVENT,
                            TermEventPayload { session_id: sid.clone(), event },
                        );
                        false
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => true,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            };

            if lagged {
                let Ok(reattached) = host.attach(&sid, Some(last_seq)).await else { break };
                if reattached.resync_required {
                    decoder = Utf8StreamDecoder::new();
                }
                let resync_event = SessionEvent::SnapshotResync {
                    blocks: reattached.blocks.clone(),
                    resync_required: reattached.resync_required,
                    alt_screen: reattached.alt_screen,
                    cwd: reattached.cwd.clone(),
                    git_branch: reattached.git_branch.clone(),
                };
                let _ = app_handle.emit(
                    TERM_EVENT_EVENT,
                    TermEventPayload { session_id: sid.clone(), event: resync_event },
                );
                let data = decode_stream_chunk(&mut decoder, &reattached.replay);
                if !data.is_empty() {
                    let _ = app_handle.emit(
                        TERM_OUTPUT_EVENT,
                        TermOutputPayload { session_id: sid.clone(), seq: reattached.seq, data },
                    );
                }
                last_seq = reattached.seq;
                att = reattached;
            }
        }
    })
    .abort_handle()
}

pub(super) fn history_record_impl(
    warp: &PtyWarpState,
    command: String,
    cwd: Option<String>,
    exit_code: Option<i32>,
) -> Result<bool, String> {
    warp.history
        .record_with_scrubber(
            &command,
            cwd.as_deref(),
            exit_code,
            now_ms(),
            &vox_secrets::scrub_resolved_secrets,
        )
        .map_err(|e| e.to_string())
}

pub(super) fn history_query_impl(
    warp: &PtyWarpState,
    query: Option<String>,
    cwd: Option<String>,
    limit: Option<usize>,
) -> Vec<HistoryEntry> {
    warp.history.query(
        query.as_deref().unwrap_or(""),
        cwd.as_deref(),
        limit.unwrap_or(50).clamp(1, 200),
    )
}

pub(super) fn history_clear_impl(warp: &PtyWarpState) -> Result<(), String> {
    warp.history.clear().map_err(|e| e.to_string())
}

pub(super) fn complete_impl(
    warp: &PtyWarpState,
    req: TermCompletionRequest,
) -> Vec<TermCompletionItem> {
    let limit = req.limit.unwrap_or(32).clamp(1, 100);
    let mut items = warp.completions.complete(&req);
    let trimmed = req.line.trim_start();
    let lead_ws = req.line.len() - trimmed.len();
    if !trimmed.is_empty() && !req.line.contains('\n') && req.cursor >= req.line.len() {
        for entry in warp.history.query("", req.cwd.as_deref(), 50) {
            if entry.command.starts_with(trimmed)
                && entry.command != trimmed
                && !items.iter().any(|i| i.insert_text == entry.command)
            {
                items.push(TermCompletionItem {
                    label: entry.command.clone(),
                    insert_text: entry.command,
                    kind: TermCompletionKind::History,
                    detail: Some("recent history".to_string()),
                    score: 85,
                    replace_start: lead_ws,
                    replace_end: req.line.len(),
                });
            }
        }
        items.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.label.cmp(&b.label)));
        items.truncate(limit);
    }
    items
}

pub(super) fn runbooks_list_impl(cwd: Option<String>) -> Vec<RunbookDescriptor> {
    let hint = cwd
        .as_deref()
        .map(Path::new)
        .unwrap_or_else(|| Path::new("."));
    let repo = vox_repository::discover_repository_or_fallback(hint);
    vox_terminal_core::runbooks::scan_workspace_runbooks(&repo.root)
}

pub(super) async fn voxify_on_host(
    host: &dyn SessionHost,
    session_id: &str,
    shell_hint: Option<&str>,
) -> Result<(), String> {
    host.voxify_subshell(session_id, shell_hint)
        .await
        .map_err(|e| e.to_string())
}

pub(super) async fn session_voxify_impl(
    manager: &super::PtyManager,
    daemon: &Arc<super::super::daemon::PersistentDaemon>,
    session_id: String,
    shell_hint: Option<String>,
) -> Result<(), String> {
    let host = manager.host_for(daemon).await;
    voxify_on_host(host.as_ref(), &session_id, shell_hint.as_deref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_terminal_core::OpenSpec;

    #[tokio::test]
    async fn warp_helpers_scrub_history_complete_tokens_list_runbooks_and_voxify_session() {
        let warp = PtyWarpState::in_memory_for_tests(64);

        // 1. Ignorespace skipped, normal + secret commands recorded & scrubbed via Clavis + vox_redact
        assert!(
            !history_record_impl(&warp, " secret-cmd".into(), Some("/repo".into()), Some(0))
                .unwrap()
        );
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        assert!(
            history_record_impl(
                &warp,
                format!("curl -H 'Authorization: Bearer {jwt}' https://example.com"),
                Some("/repo".into()),
                Some(0),
            )
            .unwrap()
        );

        let rows = history_query_impl(&warp, Some("curl".into()), Some("/repo".into()), Some(10));
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].command.contains(jwt));

        history_clear_impl(&warp).unwrap();
        assert!(history_query_impl(&warp, None, None, Some(10)).is_empty());

        // 2. CompletionEngine + HistoryStore inline completion integration
        history_record_impl(
            &warp,
            "cargo check --all-targets".into(),
            Some("/repo".into()),
            Some(0),
        )
        .unwrap();
        let items = complete_impl(
            &warp,
            TermCompletionRequest {
                line: "cargo ch".into(),
                cursor: 8,
                cwd: Some("/repo".into()),
                shell_mode: None,
                limit: Some(15),
            },
        );
        assert!(
            items
                .iter()
                .any(|i| i.label == "check" && i.kind == TermCompletionKind::Subcommand)
        );
        assert!(
            items
                .iter()
                .any(|i| i.insert_text == "cargo check --all-targets"
                    && i.kind == TermCompletionKind::History)
        );

        // 3. Runbooks discovery against a temporary workspace root
        let dir = tempfile::tempdir().expect("tempdir");
        let scripts_dir = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir).unwrap();
        std::fs::write(
            scripts_dir.join("smoke.vox"),
            "// @title Smoke Check\n// @desc Verify build\nfn main() {}\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();

        let runbooks = runbooks_list_impl(Some(dir.path().to_string_lossy().into_owned()));
        assert!(runbooks.iter().any(|r| r.title == "Smoke Check"));

        // 4. Subshell voxify writes non-echo hook payload to an open PTY session
        let manager = super::super::PtyManager::default();
        let host = manager.resolve_local_host();
        let sid = host
            .open(OpenSpec {
                cols: 80,
                rows: 24,
                shell: None,
            })
            .await
            .expect("open local pty");
        voxify_on_host(host.as_ref(), &sid, Some("bash"))
            .await
            .expect("voxify open session");
        assert!(
            voxify_on_host(host.as_ref(), "missing-session", Some("bash"))
                .await
                .is_err()
        );
        host.close(&sid).await.expect("close local pty");
    }
}
