use serde_json::{Map, Value};

fn parse_obj(s: &str) -> Map<String, Value> {
    serde_json::from_str::<Value>(s)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

#[inline(never)]
pub(super) fn schema_for_tool(name: &str) -> Option<Map<String, Value>> {
    let schema = match name {
        "vox_terminal_exec" => parse_obj(
            r#"{
                "type": "object",
                "additionalProperties": false,
                "required": ["command"],
                "properties": {
                    "command": {
                        "type": "string",
                        "minLength": 1,
                        "description": "Shell command line to run in the bound stateful terminal session."
                    },
                    "session_id": {
                        "type": "string",
                        "minLength": 1,
                        "description": "Explicit terminal session ID (e.g. term-1). Optional when chat_thread_id resolves a sticky session."
                    },
                    "chat_thread_id": {
                        "type": "string",
                        "minLength": 1,
                        "description": "Chat thread ID used to resolve or bind a sticky PTY session across turns."
                    },
                    "cwd": {
                        "type": "string",
                        "minLength": 1,
                        "description": "Optional initial working directory when opening a new terminal session."
                    },
                    "wait_ms": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 120000,
                        "default": 10000,
                        "description": "Bounded wait window in milliseconds before returning running or awaiting_input (0..=120000)."
                    },
                    "background": {
                        "type": "boolean",
                        "default": false,
                        "description": "Start the command with wait_ms=0 so long-running processes return immediately."
                    },
                    "rationale": {
                        "type": "string",
                        "description": "Short operator-facing explanation for why this command is executed."
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "Optional logical caller agent identifier recorded on the terminal block."
                    }
                }
            }"#,
        ),
        "vox_terminal_read" => parse_obj(
            r#"{
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "session_id": {
                        "type": "string",
                        "minLength": 1,
                        "description": "Target terminal session ID (optional when chat_thread_id resolves a sticky session)."
                    },
                    "chat_thread_id": {
                        "type": "string",
                        "minLength": 1,
                        "description": "Chat thread ID used to resolve the sticky terminal session."
                    },
                    "block_id": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Specific terminal block ID to inspect."
                    },
                    "after_seq": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Return blocks only when the session sequence number exceeds after_seq."
                    },
                    "line_start": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "1-based inclusive start line when slicing a block's output."
                    },
                    "line_end": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "1-based inclusive end line when slicing a block's output."
                    },
                    "tail_blocks": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Maximum number of most-recent terminal blocks to return (1..=100)."
                    },
                    "pinned_only": {
                        "type": "boolean",
                        "default": false,
                        "description": "Filter returned blocks to operator-pinned blocks only."
                    },
                    "failed_only": {
                        "type": "boolean",
                        "default": false,
                        "description": "Filter returned blocks to failed (non-zero exit) blocks only."
                    }
                }
            }"#,
        ),
        "vox_terminal_send_input" => parse_obj(
            r#"{
                "type": "object",
                "additionalProperties": false,
                "required": ["session_id", "data"],
                "properties": {
                    "session_id": {
                        "type": "string",
                        "minLength": 1,
                        "description": "Target terminal session ID awaiting interactive non-secret input."
                    },
                    "data": {
                        "type": "string",
                        "description": "Non-secret stdin text to send (credential prompts are rejected fail-closed with CREDENTIAL_PROMPT_USER_ONLY)."
                    },
                    "block_id": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Target running or awaiting_input block ID (defaults to active block when omitted)."
                    },
                    "append_newline": {
                        "type": "boolean",
                        "default": true,
                        "description": "Append a trailing newline after data."
                    },
                    "wait_ms": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 120000,
                        "default": 5000,
                        "description": "Optional settle wait in milliseconds before reading back updated block state."
                    }
                }
            }"#,
        ),
        "vox_terminal_sessions" => parse_obj(
            r#"{
                "type": "object",
                "additionalProperties": false,
                "properties": {}
            }"#,
        ),
        _ => return None,
    };
    Some(schema)
}
