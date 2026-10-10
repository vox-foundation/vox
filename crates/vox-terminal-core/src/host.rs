//! Session host: owns live [`Session`]s (and their PTYs) so any number of
//! front-ends can attach to the same one. `LocalHost` is the in-process
//! implementation; the daemon exposes one over `term.*` (see
//! `docs/src/architecture/terminal-session-host-design-2026.md`).
//!
//! The `SessionHost` trait is deliberately not extracted yet: it lands with the
//! second implementation (`DaemonHost`, task H5) so it is shaped by two real users.

use std::collections::{HashMap, VecDeque};
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

/// Default raw-output replay window per session.
// ponytail: fixed 2 MiB; read it from vox-config if memory pressure shows up.
pub const DEFAULT_REPLAY_CAP: usize = 2 * 1024 * 1024;

/// One chunk of raw PTY output. `seq` is 1-based and gap-free per session; a
/// receiver that sees a gap (broadcast lag) should re-attach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFrame {
    pub seq: u64,
    pub bytes: Vec<u8>,
}

/// Everything a front-end needs to join a live session: structured blocks, the
/// recent raw bytes (re-parsed by its own VT engine so full-screen programs
/// redraw), and both live streams. All taken under one lock, so no event or
/// byte falls between the snapshot and the first received frame.
pub struct Attachment {
    pub blocks: Vec<Block>,
    /// Last `replay_cap` bytes of raw PTY output.
    pub replay: Vec<u8>,
    /// `seq` of the newest frame included in `replay` (0 = none yet).
    pub seq: u64,
    pub events: broadcast::Receiver<SessionEvent>,
    pub output: broadcast::Receiver<OutputFrame>,
}

struct Replay {
    buf: VecDeque<u8>,
    seq: u64,
    cap: usize,
}

struct Entry {
    session: Mutex<Session>,
    pty: Mutex<Option<PtyHandle>>,
    replay: Mutex<Replay>,
    out_tx: broadcast::Sender<OutputFrame>,
}

impl Entry {
    fn new(id: &str, pty: Option<PtyHandle>, replay_cap: usize) -> Arc<Self> {
        Arc::new(Self {
            session: Mutex::new(Session::new(id)),
            pty: Mutex::new(pty),
            replay: Mutex::new(Replay {
                buf: VecDeque::new(),
                seq: 0,
                cap: replay_cap,
            }),
            out_tx: broadcast::channel(256).0,
        })
    }

    /// Record raw output (ring + seq), fan it out, and feed the block parser.
    /// Lock order is replay → session everywhere, so attach() sees both consistently.
    fn feed(&self, bytes: &[u8]) {
        let mut r = lock(&self.replay);
        r.seq += 1;
        r.buf.extend(bytes);
        let over = r.buf.len().saturating_sub(r.cap);
        r.buf.drain(..over);
        let _ = self.out_tx.send(OutputFrame {
            seq: r.seq,
            bytes: bytes.to_vec(),
        });
        lock(&self.session).on_pty_bytes(bytes);
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

pub struct LocalHost {
    sessions: Mutex<HashMap<SessionId, Arc<Entry>>>,
    next: Mutex<u64>,
    replay_cap: usize,
}

impl Default for LocalHost {
    fn default() -> Self {
        Self::with_replay_cap(DEFAULT_REPLAY_CAP)
    }
}

impl LocalHost {
    pub fn with_replay_cap(replay_cap: usize) -> Self {
        Self {
            sessions: Mutex::default(),
            next: Mutex::default(),
            replay_cap,
        }
    }

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
        let entry = Entry::new(&id, Some(handle), self.replay_cap);
        let pump = Arc::clone(&entry);
        tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                pump.feed(&bytes);
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
        let r = lock(&entry.replay);
        let s = lock(&entry.session);
        Ok(Attachment {
            blocks: s.blocks().to_vec(),
            replay: r.buf.iter().copied().collect(),
            seq: r.seq,
            events: s.subscribe(),
            output: entry.out_tx.subscribe(),
        })
    }

    pub fn block(&self, id: &str, block: BlockId) -> Option<Block> {
        let entry = self.entry(id).ok()?;
        lock(&entry.session)
            .blocks()
            .iter()
            .find(|b| b.id == block)
            .cloned()
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
        assert_eq!(host.block(&id, a.blocks[0].id), Some(a.blocks[0].clone()));
        assert_eq!(host.block(&id, BlockId(999)), None);
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
    async fn replay_keeps_only_the_newest_bytes_and_tracks_seq() {
        let host = LocalHost::with_replay_cap(8);
        // No PTY: a real shell's startup output would race the exact assertions.
        let id = "t".to_string();
        let entry = Entry::new(&id, None, host.replay_cap);
        lock(&host.sessions).insert(id.clone(), Arc::clone(&entry));
        entry.feed(b"hello ");
        entry.feed(b"world!!!");

        let mut a = host.attach(&id).unwrap();
        assert_eq!(a.replay, b"world!!!");
        assert_eq!(a.seq, 2);

        // The next frame continues exactly where the snapshot ended: no gap.
        entry.feed(b"x");
        let f = a.output.recv().await.unwrap();
        assert_eq!((f.seq, f.bytes), (3, b"x".to_vec()));
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
