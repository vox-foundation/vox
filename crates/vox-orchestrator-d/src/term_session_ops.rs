//! `TermDispatch` RPC handlers for custom block streaming, `/clear`, `/compact`,
//! MENS feedback, and `control_epoch` (`TERM-AGENT-01`, `TERM-AGENT-03`, `TERM-AGENT-04`).

use std::sync::Arc;

use serde_json::{Value, json};
use vox_terminal_core::block::{BlockId, BlockKind, BlockOrigin, OutputChunk};
use vox_terminal_core::daemon_host as d;
use vox_terminal_core::{BlockFeedbackVerdict, LocalHost};

use super::{refuse_agent_origin, req_u64, str_param};

pub(crate) async fn try_handle_session_op(
    host: &Arc<LocalHost>,
    method: &str,
    p: &Value,
) -> Option<anyhow::Result<Value>> {
    match method {
        d::CUSTOM_OPEN => Some((|| {
            let sid = str_param(p, "session_id")?;
            let kind: BlockKind = serde_json::from_value(
                p.get("kind")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("missing `kind`"))?,
            )?;
            let origin: BlockOrigin = match p.get("block_origin") {
                Some(v) if !v.is_null() => serde_json::from_value(v.clone())?,
                _ => BlockOrigin::User,
            };
            let input = str_param(p, "input")?.to_string();
            let bid = host.begin_custom_block(sid, kind, origin, input)?;
            Ok(json!({ "block_id": bid.0 }))
        })()),
        d::CUSTOM_CHUNK => Some((|| {
            let sid = str_param(p, "session_id")?;
            let bid = BlockId(req_u64(p, "block_id")?);
            let chunk: OutputChunk = serde_json::from_value(
                p.get("chunk")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("missing `chunk`"))?,
            )?;
            host.append_block_chunk(sid, bid, chunk)?;
            Ok(json!({ "ok": true }))
        })()),
        d::CUSTOM_FINISH => Some((|| {
            let sid = str_param(p, "session_id")?;
            let bid = BlockId(req_u64(p, "block_id")?);
            let exit_code = p
                .get("exit_code")
                .and_then(Value::as_i64)
                .and_then(|n| i32::try_from(n).ok())
                .unwrap_or(0);
            let cancelled = p.get("cancelled").and_then(Value::as_bool).unwrap_or(false);
            let block = host.finish_custom_block(sid, bid, exit_code, cancelled)?;
            Ok(json!({ "block": block }))
        })()),
        d::GET_BLOCK => Some((|| {
            let sid = str_param(p, "session_id")?;
            let bid = BlockId(req_u64(p, "block_id")?);
            let block = host.get_block(sid, bid)?;
            Ok(json!({ "block": block }))
        })()),
        d::CLEAR => Some(
            async {
                refuse_agent_origin(p)?;
                let sid = str_param(p, "session_id")?.to_string();
                let h = Arc::clone(host);
                let rem =
                    tokio::task::spawn_blocking(move || h.clear_unpinned_blocks(&sid)).await??;
                Ok(json!({ "remaining_blocks": rem }))
            }
            .await,
        ),
        d::COMPACT => Some(
            async {
                refuse_agent_origin(p)?;
                let sid = str_param(p, "session_id")?.to_string();
                let summary = str_param(p, "summary_markdown")?.to_string();
                let keep_recent =
                    p.get("keep_recent").and_then(Value::as_u64).unwrap_or(2) as usize;
                let h = Arc::clone(host);
                let digest = tokio::task::spawn_blocking(move || {
                    h.compact_blocks(&sid, summary, keep_recent)
                })
                .await??;
                Ok(json!({ "digest_block_id": digest.map(|id| id.0) }))
            }
            .await,
        ),
        d::FEEDBACK => Some(
            async {
                refuse_agent_origin(p)?;
                let sid = str_param(p, "session_id")?.to_string();
                let bid = BlockId(req_u64(p, "block_id")?);
                let verdict: BlockFeedbackVerdict = serde_json::from_value(
                    p.get("verdict")
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("missing `verdict`"))?,
                )?;
                let h = Arc::clone(host);
                let recorded = tokio::task::spawn_blocking(move || {
                    h.record_block_feedback(&sid, bid, verdict)
                })
                .await??;
                Ok(json!({ "recorded": recorded }))
            }
            .await,
        ),
        d::CONTROL_EPOCH => Some((|| {
            let sid = str_param(p, "session_id")?;
            let epoch = host.control_epoch(sid)?;
            Ok(json!({ "control_epoch": epoch }))
        })()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use serde_json::json;
    use vox_foundation::protocol::DispatchPayload;
    use vox_terminal_core::block::{BlockKind, BlockOrigin, OutputChunk, Stream};
    use vox_terminal_core::daemon_host as d;
    use vox_terminal_core::{
        BlockFeedbackVerdict, DaemonHost, OpenSpec, SessionEvent, SessionHost,
    };

    use crate::term_dispatch::tests::{call, start};

    #[tokio::test]
    async fn custom_blocks_clear_compact_feedback_and_origin_guards_work_over_daemon() {
        assert_eq!(d::CUSTOM_OPEN, "term.custom_open");
        assert_eq!(d::CUSTOM_CHUNK, "term.custom_chunk");
        assert_eq!(d::CUSTOM_FINISH, "term.custom_finish");
        assert_eq!(d::GET_BLOCK, "term.get_block");
        assert_eq!(d::CLEAR, "term.clear");
        assert_eq!(d::COMPACT, "term.compact");
        assert_eq!(d::FEEDBACK, "term.feedback");
        assert_eq!(d::CONTROL_EPOCH, "term.control_epoch");

        let dir = tempfile::tempdir().unwrap();
        let sink = vox_terminal_core::LocalHost::open_default_journal_sink(
            dir.path(),
            vox_secrets::scrub_resolved_secrets,
        )
        .unwrap();
        let (addr, local_host) = start(Some("t445")).await;
        let client =
            vox_orchestrator::orch_daemon::OrchDaemonClient::with_token(addr.to_string(), "t445");
        let host = DaemonHost::new(client);
        let sid = host.open(OpenSpec::default()).await.unwrap();
        let mut att = host.attach(&sid, None).await.unwrap();

        for (method, mut extra) in [
            (d::CLEAR, json!({})),
            (
                d::COMPACT,
                json!({"summary_markdown": "x", "keep_recent": 1}),
            ),
            (d::FEEDBACK, json!({"block_id": 1, "verdict": "accepted"})),
        ] {
            extra["session_id"] = json!(sid);
            extra["origin"] = json!("agent");
            let resp = call(addr, method, extra, Some("t445")).await;
            assert!(
                matches!(resp, DispatchPayload::Error { ref message, .. } if message.contains("refused")),
                "expected agent origin refusal on {method}, got {resp:?}"
            );
        }

        let b1 = host
            .begin_custom_block(
                &sid,
                BlockKind::AgentTurn,
                BlockOrigin::Agent {
                    agent_id: "terminal-agent".into(),
                },
                "? check status".into(),
            )
            .await
            .unwrap();
        host.append_block_chunk(&sid, b1, OutputChunk::text(Stream::Agent, "All clean"))
            .await
            .unwrap();
        let closed = host
            .finish_custom_block(&sid, b1, 0, false)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(closed.plain_output(), "All clean");
        assert_eq!(
            host.get_block(&sid, b1)
                .await
                .unwrap()
                .unwrap()
                .plain_output(),
            "All clean"
        );
        assert!(
            host.record_block_feedback(&sid, b1, BlockFeedbackVerdict::Accepted)
                .await
                .unwrap()
        );
        let legacy_verdict: BlockFeedbackVerdict =
            serde_json::from_value(json!({ "corrected": { "corrected_command": "cargo check" } }))
                .expect("backward-compat serde without original_command");
        assert!(
            host.record_block_feedback(&sid, b1, legacy_verdict)
                .await
                .unwrap()
        );
        assert!(
            host.record_block_feedback(
                &sid,
                vox_terminal_core::block::BlockId(0),
                BlockFeedbackVerdict::Corrected {
                    corrected_command: "cargo test -p vox-cli".into(),
                    original_command: Some("cargo test --broken".into()),
                }
            )
            .await
            .unwrap()
        );
        assert_eq!(host.control_epoch(&sid).await.unwrap(), 0);

        let b2 = host
            .begin_custom_block(
                &sid,
                BlockKind::SlashCommand,
                BlockOrigin::User,
                "/budget".into(),
            )
            .await
            .unwrap();
        host.finish_custom_block(&sid, b2, 0, false).await.unwrap();

        let digest_id = host
            .compact_blocks(&sid, "Compacted summary".into(), 1)
            .await
            .unwrap()
            .expect("compacted older block");
        assert!(host.get_block(&sid, digest_id).await.unwrap().is_some());

        let rem = host.clear_unpinned_blocks(&sid).await.unwrap();
        assert_eq!(rem, 0);

        let mut saw_snapshot_resync = false;
        // drift-allow(timeout-literal,duplicate-numeric-literal): local test event wait bound
        tokio::time::timeout(Duration::from_secs(3), async {
            while !saw_snapshot_resync {
                if matches!(
                    att.events.recv().await.unwrap(),
                    SessionEvent::SnapshotResync {
                        resync_required: false,
                        ..
                    }
                ) {
                    saw_snapshot_resync = true;
                }
            }
        })
        .await
        .expect("DaemonHost::route delivered SnapshotResync");

        let sink_host =
            Arc::new(vox_terminal_core::LocalHost::default().with_transcript_sink(sink));
        let s2 = sink_host.open_synthetic(80, 24);
        let sb = sink_host
            .begin_custom_block(
                &s2,
                BlockKind::SlashCommand,
                BlockOrigin::User,
                "/context".into(),
            )
            .unwrap();
        sink_host.finish_custom_block(&s2, sb, 0, false).unwrap();
        sink_host
            .record_block_feedback(&s2, sb, BlockFeedbackVerdict::Accepted)
            .unwrap();
        sink_host.clear_unpinned_blocks(&s2).unwrap();
        assert!(dir.path().join("transcript.journal").is_file());
        assert!(dir.path().join("corpus.jsonl").is_file());

        local_host.close(&sid).unwrap();
    }
}
