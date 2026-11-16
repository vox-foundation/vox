//! Corpus writer — serialises curated + redacted events to JSONL for MENS training.
//!
//! Each line is a JSON object with `{"session_id":…,"seq":…,"kind":…}`.
//! The writer flushes after every write so partial outputs are recoverable.

use anyhow::Result;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::curate::{CurationPolicy, Decision, DefaultPolicy};
use super::redact::{redact_block_with, scrub_ansi_and_redact_with};
use crate::transcript::{TranscriptEvent, TranscriptKind, TranscriptSink};

/// Writes [`TranscriptEvent`]s as JSONL, redacting each event's text fields.
pub struct CorpusWriter<W: Write> {
    out: BufWriter<W>,
}

impl CorpusWriter<File> {
    /// Open or create an append-only JSONL corpus file on disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self::new(file))
    }
}

impl<W: Write> CorpusWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            out: BufWriter::new(inner),
        }
    }

    fn write_pre_redacted(&mut self, redacted: &TranscriptEvent) -> Result<()> {
        let line = serde_json::to_string(redacted)?;
        self.out.write_all(line.as_bytes())?;
        self.out.write_all(b"\n")?;
        self.out.flush()?;
        Ok(())
    }

    /// Writes a single event, redacting PII, then flushes.
    pub fn write_event(&mut self, event: &TranscriptEvent) -> Result<()> {
        self.record(event)
    }

    /// Backward-compatible alias for [`Self::write_event`].
    pub fn record(&mut self, event: &TranscriptEvent) -> Result<()> {
        let _ = self.record_with(event, &|s: &str| s.to_owned())?;
        Ok(())
    }

    /// Redact `event` via 3-stage pipeline and write to JSONL unless dropped (`credential_prompt`).
    pub fn record_with<F>(&mut self, event: &TranscriptEvent, extra_scrub: &F) -> Result<bool>
    where
        F: Fn(&str) -> String + ?Sized,
    {
        let Some(redacted) = redact_event_with(event, extra_scrub) else {
            return Ok(false);
        };
        self.write_pre_redacted(&redacted)?;
        Ok(true)
    }

    /// Writes all events from a batch.
    pub fn write_batch(&mut self, events: &[TranscriptEvent]) -> Result<()> {
        for ev in events {
            self.write_event(ev)?;
        }
        Ok(())
    }

    /// Flush buffered JSONL bytes to the underlying writer.
    pub fn flush(&mut self) -> Result<()> {
        self.out.flush()?;
        Ok(())
    }
}

/// Scrub all fields of `ev` through 3-stage redaction, returning `None` if `credential_prompt` is set.
pub fn redact_event_with<F>(ev: &TranscriptEvent, extra_scrub: &F) -> Option<TranscriptEvent>
where
    F: Fn(&str) -> String + ?Sized,
{
    if matches!(
        &ev.kind,
        TranscriptKind::Accepted { block } | TranscriptKind::Rejected { block }
            if block.credential_prompt
    ) {
        return None;
    }
    let kind = match &ev.kind {
        TranscriptKind::Submitted { intent, input } => TranscriptKind::Submitted {
            intent: scrub_ansi_and_redact_with(intent, extra_scrub),
            input: scrub_ansi_and_redact_with(input, extra_scrub),
        },
        TranscriptKind::Output { stream, text } => TranscriptKind::Output {
            stream: stream.clone(),
            text: scrub_ansi_and_redact_with(text, extra_scrub),
        },
        TranscriptKind::AgentTurn { text } => TranscriptKind::AgentTurn {
            text: scrub_ansi_and_redact_with(text, extra_scrub),
        },
        TranscriptKind::Corrected { from, to } => TranscriptKind::Corrected {
            from: scrub_ansi_and_redact_with(from, extra_scrub),
            to: scrub_ansi_and_redact_with(to, extra_scrub),
        },
        TranscriptKind::Accepted { block } => TranscriptKind::Accepted {
            block: redact_block_with(block, extra_scrub),
        },
        TranscriptKind::Rejected { block } => TranscriptKind::Rejected {
            block: redact_block_with(block, extra_scrub),
        },
        TranscriptKind::ExitStatus { code } => TranscriptKind::ExitStatus { code: *code },
    };
    Some(TranscriptEvent {
        session_id: ev.session_id.clone(),
        seq: ev.seq,
        kind,
    })
}

/// Dual transcript sink that scrubs each [`TranscriptEvent`] once before persisting to both
/// a crash-safe [`TranscriptSink`] (`JournalSink`) and an optional curated JSONL [`CorpusWriter`].
pub struct RedactingTranscriptSink {
    journal: Arc<dyn TranscriptSink>,
    corpus: Mutex<Option<CorpusWriter<File>>>,
    policy: Box<dyn CurationPolicy>,
    extra_scrub: Box<dyn Fn(&str) -> String + Send + Sync>,
}

impl RedactingTranscriptSink {
    pub fn new(
        journal: Arc<dyn TranscriptSink>,
        corpus: Option<CorpusWriter<File>>,
        extra_scrub: impl Fn(&str) -> String + Send + Sync + 'static,
    ) -> Self {
        Self {
            journal,
            corpus: Mutex::new(corpus),
            policy: Box::new(DefaultPolicy::default()),
            extra_scrub: Box::new(extra_scrub),
        }
    }
}

impl TranscriptSink for RedactingTranscriptSink {
    fn append(&self, event: &TranscriptEvent) -> Result<()> {
        let Some(redacted) = redact_event_with(event, self.extra_scrub.as_ref()) else {
            return Ok(());
        };
        self.journal.append(&redacted)?;
        if self.policy.decide(&redacted) == Decision::Keep
            && let Some(ref mut writer) = *self.corpus.lock().unwrap_or_else(|e| e.into_inner())
        {
            writer.write_pre_redacted(&redacted)?;
        }
        Ok(())
    }

    fn flush(&self) -> Result<()> {
        self.journal.flush()?;
        if let Some(ref mut writer) = *self.corpus.lock().unwrap_or_else(|e| e.into_inner()) {
            writer.flush()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::{JournalSink, TranscriptEvent, TranscriptKind};

    #[test]
    fn writes_valid_jsonl() {
        let events = vec![
            TranscriptEvent {
                session_id: "s1".into(),
                seq: 0,
                kind: TranscriptKind::Submitted {
                    intent: "shell".into(),
                    input: "ls -la".into(),
                },
            },
            TranscriptEvent {
                session_id: "s1".into(),
                seq: 1,
                kind: TranscriptKind::Output {
                    stream: "stdout".into(),
                    text: "total 4\n".into(),
                },
            },
        ];

        let mut buf: Vec<u8> = Vec::new();
        {
            let mut writer = CorpusWriter::new(&mut buf);
            writer.write_batch(&events).unwrap();
        }

        let text = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            serde_json::from_str::<serde_json::Value>(line).expect("invalid JSON line");
        }
    }

    #[test]
    fn email_redacted_in_output() {
        let event = TranscriptEvent {
            session_id: "s1".into(),
            seq: 0,
            kind: TranscriptKind::Output {
                stream: "stdout".into(),
                text: "sent by alice@example.com".into(),
            },
        };
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut writer = CorpusWriter::new(&mut buf);
            writer.write_event(&event).unwrap();
        }

        let text = String::from_utf8(buf).unwrap();
        assert!(!text.contains("alice@example.com"), "PII leaked: {text}");
        assert!(text.contains("[REDACTED_EMAIL]"));
    }

    #[test]
    fn accepted_and_rejected_blocks_scrub_3_stages_and_drop_credential_prompts() {
        use crate::block::{Block, BlockId, BlockKind, OutputChunk, Stream};

        let sentinel_marker = "clavis-internal-tok-987654";
        let extra = |s: &str| s.replace(sentinel_marker, "[CLAVIS_SCRUBBED]");

        let mut block = Block::new(
            BlockId(7),
            BlockKind::Shell,
            format!(
                "curl -H '\x1b[31m{sentinel_marker}\x1b[0m' sk-live-1234567890abcdef1234567890"
            ),
        );
        block.push(OutputChunk::text(
            Stream::Stdout,
            format!("out \x1b[32m{sentinel_marker}\x1b[0m sk-live-1234567890abcdef1234567890"),
        ));
        block.cwd = Some(format!("/repo/{sentinel_marker}"));
        block.git_branch = Some(format!("feat/{sentinel_marker}"));

        let dir = tempfile::tempdir().unwrap();
        let journal_path = dir.path().join("transcript.journal");
        let corpus_path = dir.path().join("corpus.jsonl");
        let sink = RedactingTranscriptSink::new(
            Arc::new(JournalSink::open(&journal_path).unwrap()),
            Some(CorpusWriter::open(&corpus_path).unwrap()),
            move |s: &str| s.replace(sentinel_marker, "[CLAVIS_SCRUBBED]"),
        );

        let mut buf: Vec<u8> = Vec::new();
        {
            let mut writer = CorpusWriter::new(&mut buf);
            let ev_acc = TranscriptEvent {
                session_id: "s1".into(),
                seq: 1,
                kind: TranscriptKind::Accepted {
                    block: block.clone(),
                },
            };
            let ev_rej = TranscriptEvent {
                session_id: "s1".into(),
                seq: 2,
                kind: TranscriptKind::Rejected {
                    block: block.clone(),
                },
            };
            assert!(writer.record_with(&ev_acc, &extra).unwrap());
            assert!(writer.record_with(&ev_rej, &extra).unwrap());
            sink.append(&ev_acc).unwrap();
            sink.append(&ev_rej).unwrap();

            let mut cred_block = block.clone();
            cred_block.credential_prompt = true;
            let ev_cred = TranscriptEvent {
                session_id: "s1".into(),
                seq: 3,
                kind: TranscriptKind::Accepted { block: cred_block },
            };
            assert!(!writer.record_with(&ev_cred, &extra).unwrap());
            sink.append(&ev_cred).unwrap();
        }
        sink.flush().unwrap();

        for raw in [
            String::from_utf8(buf).unwrap(),
            std::fs::read_to_string(&journal_path).unwrap(),
            std::fs::read_to_string(&corpus_path).unwrap(),
        ] {
            assert_eq!(raw.lines().count(), 2, "credential_prompt must be dropped");
            assert!(!raw.contains(sentinel_marker), "Clavis token leaked: {raw}");
            assert!(
                !raw.contains("sk-live-1234567890abcdef1234567890"),
                "API key leaked: {raw}"
            );
            assert!(!raw.contains("\x1b["), "ANSI CSI leaked: {raw}");
        }
    }
}
