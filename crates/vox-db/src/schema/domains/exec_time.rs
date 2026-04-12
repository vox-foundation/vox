pub const SCHEMA_EXEC_TIME: &str = r#"
CREATE TABLE IF NOT EXISTS agent_exec_history (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    tool_key          TEXT    NOT NULL,
    repository_id     TEXT    NOT NULL DEFAULT '',
    outcome           TEXT    NOT NULL DEFAULT 'success',
    duration_ms             INTEGER NOT NULL,
    timeout_budget_ms       INTEGER,
    compute_tokens_used     INTEGER,
    vendor_cost_usd_micros  INTEGER,
    attention_cost_ms       INTEGER,
    recorded_at             INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);
CREATE INDEX IF NOT EXISTS idx_aeh_tool_repo
    ON agent_exec_history (tool_key, repository_id, outcome);
CREATE INDEX IF NOT EXISTS idx_aeh_recorded_at
    ON agent_exec_history (recorded_at);
"#;
