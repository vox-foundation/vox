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

use crate::block::{Block, BlockId, OutputChunk};
use crate::host::{Attachment, OpenSpec, OutputFrame, SessionHost, SessionId, SessionInfo};
use crate::session::SessionEvent;

// vox:defactored-from vox_foundation::protocol::term_method 2026-10-09
// (this crate has no vox-foundation edge; `vox-orchestrator-d` pins these to the real
// constants in `term_dispatch::tests::daemon_host_method_names_match_the_protocol`).
pub const OPEN: &str = "term.open";
pub const LIST: &str = "term.list";
pub const ATTACH: &str = "term.attach";
pub const INPUT: &str = "term.input";
pub const SUBMIT: &str = "term.submit";
pub const RESIZE: &str = "term.resize";
pub const CLOSE: &str = "term.close";

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

/// Translate one pushed `term.attach` frame onto the local event channels.
/// Returns `false` when the stream is over (`exit` / `lagged`).
fn route(
    frame: &Value,
    events: &broadcast::Sender<SessionEvent>,
    output: &broadcast::Sender<OutputFrame>,
) -> bool {
    match frame.get("type").and_then(Value::as_str) {
        Some("output") => {
            let seq = frame.get("seq").and_then(Value::as_u64).unwrap_or(0);
            let bytes = frame
                .get("bytes_b64")
                .and_then(Value::as_str)
                .and_then(|s| B64.decode(s).ok())
                .unwrap_or_default();
            let _ = output.send(OutputFrame { seq, bytes });
        }
        Some("block_opened") => {
            if let Some(id) = id_of(frame) {
                let _ = events.send(SessionEvent::BlockOpened { id });
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
                let _ = events.send(SessionEvent::BlockClosed { id });
            }
        }
        Some("agent") => {
            let text = frame
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let _ = events.send(SessionEvent::AgentMessage { text: text.into() });
        }
        Some("exit") | Some("lagged") => return false,
        _ => {}
    }
    true
}

/// Forward pushed frames to the local receivers until the session ends or nobody is
/// listening any more (every `Attachment` receiver dropped). Dropping the senders on
/// return closes both receivers, which is how a front-end learns the session ended.
async fn pump(
    mut rx: mpsc::Receiver<Value>,
    events: broadcast::Sender<SessionEvent>,
    output: broadcast::Sender<OutputFrame>,
) {
    while let Some(frame) = rx.recv().await {
        if !route(&frame, &events, &output) {
            break;
        }
        // ponytail: noticed on the next frame, so an idle detached session holds its
        // stream until output arrives; add a `closed()` select if that ever matters.
        if events.receiver_count() == 0 && output.receiver_count() == 0 {
            break;
        }
    }
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
                Ok(SessionInfo {
                    id: str_field(s, "session_id")?.to_string(),
                    blocks: s.get("blocks").and_then(Value::as_u64).unwrap_or(0) as usize,
                    security_level,
                    control_mode,
                })
            })
            .collect()
    }

    async fn attach(&self, id: &str) -> Result<Attachment> {
        let (tx, mut rx) = mpsc::channel::<Value>(256);
        let client = self.client.clone();
        let params = json!({"session_id": id});
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

        // Receivers exist before the pump starts, so no frame is lost between the
        // snapshot and the caller's first `recv`.
        let (ev_tx, events) = broadcast::channel(256);
        let (out_tx, output) = broadcast::channel(256);
        tokio::spawn(async move {
            pump(rx, ev_tx, out_tx).await;
            // Detached or ended: stop reading the daemon stream (closes the connection).
            stream.abort();
        });
        Ok(Attachment {
            blocks,
            replay,
            seq,
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

    async fn close(&self, id: &str) -> Result<()> {
        self.client
            .call(CLOSE, json!({"session_id": id}))
            .await
            .map(drop)
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

        drop((ev_rx, out_rx));
        tx.send(json!({"type": "agent", "text": "nobody listening"}))
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), task)
            .await
            .expect("pump must stop after the last receiver is gone")
            .unwrap();
    }

    #[test]
    fn route_maps_wire_frames_and_stops_on_exit() {
        let (ev, mut ev_rx) = broadcast::channel(8);
        let (out, mut out_rx) = broadcast::channel(8);
        assert!(route(&json!({"type": "block_opened", "id": 3}), &ev, &out));
        assert!(matches!(
            ev_rx.try_recv().unwrap(),
            SessionEvent::BlockOpened { id } if id == BlockId(3)
        ));
        assert!(route(
            &json!({"type": "output", "seq": 7, "bytes_b64": B64.encode("hi")}),
            &ev,
            &out
        ));
        assert_eq!(
            out_rx.try_recv().unwrap(),
            OutputFrame {
                seq: 7,
                bytes: b"hi".to_vec()
            }
        );
        assert!(route(&json!({"type": "unknown"}), &ev, &out));
        assert!(!route(&json!({"type": "exit"}), &ev, &out));
        assert!(!route(&json!({"type": "lagged"}), &ev, &out));
    }
}
