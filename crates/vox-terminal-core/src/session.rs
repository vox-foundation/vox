//! Session state machine: wires `Osc633Parser` → `Block` transitions.
//!
//! `Session` owns the block list and the current open block. It applies
//! `Osc633Event`s to blocks, assigns monotonic `BlockId`s, and emits
//! `SessionEvent`s for front-end rendering.

use tokio::sync::broadcast;

use crate::block::{Block, BlockId, BlockKind, OutputChunk, Stream};
use crate::commands::{CommandRegistry, CommandResult};
use crate::input::InputIntent;
use crate::osc633::{Osc633Event, Osc633Parser};
use crate::pty::run_shell_capture;

#[derive(Debug, Clone)]
pub enum SessionEvent {
    BlockOpened { id: BlockId },
    OutputAppended { id: BlockId, chunk: OutputChunk },
    BlockClosed { id: BlockId },
    AgentMessage { text: String },
}

pub struct Session {
    pub id: String,
    blocks: Vec<Block>,
    next_id: u64,
    open_id: Option<BlockId>,
    parser: Osc633Parser,
    tx: broadcast::Sender<SessionEvent>,
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            id: id.into(),
            blocks: vec![],
            next_id: 1,
            open_id: None,
            parser: Osc633Parser::new(),
            tx,
        }
    }

    /// Subscribe to `SessionEvent`s (for front-ends).
    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.tx.subscribe()
    }

    /// Feed raw PTY bytes into the session. Parses OSC-633 markers and updates
    /// the block list accordingly.
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

    /// Dispatch a classified [`InputIntent`] (from `InputBox::submit`) and
    /// return the id of the block it produced. Track 4: this is the wiring
    /// `app.rs`'s Enter handler was previously discarding.
    ///
    /// `Shell` and `Command` intents actually execute; `VoxNative` and
    /// `Agent` are acknowledged with a stub block rather than silently
    /// dropped — full REPL eval / agent dispatch is out of scope here.
    pub fn submit(&mut self, intent: InputIntent) -> BlockId {
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
                self.push_block(BlockKind::Shell, cmd, chunks, code)
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
                    format!("/{name} {args}").trim().to_string(),
                    chunks,
                    if ok { 0 } else { 1 },
                )
            }
            InputIntent::VoxNative(src) => self.push_block(
                BlockKind::VoxNative,
                src,
                vec![OutputChunk::text(
                    Stream::Stdout,
                    "vox-native eval not yet wired to the TUI",
                )],
                0,
            ),
            InputIntent::Agent(prompt) => self.push_block(
                BlockKind::AgentTurn,
                prompt,
                vec![OutputChunk::text(
                    Stream::Agent,
                    "agent dispatch not yet wired to the TUI",
                )],
                0,
            ),
        }
    }

    /// Push a fully-formed (already-completed) block and emit the same event
    /// sequence a PTY-driven block would (`Opened` → `OutputAppended`* → `Closed`).
    fn push_block(
        &mut self,
        kind: BlockKind,
        input: String,
        output: Vec<OutputChunk>,
        exit: i32,
    ) -> BlockId {
        let id = BlockId(self.next_id);
        self.next_id += 1;
        let mut block = Block::new(id, kind, input);
        let _ = self.tx.send(SessionEvent::BlockOpened { id });
        for chunk in output {
            block.push(chunk.clone());
            let _ = self.tx.send(SessionEvent::OutputAppended {
                id,
                chunk: chunk.clone(),
            });
        }
        block.finish(exit);
        self.blocks.push(block);
        let _ = self.tx.send(SessionEvent::BlockClosed { id });
        id
    }

    fn apply_osc(&mut self, ev: Osc633Event) {
        match ev {
            Osc633Event::PromptStart => {
                // Close any open block without an exit (e.g. interrupted)
                if let Some(id) = self.open_id.take() {
                    if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
                        b.finish(1);
                    }
                    let _ = self.tx.send(SessionEvent::BlockClosed { id });
                }
                // Start a new pending block
                let id = BlockId(self.next_id);
                self.next_id += 1;
                self.blocks.push(Block::new(id, BlockKind::Shell, ""));
                self.open_id = Some(id);
                let _ = self.tx.send(SessionEvent::BlockOpened { id });
            }
            Osc633Event::CommandLine(cmd) => {
                if let Some(id) = self.open_id
                    && let Some(b) = self.blocks.iter_mut().find(|b| b.id == id)
                {
                    b.input = cmd;
                }
            }
            Osc633Event::PreExec => {
                // Output capture begins; nothing structural to do here
            }
            Osc633Event::Output(text) => {
                if let Some(id) = self.open_id {
                    let chunk = OutputChunk::text(Stream::Stdout, &text);
                    if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
                        b.push(chunk.clone());
                    }
                    let _ = self.tx.send(SessionEvent::OutputAppended { id, chunk });
                }
            }
            Osc633Event::Exit(code) => {
                if let Some(id) = self.open_id.take() {
                    if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
                        b.finish(code);
                    }
                    let _ = self.tx.send(SessionEvent::BlockClosed { id });
                }
            }
            Osc633Event::PromptEnd => {}
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
                SessionEvent::BlockOpened { id: eid } if eid == id => saw_opened = true,
                SessionEvent::BlockClosed { id: eid } if eid == id => saw_closed = true,
                _ => {}
            }
        }
        assert!(saw_opened, "expected a BlockOpened event");
        assert!(saw_closed, "expected a BlockClosed event");
    }
}
