//! Console → agent messaging: send a free-form note from the console operator to
//! a running agent's durable A2A inbox through the daemon's `vox_a2a_send` (`route: "db"`).
//!
//! Sender identity: the console operator is the human at the keyboard, not an
//! orchestrator-spawned agent. Orchestrator agent ids come from an `AtomicU64`
//! generator starting at 1, so `AgentId(0)` is reserved as the console operator
//! sentinel (the daemon's `CONSOLE_OPERATOR_SENDER_ID`).

use std::sync::Arc;

use serde_json::{Value, json};
use tauri::State;
use vox_orchestrator::types::AgentId;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, tool_data};

/// The reserved sender id for messages originating from the console operator.
pub const CONSOLE_OPERATOR_AGENT_ID: u64 = 0;

const SEND_TOOL: &str = "vox_a2a_send";
const READ_TERM_TOOL: &str = "vox_terminal_read";

/// Execute the 3-stage terminal inbox scrubber:
/// 1: `vox_terminal_core::block::strip_ansi_and_normalize` (ECMA-48 + CR overwrite)
/// 2: `vox_secrets::scrub_resolved_secrets` (exact-value Clavis redaction -> `[REDACTED:CLAVIS]`)
/// 3: `vox_terminal_core::corpus::redact::redact_owned` (`vox-redact` regex pass)
pub(crate) fn scrub_for_agent_inbox(raw: &str) -> String {
    let stage1 = vox_terminal_core::block::strip_ansi_and_normalize(raw);
    let stage2 = vox_secrets::scrub_resolved_secrets(&stage1);
    vox_terminal_core::corpus::redact::redact_owned(&stage2)
}

/// Parse a receiver agent id string into an `AgentId`. Accepts both the display
/// form ("A-05") and a bare number ("5"), matching `AgentId`'s `FromStr`.
pub fn parse_receiver(raw: &str) -> Result<AgentId, String> {
    let id = raw
        .trim()
        .parse::<AgentId>()
        .map_err(|_| format!("invalid agent id: {raw:?}"))?;
    if id == AgentId(CONSOLE_OPERATOR_AGENT_ID) {
        return Err("agent id 0 is reserved for the console operator".into());
    }
    Ok(id)
}

async fn dispatch_durable_a2a<B: BackendCalls>(
    b: &B,
    receiver: AgentId,
    payload: String,
) -> Result<String, String> {
    let args = json!({
        "sender_id": CONSOLE_OPERATOR_AGENT_ID,
        "receiver_id": receiver.0,
        "msg_type": "free_form",
        "payload": payload,
        "route": "db",
    });
    let data: Value = tool_data(SEND_TOOL, b.call_tool(SEND_TOOL, args).await?)?;
    data.get("db_message_uuid")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("{SEND_TOOL} returned no db_message_uuid"))
}

pub(crate) async fn send_to_agent_impl<B: BackendCalls>(
    b: &B,
    agent_id: String,
    body: String,
) -> Result<String, String> {
    if body.trim().is_empty() {
        return Err("message body is empty".into());
    }
    let receiver = parse_receiver(&agent_id)?;
    let scrubbed = scrub_for_agent_inbox(&body);
    let (payload, _, _) = vox_terminal_core::corpus::redact::truncate_terminal_block(
        &scrubbed,
        "a2a",
        0,
        vox_terminal_core::corpus::redact::DEFAULT_BLOCK_HEAD_LINES,
        vox_terminal_core::corpus::redact::DEFAULT_BLOCK_TAIL_LINES,
        vox_terminal_core::corpus::redact::DEFAULT_BLOCK_MAX_BYTES,
    );
    dispatch_durable_a2a(b, receiver, payload).await
}

pub(crate) async fn send_terminal_block_to_agent_impl<B: BackendCalls>(
    b: &B,
    agent_id: String,
    session_id: String,
    block_id: u64,
    line_start: Option<usize>,
    line_end: Option<usize>,
    note: Option<String>,
) -> Result<String, String> {
    let sid = session_id.trim();
    if sid.is_empty() {
        return Err("session_id is empty".into());
    }
    let receiver = parse_receiver(&agent_id)?;
    let line_end = line_end.or(line_start);

    let read_data: Value = tool_data(
        READ_TERM_TOOL,
        b.call_tool(
            READ_TERM_TOOL,
            json!({
                "session_id": sid,
                "block_id": block_id,
                "line_start": line_start,
                "line_end": line_end,
            }),
        )
        .await?,
    )?;

    let blocks = read_data
        .get("blocks")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("terminal block {block_id} not found in session {sid}"))?;
    let block = blocks
        .iter()
        .find(|blk| blk.get("id").and_then(Value::as_u64) == Some(block_id))
        .or_else(|| blocks.first().filter(|blk| blk.get("id").is_none()))
        .ok_or_else(|| format!("terminal block {block_id} not found in session {sid}"))?;

    let raw_input = block.get("input").and_then(Value::as_str).unwrap_or("");
    let raw_output = block.get("output").and_then(Value::as_str).unwrap_or("");
    let scrubbed_input = scrub_for_agent_inbox(raw_input);
    let scrubbed_output = scrub_for_agent_inbox(raw_output);
    let (bounded_output, _, _) = vox_terminal_core::corpus::redact::truncate_terminal_block(
        &scrubbed_output,
        sid,
        block_id,
        vox_terminal_core::corpus::redact::DEFAULT_BLOCK_HEAD_LINES,
        vox_terminal_core::corpus::redact::DEFAULT_BLOCK_TAIL_LINES,
        vox_terminal_core::corpus::redact::DEFAULT_BLOCK_MAX_BYTES,
    );

    let citation = match (line_start, line_end) {
        (Some(s), Some(e)) => format!("@term:{sid}#{block_id}:L{s}-{e}"),
        (Some(s), None) => format!("@term:{sid}#{block_id}:L{s}-{s}"),
        (None, Some(e)) => format!("@term:{sid}#{block_id}:L1-{e}"),
        (None, None) => format!("@term:{sid}#{block_id}"),
    };

    let mut formatted = String::new();
    formatted.push_str(&citation);
    if let Some(code) = block.get("exit_code").and_then(Value::as_i64) {
        formatted.push_str(&format!(" (exit {code})"));
    }
    formatted.push('\n');
    if !scrubbed_input.is_empty() {
        formatted.push_str("$ ");
        formatted.push_str(&scrubbed_input);
        formatted.push('\n');
    }
    if !bounded_output.is_empty() {
        formatted.push_str(&bounded_output);
        if !bounded_output.ends_with('\n') {
            formatted.push('\n');
        }
    }
    if let Some(raw_note) = note {
        let trimmed_note = raw_note.trim();
        if !trimmed_note.is_empty() {
            let scrubbed_note = scrub_for_agent_inbox(trimmed_note);
            let (bounded_note, _, _) = vox_terminal_core::corpus::redact::truncate_terminal_block(
                &scrubbed_note,
                sid,
                block_id,
                vox_terminal_core::corpus::redact::DEFAULT_BLOCK_HEAD_LINES,
                vox_terminal_core::corpus::redact::DEFAULT_BLOCK_TAIL_LINES,
                vox_terminal_core::corpus::redact::DEFAULT_BLOCK_MAX_BYTES,
            );
            formatted.push_str("\nNote: ");
            formatted.push_str(&bounded_note);
        }
    }

    dispatch_durable_a2a(b, receiver, formatted).await
}

/// Send a free-form note to an agent's A2A inbox. Returns the new message id.
#[tauri::command]
pub async fn send_to_agent(
    backend: State<'_, Arc<BackendState>>,
    agent_id: String,
    body: String,
) -> Result<String, String> {
    send_to_agent_impl(&backend.current().await, agent_id, body).await
}

/// Resolve a terminal block on the Rust backend, 3-stage scrub + cap its output, and send it to
/// an agent's durable A2A inbox (`route: "db"`). Returns the `db_message_uuid`.
#[tauri::command]
pub async fn send_terminal_block_to_agent(
    backend: State<'_, Arc<BackendState>>,
    agent_id: String,
    session_id: String,
    block_id: u64,
    line_start: Option<usize>,
    line_end: Option<usize>,
    note: Option<String>,
) -> Result<String, String> {
    send_terminal_block_to_agent_impl(
        &backend.current().await,
        agent_id,
        session_id,
        block_id,
        line_start,
        line_end,
        note,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    #[test]
    fn parses_bare_number() {
        assert_eq!(parse_receiver("5").unwrap(), AgentId(5));
    }

    #[test]
    fn parses_display_form() {
        assert_eq!(parse_receiver("A-07").unwrap(), AgentId(7));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_receiver("not-an-id").is_err());
    }

    #[test]
    fn rejects_reserved_zero() {
        assert!(parse_receiver("0").is_err());
        assert!(parse_receiver("A-00").is_err());
    }

    #[tokio::test]
    async fn send_to_agent_uses_durable_a2a_route() {
        let b = FakeBackend::new().on_tool(
            SEND_TOOL,
            Ok(json!({"success": true, "data": {"message_id": null, "db_message_uuid": "m-1"}})),
        );
        let id = send_to_agent_impl(&b, "A-05".into(), "hello".into())
            .await
            .unwrap();
        assert_eq!(id, "m-1");
        let calls = b.calls();
        assert_eq!(calls[0].0, format!("tool:{SEND_TOOL}"));
        assert_eq!(
            calls[0].1,
            json!({"sender_id": 0, "receiver_id": 5, "msg_type": "free_form", "payload": "hello", "route": "db"})
        );
    }

    #[tokio::test]
    async fn empty_body_and_reserved_receiver_never_reach_the_daemon() {
        let b = FakeBackend::new();
        assert!(
            send_to_agent_impl(&b, "A-05".into(), "  ".into())
                .await
                .is_err()
        );
        assert!(
            send_to_agent_impl(&b, "0".into(), "hi".into())
                .await
                .is_err()
        );
        assert!(b.calls().is_empty());
    }

    #[tokio::test]
    async fn daemon_rejection_surfaces_as_error() {
        let b = FakeBackend::new().on_tool(
            SEND_TOOL,
            Ok(json!({"success": false, "error": "Receiver agent 5 is not registered"})),
        );
        let err = send_to_agent_impl(&b, "5".into(), "hi".into())
            .await
            .unwrap_err();
        assert!(err.contains("not registered"), "{err}");
    }

    #[tokio::test]
    async fn send_to_agent_scrubs_ansi_paths_and_tokens_before_a2a_send() {
        let b = FakeBackend::new().on_tool(
            SEND_TOOL,
            Ok(json!({"success": true, "data": {"db_message_uuid": "m-scrub-1"}})),
        );
        // vox-arch-check: allow abs-path
        let raw_body =
            "check \x1b[31msk-proj-abcdefghijklmnopqrstuvwxyz1234\x1b[0m in /Users/alice/.env";
        let uuid = send_to_agent_impl(&b, "A-05".into(), raw_body.into())
            .await
            .unwrap();
        assert_eq!(uuid, "m-scrub-1");
        let calls = b.calls();
        assert_eq!(calls.len(), 1);
        let payload = calls[0].1["payload"].as_str().unwrap();
        assert!(!payload.contains("\x1b["));
        assert!(!payload.contains("sk-proj-"));
        assert!(!payload.contains("alice"));
        assert!(payload.contains("[REDACTED_KEY]"));
        assert!(payload.contains("~[REDACTED_PATH]"));
    }

    #[tokio::test]
    async fn send_terminal_block_to_agent_resolves_backend_scrubs_and_truncates() {
        let mut long_lines: Vec<String> = (1..=260).map(|i| format!("row-{i}")).collect();
        // vox-arch-check: allow abs-path
        long_lines[0] =
            "boot \x1b[31m/Users/alice/app\x1b[0m key=sk-proj-abcdefghijklmnopqrstuvwxyz1234"
                .to_string();
        let raw_block_out = long_lines.join("\n");

        let b = FakeBackend::new()
            .on_tool(
                READ_TERM_TOOL,
                Ok(json!({
                    "success": true,
                    "data": {
                        "session_id": "term-1",
                        "blocks": [{
                            "id": 7,
                            "input": "cat \x1b[32m/Users/alice/cfg\x1b[0m",
                            "output": raw_block_out,
                            "status": "failed",
                            "exit_code": 1
                        }]
                    }
                })),
            )
            .on_tool(
                SEND_TOOL,
                Ok(json!({
                    "success": true,
                    "data": { "message_id": null, "db_message_uuid": "m-term-7" }
                })),
            );

        let uuid = send_terminal_block_to_agent_impl(
            &b,
            "A-05".into(),
            "term-1".into(),
            7,
            Some(1),
            Some(20),
            Some("check key sk-proj-abcdefghijklmnopqrstuvwxyz1234".into()),
        )
        .await
        .expect("send_terminal_block_to_agent_impl");
        assert_eq!(uuid, "m-term-7");

        let calls = b.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, format!("tool:{READ_TERM_TOOL}"));
        assert_eq!(
            calls[0].1,
            json!({
                "session_id": "term-1",
                "block_id": 7,
                "line_start": 1,
                "line_end": 20,
            })
        );
        assert_eq!(calls[1].0, format!("tool:{SEND_TOOL}"));
        assert_eq!(calls[1].1["sender_id"], json!(CONSOLE_OPERATOR_AGENT_ID));
        assert_eq!(calls[1].1["receiver_id"], json!(5));
        assert_eq!(calls[1].1["msg_type"], json!("free_form"));
        assert_eq!(calls[1].1["route"], json!("db"));

        let sent_payload = calls[1].1["payload"].as_str().expect("payload str");
        assert!(sent_payload.starts_with("@term:term-1#7:L1-20 (exit 1)\n"));
        assert!(!sent_payload.contains("\x1b["));
        assert!(!sent_payload.contains("alice"));
        assert!(!sent_payload.contains("sk-proj-"));
        assert!(sent_payload.contains("[REDACTED_KEY]"));
        assert!(sent_payload.contains("~[REDACTED_PATH]"));
        // Head-40 / tail-160 truncation omits middle lines (41..=100) and keeps tail row-260.
        assert!(!sent_payload.contains("\nrow-50\n"));
        assert!(sent_payload.contains("row-260"));
        assert!(sent_payload.contains("Note: check key [REDACTED_KEY]"));

        // Verify Clavis exact-value replacement token parity via vox_secrets::scrub_secret_needles.
        let stage3_demo = vox_secrets::scrub_secret_needles(
            &scrub_for_agent_inbox("clavis_tok=custom_clavis_needle_xyz"),
            &["custom_clavis_needle_xyz"],
        );
        assert_eq!(stage3_demo, "clavis_tok=[REDACTED:CLAVIS]");

        // IN-01: single-line line_start without line_end normalizes to L5-5 so TERM_MENTION_RE parses it.
        let b_single = FakeBackend::new()
            .on_tool(
                READ_TERM_TOOL,
                Ok(json!({
                    "success": true,
                    "data": {
                        "session_id": "term-1",
                        "blocks": [{"id": 7, "input": "pwd", "output": "line5", "status": "ok", "exit_code": 0}]
                    }
                })),
            )
            .on_tool(
                SEND_TOOL,
                Ok(json!({"success": true, "data": {"db_message_uuid": "m-single"}})),
            );
        send_terminal_block_to_agent_impl(
            &b_single,
            "A-05".into(),
            "term-1".into(),
            7,
            Some(5),
            None,
            None,
        )
        .await
        .expect("single line");
        let single_calls = b_single.calls();
        assert_eq!(single_calls[0].1["line_end"], json!(5));
        assert!(
            single_calls[1].1["payload"]
                .as_str()
                .unwrap()
                .starts_with("@term:term-1#7:L5-5")
        );
    }

    #[tokio::test]
    async fn send_terminal_block_to_agent_validates_inputs_and_missing_blocks() {
        let b = FakeBackend::new();
        assert!(
            send_terminal_block_to_agent_impl(&b, "A-05".into(), "   ".into(), 1, None, None, None)
                .await
                .is_err()
        );
        assert!(
            send_terminal_block_to_agent_impl(&b, "0".into(), "term-1".into(), 1, None, None, None)
                .await
                .is_err()
        );
        assert!(
            send_terminal_block_to_agent_impl(
                &b,
                "A-00".into(),
                "term-1".into(),
                1,
                None,
                None,
                None,
            )
            .await
            .is_err()
        );
        assert!(
            b.calls().is_empty(),
            "validation failures must fail before calling backend tools"
        );

        let b_empty = FakeBackend::new().on_tool(
            READ_TERM_TOOL,
            Ok(json!({"success": true, "data": {"session_id": "term-1", "blocks": []}})),
        );
        let err = send_terminal_block_to_agent_impl(
            &b_empty,
            "A-05".into(),
            "term-1".into(),
            99,
            None,
            None,
            None,
        )
        .await
        .unwrap_err();
        assert!(err.contains("99"), "{err}");
        assert_eq!(b_empty.calls().len(), 1);
    }
}
