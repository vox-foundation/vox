//! Per-tab PTY sessions for the Vox Console. Each tab owns one PTY running the
//! user's shell; bytes stream to the UI as Tauri events, input is written back.
//! Windows spawns use ConPTY via portable-pty (no flashing console windows).
//!
//! Pure logic (`default_shell`, `shell_integration_snippet`) lives in
//! `vox-terminal-core::pty`. This module owns only the Tauri command wrappers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::Emitter;
use vox_terminal_core::pty::{PtyHandle, spawn_pty};
use vox_terminal_core::{
    Block, DaemonHost, LocalHost, OpenSpec, SessionEvent, SessionHost, SessionInfo,
};

// Re-export pure helpers so existing call sites in this crate still compile.
pub use vox_terminal_core::pty::default_shell;

/// Tauri event carrying a chunk of PTY output. Payload: { tab_id, data }.
pub const PTY_OUTPUT_EVENT: &str = "vox://pty-output";
/// Tauri event signalling a PTY exited. Payload: { tab_id }.
pub const PTY_EXIT_EVENT: &str = "vox://pty-exit";
/// Tauri event carrying a sequenced chunk of SessionHost output.
pub const TERM_OUTPUT_EVENT: &str = "vox://term-output";
/// Tauri event carrying a structured SessionHost lifecycle event.
pub const TERM_EVENT_EVENT: &str = "vox://term-event";

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct TermAttachmentDto {
    pub session_id: String,
    pub blocks: Vec<Block>,
    pub replay_utf8: String,
    pub seq: u64,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct TermOutputPayload {
    pub session_id: String,
    pub seq: u64,
    pub data: String,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct TermEventPayload {
    pub session_id: String,
    pub event: SessionEvent,
}

pub fn term_attachment_dto_from_parts(
    session_id: &str,
    blocks: Vec<Block>,
    replay_bytes: &[u8],
    seq: u64,
) -> TermAttachmentDto {
    TermAttachmentDto {
        session_id: session_id.to_string(),
        blocks,
        replay_utf8: String::from_utf8_lossy(replay_bytes).into_owned(),
        seq,
    }
}

/// Registry of live PTY sessions and the shared `SessionHost` bridge. Managed by Tauri state.
pub struct PtyManager {
    sessions: Mutex<HashMap<String, PtyHandle>>,
    local_host: Arc<LocalHost>,
    attached_forwarders: Mutex<HashMap<String, tokio::task::AbortHandle>>,
}

impl Default for PtyManager {
    fn default() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            local_host: Arc::new(LocalHost::default()),
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

    pub async fn host_for(
        &self,
        daemon: &Arc<super::daemon::PersistentDaemon>,
    ) -> Arc<dyn SessionHost> {
        if let Ok(addr) = daemon.ensure().await {
            let client = match daemon.token().await {
                Some(tok) => vox_orchestrator::orch_daemon::OrchDaemonClient::with_token(addr, tok),
                None => vox_orchestrator::orch_daemon::OrchDaemonClient::new(addr),
            };
            return Arc::new(DaemonHost::new(client));
        }
        self.local_host.clone()
    }
}

#[derive(serde::Serialize, Clone)]
struct PtyChunk {
    tab_id: String,
    data: String,
}

#[derive(serde::Serialize, Clone)]
struct PtyExit {
    tab_id: String,
}

#[tauri::command]
pub fn pty_spawn(
    app: tauri::AppHandle,
    manager: tauri::State<'_, PtyManager>,
    tab_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let shell = default_shell();
    let (handle, mut rx) = spawn_pty(&shell, cols, rows).map_err(|e| e.to_string())?;

    // Kill any existing session for this tab before replacing.
    if let Some(mut old) = lock(&manager.sessions).remove(&tab_id) {
        old.kill();
    }
    lock(&manager.sessions).insert(tab_id.clone(), handle);

    // Forward byte stream from core's mpsc to Tauri events.
    let app_handle = app.clone();
    let id = tab_id.clone();
    tokio::spawn(async move {
        while let Some(bytes) = rx.recv().await {
            let chunk = PtyChunk {
                tab_id: id.clone(),
                data: String::from_utf8_lossy(&bytes).to_string(),
            };
            let _ = app_handle.emit(PTY_OUTPUT_EVENT, chunk);
        }
        let _ = app_handle.emit(PTY_EXIT_EVENT, PtyExit { tab_id: id });
    });
    Ok(())
}

#[tauri::command]
pub fn pty_write(
    manager: tauri::State<'_, PtyManager>,
    tab_id: String,
    data: String,
) -> Result<(), String> {
    let mut sessions = lock(&manager.sessions);
    let handle = sessions.get_mut(&tab_id).ok_or("no such pty tab")?;
    handle.write(data.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_close(manager: tauri::State<'_, PtyManager>, tab_id: String) -> Result<(), String> {
    if let Some(mut handle) = lock(&manager.sessions).remove(&tab_id) {
        handle.kill();
    }
    Ok(())
}

#[tauri::command]
pub async fn term_session_open(
    manager: tauri::State<'_, PtyManager>,
    daemon: tauri::State<'_, Arc<super::daemon::PersistentDaemon>>,
    cols: u16,
    rows: u16,
    shell: Option<String>,
) -> Result<String, String> {
    let host = manager.host_for(daemon.inner()).await;
    host.open(OpenSpec { cols, rows, shell })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_list(
    manager: tauri::State<'_, PtyManager>,
    daemon: tauri::State<'_, Arc<super::daemon::PersistentDaemon>>,
) -> Result<Vec<SessionInfo>, String> {
    let host = manager.host_for(daemon.inner()).await;
    host.list().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_attach(
    app: tauri::AppHandle,
    manager: tauri::State<'_, PtyManager>,
    daemon: tauri::State<'_, Arc<super::daemon::PersistentDaemon>>,
    session_id: String,
) -> Result<TermAttachmentDto, String> {
    let host = manager.host_for(daemon.inner()).await;
    let mut att = host.attach(&session_id).await.map_err(|e| e.to_string())?;
    let dto = term_attachment_dto_from_parts(&session_id, att.blocks, &att.replay, att.seq);

    if let Some(prev) = lock(&manager.attached_forwarders).remove(&session_id) {
        prev.abort();
    }

    let sid = session_id.clone();
    let app_handle = app.clone();
    let jh = tokio::spawn(async move {
        loop {
            tokio::select! {
                chunk = att.output.recv() => {
                    let Ok(c) = chunk else { break };
                    let payload = TermOutputPayload {
                        session_id: sid.clone(),
                        seq: c.seq,
                        data: String::from_utf8_lossy(&c.bytes).into_owned(),
                    };
                    let _ = app_handle.emit(TERM_OUTPUT_EVENT, payload);
                }
                ev = att.events.recv() => {
                    let Ok(event) = ev else { break };
                    let payload = TermEventPayload {
                        session_id: sid.clone(),
                        event,
                    };
                    let _ = app_handle.emit(TERM_EVENT_EVENT, payload);
                }
            }
        }
    });

    lock(&manager.attached_forwarders).insert(session_id, jh.abort_handle());

    Ok(dto)
}

#[tauri::command]
pub async fn term_session_input(
    manager: tauri::State<'_, PtyManager>,
    daemon: tauri::State<'_, Arc<super::daemon::PersistentDaemon>>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let host = manager.host_for(daemon.inner()).await;
    host.input(&session_id, data.as_bytes())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_submit(
    manager: tauri::State<'_, PtyManager>,
    daemon: tauri::State<'_, Arc<super::daemon::PersistentDaemon>>,
    session_id: String,
    line: String,
) -> Result<u64, String> {
    let host = manager.host_for(daemon.inner()).await;
    let bid = host
        .submit(&session_id, &line)
        .await
        .map_err(|e| e.to_string())?;
    Ok(bid.0)
}

#[tauri::command]
pub async fn term_session_resize(
    manager: tauri::State<'_, PtyManager>,
    daemon: tauri::State<'_, Arc<super::daemon::PersistentDaemon>>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let host = manager.host_for(daemon.inner()).await;
    host.resize(&session_id, cols, rows)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn term_session_close(
    manager: tauri::State<'_, PtyManager>,
    daemon: tauri::State<'_, Arc<super::daemon::PersistentDaemon>>,
    session_id: String,
) -> Result<(), String> {
    if let Some(prev) = lock(&manager.attached_forwarders).remove(&session_id) {
        prev.abort();
    }
    let host = manager.host_for(daemon.inner()).await;
    host.close(&session_id).await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_terminal_core::pty::shell_integration_snippet;

    #[test]
    fn default_shell_is_nonempty() {
        assert!(!default_shell().is_empty());
    }

    #[test]
    fn manager_starts_empty() {
        let m = PtyManager::default();
        assert_eq!(m.count(), 0);
        assert!(!m.has("tab-1"));
    }

    #[test]
    fn integration_snippet_supports_pwsh_and_bash() {
        assert!(shell_integration_snippet("pwsh").is_some());
        assert!(shell_integration_snippet("powershell.exe").is_some());
        // vox-arch-check: allow abs-path
        assert!(shell_integration_snippet("/usr/bin/bash").is_some());
    }

    #[test]
    fn integration_snippet_skips_unknown_shells() {
        // zsh/fish gained OSC 633 integration in vox-terminal-core's Track 6
        // (Nushell effort) — see shell_integration_snippet's own doc comment
        // and test module in crates/vox-terminal-core/src/pty.rs. cmd.exe has
        // no OSC 633 support and stays unsupported.
        assert!(shell_integration_snippet("cmd.exe").is_none());
    }

    #[test]
    fn integration_snippet_supports_zsh_and_fish() {
        assert!(shell_integration_snippet("zsh").is_some());
        assert!(shell_integration_snippet("fish").is_some());
    }

    #[test]
    fn snippets_emit_633_markers() {
        assert!(shell_integration_snippet("pwsh").unwrap().contains("]633;"));
        assert!(shell_integration_snippet("bash").unwrap().contains("]633;"));
    }

    #[test]
    fn pwsh_snippet_uses_real_last_exit_code() {
        let s = shell_integration_snippet("pwsh").unwrap();
        assert!(s.contains("$LASTEXITCODE"), "must read the real exit code");
        assert!(!s.contains("__VoxLastExit"), "stale exit var removed");
    }

    #[test]
    fn bash_snippet_preserves_user_hooks() {
        let s = shell_integration_snippet("bash").unwrap();
        assert!(
            s.contains("__VOX_PREV_PROMPT_COMMAND"),
            "chains PROMPT_COMMAND"
        );
        assert!(s.contains("trap -p DEBUG"), "preserves the DEBUG trap");
    }

    #[test]
    fn uppercase_exe_suffix_is_stripped() {
        assert!(shell_integration_snippet("PowerShell.EXE").is_some());
    }

    #[tokio::test]
    async fn local_fallback_session_host_lifecycle() {
        let m = PtyManager::default();
        let host = m.resolve_local_host();
        let sid = host
            .open(OpenSpec {
                cols: 80,
                rows: 24,
                shell: None,
            })
            .await
            .unwrap();
        let listed = host.list().await.unwrap();
        assert!(listed.iter().any(|s| s.id == sid));
        let dto = term_attachment_dto_from_parts(&sid, vec![], b"hello", 7);
        assert_eq!(dto.session_id, sid);
        assert_eq!(dto.replay_utf8, "hello");
        assert_eq!(dto.seq, 7);
        host.close(&sid).await.unwrap();
    }
}
