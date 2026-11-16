//! Custom block streaming (`AgentTurn` / `SlashCommand`), `/clear` & `/compact`
//! `SnapshotResync` mutations, and `control_epoch` preemption helpers for [`Session`].

use crate::block::{Block, BlockId, BlockKind, BlockOrigin, BlockStatus, OutputChunk, Stream};
use crate::session::{Session, SessionEvent};

pub const DEFAULT_MAX_SESSION_BLOCKS: usize = 512;
pub const DEFAULT_MAX_BLOCK_OUTPUT_BYTES: usize = 256 * 1024;

impl Session {
    pub fn control_epoch(&self) -> u64 {
        self.control_epoch
    }

    pub fn set_max_blocks(&mut self, max_blocks: usize) {
        self.max_blocks = max_blocks.max(1);
        self.evict_excess_blocks();
    }

    fn evict_excess_blocks(&mut self) {
        while self.blocks.len() > self.max_blocks {
            let Some(idx) = self.blocks.iter().position(|b| {
                !b.pinned && !matches!(b.status, BlockStatus::Running | BlockStatus::AwaitingInput)
            }) else {
                break;
            };
            self.blocks.remove(idx);
        }
    }

    pub fn bump_control_epoch(&mut self) -> (u64, Vec<SessionEvent>) {
        self.control_epoch = self.control_epoch.saturating_add(1);
        let mut events = Vec::new();
        for b in &mut self.blocks {
            if b.kind == BlockKind::AgentTurn && b.status == BlockStatus::Running {
                b.cancel();
                let ev = Self::closed_event_for(b);
                let _ = self.tx.send(ev.clone());
                events.push(ev);
            }
        }
        (self.control_epoch, events)
    }

    pub fn begin_custom_block(
        &mut self,
        kind: BlockKind,
        origin: BlockOrigin,
        input: impl Into<String>,
    ) -> (BlockId, Vec<SessionEvent>) {
        let id = BlockId(self.next_id);
        self.next_id += 1;
        let block = self.new_stamped_block(id, kind, input, origin);
        let ev = Self::opened_event_for(&block);
        self.blocks.push(block);
        self.evict_excess_blocks();
        let _ = self.tx.send(ev.clone());
        (id, vec![ev])
    }

    pub fn append_block_chunk(&mut self, id: BlockId, chunk: OutputChunk) -> Vec<SessionEvent> {
        let max_bytes = self.max_block_bytes;
        let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) else {
            return vec![];
        };
        if let Some(last) = b.output.last_mut()
            && last.stream == chunk.stream
        {
            last.text.push_str(&chunk.text);
        } else {
            b.push(chunk.clone());
        }
        let mut total_bytes: usize = b.output.iter().map(|c| c.text.len()).sum();
        while total_bytes > max_bytes && !b.output.is_empty() {
            let excess = total_bytes - max_bytes;
            if b.output[0].text.len() <= excess {
                total_bytes -= b.output[0].text.len();
                b.output.remove(0);
            } else {
                let mut cut = excess;
                while cut < b.output[0].text.len() && !b.output[0].text.is_char_boundary(cut) {
                    cut += 1;
                }
                b.output[0].text.drain(..cut);
                break;
            }
        }
        let ev = SessionEvent::OutputAppended { id, chunk };
        let _ = self.tx.send(ev.clone());
        vec![ev]
    }

    pub fn finish_custom_block(
        &mut self,
        id: BlockId,
        exit_code: i32,
    ) -> Option<(Block, Vec<SessionEvent>)> {
        let b = self.blocks.iter_mut().find(|b| b.id == id)?;
        b.finish(exit_code);
        let closed = b.clone();
        let ev = Self::closed_event_for(&closed);
        let _ = self.tx.send(ev.clone());
        Some((closed, vec![ev]))
    }

    pub fn cancel_custom_block(&mut self, id: BlockId) -> Option<(Block, Vec<SessionEvent>)> {
        let b = self.blocks.iter_mut().find(|b| b.id == id)?;
        b.cancel();
        let closed = b.clone();
        let ev = Self::closed_event_for(&closed);
        let _ = self.tx.send(ev.clone());
        Some((closed, vec![ev]))
    }

    pub fn clear_unpinned_blocks(&mut self) -> Vec<SessionEvent> {
        self.blocks.retain(|b| {
            b.pinned || matches!(b.status, BlockStatus::Running | BlockStatus::AwaitingInput)
        });
        let ev = SessionEvent::SnapshotResync {
            blocks: self.blocks.clone(),
            resync_required: false,
            alt_screen: self.is_alt_screen(),
            cwd: self.cwd.clone(),
            git_branch: self.git_branch.clone(),
        };
        let _ = self.tx.send(ev.clone());
        vec![ev]
    }

    pub fn compact_blocks(
        &mut self,
        summary_markdown: impl Into<String>,
        keep_recent: usize,
    ) -> (Option<BlockId>, Vec<SessionEvent>) {
        let unpinned_completed_indices: Vec<usize> = self
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                !b.pinned
                    && !b.credential_prompt
                    && !matches!(b.status, BlockStatus::Running | BlockStatus::AwaitingInput)
            })
            .map(|(i, _)| i)
            .collect();
        if unpinned_completed_indices.len() <= keep_recent {
            return (None, vec![]);
        }
        let drop_count = unpinned_completed_indices.len() - keep_recent;
        let to_compact = &unpinned_completed_indices[..drop_count];

        let digest_id = BlockId(self.next_id);
        self.next_id += 1;
        let mut digest = self.new_stamped_block(
            digest_id,
            BlockKind::SlashCommand,
            "/compact",
            BlockOrigin::Agent {
                agent_id: "terminal-compact".into(),
            },
        );
        digest.push(OutputChunk::text(Stream::Agent, summary_markdown));
        digest.finish(0);

        let mut next_blocks = Vec::with_capacity(self.blocks.len() - drop_count + 1);
        next_blocks.push(digest);
        for (idx, block) in std::mem::take(&mut self.blocks).into_iter().enumerate() {
            if to_compact.binary_search(&idx).is_err() {
                next_blocks.push(block);
            }
        }
        self.blocks = next_blocks;
        let ev = SessionEvent::SnapshotResync {
            blocks: self.blocks.clone(),
            resync_required: false,
            alt_screen: self.is_alt_screen(),
            cwd: self.cwd.clone(),
            git_branch: self.git_branch.clone(),
        };
        let _ = self.tx.send(ev.clone());
        (Some(digest_id), vec![ev])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_block_streams_without_interleaving_pty_shell_bytes() {
        let mut session = Session::new("s-agent");
        let shell_id = session.begin_pty_submission("cargo test".into(), BlockOrigin::User);
        session.on_pty_bytes(b"\x1b]633;C\x07running tests...\n");
        assert_eq!(session.open_block_id(), Some(shell_id));

        let (agent_id, open_evs) = session.begin_custom_block(
            BlockKind::AgentTurn,
            BlockOrigin::Agent {
                agent_id: "terminal-agent".into(),
            },
            "? find open ports",
        );
        assert_ne!(agent_id, shell_id);
        assert_eq!(session.open_block_id(), Some(shell_id));
        assert!(matches!(
            open_evs.as_slice(),
            [SessionEvent::BlockOpened { id, kind: Some(BlockKind::AgentTurn), input, .. }]
                if *id == agent_id && input == "? find open ports"
        ));

        session.on_pty_bytes(b"shell-line-2\n");
        let append_evs =
            session.append_block_chunk(agent_id, OutputChunk::text(Stream::Agent, "Use `lsof -i`"));
        assert!(matches!(
            append_evs.as_slice(),
            [SessionEvent::OutputAppended { id, chunk }]
                if *id == agent_id && chunk.stream == Stream::Agent && chunk.text == "Use `lsof -i`"
        ));
        let shell_block = session.blocks().iter().find(|b| b.id == shell_id).unwrap();
        assert!(shell_block.plain_output().contains("shell-line-2"));
        assert!(!shell_block.plain_output().contains("lsof"));

        let (closed_agent, close_evs) = session.finish_custom_block(agent_id, 0).unwrap();
        assert_eq!(closed_agent.status, BlockStatus::Ok);
        assert_eq!(closed_agent.plain_output(), "Use `lsof -i`");
        assert!(matches!(
            close_evs.as_slice(),
            [SessionEvent::BlockClosed { id, exit_code: Some(0), status: Some(BlockStatus::Ok) }]
                if *id == agent_id
        ));
        assert_eq!(session.open_block_id(), Some(shell_id));

        let (fail_id, _) =
            session.begin_custom_block(BlockKind::SlashCommand, BlockOrigin::User, "/budget bad");
        assert_eq!(
            session.finish_custom_block(fail_id, 2).unwrap().0.status,
            BlockStatus::Failed
        );
        let (cancel_id, _) =
            session.begin_custom_block(BlockKind::AgentTurn, BlockOrigin::User, "? cancel");
        let cancelled = session.cancel_custom_block(cancel_id).unwrap().0;
        assert_eq!(cancelled.status, BlockStatus::Cancelled);
        assert_eq!(cancelled.exit_code, Some(130));
    }

    #[test]
    fn clear_and_compact_preserve_pinned_and_running_blocks_and_emit_snapshot_resync() {
        let mut session = Session::new("s-compact");
        let (b1, _) = session.begin_custom_block(BlockKind::SlashCommand, BlockOrigin::User, "/m1");
        session.finish_custom_block(b1, 0);
        let (b2, _) = session.begin_custom_block(BlockKind::SlashCommand, BlockOrigin::User, "/m2");
        session.finish_custom_block(b2, 0);
        session.set_block_pinned(b2, true).unwrap();
        let (b3, _) = session.begin_custom_block(BlockKind::SlashCommand, BlockOrigin::User, "/m3");
        session.finish_custom_block(b3, 0);
        let (b_cred, _) = session.begin_custom_block(BlockKind::Shell, BlockOrigin::User, "sudo");
        session.mark_block_awaiting_input(
            b_cred,
            &crate::stall_detector::StallSignal {
                prompt_tail: "Password:".into(),
                prompt_kind: crate::stall_detector::PromptKind::Credential,
                credential_prompt: true,
                silence_ms: 1500,
            },
        );
        session.finish_custom_block(b_cred, 0);
        let (b4, _) = session.begin_custom_block(BlockKind::SlashCommand, BlockOrigin::User, "/m4");
        session.finish_custom_block(b4, 0);
        let (b5, _) = session.begin_custom_block(
            BlockKind::AgentTurn,
            BlockOrigin::Agent {
                agent_id: "live".into(),
            },
            "? still running",
        );

        let (digest_id_opt, compact_evs) = session.compact_blocks("Digest text", 1);
        let digest_id = digest_id_opt.expect("older blocks compacted");
        let ids: Vec<BlockId> = session.blocks().iter().map(|b| b.id).collect();
        assert_eq!(ids, vec![digest_id, b2, b_cred, b4, b5]);
        let digest = &session.blocks()[0];
        assert_eq!(digest.kind, BlockKind::SlashCommand);
        assert_eq!(digest.status, BlockStatus::Ok);
        assert_eq!(digest.input, "/compact");
        assert_eq!(
            digest.origin,
            BlockOrigin::Agent {
                agent_id: "terminal-compact".into()
            }
        );
        assert_eq!(
            digest.output,
            vec![OutputChunk::text(Stream::Agent, "Digest text")]
        );
        assert!(matches!(
            compact_evs.as_slice(),
            [SessionEvent::SnapshotResync { blocks, resync_required: false, .. }] if blocks.len() == 5
        ));

        let clear_evs = session.clear_unpinned_blocks();
        let remaining_ids: Vec<BlockId> = session.blocks().iter().map(|b| b.id).collect();
        assert_eq!(remaining_ids, vec![b2, b5]);
        assert!(matches!(
            clear_evs.as_slice(),
            [SessionEvent::SnapshotResync { blocks, resync_required: false, .. }] if blocks.len() == 2
        ));
    }

    #[test]
    fn bump_control_epoch_cancels_running_agent_turn_blocks_and_ring_evicts_unpinned() {
        let mut session = Session::new("s-epoch");
        let (agent_id, _) = session.begin_custom_block(
            BlockKind::AgentTurn,
            BlockOrigin::Agent {
                agent_id: "autopilot".into(),
            },
            "? streaming",
        );
        let (epoch, evs) = session.bump_control_epoch();
        assert_eq!(epoch, 1);
        assert_eq!(session.control_epoch(), 1);
        assert_eq!(
            session
                .blocks()
                .iter()
                .find(|b| b.id == agent_id)
                .unwrap()
                .status,
            BlockStatus::Cancelled
        );
        assert!(matches!(
            evs.as_slice(),
            [SessionEvent::BlockClosed { id, status: Some(BlockStatus::Cancelled), .. }]
                if *id == agent_id
        ));

        session.set_max_blocks(2);
        let (p_id, _) =
            session.begin_custom_block(BlockKind::SlashCommand, BlockOrigin::User, "/pin");
        session.finish_custom_block(p_id, 0);
        session.set_block_pinned(p_id, true).unwrap();
        let (e_id, _) =
            session.begin_custom_block(BlockKind::SlashCommand, BlockOrigin::User, "/extra");
        session.finish_custom_block(e_id, 0);
        assert_eq!(session.blocks().len(), 2);
        assert!(session.blocks().iter().any(|b| b.id == p_id));
        assert!(session.blocks().iter().any(|b| b.id == e_id));
    }
}
