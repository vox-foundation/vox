//! Session host: owns live [`Session`]s (and their PTYs) so any number of
//! front-ends can attach to the same one. `LocalHost` is the in-process
//! implementation; the daemon exposes one over `term.*` (see
//! `docs/src/architecture/terminal-session-host-design-2026.md`).
//!
//! [`SessionHost`] is the front-end-facing seam: `LocalHost` serves a session
//! in-process, `DaemonHost` (see `daemon_host`) attaches to one hosted by the daemon.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::block::{Block, BlockId, BlockOrigin};
use crate::input::classify;
use crate::pty::{PtyHandle, default_shell, spawn_pty};
use crate::session::{Session, SessionEvent};

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
            cols: 80,
            rows: 24,
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
        let trimmed = cmd
            .trim()
            .strip_prefix('!')
            .or_else(|| cmd.trim().strip_prefix("/sh "))
            .or_else(|| cmd.trim().strip_prefix("/nu "))
            .unwrap_or(cmd.trim())
            .trim();
        let lower = trimmed.to_ascii_lowercase();

        // Always reject catastrophic floor commands at every level
        for forbidden in [
            "rm -rf /",
            "git push --force",
            "git reset --hard",
            "mkfs",
            "dd if=",
        ] {
            if lower.contains(forbidden) {
                return Err(format!(
                    "security level {self:?} blocked destructive floor pattern `{forbidden}`"
                ));
            }
        }

        match self {
            Self::L1ReadOnly => {
                let first = lower.split_whitespace().next().unwrap_or("");
                let read_only_allow = [
                    "echo", "ls", "pwd", "cat", "rg", "grep", "find", "head", "tail", "wc",
                    "which", "whoami", "env", "date", "true", "git",
                ];
                if !read_only_allow.contains(&first) {
                    return Err(format!(
                        "security level L1ReadOnly only permits read-only inspection commands (got `{first}`)"
                    ));
                }
                if first == "git" {
                    let sub = lower.split_whitespace().nth(1).unwrap_or("");
                    if !["status", "log", "diff", "show", "branch", "rev-parse"].contains(&sub) {
                        return Err(format!(
                            "security level L1ReadOnly only permits read-only git subcommands (got `git {sub}`)"
                        ));
                    }
                }
                Ok(())
            }
            Self::L2WorkspaceBuild => {
                for mutating in ["rm ", "mv ", "cp ", "chmod ", "chown ", "sudo ", "su "] {
                    if lower.starts_with(mutating) || lower.contains(&format!(" {mutating}")) {
                        return Err(format!(
                            "security level L2WorkspaceBuild blocked mutating command `{trimmed}`"
                        ));
                    }
                }
                Ok(())
            }
            Self::L3StructuredDefault => {
                for sys_cmd in ["sudo ", "su ", "systemctl ", "launchctl ", "brew ", "apt "] {
                    if lower.starts_with(sys_cmd) || lower.contains(&format!(" {sys_cmd}")) {
                        return Err(format!(
                            "security level L3StructuredDefault blocked system-level command `{trimmed}`"
                        ));
                    }
                }
                Ok(())
            }
            Self::L4HostGated | Self::L5AutopilotFloor => Ok(()),
        }
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub blocks: usize,
    #[serde(default)]
    pub security_level: TermSecurityLevel,
    #[serde(default)]
    pub control_mode: TermControlMode,
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
    /// Last `replay_cap` bytes of raw PTY output.
    pub replay: Vec<u8>,
    /// `seq` of the newest frame included in `replay` (0 = none yet).
    pub seq: u64,
    pub events: broadcast::Receiver<SessionEvent>,
    pub output: broadcast::Receiver<OutputFrame>,
}

/// What a front-end needs from a terminal session host, local or remote.
#[async_trait]
pub trait SessionHost: Send + Sync {
    async fn open(&self, spec: OpenSpec) -> Result<SessionId>;
    async fn list(&self) -> Result<Vec<SessionInfo>>;
    async fn attach(&self, id: &str) -> Result<Attachment>;
    async fn submit(&self, id: &str, line: &str) -> Result<BlockId>;
    async fn input(&self, id: &str, bytes: &[u8]) -> Result<()>;
    async fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()>;
    async fn close(&self, id: &str) -> Result<()>;
}

#[async_trait]
impl SessionHost for LocalHost {
    async fn open(&self, spec: OpenSpec) -> Result<SessionId> {
        LocalHost::open(self, spec).await
    }
    async fn list(&self) -> Result<Vec<SessionInfo>> {
        Ok(LocalHost::list(self))
    }
    async fn attach(&self, id: &str) -> Result<Attachment> {
        LocalHost::attach(self, id)
    }
    async fn submit(&self, id: &str, line: &str) -> Result<BlockId> {
        LocalHost::submit(self, id, line)
    }
    async fn input(&self, id: &str, bytes: &[u8]) -> Result<()> {
        LocalHost::input(self, id, bytes)
    }
    async fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        LocalHost::resize(self, id, cols, rows)
    }
    async fn close(&self, id: &str) -> Result<()> {
        LocalHost::close(self, id)
    }
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
    security_level: Mutex<TermSecurityLevel>,
    control_mode: Mutex<TermControlMode>,
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
            security_level: Mutex::new(TermSecurityLevel::default()),
            control_mode: Mutex::new(TermControlMode::default()),
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

    /// Spawn a PTY running the configured or default shell and start pumping its bytes into a
    /// new `Session`. Must be called inside a tokio runtime.
    pub async fn open(&self, spec: OpenSpec) -> Result<SessionId> {
        let def = default_shell();
        let shell = spec.shell.as_deref().unwrap_or(&def);
        let (handle, mut rx) = spawn_pty(shell, spec.cols, spec.rows)?;
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
                security_level: *lock(&e.security_level),
                control_mode: *lock(&e.control_mode),
            })
            .collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    pub fn set_security_level(&self, id: &str, level: TermSecurityLevel) -> Result<()> {
        let entry = self.entry(id)?;
        *lock(&entry.security_level) = level;
        Ok(())
    }

    pub fn security_level(&self, id: &str) -> Result<TermSecurityLevel> {
        let entry = self.entry(id)?;
        Ok(*lock(&entry.security_level))
    }

    pub fn set_control_mode(&self, id: &str, mode: TermControlMode) -> Result<()> {
        let entry = self.entry(id)?;
        *lock(&entry.control_mode) = mode;
        Ok(())
    }

    pub fn control_mode(&self, id: &str) -> Result<TermControlMode> {
        let entry = self.entry(id)?;
        Ok(*lock(&entry.control_mode))
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
        self.submit_with_origin(id, line, BlockOrigin::User)
    }

    /// Classify `line` and run it through the session with an explicit [`BlockOrigin`].
    pub fn submit_with_origin(&self, id: &str, line: &str, origin: BlockOrigin) -> Result<BlockId> {
        let entry = self.entry(id)?;
        match &origin {
            BlockOrigin::User => {
                let mut mode = lock(&entry.control_mode);
                if *mode == TermControlMode::Autopilot {
                    *mode = TermControlMode::Manual;
                }
            }
            BlockOrigin::Agent { .. } => {
                let level = *lock(&entry.security_level);
                level
                    .allows_agent_command(line)
                    .map_err(|reason| anyhow!(reason))?;
            }
            BlockOrigin::System => {}
        }
        let block = lock(&entry.session).submit_with_origin(classify(line), origin);
        Ok(block)
    }

    /// Return the last `max_blocks` blocks for `id` with `input` and `plain_output()`
    /// scrubbed through `crate::corpus::redact::redact_owned`.
    pub fn redacted_context_bundle(
        &self,
        id: &str,
        max_blocks: usize,
    ) -> Result<Vec<RedactedBlockContext>> {
        let entry = self.entry(id)?;
        let s = lock(&entry.session);
        let blocks = s.blocks();
        let start = blocks.len().saturating_sub(max_blocks);
        Ok(blocks[start..]
            .iter()
            .map(|b| RedactedBlockContext {
                id: b.id.0,
                input: crate::corpus::redact::redact_owned(&b.input),
                output: crate::corpus::redact::redact_owned(&b.plain_output()),
                exit_code: b.exit_code,
                origin: b.origin.clone(),
            })
            .collect())
    }

    /// Raw bytes to the PTY (keystrokes). Automatically drops `Autopilot` to `Manual` on user input.
    pub fn input(&self, id: &str, bytes: &[u8]) -> Result<()> {
        let entry = self.entry(id)?;
        if !bytes.is_empty() {
            let mut mode = lock(&entry.control_mode);
            if *mode == TermControlMode::Autopilot {
                *mode = TermControlMode::Manual;
            }
        }
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
}
