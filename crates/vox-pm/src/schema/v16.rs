/// V16: **processing runs** (durable job traces), **per-step diagnostics**, and **append-only audit** rows.
///
/// SQL is `execute_batch`-safe (no row-returning statements). APIs should treat `audit_log` as append-only.
pub const SCHEMA_V16: &str = "
CREATE TABLE IF NOT EXISTS processing_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_kind TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued',
    scope_kind TEXT NOT NULL DEFAULT '',
    scope_id TEXT NOT NULL DEFAULT '',
    correlation_id TEXT NOT NULL DEFAULT '',
    payload_json TEXT,
    error_text TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    started_at TEXT,
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_processing_runs_status_created ON processing_runs(status, created_at);
CREATE INDEX IF NOT EXISTS idx_processing_runs_scope ON processing_runs(scope_kind, scope_id);
CREATE INDEX IF NOT EXISTS idx_processing_runs_kind ON processing_runs(run_kind);

CREATE TABLE IF NOT EXISTS processing_run_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    processing_run_id INTEGER NOT NULL REFERENCES processing_runs(id) ON DELETE CASCADE,
    step_index INTEGER NOT NULL,
    step_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    detail_json TEXT,
    started_at_ms INTEGER NOT NULL DEFAULT 0,
    finished_at_ms INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(processing_run_id, step_index)
);

CREATE INDEX IF NOT EXISTS idx_processing_run_steps_run ON processing_run_steps(processing_run_id);

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    actor_kind TEXT NOT NULL,
    actor_id TEXT NOT NULL DEFAULT '',
    action TEXT NOT NULL,
    resource_kind TEXT NOT NULL DEFAULT '',
    resource_id TEXT NOT NULL DEFAULT '',
    scope_kind TEXT NOT NULL DEFAULT '',
    scope_id TEXT NOT NULL DEFAULT '',
    payload_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_audit_log_scope_created ON audit_log(scope_kind, scope_id, created_at);
CREATE INDEX IF NOT EXISTS idx_audit_log_resource_created ON audit_log(resource_kind, resource_id, created_at);
";
