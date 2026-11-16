//! [`DaemonHost`]: a [`SessionHost`] that attaches to sessions hosted by
//! `vox-orchestrator-d` over its `term.*` methods, so `vox-term` and the Axis GUI
//! can share one live session. Design:
//! `docs/src/architecture/terminal-session-host-design-2026.md`.

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use serde_json::{Value, json};
use tokio::sync::{broadcast, mpsc};
use vox_orchestrator::orch_daemon::OrchDaemonClient;

use crate::block::{Block, BlockId, BlockKind, BlockOrigin, OutputChunk};
use crate::host::{
    Attachment, BlockFeedbackVerdict, OpenSpec, OutputFrame, SessionHost, SessionId, SessionInfo,
    TermControlMode, TermSecurityLevel,
};
use crate::pty::TermSignal;
use crate::session::SessionEvent;
use crate::stall_detector::PromptKind;

#[path = "daemon_host_ops.rs"]
mod daemon_host_ops;
pub use daemon_host_ops::*;

// vox:defactored-from vox_foundation::protocol::term_method 2026-10-09
// (this crate has no vox-foundation edge; `vox-orchestrator-d` pins these to the real
// constants in `term_dispatch::tests::daemon_host_method_names_match_the_protocol`).
pub const OPEN: &str = "term.open";
pub const LIST: &str = "term.list";
pub const ATTACH: &str = "term.attach";
pub const INPUT: &str = "term.input";
pub const SUBMIT: &str = "term.submit";
pub const RESIZE: &str = "term.resize";
pub const SIGNAL: &str = "term.signal";
pub const CLOSE: &str = "term.close";
pub const SECURITY: &str = "term.security";
pub const CONTROL: &str = "term.control";
pub const PIN: &str = "term.pin";
pub const PROMOTE_BG: &str = "term.promote_bg";
pub const VOXIFY: &str = "term.voxify";

/// Maximum consecutive `"lagged"` re-subscribe attempts without `seq` or event progress (`T-441-04`).
const MAX_LAG_RESUBSCRIBE_ATTEMPTS: usize = 64;

pub struct DaemonHost {
    client: OrchDaemonClient,
}

impl DaemonHost {
    pub fn new(client: OrchDaemonClient) -> Self {
        Self { client }
    }
}

/// Locate a `vox-orchestrator-d` binary adjacent to the current executable or on `PATH`.
fn find_daemon_binary() -> Option<std::path::PathBuf> {
    let bin_name = if cfg!(windows) {
        "vox-orchestrator-d.exe"
    } else {
        "vox-orchestrator-d"
    };
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let sibling = dir.join(bin_name);
        if sibling.is_file() {
            return Some(sibling);
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(bin_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Attempt to spawn `vox-orchestrator-d` in the background on cold start and return
/// a connected [`OrchDaemonClient`] once it answers a ping.
async fn try_autospawn_daemon() -> Option<OrchDaemonClient> {
    if std::env::var("VOX_TERM_NO_AUTOSPAWN").ok().as_deref() == Some("1") {
        return None;
    }
    let bin = find_daemon_binary()?;
    let mut cmd = std::process::Command::new(bin);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _child = cmd.spawn().ok()?;
    for _ in 0..20 {
        // drift-allow(timeout-literal,duplicate-numeric-literal): cold-start daemon poll interval
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let client = OrchDaemonClient::from_env_or_token_file();
        if let Ok(Ok(_)) =
            // drift-allow(timeout-literal,duplicate-numeric-literal): fast loopback daemon probe bound
            tokio::time::timeout(std::time::Duration::from_millis(200), client.ping()).await
        {
            return Some(client);
        }
    }
    None
}

/// Probe `vox-orchestrator-d` with a 250ms timeout (auto-spawning it on cold start
/// when available) and return a [`DaemonHost`] when reachable, or fall back to an
/// in-process [`crate::host::LocalHost`].
pub async fn resolve_session_host(
    force_local: bool,
) -> std::sync::Arc<dyn crate::host::SessionHost> {
    if !force_local {
        let client = OrchDaemonClient::from_env_or_token_file();
        if let Ok(Ok(_)) =
            // drift-allow(timeout-literal,duplicate-numeric-literal): fast loopback daemon probe bound
            tokio::time::timeout(std::time::Duration::from_millis(250), client.ping()).await
        {
            return std::sync::Arc::new(DaemonHost::new(client));
        }
        if let Some(spawned_client) = try_autospawn_daemon().await {
            return std::sync::Arc::new(DaemonHost::new(spawned_client));
        }
    }
    std::sync::Arc::new(crate::host::LocalHost::default())
}

fn str_field<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("daemon response missing `{key}`"))
}

fn id_of(v: &Value) -> Option<BlockId> {
    v.get("id").and_then(Value::as_u64).map(BlockId)
}

/// Translate one pushed `term.attach` frame onto the local event channels, updating
/// `last_seen_seq` on `"output"` frames.
/// Returns `false` when the stream ends or requires lag recovery (`exit` / `lagged`).
pub fn route(
    frame: &Value,
    events: &broadcast::Sender<SessionEvent>,
    output: &broadcast::Sender<OutputFrame>,
    last_seen_seq: &mut u64,
) -> bool {
    match frame.get("type").and_then(Value::as_str) {
        Some("output") => {
            let seq = frame.get("seq").and_then(Value::as_u64).unwrap_or(0);
            if seq > *last_seen_seq {
                *last_seen_seq = seq;
            }
            let bytes = frame
                .get("bytes_b64")
                .and_then(Value::as_str)
                .and_then(|s| B64.decode(s).ok())
                .unwrap_or_default();
            let _ = output.send(OutputFrame { seq, bytes });
        }
        Some("block_opened") => {
            if let Some(id) = id_of(frame) {
                let snap_block = frame
                    .get("block")
                    .and_then(|b| serde_json::from_value::<Block>(b.clone()).ok());
                let kind = frame
                    .get("kind")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .or_else(|| snap_block.as_ref().map(|b| b.kind));
                let input = frame
                    .get("input")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .or_else(|| snap_block.as_ref().map(|b| b.input.clone()))
                    .unwrap_or_default();
                let origin = frame
                    .get("origin")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .or_else(|| snap_block.as_ref().map(|b| b.origin.clone()));
                let cwd = frame
                    .get("cwd")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .or_else(|| snap_block.as_ref().and_then(|b| b.cwd.clone()));
                let git_branch = frame
                    .get("git_branch")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .or_else(|| snap_block.as_ref().and_then(|b| b.git_branch.clone()));
                let _ = events.send(SessionEvent::BlockOpened {
                    id,
                    kind,
                    input,
                    origin,
                    cwd,
                    git_branch,
                });
            }
        }
        Some("block_output") => {
            if let (Some(id), Some(chunk)) = (
                id_of(frame),
                frame
                    .get("chunk")
                    .and_then(|c| serde_json::from_value::<OutputChunk>(c.clone()).ok()),
            ) {
                let _ = events.send(SessionEvent::OutputAppended { id, chunk });
            }
        }
        Some("block_closed") => {
            if let Some(id) = id_of(frame) {
                let snap_block = frame
                    .get("block")
                    .and_then(|b| serde_json::from_value::<Block>(b.clone()).ok());
                let exit_code = frame
                    .get("exit_code")
                    .and_then(Value::as_i64)
                    .and_then(|n| i32::try_from(n).ok())
                    .or_else(|| snap_block.as_ref().and_then(|b| b.exit_code));
                let status = frame
                    .get("status")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .or_else(|| snap_block.as_ref().map(|b| b.status));
                let _ = events.send(SessionEvent::BlockClosed {
                    id,
                    exit_code,
                    status,
                });
            }
        }
        Some("agent") => {
            let text = frame
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let _ = events.send(SessionEvent::AgentMessage { text: text.into() });
        }
        Some("alt_screen") => {
            let active = frame
                .get("active")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let _ = events.send(SessionEvent::AltScreenChanged { active });
        }
        Some("cwd_changed") => {
            if let Some(cwd) = frame.get("cwd").and_then(Value::as_str) {
                let git_branch = frame
                    .get("git_branch")
                    .and_then(Value::as_str)
                    .map(String::from);
                let _ = events.send(SessionEvent::CwdChanged {
                    cwd: cwd.to_string(),
                    git_branch,
                });
            }
        }
        Some("block_awaiting_input") => {
            if let (Some(id), Some(prompt_kind)) = (
                id_of(frame),
                frame
                    .get("prompt_kind")
                    .and_then(|v| serde_json::from_value::<PromptKind>(v.clone()).ok()),
            ) {
                let prompt_tail = frame
                    .get("prompt_tail")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let credential_prompt = frame
                    .get("credential_prompt")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let _ = events.send(SessionEvent::BlockAwaitingInput {
                    id,
                    prompt_tail,
                    prompt_kind,
                    credential_prompt,
                });
            }
        }
        Some("block_pinned") => {
            if let Some(id) = id_of(frame) {
                let pinned = frame
                    .get("pinned")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let _ = events.send(SessionEvent::BlockPinned { id, pinned });
            }
        }
        Some("security_changed") => {
            if let Some(security_level) = frame
                .get("security_level")
                .and_then(|v| serde_json::from_value::<TermSecurityLevel>(v.clone()).ok())
            {
                let _ = events.send(SessionEvent::SecurityLevelChanged { security_level });
            }
        }
        Some("control_changed") => {
            if let Some(control_mode) = frame
                .get("control_mode")
                .and_then(|v| serde_json::from_value::<TermControlMode>(v.clone()).ok())
            {
                let _ = events.send(SessionEvent::ControlModeChanged { control_mode });
            }
        }
        Some("snapshot") => {
            let blocks: Vec<Block> = frame
                .get("blocks")
                .cloned()
                .and_then(|b| serde_json::from_value(b).ok())
                .unwrap_or_default();
            let resync_required = frame
                .get("resync_required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let alt_screen = frame
                .get("alt_screen")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let cwd = frame.get("cwd").and_then(Value::as_str).map(String::from);
            let git_branch = frame
                .get("git_branch")
                .and_then(Value::as_str)
                .map(String::from);
            let _ = events.send(SessionEvent::SnapshotResync {
                blocks,
                resync_required,
                alt_screen,
                cwd,
                git_branch,
            });
        }
        Some("exit") | Some("lagged") => return false,
        _ => {}
    }
    true
}

/// Forward pushed frames to local receivers until the session ends or all `Attachment`
/// receivers are dropped, automatically re-subscribing with `after_seq = last_seen_seq`
/// when the daemon stream reports `"lagged"`.
async fn pump_with_resubscribe<F>(
    mut rx: mpsc::Receiver<Value>,
    mut stream: Option<tokio::task::JoinHandle<Result<()>>>,
    mut last_seen_seq: u64,
    events: broadcast::Sender<SessionEvent>,
    output: broadcast::Sender<OutputFrame>,
    mut resubscribe: F,
) where
    F: FnMut(u64) -> (mpsc::Receiver<Value>, tokio::task::JoinHandle<Result<()>>) + Send + 'static,
{
    let mut consecutive_lags = 0usize;
    loop {
        if events.receiver_count() == 0 && output.receiver_count() == 0 {
            break;
        }
        let maybe_frame = tokio::select! {
            f = rx.recv() => f,
            _ = async {
                tokio::join!(events.closed(), output.closed());
            } => break,
        };
        let Some(frame) = maybe_frame else {
            break;
        };
        let is_lagged = frame.get("type").and_then(Value::as_str) == Some("lagged");
        if !route(&frame, &events, &output, &mut last_seen_seq) {
            if is_lagged
                && (events.receiver_count() > 0 || output.receiver_count() > 0)
                && consecutive_lags < MAX_LAG_RESUBSCRIBE_ATTEMPTS
            {
                consecutive_lags += 1;
                if let Some(handle) = stream.take() {
                    handle.abort();
                }
                let (mut new_rx, new_stream) = resubscribe(last_seen_seq);
                stream = Some(new_stream);

                let maybe_snap = tokio::select! {
                    s = new_rx.recv() => s,
                    _ = async {
                        tokio::join!(events.closed(), output.closed());
                    } => break,
                };
                let Some(snap) = maybe_snap else {
                    break;
                };
                if snap.get("type").and_then(Value::as_str) != Some("snapshot") {
                    break;
                }
                let snap_seq = snap
                    .get("seq")
                    .and_then(Value::as_u64)
                    .unwrap_or(last_seen_seq);
                let blocks: Vec<Block> = snap
                    .get("blocks")
                    .and_then(|b| serde_json::from_value(b.clone()).ok())
                    .unwrap_or_default();
                let resync_required = snap
                    .get("resync_required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let alt_screen = snap
                    .get("alt_screen")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let cwd = snap.get("cwd").and_then(Value::as_str).map(String::from);
                let git_branch = snap
                    .get("git_branch")
                    .and_then(Value::as_str)
                    .map(String::from);
                let _ = events.send(SessionEvent::SnapshotResync {
                    blocks,
                    resync_required,
                    alt_screen,
                    cwd,
                    git_branch,
                });
                let delta_bytes = snap
                    .get("replay_b64")
                    .and_then(Value::as_str)
                    .and_then(|s| B64.decode(s).ok())
                    .unwrap_or_default();
                if !delta_bytes.is_empty() {
                    let _ = output.send(OutputFrame {
                        seq: snap_seq,
                        bytes: delta_bytes,
                    });
                }
                let _ = events.send(SessionEvent::AltScreenChanged { active: alt_screen });
                if snap_seq > last_seen_seq {
                    last_seen_seq = snap_seq;
                    consecutive_lags = 0;
                } else {
                    // drift-allow(timeout-literal,duplicate-numeric-literal): lag resubscribe spin guard
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                rx = new_rx;
                continue;
            }
            break;
        }
        consecutive_lags = 0;
    }
    if let Some(handle) = stream {
        handle.abort();
    }
}

#[cfg(test)]
async fn pump(
    rx: mpsc::Receiver<Value>,
    events: broadcast::Sender<SessionEvent>,
    output: broadcast::Sender<OutputFrame>,
) {
    pump_with_resubscribe(rx, None, 0, events, output, |_after_seq| {
        let (_tx, empty_rx) = mpsc::channel(1);
        let noop = tokio::spawn(async { Ok(()) });
        (empty_rx, noop)
    })
    .await;
}

#[async_trait]
impl SessionHost for DaemonHost {
    async fn open(&self, spec: OpenSpec) -> Result<SessionId> {
        let r = self
            .client
            .call(
                OPEN,
                json!({"cols": spec.cols, "rows": spec.rows, "shell": spec.shell}),
            )
            .await?;
        Ok(str_field(&r, "session_id")?.to_string())
    }

    async fn list(&self) -> Result<Vec<SessionInfo>> {
        let r = self.client.call(LIST, json!({})).await?;
        r.as_array()
            .ok_or_else(|| anyhow!("term.list: expected an array"))?
            .iter()
            .map(|s| {
                let security_level = s
                    .get("security_level")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let control_mode = s
                    .get("control_mode")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let alt_screen = s
                    .get("alt_screen")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let cwd = s.get("cwd").and_then(Value::as_str).map(String::from);
                let git_branch = s
                    .get("git_branch")
                    .and_then(Value::as_str)
                    .map(String::from);
                Ok(SessionInfo {
                    id: str_field(s, "session_id")?.to_string(),
                    blocks: s.get("blocks").and_then(Value::as_u64).unwrap_or(0) as usize,
                    security_level,
                    control_mode,
                    alt_screen,
                    cwd,
                    git_branch,
                })
            })
            .collect()
    }

    async fn attach(&self, id: &str, after_seq: Option<u64>) -> Result<Attachment> {
        let (tx, mut rx) = mpsc::channel::<Value>(256);
        let client = self.client.clone();
        let params = json!({"session_id": id, "after_seq": after_seq});
        let stream =
            tokio::spawn(async move { client.subscribe_with_method(ATTACH, params, tx).await });

        // First frame is the snapshot; if the stream ends first, the task holds why.
        let Some(snap) = rx.recv().await else {
            return match stream.await {
                Ok(Err(e)) => Err(e),
                Ok(Ok(())) => Err(anyhow!("term.attach: stream closed before the snapshot")),
                Err(e) => Err(anyhow!("term.attach task failed: {e}")),
            };
        };
        anyhow::ensure!(
            snap.get("type").and_then(Value::as_str) == Some("snapshot"),
            "term.attach: first frame was not a snapshot: {snap}"
        );
        let blocks: Vec<Block> = serde_json::from_value(snap["blocks"].clone())
            .context("term.attach: bad snapshot blocks")?;
        let replay = B64
            .decode(str_field(&snap, "replay_b64")?)
            .context("term.attach: bad replay bytes")?;
        let seq = snap.get("seq").and_then(Value::as_u64).unwrap_or(0);
        let resync_required = snap
            .get("resync_required")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| after_seq.is_none());
        let alt_screen = snap
            .get("alt_screen")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let cwd = snap.get("cwd").and_then(Value::as_str).map(String::from);
        let git_branch = snap
            .get("git_branch")
            .and_then(Value::as_str)
            .map(String::from);

        // Receivers exist before the pump starts, so no frame is lost between the
        // snapshot and the caller's first `recv`.
        let (ev_tx, events) = broadcast::channel(256);
        let (out_tx, output) = broadcast::channel(256);
        let resub_client = self.client.clone();
        let session_id = id.to_string();
        tokio::spawn(async move {
            pump_with_resubscribe(rx, Some(stream), seq, ev_tx, out_tx, move |cursor_seq| {
                let (tx2, rx2) = mpsc::channel::<Value>(256);
                let c = resub_client.clone();
                let sid = session_id.clone();
                let p = json!({"session_id": sid, "after_seq": cursor_seq});
                let handle =
                    tokio::spawn(async move { c.subscribe_with_method(ATTACH, p, tx2).await });
                (rx2, handle)
            })
            .await;
        });
        Ok(Attachment {
            blocks,
            replay,
            seq,
            resync_required,
            alt_screen,
            cwd,
            git_branch,
            events,
            output,
        })
    }

    async fn submit(&self, id: &str, line: &str) -> Result<BlockId> {
        let r = self
            .client
            .call(SUBMIT, json!({"session_id": id, "line": line}))
            .await?;
        r.get("block_id")
            .and_then(Value::as_u64)
            .map(BlockId)
            .ok_or_else(|| anyhow!("term.submit: missing block_id"))
    }

    async fn input(&self, id: &str, bytes: &[u8]) -> Result<()> {
        self.client
            .call(
                INPUT,
                json!({"session_id": id, "bytes_b64": B64.encode(bytes)}),
            )
            .await
            .map(drop)
    }

    async fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        self.client
            .call(
                RESIZE,
                json!({"session_id": id, "cols": cols, "rows": rows}),
            )
            .await
            .map(drop)
    }

    async fn signal(&self, id: &str, sig: TermSignal) -> Result<()> {
        self.client
            .call(SIGNAL, json!({"session_id": id, "signal": sig}))
            .await
            .map(drop)
    }

    async fn close(&self, id: &str) -> Result<()> {
        self.client
            .call(CLOSE, json!({"session_id": id}))
            .await
            .map(drop)
    }

    async fn set_security_level(&self, id: &str, level: TermSecurityLevel) -> Result<()> {
        self.client
            .call(SECURITY, json!({"session_id": id, "security_level": level}))
            .await
            .map(drop)
    }

    async fn set_control_mode(&self, id: &str, mode: TermControlMode) -> Result<()> {
        self.client
            .call(CONTROL, json!({"session_id": id, "control_mode": mode}))
            .await
            .map(drop)
    }

    async fn pin_block(&self, id: &str, block_id: BlockId, pinned: bool) -> Result<()> {
        self.client
            .call(
                PIN,
                json!({"session_id": id, "block_id": block_id.0, "pinned": pinned}),
            )
            .await
            .map(drop)
    }

    async fn promote_bg(&self, id: &str, block_id: BlockId) -> Result<()> {
        self.client
            .call(
                PROMOTE_BG,
                json!({"session_id": id, "block_id": block_id.0}),
            )
            .await
            .map(drop)
    }

    async fn voxify_subshell(&self, id: &str, shell_hint: Option<&str>) -> Result<()> {
        self.client
            .call(VOXIFY, json!({"session_id": id, "shell_hint": shell_hint}))
            .await
            .map(drop)
    }

    async fn begin_custom_block(
        &self,
        id: &str,
        kind: BlockKind,
        origin: BlockOrigin,
        input: String,
    ) -> Result<BlockId> {
        self.rpc_begin_custom_block(id, kind, origin, input).await
    }

    async fn append_block_chunk(
        &self,
        id: &str,
        block_id: BlockId,
        chunk: OutputChunk,
    ) -> Result<()> {
        self.rpc_append_block_chunk(id, block_id, chunk).await
    }

    async fn finish_custom_block(
        &self,
        id: &str,
        block_id: BlockId,
        exit_code: i32,
        cancelled: bool,
    ) -> Result<Option<Block>> {
        self.rpc_finish_custom_block(id, block_id, exit_code, cancelled)
            .await
    }

    async fn get_block(&self, id: &str, block_id: BlockId) -> Result<Option<Block>> {
        self.rpc_get_block(id, block_id).await
    }

    async fn clear_unpinned_blocks(&self, id: &str) -> Result<usize> {
        self.rpc_clear_unpinned_blocks(id).await
    }

    async fn compact_blocks(
        &self,
        id: &str,
        summary_markdown: String,
        keep_recent: usize,
    ) -> Result<Option<BlockId>> {
        self.rpc_compact_blocks(id, summary_markdown, keep_recent)
            .await
    }

    async fn record_block_feedback(
        &self,
        id: &str,
        block_id: BlockId,
        verdict: BlockFeedbackVerdict,
    ) -> Result<bool> {
        self.rpc_record_block_feedback(id, block_id, verdict).await
    }

    async fn control_epoch(&self, id: &str) -> Result<u64> {
        self.rpc_control_epoch(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pump_stops_once_every_receiver_is_dropped() {
        let (tx, rx) = mpsc::channel(8);
        let (ev, ev_rx) = broadcast::channel(8);
        let (out, out_rx) = broadcast::channel(8);
        let task = tokio::spawn(pump(rx, ev, out));

        tx.send(json!({"type": "agent", "text": "still attached"}))
            .await
            .unwrap();
        tokio::task::yield_now().await;
        assert!(
            !task.is_finished(),
            "must keep running while a receiver exists"
        );

        let _keep_idle_sender = tx;
        drop((ev_rx, out_rx));
        tokio::time::timeout(std::time::Duration::from_secs(2), task)
            .await
            .expect("pump must stop immediately on idle stream after the last receiver is gone")
            .unwrap();
    }

    #[test]
    fn route_maps_wire_frames_and_stops_on_exit() {
        let (ev, mut ev_rx) = broadcast::channel(8);
        let (out, mut out_rx) = broadcast::channel(8);
        let mut last_seq = 0u64;
        assert!(route(
            &json!({"type": "block_opened", "id": 3, "input": "echo hi"}),
            &ev,
            &out,
            &mut last_seq
        ));
        assert!(matches!(
            ev_rx.try_recv().unwrap(),
            SessionEvent::BlockOpened { id, ref input, .. } if id == BlockId(3) && input == "echo hi"
        ));
        assert!(route(
            &json!({"type": "block_closed", "id": 3, "exit_code": 7, "status": "Failed"}),
            &ev,
            &out,
            &mut last_seq
        ));
        assert!(matches!(
            ev_rx.try_recv().unwrap(),
            SessionEvent::BlockClosed {
                id,
                exit_code: Some(7),
                status: Some(crate::block::BlockStatus::Failed),
            } if id == BlockId(3)
        ));
        assert!(route(
            &json!({"type": "output", "seq": 7, "bytes_b64": B64.encode("hi")}),
            &ev,
            &out,
            &mut last_seq
        ));
        assert_eq!(last_seq, 7);
        assert_eq!(
            out_rx.try_recv().unwrap(),
            OutputFrame {
                seq: 7,
                bytes: b"hi".to_vec()
            }
        );
        assert!(route(&json!({"type": "unknown"}), &ev, &out, &mut last_seq));
        assert!(route(
            &json!({
                "type": "snapshot",
                "blocks": [],
                "resync_required": true,
                "alt_screen": false,
                "cwd": "/repo",
                "git_branch": "main"
            }),
            &ev,
            &out,
            &mut last_seq
        ));
        assert!(matches!(
            ev_rx.try_recv().expect("snapshot frame routed"),
            SessionEvent::SnapshotResync {
                resync_required: true,
                alt_screen: false,
                ref cwd,
                ref git_branch,
                ..
            } if cwd.as_deref() == Some("/repo") && git_branch.as_deref() == Some("main")
        ));
        assert!(!route(&json!({"type": "exit"}), &ev, &out, &mut last_seq));
        assert!(!route(&json!({"type": "lagged"}), &ev, &out, &mut last_seq));
    }

    #[tokio::test]
    async fn pump_resubscribes_with_after_seq_on_lagged_and_survives_zero_seq_event_lags() {
        let (tx1, rx1) = mpsc::channel(8);
        let (ev_tx, mut ev_rx) = broadcast::channel(64);
        let (out_tx, mut out_rx) = broadcast::channel(16);
        let seen_cursors = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let cursors_clone = std::sync::Arc::clone(&seen_cursors);
        let resub_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let resub_clone = std::sync::Arc::clone(&resub_count);

        let task = tokio::spawn(async move {
            pump_with_resubscribe(rx1, None, 2, ev_tx, out_tx, move |cursor| {
                cursors_clone.lock().unwrap().push(cursor);
                let attempt = resub_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let (tx2, rx2) = mpsc::channel(8);
                let handle = tokio::spawn(async move {
                    if attempt < 9 {
                        // 9 consecutive valid snapshots with zero seq progress (pure event lags)
                        tx2.send(json!({
                            "type": "snapshot",
                            "blocks": [],
                            "replay_b64": "",
                            "seq": cursor,
                            "resync_required": false,
                            "alt_screen": false,
                        }))
                        .await
                        .unwrap();
                        tx2.send(json!({"type": "lagged"})).await.unwrap();
                    } else {
                        tx2.send(json!({
                            "type": "snapshot",
                            "blocks": [],
                            "replay_b64": B64.encode(b"delta-6..8"),
                            "seq": 8,
                            "resync_required": true,
                            "alt_screen": true,
                        }))
                        .await
                        .unwrap();
                        tx2.send(json!({
                            "type": "output",
                            "seq": 9,
                            "bytes_b64": B64.encode(b"live-9"),
                        }))
                        .await
                        .unwrap();
                        tx2.send(json!({"type": "exit"})).await.unwrap();
                    }
                    Ok(())
                });
                (rx2, handle)
            })
            .await;
        });

        tx1.send(json!({
            "type": "output",
            "seq": 5,
            "bytes_b64": B64.encode(b"live-5"),
        }))
        .await
        .unwrap();
        tx1.send(json!({"type": "lagged"})).await.unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(2), task)
            .await
            .expect("pump completed cleanly")
            .unwrap();

        assert_eq!(seen_cursors.lock().unwrap().len(), 10);
        assert_eq!(
            out_rx.try_recv().unwrap(),
            OutputFrame {
                seq: 5,
                bytes: b"live-5".to_vec()
            }
        );
        assert_eq!(
            out_rx.try_recv().unwrap(),
            OutputFrame {
                seq: 8,
                bytes: b"delta-6..8".to_vec()
            }
        );
        assert_eq!(
            out_rx.try_recv().unwrap(),
            OutputFrame {
                seq: 9,
                bytes: b"live-9".to_vec()
            }
        );

        let mut saw_resync = false;
        let mut saw_alt_true = false;
        while let Ok(ev) = ev_rx.try_recv() {
            match ev {
                SessionEvent::SnapshotResync {
                    resync_required: true,
                    alt_screen: true,
                    ..
                } => saw_resync = true,
                SessionEvent::AltScreenChanged { active: true } => saw_alt_true = true,
                _ => {}
            }
        }
        assert!(saw_resync, "expected SnapshotResync event on lag recovery");
        assert!(saw_alt_true, "expected AltScreenChanged(true)");
    }
}
