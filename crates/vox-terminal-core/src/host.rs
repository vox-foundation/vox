//! Session host: owns live [`Session`]s (and their PTYs) so any number of
//! front-ends can attach to the same one. `LocalHost` is the in-process
//! implementation; the daemon exposes one over `term.*` (see
//! `docs/src/architecture/terminal-session-host-design-2026.md`).
//!
//! [`SessionHost`] is the front-end-facing seam: `LocalHost` serves a session
//! in-process, `DaemonHost` (see `daemon_host`) attaches to one hosted by the daemon.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::block::{Block, BlockId, BlockKind, BlockOrigin, OutputChunk, Stream};
use crate::input::{InputIntent, classify};
use crate::pty::{PtyHandle, TermSignal};
use crate::session::{Session, SessionEvent};

#[path = "host_agent.rs"]
mod host_agent;
pub use host_agent::BlockFeedbackVerdict;

pub type SessionId = String;

#[derive(Debug, Clone)]
pub struct OpenSpec {
    pub cols: u16,
    pub rows: u16,
    pub shell: Option<String>,
}

impl Default for OpenSpec {
    fn default() -> Self {
        Self {
            cols: 120,
            rows: 32,
            shell: None,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    Default,
)]
#[serde(rename_all = "snake_case")]
pub enum TermSecurityLevel {
    #[serde(alias = "1", alias = "read_only")]
    L1ReadOnly = 1,
    #[serde(alias = "2", alias = "workspace_build", alias = "safe_exec")]
    L2WorkspaceBuild = 2,
    #[default]
    #[serde(alias = "3", alias = "structured_default", alias = "workspace_write")]
    L3StructuredDefault = 3,
    #[serde(alias = "4", alias = "host_gated", alias = "system_exec")]
    L4HostGated = 4,
    #[serde(alias = "5", alias = "autopilot_floor")]
    L5AutopilotFloor = 5,
}

impl TermSecurityLevel {
    #[allow(non_upper_case_globals)]
    pub const L2SafeExec: Self = Self::L2WorkspaceBuild;
    #[allow(non_upper_case_globals)]
    pub const L3WorkspaceWrite: Self = Self::L3StructuredDefault;
    #[allow(non_upper_case_globals)]
    pub const L4SystemExec: Self = Self::L4HostGated;

    /// Validate whether an agent-issued command is allowed under this security level.
    pub fn allows_agent_command(&self, cmd: &str) -> Result<(), String> {
        crate::agent_exec::allows_agent_command(*self, cmd)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TermControlMode {
    Manual,
    #[default]
    CoPilot,
    Autopilot,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub blocks: usize,
    #[serde(default)]
    pub security_level: TermSecurityLevel,
    #[serde(default)]
    pub control_mode: TermControlMode,
    #[serde(default)]
    pub alt_screen: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
}

/// Redacted projection of a completed/open terminal [`Block`] safe for LLM chat context.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RedactedBlockContext {
    pub id: u64,
    pub input: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub origin: BlockOrigin,
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
    /// Retained raw PTY output bytes (full window when `resync_required`, or frames > `after_seq`).
    pub replay: Vec<u8>,
    /// `seq` of the newest frame included in `replay` (0 = none yet).
    pub seq: u64,
    /// True on initial attach (`after_seq: None`) or when `after_seq` was evicted from the ring.
    pub resync_required: bool,
    /// True when the session's VT shadow grid is currently in `TermMode::ALT_SCREEN`.
    pub alt_screen: bool,
    /// Latest working directory reported by `OSC 7` or `OSC 633;P;Cwd`.
    pub cwd: Option<String>,
    /// Latest git branch reported by `OSC 633;P;GitBranch`.
    pub git_branch: Option<String>,
    pub events: broadcast::Receiver<SessionEvent>,
    pub output: broadcast::Receiver<OutputFrame>,
}

/// What a front-end needs from a terminal session host, local or remote.
#[async_trait]
pub trait SessionHost: Send + Sync {
    async fn open(&self, spec: OpenSpec) -> Result<SessionId>;
    async fn list(&self) -> Result<Vec<SessionInfo>>;
    async fn attach(&self, id: &str, after_seq: Option<u64>) -> Result<Attachment>;
    async fn submit(&self, id: &str, line: &str) -> Result<BlockId>;
    async fn input(&self, id: &str, bytes: &[u8]) -> Result<()>;
    async fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()>;
    async fn signal(&self, id: &str, sig: TermSignal) -> Result<()>;
    async fn close(&self, id: &str) -> Result<()>;
    async fn set_security_level(&self, id: &str, level: TermSecurityLevel) -> Result<()>;
    async fn set_control_mode(&self, id: &str, mode: TermControlMode) -> Result<()>;
    async fn pin_block(&self, id: &str, block_id: BlockId, pinned: bool) -> Result<()>;
    async fn promote_bg(&self, id: &str, block_id: BlockId) -> Result<()>;
    async fn voxify_subshell(&self, id: &str, shell_hint: Option<&str>) -> Result<()>;
    async fn begin_custom_block(
        &self,
        _id: &str,
        _kind: BlockKind,
        _origin: BlockOrigin,
        _input: String,
    ) -> Result<BlockId> {
        anyhow::bail!("unsupported in mock")
    }
    async fn append_block_chunk(
        &self,
        _id: &str,
        _block_id: BlockId,
        _chunk: OutputChunk,
    ) -> Result<()> {
        anyhow::bail!("unsupported in mock")
    }
    async fn finish_custom_block(
        &self,
        _id: &str,
        _block_id: BlockId,
        _exit_code: i32,
        _cancelled: bool,
    ) -> Result<Option<Block>> {
        anyhow::bail!("unsupported in mock")
    }
    async fn get_block(&self, _id: &str, _block_id: BlockId) -> Result<Option<Block>> {
        anyhow::bail!("unsupported in mock")
    }
    async fn clear_unpinned_blocks(&self, _id: &str) -> Result<usize> {
        anyhow::bail!("unsupported in mock")
    }
    async fn compact_blocks(
        &self,
        _id: &str,
        _summary_markdown: String,
        _keep_recent: usize,
    ) -> Result<Option<BlockId>> {
        anyhow::bail!("unsupported in mock")
    }
    async fn record_block_feedback(
        &self,
        _id: &str,
        _block_id: BlockId,
        _verdict: BlockFeedbackVerdict,
    ) -> Result<bool> {
        anyhow::bail!("unsupported in mock")
    }
    async fn control_epoch(&self, _id: &str) -> Result<u64> {
        anyhow::bail!("unsupported in mock")
    }
}

#[async_trait]
impl SessionHost for LocalHost {
    async fn open(&self, spec: OpenSpec) -> Result<SessionId> {
        LocalHost::open(self, spec).await
    }
    async fn list(&self) -> Result<Vec<SessionInfo>> {
        Ok(LocalHost::list(self))
    }
    async fn attach(&self, id: &str, after_seq: Option<u64>) -> Result<Attachment> {
        LocalHost::attach_since(self, id, after_seq)
    }
    async fn submit(&self, id: &str, line: &str) -> Result<BlockId> {
        LocalHost::submit_async(self, id, line).await
    }
    async fn input(&self, id: &str, bytes: &[u8]) -> Result<()> {
        LocalHost::input(self, id, bytes)
    }
    async fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        LocalHost::resize(self, id, cols, rows)
    }
    async fn signal(&self, id: &str, sig: TermSignal) -> Result<()> {
        LocalHost::signal(self, id, sig)
    }
    async fn close(&self, id: &str) -> Result<()> {
        LocalHost::close(self, id)
    }
    async fn set_security_level(&self, id: &str, level: TermSecurityLevel) -> Result<()> {
        LocalHost::set_security_level(self, id, level)
    }
    async fn set_control_mode(&self, id: &str, mode: TermControlMode) -> Result<()> {
        LocalHost::set_control_mode(self, id, mode)
    }
    async fn pin_block(&self, id: &str, block_id: BlockId, pinned: bool) -> Result<()> {
        LocalHost::pin_block(self, id, block_id, pinned)
    }
    async fn promote_bg(&self, id: &str, block_id: BlockId) -> Result<()> {
        LocalHost::promote_bg(self, id, block_id)
    }
    async fn voxify_subshell(&self, id: &str, shell_hint: Option<&str>) -> Result<()> {
        LocalHost::voxify_subshell(self, id, shell_hint)
    }
    async fn begin_custom_block(
        &self,
        id: &str,
        kind: BlockKind,
        origin: BlockOrigin,
        input: String,
    ) -> Result<BlockId> {
        LocalHost::begin_custom_block(self, id, kind, origin, input)
    }
    async fn append_block_chunk(
        &self,
        id: &str,
        block_id: BlockId,
        chunk: OutputChunk,
    ) -> Result<()> {
        LocalHost::append_block_chunk(self, id, block_id, chunk)
    }
    async fn finish_custom_block(
        &self,
        id: &str,
        block_id: BlockId,
        exit_code: i32,
        cancelled: bool,
    ) -> Result<Option<Block>> {
        LocalHost::finish_custom_block(self, id, block_id, exit_code, cancelled)
    }
    async fn get_block(&self, id: &str, block_id: BlockId) -> Result<Option<Block>> {
        LocalHost::get_block(self, id, block_id)
    }
    async fn clear_unpinned_blocks(&self, id: &str) -> Result<usize> {
        LocalHost::clear_unpinned_blocks(self, id)
    }
    async fn compact_blocks(
        &self,
        id: &str,
        summary_markdown: String,
        keep_recent: usize,
    ) -> Result<Option<BlockId>> {
        LocalHost::compact_blocks(self, id, summary_markdown, keep_recent)
    }
    async fn record_block_feedback(
        &self,
        id: &str,
        block_id: BlockId,
        verdict: BlockFeedbackVerdict,
    ) -> Result<bool> {
        LocalHost::record_block_feedback(self, id, block_id, verdict)
    }
    async fn control_epoch(&self, id: &str) -> Result<u64> {
        LocalHost::control_epoch(self, id)
    }
}

pub(crate) struct Replay {
    pub(crate) frames: VecDeque<OutputFrame>,
    pub(crate) total_bytes: usize,
    pub(crate) seq: u64,
    pub(crate) cap: usize,
    pub(crate) truncated_front: bool,
}

pub(crate) struct Entry {
    pub(crate) session: Mutex<Session>,
    pub(crate) pty: Mutex<Option<PtyHandle>>,
    pub(crate) replay: Mutex<Replay>,
    pub(crate) settled_cv: Condvar,
    pub(crate) settled_notify: tokio::sync::Notify,
    pub(crate) pty_eof: AtomicBool,
    pub(crate) out_tx: broadcast::Sender<OutputFrame>,
    pub(crate) security_level: Mutex<TermSecurityLevel>,
    pub(crate) control_mode: Mutex<TermControlMode>,
    pub(crate) last_output_at: Mutex<Instant>,
    pub(crate) promoted_bg: AtomicBool,
    pub(crate) shell_name: String,
    pub(crate) agent_exec_lock: tokio::sync::Mutex<()>,
}

impl Entry {
    #[cfg(test)]
    fn new(id: &str, pty: Option<PtyHandle>, replay_cap: usize) -> Arc<Self> {
        Self::new_with_size_and_shell(id, pty, replay_cap, 120, 32, String::new())
    }

    pub(crate) fn new_with_size_and_shell(
        id: &str,
        pty: Option<PtyHandle>,
        replay_cap: usize,
        cols: u16,
        rows: u16,
        shell_name: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            session: Mutex::new(Session::with_size(id, cols, rows)),
            pty: Mutex::new(pty),
            replay: Mutex::new(Replay {
                frames: VecDeque::new(),
                total_bytes: 0,
                seq: 0,
                cap: replay_cap,
                truncated_front: false,
            }),
            settled_cv: Condvar::new(),
            settled_notify: tokio::sync::Notify::new(),
            pty_eof: AtomicBool::new(false),
            out_tx: broadcast::channel(256).0,
            security_level: Mutex::new(TermSecurityLevel::default()),
            control_mode: Mutex::new(TermControlMode::default()),
            last_output_at: Mutex::new(Instant::now()),
            promoted_bg: AtomicBool::new(false),
            shell_name,
            agent_exec_lock: tokio::sync::Mutex::new(()),
        })
    }

    /// Record raw output (ring + seq), fan it out, and feed the block parser.
    /// Lock order is replay → session everywhere, so attach() sees both consistently.
    pub(crate) fn feed(&self, bytes: &[u8]) {
        if !bytes.is_empty() {
            *lock(&self.last_output_at) = Instant::now();
        }
        let mut r = lock(&self.replay);
        r.seq += 1;
        let seq = r.seq;
        if r.cap == 0 {
            r.frames.clear();
            r.total_bytes = 0;
            r.truncated_front = !bytes.is_empty();
        } else if bytes.len() > r.cap {
            r.frames.clear();
            let tail = bytes[bytes.len() - r.cap..].to_vec();
            r.total_bytes = tail.len();
            r.truncated_front = true;
            r.frames.push_back(OutputFrame { seq, bytes: tail });
        } else {
            r.total_bytes += bytes.len();
            r.frames.push_back(OutputFrame {
                seq,
                bytes: bytes.to_vec(),
            });
            while r.total_bytes > r.cap {
                if let Some(evicted) = r.frames.pop_front() {
                    r.total_bytes = r.total_bytes.saturating_sub(evicted.bytes.len());
                    r.truncated_front = false;
                } else {
                    r.total_bytes = 0;
                    r.truncated_front = false;
                    break;
                }
            }
        }
        let _ = self.out_tx.send(OutputFrame {
            seq,
            bytes: bytes.to_vec(),
        });
        lock(&self.session).on_pty_bytes(bytes);
        drop(r);
        self.settled_cv.notify_all();
        self.settled_notify.notify_waiters();
    }

    pub(crate) fn wait_until<P>(&self, timeout: Duration, pred: P)
    where
        P: Fn(&Session) -> bool,
    {
        let deadline = Instant::now() + timeout;
        let mut sess = lock(&self.session);
        while !pred(&sess) && !self.pty_eof.load(Ordering::Acquire) {
            let Some(rem) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            if rem.is_zero() {
                break;
            }
            sess = match self.settled_cv.wait_timeout(sess, rem) {
                Ok((guard, _)) => guard,
                Err(poisoned) => poisoned.into_inner().0,
            };
        }
    }

    /// Async non-blocking wait using `tokio::sync::Notify` (never stalls Tokio worker threads).
    pub(crate) async fn wait_until_async<P>(&self, timeout: Duration, pred: P)
    where
        P: Fn(&Session) -> bool,
    {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let notified = self.settled_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            {
                let sess = lock(&self.session);
                if pred(&sess) || self.pty_eof.load(Ordering::Acquire) {
                    return;
                }
            }
            let Some(rem) = deadline.checked_duration_since(tokio::time::Instant::now()) else {
                return;
            };
            if rem.is_zero() {
                return;
            }
            if tokio::time::timeout(rem, notified).await.is_err() {
                return;
            }
        }
    }
}

pub(crate) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn has_open_agent_pty(s: &Session) -> bool {
    s.open_block_id().is_some_and(|open_id| {
        s.blocks()
            .iter()
            .any(|b| b.id == open_id && matches!(b.origin, BlockOrigin::Agent { .. }))
    })
}

fn has_running_agent_turn(s: &Session) -> bool {
    s.blocks().iter().any(|b| {
        b.kind == BlockKind::AgentTurn
            && b.status == crate::block::BlockStatus::Running
            && matches!(b.origin, BlockOrigin::Agent { .. })
    })
}

fn preempt_agent_turns_and_pty(entry: &Entry, emit_mode: TermControlMode, force_bump: bool) {
    let interrupt_agent_pty = {
        let mut s = lock(&entry.session);
        let agent_pty = has_open_agent_pty(&s);
        let agent_turn = has_running_agent_turn(&s);
        if force_bump || (emit_mode == TermControlMode::Manual && (agent_pty || agent_turn)) {
            let _ = s.bump_control_epoch();
        }
        s.emit_event(SessionEvent::ControlModeChanged {
            control_mode: emit_mode,
        });
        emit_mode == TermControlMode::Manual && agent_pty
    };
    if interrupt_agent_pty && let Some(ref mut pty) = *lock(&entry.pty) {
        let _ = pty.signal(TermSignal::Interrupt);
    }
    entry.settled_cv.notify_all();
    entry.settled_notify.notify_waiters();
}

pub struct LocalHost {
    pub(crate) sessions: Mutex<HashMap<SessionId, Arc<Entry>>>,
    pub(crate) next: Mutex<u64>,
    pub(crate) replay_cap: usize,
    pub(crate) transcript_sink: Option<Arc<dyn crate::transcript::TranscriptSink>>,
    pub(crate) transcript_seq: AtomicU64,
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
            transcript_sink: None,
            transcript_seq: AtomicU64::new(0),
        }
    }

    pub(crate) fn entry(&self, id: &str) -> Result<Arc<Entry>> {
        lock(&self.sessions)
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("no such terminal session: {id}"))
    }

    /// Spawn a PTY running the configured or default shell and start pumping its bytes into a
    /// new `Session` directly from the OS reader thread.
    pub async fn open(&self, spec: OpenSpec) -> Result<SessionId> {
        self.open_with_cwd(spec, None).await
    }

    /// Open a deterministic PTY-less synthetic session (used by unit/integration harnesses).
    pub fn open_synthetic(&self, cols: u16, rows: u16) -> SessionId {
        let cols = if cols == 0 { 120 } else { cols };
        let rows = if rows == 0 { 32 } else { rows };
        let id = {
            let mut n = lock(&self.next);
            *n += 1;
            format!("term-{n}")
        };
        let entry =
            Entry::new_with_size_and_shell(&id, None, self.replay_cap, cols, rows, String::new());
        lock(&self.sessions).insert(id.clone(), entry);
        id
    }

    /// Feed raw PTY output bytes directly into `id`'s replay ring, broadcast stream, and parser.
    pub fn feed_raw_output(&self, id: &str, bytes: &[u8]) -> Result<()> {
        let entry = self.entry(id)?;
        entry.feed(bytes);
        Ok(())
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        let mut v: Vec<_> = lock(&self.sessions)
            .iter()
            .map(|(id, e)| {
                let s = lock(&e.session);
                SessionInfo {
                    id: id.clone(),
                    blocks: s.blocks().len(),
                    security_level: *lock(&e.security_level),
                    control_mode: *lock(&e.control_mode),
                    alt_screen: s.is_alt_screen(),
                    cwd: s.cwd().map(str::to_owned),
                    git_branch: s.git_branch().map(str::to_owned),
                }
            })
            .collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    pub fn set_security_level(&self, id: &str, level: TermSecurityLevel) -> Result<()> {
        let entry = self.entry(id)?;
        *lock(&entry.security_level) = level;
        lock(&entry.session).emit_event(SessionEvent::SecurityLevelChanged {
            security_level: level,
        });
        Ok(())
    }

    pub fn security_level(&self, id: &str) -> Result<TermSecurityLevel> {
        let entry = self.entry(id)?;
        Ok(*lock(&entry.security_level))
    }

    pub fn set_control_mode(&self, id: &str, mode: TermControlMode) -> Result<()> {
        let entry = self.entry(id)?;
        let prev = {
            let mut guard = lock(&entry.control_mode);
            let p = *guard;
            *guard = mode;
            p
        };
        preempt_agent_turns_and_pty(
            &entry,
            mode,
            prev == TermControlMode::Autopilot && mode == TermControlMode::Manual,
        );
        Ok(())
    }

    pub fn control_mode(&self, id: &str) -> Result<TermControlMode> {
        let entry = self.entry(id)?;
        Ok(*lock(&entry.control_mode))
    }

    pub fn attach(&self, id: &str) -> Result<Attachment> {
        self.attach_since(id, None)
    }

    /// Attach to session `id`, optionally replaying only frames with `seq > after_seq`.
    pub fn attach_since(&self, id: &str, after_seq: Option<u64>) -> Result<Attachment> {
        let entry = self.entry(id)?;
        let r = lock(&entry.replay);
        let s = lock(&entry.session);

        let collect_all = || -> Vec<u8> {
            let mut buf = Vec::with_capacity(r.total_bytes);
            for frame in &r.frames {
                buf.extend_from_slice(&frame.bytes);
            }
            buf
        };

        let (replay, resync_required) = match after_seq {
            None => (collect_all(), true),
            Some(cursor) if cursor >= r.seq => (Vec::new(), false),
            Some(cursor)
                if !r.truncated_front
                    && r.frames
                        .front()
                        .is_some_and(|oldest| cursor + 1 >= oldest.seq) =>
            {
                let mut delta = Vec::new();
                for frame in r.frames.iter().filter(|f| f.seq > cursor) {
                    delta.extend_from_slice(&frame.bytes);
                }
                (delta, false)
            }
            Some(_) => (collect_all(), true),
        };

        Ok(Attachment {
            blocks: s.blocks().to_vec(),
            replay,
            seq: r.seq,
            resync_required,
            alt_screen: s.is_alt_screen(),
            cwd: s.cwd().map(str::to_owned),
            git_branch: s.git_branch().map(str::to_owned),
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
}

enum SubmitWaitTarget {
    Settled(BlockId),
    AwaitPty {
        block_id: BlockId,
        timeout: Duration,
        wait_block_gone: Option<BlockId>,
    },
}

fn prepare_submit_entry(
    host: &LocalHost,
    id: &str,
    line: &str,
    origin: &BlockOrigin,
) -> Result<(Arc<Entry>, TermSecurityLevel, bool)> {
    let entry = host.entry(id)?;
    let security_level = *lock(&entry.security_level);
    match origin {
        BlockOrigin::User => {
            let flipped = {
                let mut mode = lock(&entry.control_mode);
                if *mode == TermControlMode::Autopilot {
                    *mode = TermControlMode::Manual;
                    true
                } else {
                    false
                }
            };
            if flipped {
                let mut s = lock(&entry.session);
                let _ = s.bump_control_epoch();
                s.emit_event(SessionEvent::ControlModeChanged {
                    control_mode: TermControlMode::Manual,
                });
            }
        }
        BlockOrigin::Agent { .. } => {
            security_level
                .allows_agent_command(line)
                .map_err(|reason| anyhow!(reason))?;
        }
        BlockOrigin::System => {}
    }

    let has_osc633_pty = !entry.pty_eof.load(Ordering::Acquire)
        && lock(&entry.pty)
            .as_ref()
            .is_some_and(PtyHandle::supports_osc633);
    Ok((entry, security_level, has_osc633_pty))
}

fn dispatch_submit_on_entry(
    id: &str,
    entry: &Entry,
    security_level: TermSecurityLevel,
    has_osc633_pty: bool,
    line: &str,
    origin: BlockOrigin,
) -> Result<SubmitWaitTarget> {
    let intent = classify(line);
    if has_osc633_pty {
        let active_running_id = lock(&entry.session).open_block_id();
        if let Some(running_id) = active_running_id {
            if matches!(origin, BlockOrigin::Agent { .. }) {
                anyhow::bail!(
                    "agent cannot inject stdin into an already-running terminal block ({running_id:?})"
                );
            }
            if matches!(intent, InputIntent::Command { .. } | InputIntent::Agent(_)) {
                let bid = lock(&entry.session).submit_with_origin_and_level(
                    intent,
                    origin,
                    security_level,
                );
                return Ok(SubmitWaitTarget::Settled(bid));
            }
            {
                let mut pty_guard = lock(&entry.pty);
                let pty = pty_guard
                    .as_mut()
                    .ok_or_else(|| anyhow!("terminal session {id} is closed"))?;
                pty.write(format!("{line}\n").as_bytes())?;
            }
            return Ok(SubmitWaitTarget::AwaitPty {
                block_id: running_id,
                timeout: Duration::from_millis(500),
                wait_block_gone: Some(running_id),
            });
        }

        let pty_shell_target: Option<(String, String)> = match intent {
            InputIntent::Shell(cmd) => Some((cmd.clone(), cmd)),
            InputIntent::StructuredShell(pipeline) => {
                if entry.shell_name == "nu" {
                    let wire = crate::pty::format_nu_structured_command(&pipeline);
                    Some((format!("!nu {pipeline}"), wire))
                } else {
                    let bid = lock(&entry.session).submit_with_origin_and_level(
                        InputIntent::StructuredShell(pipeline),
                        origin,
                        security_level,
                    );
                    return Ok(SubmitWaitTarget::Settled(bid));
                }
            }
            InputIntent::VoxNative(src) => {
                let trimmed = src.trim().to_string();
                let is_explicit_vox = crate::vox_interp::is_explicit_vox_syntax(&trimmed);
                let should_eval = lock(&entry.session).should_eval_vox(&trimmed);
                if should_eval {
                    let eval_res = lock(&entry.session).eval_vox_line(&trimmed, security_level);
                    match eval_res {
                        Ok(out) => {
                            let bid = lock(&entry.session).push_completed_block(
                                BlockKind::VoxNative,
                                origin,
                                src,
                                vec![OutputChunk::text(Stream::Stdout, out)],
                                0,
                            );
                            return Ok(SubmitWaitTarget::Settled(bid));
                        }
                        Err(err) if is_explicit_vox => {
                            let bid = lock(&entry.session).push_completed_block(
                                BlockKind::VoxNative,
                                origin,
                                src,
                                vec![OutputChunk::text(Stream::Stderr, err.to_string())],
                                1,
                            );
                            return Ok(SubmitWaitTarget::Settled(bid));
                        }
                        Err(_) => Some((src, trimmed)),
                    }
                } else {
                    Some((src, trimmed))
                }
            }
            other => {
                let bid = lock(&entry.session).submit_with_origin_and_level(
                    other,
                    origin,
                    security_level,
                );
                return Ok(SubmitWaitTarget::Settled(bid));
            }
        };

        if let Some((display_input, shell_cmd)) = pty_shell_target {
            let block_id = lock(&entry.session).begin_pty_submission(display_input, origin);
            let write_res = {
                let mut pty_guard = lock(&entry.pty);
                match pty_guard.as_mut() {
                    Some(pty) => pty.write(format!("{shell_cmd}\n").as_bytes()),
                    None => Err(anyhow!("terminal session {id} is closed")),
                }
            };
            if let Err(err) = write_res {
                lock(&entry.session).finish_open_block_on_eof(-1);
                return Err(err);
            }
            return Ok(SubmitWaitTarget::AwaitPty {
                block_id,
                timeout: Duration::from_millis(5000),
                wait_block_gone: None,
            });
        }
    }

    let block =
        lock(&entry.session).submit_with_origin_and_level(classify(line), origin, security_level);
    Ok(SubmitWaitTarget::Settled(block))
}

impl LocalHost {
    /// Classify `line` and run it through the session asynchronously without blocking Tokio workers.
    pub async fn submit_async(&self, id: &str, line: &str) -> Result<BlockId> {
        self.submit_with_origin_async(id, line, BlockOrigin::User)
            .await
    }

    /// Classify `line` and run it through the session asynchronously using `tokio::sync::Notify`.
    pub async fn submit_with_origin_async(
        &self,
        id: &str,
        line: &str,
        origin: BlockOrigin,
    ) -> Result<BlockId> {
        let (entry, security_level, has_osc633_pty) =
            prepare_submit_entry(self, id, line, &origin)?;
        let active_running_id = if has_osc633_pty && matches!(origin, BlockOrigin::Agent { .. }) {
            lock(&entry.session).open_block_id()
        } else {
            None
        };
        if let Some(running_id) = active_running_id {
            entry
                .wait_until_async(Duration::from_millis(5000), |s| {
                    s.open_block_id() != Some(running_id) && s.is_at_prompt()
                })
                .await;
        }
        match dispatch_submit_on_entry(id, &entry, security_level, has_osc633_pty, line, origin)? {
            SubmitWaitTarget::Settled(bid) => Ok(bid),
            SubmitWaitTarget::AwaitPty {
                block_id,
                timeout,
                wait_block_gone,
            } => {
                let target_gone = wait_block_gone.unwrap_or(block_id);
                entry
                    .wait_until_async(timeout, |s| {
                        (s.open_block_id() != Some(target_gone) && s.is_at_prompt())
                            || (wait_block_gone.is_none() && s.is_alt_screen())
                    })
                    .await;
                Ok(block_id)
            }
        }
    }

    /// Classify `line` and run it through the session (`!cmd`, `/cmd`, `?ask`, …).
    pub fn submit(&self, id: &str, line: &str) -> Result<BlockId> {
        self.submit_with_origin(id, line, BlockOrigin::User)
    }

    /// Classify `line` and run it through the session with an explicit [`BlockOrigin`].
    pub fn submit_with_origin(&self, id: &str, line: &str, origin: BlockOrigin) -> Result<BlockId> {
        let (entry, security_level, has_osc633_pty) =
            prepare_submit_entry(self, id, line, &origin)?;
        let active_running_id = if has_osc633_pty && matches!(origin, BlockOrigin::Agent { .. }) {
            lock(&entry.session).open_block_id()
        } else {
            None
        };
        if let Some(running_id) = active_running_id {
            entry.wait_until(Duration::from_millis(5000), |s| {
                s.open_block_id() != Some(running_id) && s.is_at_prompt()
            });
        }
        match dispatch_submit_on_entry(id, &entry, security_level, has_osc633_pty, line, origin)? {
            SubmitWaitTarget::Settled(bid) => Ok(bid),
            SubmitWaitTarget::AwaitPty {
                block_id,
                timeout,
                wait_block_gone,
            } => {
                let target_gone = wait_block_gone.unwrap_or(block_id);
                entry.wait_until(timeout, |s| {
                    (s.open_block_id() != Some(target_gone) && s.is_at_prompt())
                        || (wait_block_gone.is_none() && s.is_alt_screen())
                });
                Ok(block_id)
            }
        }
    }

    /// Return the last `max_blocks` blocks for `id` with `input` and `plain_output()`
    /// scrubbed and bounded through `crate::corpus::redact`.
    pub fn redacted_context_bundle(
        &self,
        id: &str,
        max_blocks: usize,
    ) -> Result<Vec<RedactedBlockContext>> {
        use crate::corpus::redact::{
            DEFAULT_BLOCK_HEAD_LINES, DEFAULT_BLOCK_MAX_BYTES, DEFAULT_BLOCK_TAIL_LINES,
            scrub_ansi_and_redact, truncate_terminal_block,
        };
        let entry = self.entry(id)?;
        let s = lock(&entry.session);
        let blocks = s.blocks();
        let start = blocks.len().saturating_sub(max_blocks);
        Ok(blocks[start..]
            .iter()
            .map(|b| {
                let scrubbed_out = scrub_ansi_and_redact(&b.plain_output());
                let (bounded_out, _, _) = truncate_terminal_block(
                    &scrubbed_out,
                    id,
                    b.id.0,
                    DEFAULT_BLOCK_HEAD_LINES,
                    DEFAULT_BLOCK_TAIL_LINES,
                    DEFAULT_BLOCK_MAX_BYTES,
                );
                RedactedBlockContext {
                    id: b.id.0,
                    input: scrub_ansi_and_redact(&b.input),
                    output: bounded_out,
                    exit_code: b.exit_code,
                    origin: b.origin.clone(),
                }
            })
            .collect())
    }

    /// Raw bytes to the PTY (keystrokes). Automatically drops `Autopilot` to `Manual` on user input.
    pub fn input(&self, id: &str, bytes: &[u8]) -> Result<()> {
        let entry = self.entry(id)?;
        if !bytes.is_empty() {
            let flipped = {
                let mut mode = lock(&entry.control_mode);
                if *mode == TermControlMode::Autopilot {
                    *mode = TermControlMode::Manual;
                    true
                } else {
                    false
                }
            };
            if flipped {
                preempt_agent_turns_and_pty(&entry, TermControlMode::Manual, true);
            }
        }
        let mut pty = lock(&entry.pty);
        pty.as_mut()
            .ok_or_else(|| anyhow!("terminal session {id} is closed"))?
            .write(bytes)
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        let entry = self.entry(id)?;
        {
            let pty = lock(&entry.pty);
            pty.as_ref()
                .ok_or_else(|| anyhow!("terminal session {id} is closed"))?
                .resize(cols, rows)?;
        }
        lock(&entry.session).resize(cols, rows);
        Ok(())
    }

    pub fn signal(&self, id: &str, sig: TermSignal) -> Result<()> {
        let entry = self.entry(id)?;
        {
            let mut pty = lock(&entry.pty);
            pty.as_mut()
                .ok_or_else(|| anyhow!("terminal session {id} is closed"))?
                .signal(sig)?;
        }
        if matches!(sig, TermSignal::Terminate | TermSignal::Kill) {
            lock(&entry.session).cancel_open_block();
        }
        entry.settled_cv.notify_all();
        entry.settled_notify.notify_waiters();
        Ok(())
    }

    pub fn session_cwd(&self, id: &str) -> Result<Option<String>> {
        let entry = self.entry(id)?;
        Ok(lock(&entry.session).cwd().map(str::to_owned))
    }

    pub fn session_git_branch(&self, id: &str) -> Result<Option<String>> {
        let entry = self.entry(id)?;
        Ok(lock(&entry.session).git_branch().map(str::to_owned))
    }

    pub fn voxify_subshell(&self, id: &str, shell_hint: Option<&str>) -> Result<()> {
        let payload = crate::voxify::build_voxify_payload(shell_hint);
        self.input(id, payload.as_bytes())
    }

    pub fn close(&self, id: &str) -> Result<()> {
        let entry = lock(&self.sessions)
            .remove(id)
            .ok_or_else(|| anyhow!("no such terminal session: {id}"))?;
        let taken_pty = lock(&entry.pty).take();
        if let Some(mut pty) = taken_pty {
            pty.kill();
        }
        entry.pty_eof.store(true, Ordering::Release);
        entry.settled_cv.notify_all();
        entry.settled_notify.notify_waiters();
        Ok(())
    }
}

impl Drop for LocalHost {
    fn drop(&mut self) {
        let entries: Vec<Arc<Entry>> = lock(&self.sessions).drain().map(|(_, e)| e).collect();
        for entry in entries {
            let taken_pty = lock(&entry.pty).take();
            if let Some(mut pty) = taken_pty {
                pty.kill();
            }
            entry.pty_eof.store(true, Ordering::Release);
            entry.settled_cv.notify_all();
            entry.settled_notify.notify_waiters();
        }
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

    #[tokio::test]
    async fn context_bundle_redacts_secrets_and_caps_blocks() {
        let host = LocalHost::default();
        let id = host.open(OpenSpec::default()).await.unwrap();
        host.submit(&id, "!echo first").unwrap();
        host.submit(
            &id,
            "!echo Authorization: Bearer sk-abcdefghijklmnopqrstuvwxyz123456",
        )
        .unwrap();
        host.submit_with_origin(
            &id,
            "!echo third-from-agent",
            BlockOrigin::Agent {
                agent_id: "orch-9".to_string(),
            },
        )
        .unwrap();

        let bundle = host.redacted_context_bundle(&id, 2).unwrap();
        assert_eq!(bundle.len(), 2);
        assert!(
            !bundle[0]
                .input
                .contains("sk-abcdefghijklmnopqrstuvwxyz123456")
        );
        assert!(
            !bundle[0]
                .output
                .contains("sk-abcdefghijklmnopqrstuvwxyz123456")
        );
        assert!(bundle[0].output.contains("[REDACTED_KEY]"));
        assert_eq!(
            bundle[1].origin,
            BlockOrigin::Agent {
                agent_id: "orch-9".to_string()
            }
        );
        host.close(&id).unwrap();
    }

    #[tokio::test]
    async fn security_levels_gate_agent_commands_and_autopilot_drops_on_user_input() {
        let host = LocalHost::default();
        let id = host.open(OpenSpec::default()).await.unwrap();

        // L1ReadOnly permits echo/ls/git status, rejects cargo/rm
        host.set_security_level(&id, TermSecurityLevel::L1ReadOnly)
            .unwrap();
        assert!(
            host.submit_with_origin(
                &id,
                "!echo allowed",
                BlockOrigin::Agent {
                    agent_id: "a1".into()
                }
            )
            .is_ok()
        );
        assert!(
            host.submit_with_origin(
                &id,
                "!cargo build",
                BlockOrigin::Agent {
                    agent_id: "a1".into()
                }
            )
            .is_err()
        );

        // L2SafeExec permits cargo/echo, rejects rm
        host.set_security_level(&id, TermSecurityLevel::L2SafeExec)
            .unwrap();
        assert!(
            host.submit_with_origin(
                &id,
                "!rm somefile",
                BlockOrigin::Agent {
                    agent_id: "a1".into()
                }
            )
            .is_err()
        );

        // Autopilot automatically drops to Manual on user submit or keystroke input
        host.set_control_mode(&id, TermControlMode::Autopilot)
            .unwrap();
        assert_eq!(host.control_mode(&id).unwrap(), TermControlMode::Autopilot);
        host.submit(&id, "!echo user-took-wheel").unwrap();
        assert_eq!(host.control_mode(&id).unwrap(), TermControlMode::Manual);

        host.set_control_mode(&id, TermControlMode::Autopilot)
            .unwrap();
        host.input(&id, b"a").unwrap();
        assert_eq!(host.control_mode(&id).unwrap(), TermControlMode::Manual);

        host.close(&id).unwrap();
    }

    #[test]
    fn allows_agent_command_blocks_multiline_chaining_path_and_wrapper_bypasses() {
        for level in [
            TermSecurityLevel::L1ReadOnly,
            TermSecurityLevel::L2WorkspaceBuild,
            TermSecurityLevel::L3StructuredDefault,
            TermSecurityLevel::L4HostGated,
            TermSecurityLevel::L5AutopilotFloor,
        ] {
            assert!(level.allows_agent_command("!echo ok\nrm -rf .").is_err());
            assert!(level.allows_agent_command("!echo ok\rwhoami").is_err());
        }

        let l1 = TermSecurityLevel::L1ReadOnly;
        for bad_l1 in [
            "!echo ok; ls",
            "!ls && pwd",
            "!ls | cat",
            "!echo $(whoami)",
            "!echo `whoami`",
            "!echo hi > out.txt",
            "!cat < in.txt",
            "!./ls",
            "!env sh -c id",
            "!find . -delete",
            "!find . -exec rm {} +",
            "!git branch -D main",
        ] {
            assert!(
                l1.allows_agent_command(bad_l1).is_err(),
                "L1ReadOnly should reject `{bad_l1}`"
            );
        }
        assert!(l1.allows_agent_command("!git status").is_ok());
        assert!(l1.allows_agent_command("!env").is_ok());

        let l2 = TermSecurityLevel::L2WorkspaceBuild;
        for bad_l2 in [
            "!echo ok;rm foo",
            "!true && rm -rf target",
            "!ls | rm foo",
            "!bin/rm foo",
            "!env rm foo",
            "!FOO=1 command -p rm foo",
            "!echo $(sudo id)",
        ] {
            assert!(
                l2.allows_agent_command(bad_l2).is_err(),
                "L2WorkspaceBuild should reject `{bad_l2}`"
            );
        }
        assert!(l2.allows_agent_command("!cargo test && echo done").is_ok());

        let l3 = TermSecurityLevel::L3StructuredDefault;
        for bad_l3 in [
            "!echo ok;sudo id",
            "!true&&sudo id",
            "!bin/sudo id",
            "!command sudo id",
            "!env -i su root",
        ] {
            assert!(
                l3.allows_agent_command(bad_l3).is_err(),
                "L3StructuredDefault should reject `{bad_l3}`"
            );
        }
        assert!(l3.allows_agent_command("!rm -f temp.out").is_ok());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn agent_cannot_inject_stdin_into_running_block_and_slash_commands_bypass_stdin() {
        let host = LocalHost::default();
        let id = host.open(OpenSpec::default()).await.unwrap();

        let running_id = host
            .submit(&id, "!read VOX_LINE && echo got:$VOX_LINE")
            .unwrap();
        assert_eq!(
            host.block(&id, running_id).unwrap().status,
            crate::block::BlockStatus::Running
        );

        let err = host
            .submit_with_origin(
                &id,
                "!echo allowed",
                BlockOrigin::Agent {
                    agent_id: "agent-1".to_string(),
                },
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("cannot inject stdin"),
            "unexpected error: {err}"
        );

        let slash_id = host.submit(&id, "/help").unwrap();
        assert_ne!(slash_id, running_id);
        assert_eq!(
            host.block(&id, slash_id).unwrap().kind,
            BlockKind::SlashCommand
        );
        assert_eq!(
            host.block(&id, running_id).unwrap().status,
            crate::block::BlockStatus::Running
        );

        let resumed_id = host.submit(&id, "user-answer").unwrap();
        assert_eq!(resumed_id, running_id);
        host.close(&id).unwrap();
    }

    #[tokio::test]
    async fn input_preempts_autopilot_bumps_epoch_and_handles_follow_up_set_control_mode() {
        let host = LocalHost::default();
        let sid = host.open(OpenSpec::default()).await.unwrap();
        let mut att = host.attach(&sid).unwrap();

        host.set_control_mode(&sid, TermControlMode::Autopilot)
            .unwrap();
        let agent_bid = host
            .begin_custom_block(
                &sid,
                BlockKind::AgentTurn,
                BlockOrigin::Agent {
                    agent_id: "auto".into(),
                },
                "? stream".into(),
            )
            .unwrap();
        assert_eq!(host.control_epoch(&sid).unwrap(), 0);

        host.input(&sid, b" ").unwrap();
        assert_eq!(host.control_mode(&sid).unwrap(), TermControlMode::Manual);
        assert_eq!(host.control_epoch(&sid).unwrap(), 1);
        assert_eq!(
            host.get_block(&sid, agent_bid).unwrap().unwrap().status,
            crate::block::BlockStatus::Cancelled
        );
        let mut saw_manual = false;
        while let Ok(ev) = att.events.try_recv() {
            if matches!(
                ev,
                SessionEvent::ControlModeChanged {
                    control_mode: TermControlMode::Manual
                }
            ) {
                saw_manual = true;
            }
        }
        assert!(saw_manual, "input() must emit ControlModeChanged::Manual");

        {
            let entry = host.entry(&sid).unwrap();
            let mut s = lock(&entry.session);
            let _ = s.begin_pty_submission(
                "sleep 10".into(),
                BlockOrigin::Agent {
                    agent_id: "auto".into(),
                },
            );
        }
        host.set_control_mode(&sid, TermControlMode::Manual)
            .unwrap();
        assert_eq!(host.control_epoch(&sid).unwrap(), 2);
        host.close(&sid).unwrap();
    }

    #[test]
    fn submit_async_does_not_block_current_thread_tokio_executor() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let host = Arc::new(LocalHost::default());
            let sid = host.open_synthetic(80, 24);
            let entry = host.entry(&sid).unwrap();
            let bid =
                lock(&entry.session).begin_pty_submission("sleep 1".into(), BlockOrigin::User);

            let ticks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let ticks_clone = Arc::clone(&ticks);
            let entry_done = Arc::clone(&entry);
            let ticker = tokio::spawn(async move {
                for i in 0..5 {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    ticks_clone.fetch_add(1, Ordering::Relaxed);
                    if i == 3 {
                        entry_done.feed(b"\x1b]633;D;0\x07\x1b]633;A\x07$ \x1b]633;B\x07");
                    }
                }
            });

            entry
                .wait_until_async(Duration::from_millis(500), |s| {
                    s.open_block_id() != Some(bid) && s.is_at_prompt()
                })
                .await;
            ticker.await.unwrap();
            assert!(
                ticks.load(Ordering::Relaxed) >= 4,
                "single-thread Tokio tasks must continue progressing while waiting asynchronously"
            );
        });
    }

    #[cfg(feature = "vox-lang")]
    #[tokio::test]
    async fn persistent_vox_repl_evaluates_non_int_types_and_remembers_let_bindings() {
        let host = LocalHost::default();
        let sid = host.open_synthetic(80, 24);

        let b1 = host.submit_async(&sid, "let x = 40").await.unwrap();
        assert_eq!(host.block(&sid, b1).unwrap().kind, BlockKind::VoxNative);
        let b2 = host.submit_async(&sid, "x + 2").await.unwrap();
        assert_eq!(host.block(&sid, b2).unwrap().kind, BlockKind::VoxNative);
        assert!(
            host.block(&sid, b2).unwrap().plain_output().contains("42"),
            "expected persistent let binding x+2 = 42, got: {:?}",
            host.block(&sid, b2).unwrap().plain_output()
        );

        let b3 = host
            .submit_async(&sid, "\"hello \" + \"vox\"")
            .await
            .unwrap();
        assert_eq!(host.block(&sid, b3).unwrap().kind, BlockKind::VoxNative);
        assert_eq!(host.block(&sid, b3).unwrap().plain_output(), "hello vox");

        let b4 = host.submit_async(&sid, "/reset").await.unwrap();
        assert_eq!(host.block(&sid, b4).unwrap().kind, BlockKind::SlashCommand);
        host.close(&sid).unwrap();
    }

    #[tokio::test]
    async fn open_with_auto_shell_resolves_ai_shell_cleanly() {
        let host = LocalHost::default();
        let sid = host
            .open(OpenSpec {
                cols: 80,
                rows: 24,
                shell: Some("auto".to_string()),
            })
            .await
            .expect("open auto shell");
        let bid = host
            .submit_async(&sid, "!echo auto-shell-ok")
            .await
            .unwrap();
        let block = host.block(&sid, bid).expect("block");
        assert!(block.plain_output().contains("auto-shell-ok"));
        host.close(&sid).unwrap();
    }
}
