//! Optional JSONL sink for orchestrator agent events (`VOX_ORCHESTRATOR_EVENT_LOG`).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::io::AsyncWriteExt;
use tokio::task::JoinHandle;

use crate::events::AgentEventKind;
use crate::orchestrator::Orchestrator;

/// Append JSON lines for every [`AgentEvent`](crate::events::AgentEvent) when **`VOX_ORCHESTRATOR_EVENT_LOG`** is set to a file path.
///
/// When `join_slot` is `Some`, any prior handle in the slot is aborted (MCP re-root). Daemon callers may pass `None`.
pub fn spawn_orchestrator_event_log_sink(
    orchestrator: Arc<Orchestrator>,
    join_slot: Option<Arc<Mutex<Option<JoinHandle<()>>>>>,
) {
    let Ok(raw) = std::env::var("VOX_ORCHESTRATOR_EVENT_LOG") else {
        return;
    };
    let path = PathBuf::from(raw);
    let orch = orchestrator.clone();
    let handle = tokio::spawn(async move {
        let mut rx = orch.event_bus().subscribe();
        while let Ok(event) = rx.recv().await {
            if matches!(event.kind, AgentEventKind::TokenStreamed { .. }) {
                continue;
            }
            let Ok(line) = serde_json::to_string(&event) else {
                continue;
            };
            if let Ok(mut f) = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
            {
                let _ = f.write_all(line.as_bytes()).await;
                let _ = f.write_all(b"\n").await;
            }
        }
    });
    if let Some(slot) = join_slot {
        let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(h) = guard.take() {
            h.abort();
        }
        *guard = Some(handle);
    }
}
