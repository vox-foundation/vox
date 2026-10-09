//! Session host: owns live [`Session`]s (and their PTYs) so any number of
//! front-ends can attach to the same one. `LocalHost` is the in-process
//! implementation; the daemon exposes one over `term.*` (see
//! `docs/src/architecture/terminal-session-host-design-2026.md`).
//!
//! The `SessionHost` trait is deliberately not extracted yet: it lands with the
//! second implementation (`DaemonHost`, task H5) so it is shaped by two real users.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Result, anyhow};
use tokio::sync::broadcast;

use crate::block::{Block, BlockId};
use crate::input::classify;
use crate::pty::{PtyHandle, default_shell, spawn_pty};
use crate::session::{Session, SessionEvent};

pub type SessionId = String;

#[derive(Debug, Clone)]
pub struct OpenSpec {
    pub cols: u16,
    pub rows: u16,
}

impl Default for OpenSpec {
    fn default() -> Self {
        Self { cols: 80, rows: 24 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub id: SessionId,
    pub blocks: usize,
}

/// Blocks as of attach time plus a live event stream. Both are taken under one
/// lock, so no event falls between the snapshot and the first received event.
pub struct Attachment {
    pub blocks: Vec<Block>,
    pub events: broadcast::Receiver<SessionEvent>,
}

struct Entry {
    session: Mutex<Session>,
    pty: Mutex<Option<PtyHandle>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

#[derive(Default)]
pub struct LocalHost {
    sessions: Mutex<HashMap<SessionId, Arc<Entry>>>,
    next: Mutex<u64>,
}

impl LocalHost {
    fn entry(&self, id: &str) -> Result<Arc<Entry>> {
        lock(&self.sessions)
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("no such terminal session: {id}"))
    }

    /// Spawn a PTY running the default shell and start pumping its bytes into a
    /// new `Session`. Must be called inside a tokio runtime.
    pub async fn open(&self, spec: OpenSpec) -> Result<SessionId> {
        let (handle, mut rx) = spawn_pty(&default_shell(), spec.cols, spec.rows)?;
        let id = {
            let mut n = lock(&self.next);
            *n += 1;
            format!("term-{n}")
        };
        let entry = Arc::new(Entry {
            session: Mutex::new(Session::new(id.as_str())),
            pty: Mutex::new(Some(handle)),
        });
        let pump = Arc::clone(&entry);
        tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                lock(&pump.session).on_pty_bytes(&bytes);
            }
        });
        lock(&self.sessions).insert(id.clone(), entry);
        Ok(id)
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        let mut v: Vec<_> = lock(&self.sessions)
            .iter()
            .map(|(id, e)| SessionInfo {
                id: id.clone(),
                blocks: lock(&e.session).blocks().len(),
            })
            .collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    pub fn attach(&self, id: &str) -> Result<Attachment> {
        let entry = self.entry(id)?;
        let s = lock(&entry.session);
        Ok(Attachment {
            blocks: s.blocks().to_vec(),
            events: s.subscribe(),
        })
    }

    /// Classify `line` and run it through the session (`!cmd`, `/cmd`, `?ask`, …).
    pub fn submit(&self, id: &str, line: &str) -> Result<BlockId> {
        let entry = self.entry(id)?;
        let block = lock(&entry.session).submit(classify(line));
        Ok(block)
    }

    /// Raw bytes to the PTY (keystrokes).
    pub fn input(&self, id: &str, bytes: &[u8]) -> Result<()> {
        let entry = self.entry(id)?;
        let mut pty = lock(&entry.pty);
        pty.as_mut()
            .ok_or_else(|| anyhow!("terminal session {id} is closed"))?
            .write(bytes)
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        let entry = self.entry(id)?;
        let pty = lock(&entry.pty);
        pty.as_ref()
            .ok_or_else(|| anyhow!("terminal session {id} is closed"))?
            .resize(cols, rows)
    }

    pub fn close(&self, id: &str) -> Result<()> {
        let entry = lock(&self.sessions)
            .remove(id)
            .ok_or_else(|| anyhow!("no such terminal session: {id}"))?;
        if let Some(mut pty) = lock(&entry.pty).take() {
            pty.kill();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn second_attach_sees_block_submitted_through_the_host() {
        let host = LocalHost::default();
        let id = host.open(OpenSpec::default()).await.unwrap();
        host.submit(&id, "!echo hi").unwrap();

        let a = host.attach(&id).unwrap();
        let b = host.attach(&id).unwrap();
        assert_eq!(a.blocks.len(), 1);
        assert!(a.blocks[0].plain_output().contains("hi"));
        assert_eq!(a.blocks, b.blocks);
        host.close(&id).unwrap();
    }

    #[tokio::test]
    async fn attached_client_receives_live_events_without_gap() {
        let host = LocalHost::default();
        let id = host.open(OpenSpec::default()).await.unwrap();
        let mut a = host.attach(&id).unwrap();
        host.submit(&id, "!echo live").unwrap();
        assert!(matches!(
            a.events.recv().await.unwrap(),
            SessionEvent::BlockOpened { .. }
        ));
        host.close(&id).unwrap();
    }

    #[tokio::test]
    async fn unknown_and_closed_sessions_error() {
        let host = LocalHost::default();
        assert!(host.attach("nope").is_err());
        assert!(host.submit("nope", "!true").is_err());
        let id = host.open(OpenSpec::default()).await.unwrap();
        host.close(&id).unwrap();
        assert!(host.input(&id, b"x").is_err());
        assert!(host.close(&id).is_err());
        assert!(host.list().is_empty());
    }
}
