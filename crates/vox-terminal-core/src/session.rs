//! Session state machine: wires `Osc633Parser` → `Block` transitions.
//!
//! `Session` owns the block list and the current open block. It applies
//! `Osc633Event`s to blocks, assigns monotonic `BlockId`s, tracks live
//! `OSC 7` / `OSC 633;P;Cwd` + `GitBranch` properties, and emits
//! `SessionEvent`s for front-end rendering.

use tokio::sync::broadcast;

use crate::block::{Block, BlockId, BlockKind, BlockOrigin, BlockStatus, OutputChunk, Stream};
use crate::commands::{CommandRegistry, CommandResult};
use crate::input::InputIntent;
use crate::osc633::{Osc633Event, Osc633Parser};
use crate::pty::run_shell_capture;

#[path = "session_agent.rs"]
mod session_agent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SessionEvent {
    BlockOpened {
        id: BlockId,
        #[serde(default)]
        kind: Option<BlockKind>,
        #[serde(default)]
        input: String,
        #[serde(default)]
        origin: Option<BlockOrigin>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        git_branch: Option<String>,
    },
    OutputAppended {
        id: BlockId,
        chunk: OutputChunk,
    },
    BlockClosed {
        id: BlockId,
        #[serde(default)]
        exit_code: Option<i32>,
        #[serde(default)]
        status: Option<BlockStatus>,
    },
    AgentMessage {
        text: String,
    },
    CwdChanged {
        cwd: String,
        git_branch: Option<String>,
    },
    AltScreenChanged {
        active: bool,
    },
    SnapshotResync {
        blocks: Vec<Block>,
        #[serde(default)]
        resync_required: bool,
        #[serde(default)]
        alt_screen: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        git_branch: Option<String>,
    },
    BlockAwaitingInput {
        id: BlockId,
        prompt_tail: String,
        prompt_kind: crate::stall_detector::PromptKind,
        #[serde(default)]
        credential_prompt: bool,
    },
    BlockPinned {
        id: BlockId,
        pinned: bool,
    },
    SecurityLevelChanged {
        security_level: crate::host::TermSecurityLevel,
    },
    ControlModeChanged {
        control_mode: crate::host::TermControlMode,
    },
}

pub struct Session {
    pub id: String,
    blocks: Vec<Block>,
    next_id: u64,
    open_id: Option<BlockId>,
    saw_prompt_start: bool,
    prompt_ready: bool,
    in_exec: bool,
    cwd: Option<String>,
    git_branch: Option<String>,
    control_epoch: u64,
    max_blocks: usize,
    max_block_bytes: usize,
    parser: Osc633Parser,
    vt_shadow: crate::vt_shadow::VtShadow,
    tx: broadcast::Sender<SessionEvent>,
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        Self::with_size(id, 120, 32)
    }

    pub fn with_size(id: impl Into<String>, cols: u16, rows: u16) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            id: id.into(),
            blocks: vec![],
            next_id: 1,
            open_id: None,
            saw_prompt_start: false,
            prompt_ready: false,
            in_exec: false,
            cwd: None,
            git_branch: None,
            control_epoch: 0,
            max_blocks: session_agent::DEFAULT_MAX_SESSION_BLOCKS,
            max_block_bytes: session_agent::DEFAULT_MAX_BLOCK_OUTPUT_BYTES,
            parser: Osc633Parser::new(),
            vt_shadow: crate::vt_shadow::VtShadow::new(cols, rows),
            tx,
        }
    }

    pub fn new_stateless(id: impl Into<String>) -> Self {
        Self::new(id)
    }

    /// Subscribe to `SessionEvent`s (for front-ends).
    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.tx.subscribe()
    }

    /// Feed raw PTY bytes into the session. Parses OSC-633 / OSC-7 markers and updates
    /// the block list and working directory metadata accordingly.
    pub fn on_pty_bytes(&mut self, bytes: &[u8]) {
        let events = self.parser.feed(bytes);
        for ev in events {
            self.apply_osc(ev);
        }
    }

    /// Snapshot of all completed + in-flight blocks.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Currently active `Running` block ID, if a command is in flight.
    pub fn open_block_id(&self) -> Option<BlockId> {
        self.open_id
    }

    /// True once the shell has rendered `OSC 633;B` (`PromptEnd`) and is awaiting input.
    pub fn is_at_prompt(&self) -> bool {
        self.prompt_ready
    }

    /// Current working directory reported via `OSC 7` or `OSC 633;P;Cwd`.
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// Current git branch reported via `OSC 633;P;GitBranch`.
    pub fn git_branch(&self) -> Option<&str> {
        self.git_branch.as_deref()
    }

    /// True when the session's VT shadow grid is inside `TermMode::ALT_SCREEN`.
    pub fn is_alt_screen(&self) -> bool {
        self.vt_shadow.is_alt_screen()
    }

    /// Resize the session's `VtShadow` grid (`cols`/`rows` clamped to `>= 1`).
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.vt_shadow.resize(cols, rows);
    }

    pub(crate) fn emit_event(&self, ev: SessionEvent) {
        let _ = self.tx.send(ev);
    }

    pub(crate) fn mark_block_awaiting_input(
        &mut self,
        id: BlockId,
        sig: &crate::stall_detector::StallSignal,
    ) {
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
            b.status = BlockStatus::AwaitingInput;
            b.credential_prompt = sig.credential_prompt;
        }
        let _ = self.tx.send(SessionEvent::BlockAwaitingInput {
            id,
            prompt_tail: sig.prompt_tail.clone(),
            prompt_kind: sig.prompt_kind,
            credential_prompt: sig.credential_prompt,
        });
    }

    pub(crate) fn resume_awaiting_block(&mut self, id: BlockId) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.open_id == Some(id),
            "block {} is not the active open block",
            id.0
        );
        let b = self
            .blocks
            .iter_mut()
            .find(|b| b.id == id)
            .ok_or_else(|| anyhow::anyhow!("no such block {}", id.0))?;
        anyhow::ensure!(
            b.status == BlockStatus::AwaitingInput,
            "block {} is not awaiting input (status: {:?})",
            id.0,
            b.status
        );
        anyhow::ensure!(
            !b.credential_prompt,
            "CREDENTIAL_PROMPT_USER_ONLY: credential prompts cannot be answered by the agent; human input required in terminal pane"
        );
        b.status = BlockStatus::Running;
        Ok(())
    }

    pub(crate) fn set_block_pinned(&mut self, id: BlockId, pinned: bool) -> anyhow::Result<()> {
        let b = self
            .blocks
            .iter_mut()
            .find(|b| b.id == id)
            .ok_or_else(|| anyhow::anyhow!("no such block {}", id.0))?;
        b.pinned = pinned;
        let _ = self.tx.send(SessionEvent::BlockPinned { id, pinned });
        Ok(())
    }

    fn new_stamped_block(
        &self,
        id: BlockId,
        kind: BlockKind,
        input: impl Into<String>,
        origin: BlockOrigin,
    ) -> Block {
        let mut block = Block::new(id, kind, input).with_origin(origin);
        block.cwd = self.cwd.clone();
        block.git_branch = self.git_branch.clone();
        block
    }

    fn opened_event_for(block: &Block) -> SessionEvent {
        SessionEvent::BlockOpened {
            id: block.id,
            kind: Some(block.kind),
            input: block.input.clone(),
            origin: Some(block.origin.clone()),
            cwd: block.cwd.clone(),
            git_branch: block.git_branch.clone(),
        }
    }

    fn closed_event_for(block: &Block) -> SessionEvent {
        SessionEvent::BlockClosed {
            id: block.id,
            exit_code: block.exit_code,
            status: Some(block.status),
        }
    }

    /// Reserve and open a stateful PTY command block before writing `<cmd>\n` to the PTY.
    pub fn begin_pty_submission(&mut self, input: String, origin: BlockOrigin) -> BlockId {
        let id = BlockId(self.next_id);
        self.next_id += 1;
        let block = self.new_stamped_block(id, BlockKind::Shell, input, origin);
        let ev = Self::opened_event_for(&block);
        self.blocks.push(block);
        self.open_id = Some(id);
        self.saw_prompt_start = false;
        self.prompt_ready = false;
        self.in_exec = false;
        let _ = self.tx.send(ev);
        id
    }

    /// Close any still-open block when the underlying PTY reaches EOF (e.g. `exit N`).
    pub fn finish_open_block_on_eof(&mut self, exit_code: i32) {
        self.in_exec = false;
        self.prompt_ready = false;
        if let Some(id) = self.open_id.take() {
            let ev = if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
                b.finish(exit_code);
                Self::closed_event_for(b)
            } else {
                SessionEvent::BlockClosed {
                    id,
                    exit_code: Some(exit_code),
                    status: Some(if exit_code == 0 {
                        BlockStatus::Ok
                    } else {
                        BlockStatus::Failed
                    }),
                }
            };
            let _ = self.tx.send(ev);
        }
    }

    /// Cancel any open foreground block (e.g. on `TermSignal::Terminate` / `Kill`).
    pub fn cancel_open_block(&mut self) {
        self.in_exec = false;
        self.prompt_ready = false;
        if let Some(id) = self.open_id.take() {
            let ev = if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
                b.cancel();
                Self::closed_event_for(b)
            } else {
                SessionEvent::BlockClosed {
                    id,
                    exit_code: Some(130),
                    status: Some(BlockStatus::Cancelled),
                }
            };
            let _ = self.tx.send(ev);
        }
    }

    /// Dispatch a classified [`InputIntent`] (stateless fallback / slash / Vox / agent).
    pub fn submit(&mut self, intent: InputIntent) -> BlockId {
        self.submit_with_origin(intent, BlockOrigin::User)
    }

    /// Dispatch a classified [`InputIntent`] with an explicit [`BlockOrigin`].
    pub fn submit_with_origin(&mut self, intent: InputIntent, origin: BlockOrigin) -> BlockId {
        match intent {
            InputIntent::Shell(cmd) => {
                let (code, stdout, stderr) = run_shell_capture(&cmd);
                let mut chunks = Vec::new();
                if !stdout.is_empty() {
                    chunks.push(OutputChunk::text(Stream::Stdout, stdout));
                }
                if !stderr.is_empty() {
                    chunks.push(OutputChunk::text(Stream::Stderr, stderr));
                }
                self.push_block(BlockKind::Shell, origin, cmd, chunks, code)
            }
            InputIntent::StructuredShell(pipeline) => {
                let (code, stdout, stderr) = crate::pty::run_nu_structured_capture(&pipeline);
                let mut chunks = Vec::new();
                if !stdout.is_empty() {
                    chunks.push(OutputChunk::text(Stream::Stdout, stdout));
                }
                if !stderr.is_empty() {
                    chunks.push(OutputChunk::text(Stream::Stderr, stderr));
                }
                self.push_block(
                    BlockKind::Shell,
                    origin,
                    format!("!nu {pipeline}"),
                    chunks,
                    code,
                )
            }
            InputIntent::Command { name, args } => {
                let result = CommandRegistry::default().dispatch(&format!("/{name} {args}"));
                let (text, ok) = match result {
                    CommandResult::Output(s) => (s, true),
                    CommandResult::Silent => (String::new(), true),
                    CommandResult::NotFound => (format!("Unknown command: /{name}"), false),
                };
                let chunks = if text.is_empty() {
                    vec![]
                } else {
                    vec![OutputChunk::text(Stream::Stdout, text)]
                };
                self.push_block(
                    BlockKind::SlashCommand,
                    origin,
                    format!("/{name} {args}").trim().to_string(),
                    chunks,
                    if ok { 0 } else { 1 },
                )
            }
            InputIntent::VoxNative(src) => {
                let trimmed = src.trim();
                let is_explicit_vox = trimmed.starts_with("fn ")
                    || trimmed.starts_with("let ")
                    || trimmed.starts_with("type ")
                    || trimmed.starts_with("import ")
                    || trimmed.starts_with('@');
                let vox_src = if trimmed.contains("fn main") {
                    trimmed.to_string()
                } else {
                    format!("fn main() -> Int {{ {trimmed} }}")
                };
                match crate::vox_interp::eval_line(&vox_src) {
                    Ok(out) => self.push_block(
                        BlockKind::VoxNative,
                        origin,
                        src,
                        vec![OutputChunk::text(Stream::Stdout, out)],
                        0,
                    ),
                    Err(err) if is_explicit_vox => self.push_block(
                        BlockKind::VoxNative,
                        origin,
                        src,
                        vec![OutputChunk::text(Stream::Stderr, err.to_string())],
                        1,
                    ),
                    Err(_) => {
                        let (code, stdout, stderr) = run_shell_capture(trimmed);
                        let mut chunks = Vec::new();
                        if !stdout.is_empty() {
                            chunks.push(OutputChunk::text(Stream::Stdout, stdout));
                        }
                        if !stderr.is_empty() {
                            chunks.push(OutputChunk::text(Stream::Stderr, stderr));
                        }
                        self.push_block(BlockKind::Shell, origin, src, chunks, code)
                    }
                }
            }
            InputIntent::Agent(prompt) => {
                let routed = crate::agent::route_agent_prompt(&prompt);
                let text = routed.summary();
                let _ = self
                    .tx
                    .send(SessionEvent::AgentMessage { text: text.clone() });
                self.push_block(
                    BlockKind::AgentTurn,
                    origin,
                    prompt,
                    vec![OutputChunk::text(Stream::Agent, text)],
                    0,
                )
            }
        }
    }

    /// Push a fully-formed (already-completed) block and emit the same event
    /// sequence a PTY-driven block would (`Opened` → `OutputAppended`* → `Closed`).
    pub(crate) fn push_completed_block(
        &mut self,
        kind: BlockKind,
        origin: BlockOrigin,
        input: String,
        output: Vec<OutputChunk>,
        exit: i32,
    ) -> BlockId {
        self.push_block(kind, origin, input, output, exit)
    }

    fn push_block(
        &mut self,
        kind: BlockKind,
        origin: BlockOrigin,
        input: String,
        output: Vec<OutputChunk>,
        exit: i32,
    ) -> BlockId {
        let id = BlockId(self.next_id);
        self.next_id += 1;
        let mut block = self.new_stamped_block(id, kind, input, origin);
        let opened_ev = Self::opened_event_for(&block);
        for chunk in &output {
            block.push(chunk.clone());
        }
        block.finish(exit);
        let closed_ev = Self::closed_event_for(&block);
        self.blocks.push(block);
        let _ = self.tx.send(opened_ev);
        for chunk in output {
            let _ = self.tx.send(SessionEvent::OutputAppended { id, chunk });
        }
        let _ = self.tx.send(closed_ev);
        id
    }

    fn update_cwd(&mut self, new_cwd: String) {
        if new_cwd.is_empty() {
            return;
        }
        let changed = self.cwd.as_deref() != Some(new_cwd.as_str());
        self.cwd = Some(new_cwd.clone());
        if changed {
            let _ = self.tx.send(SessionEvent::CwdChanged {
                cwd: new_cwd,
                git_branch: self.git_branch.clone(),
            });
        }
    }

    fn update_git_branch(&mut self, raw_branch: String) {
        let normalized = if raw_branch.trim().is_empty() {
            None
        } else {
            Some(raw_branch.trim().to_string())
        };
        let changed = self.git_branch != normalized;
        self.git_branch = normalized;
        if changed && let Some(ref cwd) = self.cwd {
            let _ = self.tx.send(SessionEvent::CwdChanged {
                cwd: cwd.clone(),
                git_branch: self.git_branch.clone(),
            });
        }
    }

    fn apply_osc(&mut self, ev: Osc633Event) {
        match ev {
            Osc633Event::PromptStart => {
                if self.vt_shadow.is_alt_screen() {
                    return;
                }
                // If an in-flight command returned to prompt without emitting 633;D
                // (e.g. SIGINT interrupted foreground job), close the open block cleanly.
                if self.in_exec
                    && let Some(id) = self.open_id.take()
                {
                    let ev = if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
                        b.finish(130);
                        Self::closed_event_for(b)
                    } else {
                        SessionEvent::BlockClosed {
                            id,
                            exit_code: Some(130),
                            status: Some(BlockStatus::Failed),
                        }
                    };
                    let _ = self.tx.send(ev);
                }
                self.in_exec = false;
                self.saw_prompt_start = true;
            }
            Osc633Event::PromptEnd => {
                if self.vt_shadow.is_alt_screen() {
                    return;
                }
                self.prompt_ready = true;
            }
            Osc633Event::CommandLine(cmd) => {
                if self.vt_shadow.is_alt_screen() {
                    return;
                }
                self.saw_prompt_start = false;
                self.prompt_ready = false;
                if let Some(id) = self.open_id {
                    if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id)
                        && b.input.is_empty()
                    {
                        b.input = cmd;
                        let ev = Self::opened_event_for(b);
                        let _ = self.tx.send(ev);
                    }
                } else {
                    let id = BlockId(self.next_id);
                    self.next_id += 1;
                    let block =
                        self.new_stamped_block(id, BlockKind::Shell, cmd, BlockOrigin::User);
                    let ev = Self::opened_event_for(&block);
                    self.blocks.push(block);
                    self.open_id = Some(id);
                    let _ = self.tx.send(ev);
                }
            }
            Osc633Event::PreExec => {
                if self.vt_shadow.is_alt_screen() {
                    return;
                }
                self.saw_prompt_start = false;
                self.prompt_ready = false;
                self.in_exec = true;
                if self.open_id.is_none() {
                    let id = BlockId(self.next_id);
                    self.next_id += 1;
                    let block = self.new_stamped_block(id, BlockKind::Shell, "", BlockOrigin::User);
                    let ev = Self::opened_event_for(&block);
                    self.blocks.push(block);
                    self.open_id = Some(id);
                    let _ = self.tx.send(ev);
                }
            }
            Osc633Event::Output(text) => {
                let tx = self.tx.clone();
                let primary_text = self.vt_shadow.feed_extracting_primary(&text, |active| {
                    let _ = tx.send(SessionEvent::AltScreenChanged { active });
                });

                // Append only primary-screen bytes emitted during command execution (`633;C` .. `633;D`)
                // so kernel TTY echo before `633;C` and full-screen TUI redraws inside `alt_screen`
                // (including entry/exit/single-chunk frames) never pollute Block::plain_output().
                if !primary_text.is_empty()
                    && self.in_exec
                    && let Some(id) = self.open_id
                {
                    let chunk = OutputChunk::text(Stream::Stdout, primary_text);
                    if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
                        b.push(chunk.clone());
                    }
                    let _ = self.tx.send(SessionEvent::OutputAppended { id, chunk });
                }
            }
            Osc633Event::Exit(code) => {
                if self.vt_shadow.is_alt_screen() {
                    return;
                }
                self.in_exec = false;
                self.prompt_ready = false;
                if let Some(id) = self.open_id.take() {
                    let ev = if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
                        b.finish(code);
                        Self::closed_event_for(b)
                    } else {
                        SessionEvent::BlockClosed {
                            id,
                            exit_code: Some(code),
                            status: Some(if code == 0 {
                                BlockStatus::Ok
                            } else {
                                BlockStatus::Failed
                            }),
                        }
                    };
                    let _ = self.tx.send(ev);
                } else if self.saw_prompt_start {
                    // Preserve synthetic `633;A` -> `633;D` compatibility for unit tests.
                    self.saw_prompt_start = false;
                    self.push_block(
                        BlockKind::Shell,
                        BlockOrigin::User,
                        String::new(),
                        vec![],
                        code,
                    );
                }
            }
            Osc633Event::Property { key, value } => match key.as_str() {
                "Cwd" => self.update_cwd(value),
                "GitBranch" => self.update_git_branch(value),
                _ => {}
            },
            Osc633Event::Osc7Cwd(path) => self.update_cwd(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockStatus;

    #[test]
    fn submit_shell_runs_the_command_and_captures_output() {
        let mut session = Session::new("test");
        let id = session.submit(InputIntent::Shell("echo hi-from-shell".to_string()));

        let block = session.blocks().iter().find(|b| b.id == id).unwrap();
        assert_eq!(block.kind, BlockKind::Shell);
        assert_eq!(block.status, BlockStatus::Ok);
        assert_eq!(block.exit_code, Some(0));
        assert!(
            block.plain_output().contains("hi-from-shell"),
            "unexpected output: {:?}",
            block.plain_output()
        );
    }

    #[test]
    fn submit_shell_surfaces_nonzero_exit_as_failed() {
        let mut session = Session::new("test");
        let id = session.submit(InputIntent::Shell("exit 7".to_string()));

        let block = session.blocks().iter().find(|b| b.id == id).unwrap();
        assert_eq!(block.status, BlockStatus::Failed);
        assert_eq!(block.exit_code, Some(7));
    }

    #[test]
    fn submit_known_command_dispatches_through_registry() {
        let mut session = Session::new("test");
        let id = session.submit(InputIntent::Command {
            name: "help".to_string(),
            args: String::new(),
        });

        let block = session.blocks().iter().find(|b| b.id == id).unwrap();
        assert_eq!(block.kind, BlockKind::SlashCommand);
        assert_eq!(block.status, BlockStatus::Ok);
        assert!(block.plain_output().contains("/help"));
    }

    #[test]
    fn submit_unknown_command_reports_failure() {
        let mut session = Session::new("test");
        let id = session.submit(InputIntent::Command {
            name: "zzzzznope".to_string(),
            args: String::new(),
        });

        let block = session.blocks().iter().find(|b| b.id == id).unwrap();
        assert_eq!(block.status, BlockStatus::Failed);
        assert!(block.plain_output().contains("Unknown command"));
    }

    #[test]
    fn submit_emits_block_lifecycle_events() {
        let mut session = Session::new("test");
        let mut rx = session.subscribe();
        let id = session.submit(InputIntent::Shell("echo x".to_string()));

        let mut saw_opened = false;
        let mut saw_closed = false;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                SessionEvent::BlockOpened { id: eid, .. } if eid == id => saw_opened = true,
                SessionEvent::BlockClosed { id: eid, .. } if eid == id => saw_closed = true,
                _ => {}
            }
        }
        assert!(saw_opened, "expected a BlockOpened event");
        assert!(saw_closed, "expected a BlockClosed event");
    }
}
