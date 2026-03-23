/// V20: **skill executions** telemetry (SSOT for reliability scoring).
pub const SCHEMA_V20: &str = "
CREATE TABLE IF NOT EXISTS skill_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    skill_id TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '',
    session_id TEXT,
    workflow_id TEXT,
    agent_id TEXT,
    status TEXT NOT NULL,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    input_hash TEXT,
    output_size INTEGER NOT NULL DEFAULT 0,
    error_kind TEXT,
    reflection_score REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_skill_executions_skill ON skill_executions(skill_id, version);
CREATE INDEX IF NOT EXISTS idx_skill_executions_status ON skill_executions(status, created_at);
CREATE INDEX IF NOT EXISTS idx_skill_executions_agent ON skill_executions(agent_id);
";
