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

#[async_trait]
impl SessionHost for DaemonHost {
    async fn open(&self, spec: OpenSpec) -> Result<SessionId> {
        let r = self
            .client
            .call(OPEN, json!({"cols": spec.cols, "rows": spec.rows}))
            .await?;
        Ok(str_field(&r, "session_id")?.to_string())
    }

    async fn list(&self) -> Result<Vec<SessionInfo>> {
        let r = self.client.call(LIST, json!({})).await?;
        r.as_array()
            .ok_or_else(|| anyhow!("term.list: expected an array"))?
            .iter()
            .map(|s| {
                Ok(SessionInfo {
                    id: str_field(s, "session_id")?.to_string(),
                    blocks: s.get("blocks").and_then(Value::as_u64).unwrap_or(0) as usize,
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
            while let Some(frame) = rx.recv().await {
                if !route(&frame, &ev_tx, &out_tx) {
                    break;
                }
            }
            // Dropping the senders closes both receivers: the session ended.
            drop(stream);
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
