//! Agent block streaming, `/clear` & `/compact` `SnapshotResync`, `control_epoch`,
//! and `TranscriptSink` feedback wiring for [`LocalHost`].

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use anyhow::{Result, anyhow};

use crate::block::{Block, BlockId, BlockKind, BlockOrigin, OutputChunk};
use crate::corpus::writer::{CorpusWriter, RedactingTranscriptSink};
use crate::host::{LocalHost, lock};
use crate::transcript::{JournalSink, TranscriptEvent, TranscriptKind, TranscriptSink};

/// Operator verdict recorded on a terminal block for the MENS training flywheel.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockFeedbackVerdict {
    Accepted,
    Rejected,
    Corrected {
        corrected_command: String,
        #[serde(default)]
        original_command: Option<String>,
    },
}

impl LocalHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_transcript_sink(mut self, sink: Arc<dyn TranscriptSink>) -> Self {
        self.transcript_sink = Some(sink);
        self
    }

    pub fn open_default_journal_sink(
        base_dir: &Path,
        extra_scrub: impl Fn(&str) -> String + Send + Sync + 'static,
    ) -> Result<Arc<dyn TranscriptSink>> {
        std::fs::create_dir_all(base_dir)?;
        let journal = JournalSink::open(base_dir.join("transcript.journal"))?;
        let corpus = CorpusWriter::open(base_dir.join("corpus.jsonl"))?;
        Ok(Arc::new(RedactingTranscriptSink::new(
            Arc::new(journal),
            Some(corpus),
            extra_scrub,
        )))
    }

    pub fn begin_custom_block(
        &self,
        id: &str,
        kind: BlockKind,
        origin: BlockOrigin,
        input: String,
    ) -> Result<BlockId> {
        let entry = self.entry(id)?;
        let (bid, _) = lock(&entry.session).begin_custom_block(kind, origin, input);
        entry.settled_cv.notify_all();
        Ok(bid)
    }

    pub fn append_block_chunk(
        &self,
        id: &str,
        block_id: BlockId,
        chunk: OutputChunk,
    ) -> Result<()> {
        let entry = self.entry(id)?;
        let _ = lock(&entry.session).append_block_chunk(block_id, chunk);
        entry.settled_cv.notify_all();
        Ok(())
    }

    pub fn finish_custom_block(
        &self,
        id: &str,
        block_id: BlockId,
        exit_code: i32,
        cancelled: bool,
    ) -> Result<Option<Block>> {
        let entry = self.entry(id)?;
        let closed = {
            let mut s = lock(&entry.session);
            if cancelled {
                s.cancel_custom_block(block_id).map(|(b, _)| b)
            } else {
                s.finish_custom_block(block_id, exit_code).map(|(b, _)| b)
            }
        };
        entry.settled_cv.notify_all();
        Ok(closed)
    }

    pub fn get_block(&self, id: &str, block_id: BlockId) -> Result<Option<Block>> {
        let entry = self.entry(id)?;
        Ok(lock(&entry.session)
            .blocks()
            .iter()
            .find(|b| b.id == block_id)
            .cloned())
    }

    pub fn clear_unpinned_blocks(&self, id: &str) -> Result<usize> {
        let entry = self.entry(id)?;
        if let Some(ref sink) = self.transcript_sink {
            sink.flush()?;
        }
        let remaining = {
            let mut s = lock(&entry.session);
            let _ = s.clear_unpinned_blocks();
            s.blocks().len()
        };
        entry.settled_cv.notify_all();
        Ok(remaining)
    }

    pub fn compact_blocks(
        &self,
        id: &str,
        summary_markdown: String,
        keep_recent: usize,
    ) -> Result<Option<BlockId>> {
        let entry = self.entry(id)?;
        if let Some(ref sink) = self.transcript_sink {
            sink.flush()?;
        }
        let (digest_id, _) = lock(&entry.session).compact_blocks(summary_markdown, keep_recent);
        entry.settled_cv.notify_all();
        Ok(digest_id)
    }

    pub fn record_block_feedback(
        &self,
        id: &str,
        block_id: BlockId,
        verdict: BlockFeedbackVerdict,
    ) -> Result<bool> {
        let entry = self.entry(id)?;
        let maybe_block = lock(&entry.session)
            .blocks()
            .iter()
            .find(|b| b.id == block_id)
            .cloned();
        if maybe_block.as_ref().is_some_and(|b| b.credential_prompt) {
            return Ok(false);
        }
        let kind = match verdict {
            BlockFeedbackVerdict::Accepted => TranscriptKind::Accepted {
                block: maybe_block
                    .ok_or_else(|| anyhow!("no such block {} in session {id}", block_id.0))?,
            },
            BlockFeedbackVerdict::Rejected => TranscriptKind::Rejected {
                block: maybe_block
                    .ok_or_else(|| anyhow!("no such block {} in session {id}", block_id.0))?,
            },
            BlockFeedbackVerdict::Corrected {
                corrected_command,
                original_command,
            } => {
                let from = match original_command
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    Some(orig) => orig.to_string(),
                    None => {
                        maybe_block
                            .ok_or_else(|| anyhow!("no such block {} in session {id}", block_id.0))?
                            .input
                    }
                };
                TranscriptKind::Corrected {
                    from,
                    to: corrected_command,
                }
            }
        };
        let seq = self.transcript_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let ev = TranscriptEvent {
            session_id: id.to_string(),
            seq,
            kind,
        };
        if let Some(ref sink) = self.transcript_sink {
            sink.append(&ev)?;
        }
        Ok(true)
    }

    pub fn control_epoch(&self, id: &str) -> Result<u64> {
        let entry = self.entry(id)?;
        Ok(lock(&entry.session).control_epoch())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{BlockStatus, Stream};
    use crate::host::TermControlMode;
    use crate::session::SessionEvent;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicUsize;

    struct SpySink {
        events: Mutex<Vec<TranscriptEvent>>,
        flushes: AtomicUsize,
    }

    impl SpySink {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                events: Mutex::new(Vec::new()),
                flushes: AtomicUsize::new(0),
            })
        }
    }

    impl TranscriptSink for SpySink {
        fn append(&self, event: &TranscriptEvent) -> Result<()> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }

        fn flush(&self) -> Result<()> {
            self.flushes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn localhost_custom_blocks_feedback_flush_before_clear_and_autopilot_preemption() {
        let spy = SpySink::new();
        let host = LocalHost::default().with_transcript_sink(spy.clone());
        let sid = host.open_synthetic(120, 32);
        let mut att = host.attach(&sid).unwrap();

        let b1 = host
            .begin_custom_block(
                &sid,
                BlockKind::SlashCommand,
                BlockOrigin::User,
                "/context".into(),
            )
            .unwrap();
        host.append_block_chunk(&sid, b1, OutputChunk::text(Stream::Agent, "ctx-1"))
            .unwrap();
        let finished = host
            .finish_custom_block(&sid, b1, 0, false)
            .unwrap()
            .unwrap();
        assert_eq!(finished.status, BlockStatus::Ok);
        assert_eq!(
            host.get_block(&sid, b1).unwrap().unwrap().plain_output(),
            "ctx-1"
        );

        assert!(
            host.record_block_feedback(&sid, b1, BlockFeedbackVerdict::Accepted)
                .unwrap()
        );
        assert!(
            host.record_block_feedback(
                &sid,
                b1,
                BlockFeedbackVerdict::Corrected {
                    corrected_command: "/context --full".into(),
                    original_command: None,
                }
            )
            .unwrap()
        );
        {
            let recorded = spy.events.lock().unwrap();
            assert_eq!(recorded.len(), 2);
            assert!(matches!(recorded[0].kind, TranscriptKind::Accepted { .. }));
            assert!(matches!(
                recorded[1].kind,
                TranscriptKind::Corrected { ref from, ref to }
                    if from == "/context" && to == "/context --full"
            ));
        }

        assert_eq!(spy.flushes.load(Ordering::SeqCst), 0);
        let remaining = host.clear_unpinned_blocks(&sid).unwrap();
        assert_eq!(remaining, 0);
        assert_eq!(spy.flushes.load(Ordering::SeqCst), 1);

        let mut saw_resync = false;
        while let Ok(ev) = att.events.try_recv() {
            if matches!(
                ev,
                SessionEvent::SnapshotResync {
                    resync_required: false,
                    ..
                }
            ) {
                saw_resync = true;
            }
        }
        assert!(
            saw_resync,
            "expected SnapshotResync on clear_unpinned_blocks"
        );

        host.set_control_mode(&sid, TermControlMode::Autopilot)
            .unwrap();
        let agent_bid = host
            .begin_custom_block(
                &sid,
                BlockKind::AgentTurn,
                BlockOrigin::Agent {
                    agent_id: "auto".into(),
                },
                "? work".into(),
            )
            .unwrap();
        assert_eq!(host.control_epoch(&sid).unwrap(), 0);
        host.set_control_mode(&sid, TermControlMode::Manual)
            .unwrap();
        assert_eq!(host.control_epoch(&sid).unwrap(), 1);
        assert_eq!(
            host.get_block(&sid, agent_bid).unwrap().unwrap().status,
            BlockStatus::Cancelled
        );
    }

    #[test]
    fn corrected_feedback_uses_explicit_original_command_and_honors_credential_guard() {
        let spy = SpySink::new();
        let host = LocalHost::default().with_transcript_sink(spy.clone());
        let sid = host.open_synthetic(80, 24);
        let agent_org = BlockOrigin::Agent {
            agent_id: "auto".into(),
        };
        let agent_bid = host
            .begin_custom_block(&sid, BlockKind::AgentTurn, agent_org, "? fix".into())
            .unwrap();
        host.finish_custom_block(&sid, agent_bid, 0, false).unwrap();

        let corr = |cmd: &str, orig: Option<&str>| BlockFeedbackVerdict::Corrected {
            corrected_command: cmd.into(),
            original_command: orig.map(Into::into),
        };
        assert!(
            host.record_block_feedback(
                &sid,
                BlockId(0),
                corr("cargo check -p vox-cli", Some("cargo test --broken"))
            )
            .unwrap()
        );
        assert!(
            host.record_block_feedback(
                &sid,
                agent_bid,
                corr("cargo clippy", Some("  cargo test --all  "))
            )
            .unwrap()
        );
        assert!(
            host.record_block_feedback(&sid, BlockId(999), corr("ls", None))
                .is_err()
        );

        let cred_bid = host
            .begin_custom_block(&sid, BlockKind::Shell, BlockOrigin::User, "sudo".into())
            .unwrap();
        let sig = crate::stall_detector::StallSignal {
            prompt_tail: "Password:".into(),
            prompt_kind: crate::stall_detector::PromptKind::Credential,
            credential_prompt: true,
            silence_ms: 1500,
        };
        lock(&host.entry(&sid).unwrap().session).mark_block_awaiting_input(cred_bid, &sig);
        assert!(
            !host
                .record_block_feedback(&sid, cred_bid, corr("echo ok", Some("sudo")))
                .unwrap()
        );

        let recorded = spy.events.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        assert!(
            matches!(recorded[0].kind, TranscriptKind::Corrected { ref from, ref to } if from == "cargo test --broken" && to == "cargo check -p vox-cli")
        );
        assert!(
            matches!(recorded[1].kind, TranscriptKind::Corrected { ref from, ref to } if from == "cargo test --all" && to == "cargo clippy")
        );
    }
}
