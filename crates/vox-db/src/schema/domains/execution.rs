//! Arca SQL: Workflow and activity execution logging.
pub const SCHEMA_EXECUTION: &str = "
CREATE TABLE IF NOT EXISTS execution_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_id TEXT,
    skill_id TEXT,
    activity_name TEXT NOT NULL,
    status TEXT NOT NULL,
    attempt INTEGER NOT NULL DEFAULT 1,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    output_size INTEGER NOT NULL DEFAULT 0,
    input BLOB,
    output BLOB,
    error TEXT,
    options TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Aggregate workflow-level record; links to execution_log rows via workflow_id.
CREATE TABLE IF NOT EXISTS workflow_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL UNIQUE,
    agent_id TEXT,
    skill_id TEXT,
    status TEXT NOT NULL DEFAULT 'running',
    step_count INTEGER NOT NULL DEFAULT 0,
    steps_ok INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,
    total_duration_ms INTEGER NOT NULL DEFAULT 0,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT
);

CREATE TABLE IF NOT EXISTS scheduled (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    function_hash TEXT NOT NULL,
    args BLOB,
    run_at TEXT NOT NULL,
    cron_expr TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_exec_log_workflow ON execution_log(workflow_id);
CREATE INDEX IF NOT EXISTS idx_exec_log_status ON execution_log(status);
CREATE INDEX IF NOT EXISTS idx_exec_log_agent ON execution_log(agent_id);
CREATE INDEX IF NOT EXISTS idx_exec_log_skill ON execution_log(skill_id);
CREATE INDEX IF NOT EXISTS idx_workflow_executions_agent ON workflow_executions(agent_id);
CREATE INDEX IF NOT EXISTS idx_workflow_executions_status ON workflow_executions(status);
CREATE INDEX IF NOT EXISTS idx_workflow_executions_skill ON workflow_executions(skill_id);
CREATE INDEX IF NOT EXISTS idx_scheduled_run_at ON scheduled(run_at);
CREATE INDEX IF NOT EXISTS idx_scheduled_status ON scheduled(status);
";
