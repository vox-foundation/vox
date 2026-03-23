/// V12: persisted **LLM tool invocations** tied to assistant `conversation_messages` rows.
///
/// `ordinal` orders multiple parallel tool calls from the same assistant turn. `status` uses
/// values such as `pending`, `running`, `succeeded`, `failed`, `cancelled`.
pub const SCHEMA_V12: &str = "
CREATE TABLE IF NOT EXISTS conversation_tool_calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_message_id INTEGER NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL DEFAULT 0,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL DEFAULT '{}',
    result_json TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    error_text TEXT,
    started_at_ms INTEGER NOT NULL DEFAULT 0,
    finished_at_ms INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_conversation_tool_calls_msg_ord
    ON conversation_tool_calls(conversation_message_id, ordinal);
CREATE INDEX IF NOT EXISTS idx_conversation_tool_calls_tool ON conversation_tool_calls(tool_name);
CREATE INDEX IF NOT EXISTS idx_conversation_tool_calls_status ON conversation_tool_calls(status);
";
