//! Per-tab PTY sessions for the Vox Console. Each tab owns one PTY running the
//! user's shell; bytes stream to the UI as Tauri events, input is written back.
//! Windows spawns use ConPTY via portable-pty (no flashing console windows).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::State;
use vox_terminal_core::pty::{PtyHandle, spawn_pty};
use vox_terminal_core::{BlockId, LocalHost, OpenSpec, SessionHost, SessionInfo};

use super::daemon::PersistentDaemon;
use super::mens_serve::MensServeState;
use crate::remote::backend::BackendState;

pub mod agent;
#[cfg(test)]
mod agent_tests;
pub mod approvals;
pub mod slash;
#[cfg(test)]
mod tests;
pub(crate) mod warp;

pub use approvals::{
    TermAttachmentDto, TermEventPayload, TermOutputPayload, term_attachment_dto_and_decoder,
};
#[cfg(test)]
pub use approvals::{term_attachment_dto_from_attachment, term_attachment_dto_from_parts};
pub use vox_terminal_core::pty::default_shell;

pub const PTY_OUTPUT_EVENT: &str = "vox://pty-output";
pub const PTY_EXIT_EVENT: &str = "vox://pty-exit";
pub const TERM_OUTPUT_EVENT: &str = "vox://term-output";
pub const TERM_EVENT_EVENT: &str = "vox://term-event";

pub(super) fn decode_stream_chunk(
    decoder: &mut vox_terminal_core::Utf8StreamDecoder,
    bytes: &[u8],
) -> String {
    decoder.push(bytes)
}

/// Registry of live PTY sessions and the shared `SessionHost` bridge. Managed by Tauri state.
pub struct PtyManager {
    sessions: Mutex<HashMap<String, PtyHandle>>,
    pub(super) local_host: Arc<LocalHost>,
    pub(super) agent_turns: Arc<agent::AgentTurnTracker>,
    attached_forwarders: Mutex<HashMap<String, tokio::task::AbortHandle>>,
}

impl Default for PtyManager {
    fn default() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            local_host: Arc::new(approvals::build_local_host_with_journal()),
            agent_turns: Arc::new(agent::AgentTurnTracker::default()),
            attached_forwarders: Mutex::new(HashMap::new()),
        }
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl PtyManager {
    #[cfg(test)]
    pub fn has(&self, tab_id: &str) -> bool {
        lock(&self.sessions).contains_key(tab_id)
    }

    #[cfg(test)]
    pub fn count(&self) -> usize {
        lock(&self.sessions).len()
    }

    #[allow(dead_code)]
    pub fn resolve_local_host(&self) -> Arc<dyn SessionHost> {
        self.local_host.clone()
    }

    pub async fn host_for(&self, daemon: &Arc<PersistentDaemon>) -> Arc<dyn SessionHost> {
        approvals::resolve_session_host(&self.local_host, daemon).await
    }
}

#[tauri::command]
pub fn pty_spawn(
    app: tauri::AppHandle,
    manager: State<'_, PtyManager>,
    tab_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let (handle, rx) = spawn_pty(&default_shell(), cols, rows).map_err(|e| e.to_string())?;
    if let Some(mut old) = lock(&manager.sessions).remove(&tab_id) {
        old.kill();
    }
    lock(&manager.sessions).insert(tab_id.clone(), handle);
    warp::spawn_pty_forwarder(app, tab_id, rx);
    Ok(())
}

#[tauri::command]
pub fn pty_write(
    manager: State<'_, PtyManager>,
    tab_id: String,
    data: String,
) -> Result<(), String> {
    let mut sessions = lock(&manager.sessions);
    let handle = sessions.get_mut(&tab_id).ok_or("no such pty tab")?;
    handle.write(data.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_close(manager: State<'_, PtyManager>, tab_id: String) -> Result<(), String> {
    if let Some(mut handle) = lock(&manager.sessions).remove(&tab_id) {
        handle.kill();
    }
    Ok(())
}

#[tauri::command]
pub async fn term_session_open(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    cols: u16,
    rows: u16,
    shell: Option<String>,
) -> Result<String, String> {
    let h = manager.host_for(daemon.inner()).await;
    h.open(OpenSpec { cols, rows, shell })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_list(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
) -> Result<Vec<SessionInfo>, String> {
    let h = manager.host_for(daemon.inner()).await;
    h.list().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_attach(
    app: tauri::AppHandle,
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
    after_seq: Option<u64>,
) -> Result<TermAttachmentDto, String> {
    let host = manager.host_for(daemon.inner()).await;
    let att = host
        .attach(&session_id, after_seq)
        .await
        .map_err(|e| e.to_string())?;
    let (dto, decoder) = term_attachment_dto_and_decoder(&session_id, &att);
    if let Some(prev) = lock(&manager.attached_forwarders).remove(&session_id) {
        prev.abort();
    }
    let handle = warp::spawn_term_forwarder(app, host, session_id.clone(), att, decoder, dto.seq);
    lock(&manager.attached_forwarders).insert(session_id, handle);
    Ok(dto)
}

#[tauri::command]
pub async fn term_session_input(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let h = manager.host_for(daemon.inner()).await;
    h.input(&session_id, data.as_bytes())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_submit(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
    line: String,
) -> Result<u64, String> {
    let h = manager.host_for(daemon.inner()).await;
    h.submit(&session_id, &line)
        .await
        .map(|b| b.0)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_resize(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let h = manager.host_for(daemon.inner()).await;
    h.resize(&session_id, cols, rows)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_close(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
) -> Result<(), String> {
    if let Some(prev) = lock(&manager.attached_forwarders).remove(&session_id) {
        prev.abort();
    }
    let h = manager.host_for(daemon.inner()).await;
    h.close(&session_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_set_security(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
    security_level: String,
) -> Result<(), String> {
    let level = warp::parse_security_level(&security_level)?;
    let h = manager.host_for(daemon.inner()).await;
    h.set_security_level(&session_id, level)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_set_control(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
    control_mode: String,
) -> Result<(), String> {
    let mode = warp::parse_control_mode(&control_mode)?;
    let h = manager.host_for(daemon.inner()).await;
    h.set_control_mode(&session_id, mode)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_pin(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
    block_id: u64,
    pinned: bool,
) -> Result<(), String> {
    let h = manager.host_for(daemon.inner()).await;
    h.pin_block(&session_id, BlockId(block_id), pinned)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_promote_bg(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
    block_id: Option<u64>,
) -> Result<bool, String> {
    let h = manager.host_for(daemon.inner()).await;
    warp::promote_bg_on_host(h.as_ref(), &session_id, block_id).await
}

#[tauri::command]
pub async fn term_history_record(
    warp: State<'_, warp::PtyWarpState>,
    command: String,
    cwd: Option<String>,
    exit_code: Option<i32>,
) -> Result<bool, String> {
    let st = warp.inner().clone();
    tokio::task::spawn_blocking(move || warp::history_record_impl(&st, command, cwd, exit_code))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn term_history_query(
    warp: State<'_, warp::PtyWarpState>,
    query: Option<String>,
    cwd: Option<String>,
    limit: Option<usize>,
) -> Vec<vox_terminal_core::HistoryEntry> {
    warp::history_query_impl(warp.inner(), query, cwd, limit)
}

#[tauri::command]
pub async fn term_history_clear(warp: State<'_, warp::PtyWarpState>) -> Result<(), String> {
    let st = warp.inner().clone();
    tokio::task::spawn_blocking(move || warp::history_clear_impl(&st))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn term_complete(
    warp: State<'_, warp::PtyWarpState>,
    line: String,
    cursor: usize,
    cwd: Option<String>,
    shell_mode: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<vox_terminal_core::TermCompletionItem>, String> {
    let st = warp.inner().clone();
    let req = vox_terminal_core::TermCompletionRequest {
        line,
        cursor,
        cwd,
        shell_mode,
        limit,
    };
    tokio::task::spawn_blocking(move || warp::complete_impl(&st, req))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_runbooks_list(
    cwd: Option<String>,
) -> Result<Vec<vox_terminal_core::RunbookDescriptor>, String> {
    tokio::task::spawn_blocking(move || warp::runbooks_list_impl(cwd))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_voxify(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
    shell_hint: Option<String>,
) -> Result<(), String> {
    warp::session_voxify_impl(manager.inner(), daemon.inner(), session_id, shell_hint).await
}

#[tauri::command]
pub async fn term_agent_prompt(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    backend: State<'_, Arc<BackendState>>,
    mens_serve: State<'_, Arc<MensServeState>>,
    session_id: String,
    prompt: String,
) -> Result<agent::TermAgentTurnResponse, String> {
    let (h, b) = (
        manager.host_for(daemon.inner()).await,
        backend.current().await,
    );
    agent::term_agent_prompt_impl(
        h,
        &b,
        mens_serve.running_port(),
        &manager.agent_turns,
        session_id,
        prompt,
    )
    .await
}

#[tauri::command]
pub async fn term_agent_explain_fix(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    backend: State<'_, Arc<BackendState>>,
    mens_serve: State<'_, Arc<MensServeState>>,
    session_id: String,
    block_id: u64,
    mode: String,
) -> Result<agent::TermAgentTurnResponse, String> {
    let (h, b) = (
        manager.host_for(daemon.inner()).await,
        backend.current().await,
    );
    agent::term_agent_explain_fix_impl(
        h,
        &b,
        mens_serve.running_port(),
        &manager.agent_turns,
        session_id,
        block_id,
        mode,
    )
    .await
}

#[tauri::command]
pub async fn term_agent_cancel(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
    block_id: u64,
) -> Result<bool, String> {
    let h = manager.host_for(daemon.inner()).await;
    agent::term_agent_cancel_impl(h, &manager.agent_turns, session_id, block_id).await
}

#[tauri::command]
pub async fn term_slash_exec(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    backend: State<'_, Arc<BackendState>>,
    mens_serve: State<'_, Arc<MensServeState>>,
    session_id: String,
    raw_line: String,
) -> Result<slash::TermSlashResponse, String> {
    let (h, b) = (
        manager.host_for(daemon.inner()).await,
        backend.current().await,
    );
    slash::term_slash_exec_impl(
        h,
        &b,
        mens_serve.running_port(),
        &manager.agent_turns,
        session_id,
        raw_line,
    )
    .await
}

#[tauri::command]
pub async fn term_block_feedback(
    manager: State<'_, PtyManager>,
    daemon: State<'_, Arc<PersistentDaemon>>,
    session_id: String,
    block_id: u64,
    verdict: String,
    corrected_command: Option<String>,
    original_command: Option<String>,
) -> Result<approvals::TermFeedbackResponse, String> {
    let h = manager.host_for(daemon.inner()).await;
    approvals::term_block_feedback_impl(
        h,
        session_id,
        block_id,
        verdict,
        corrected_command,
        original_command,
    )
    .await
}
