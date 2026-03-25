//! Arca SQL: TOESTUB persistence — findings, snapshots, suppressions, and caches.
pub const SCHEMA_TOESTUB: &str = "
CREATE TABLE IF NOT EXISTS toestub_task_queue (
    user_id TEXT NOT NULL,
    run_scope TEXT NOT NULL,
    total_findings INTEGER NOT NULL,
    fix_suggestions_json TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, run_scope)
);

CREATE TABLE IF NOT EXISTS toestub_baselines (
    name TEXT NOT NULL,
    run_scope TEXT NOT NULL,
    findings_json TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (name, run_scope)
);

CREATE TABLE IF NOT EXISTS toestub_file_cache (
    path TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    rules_version TEXT NOT NULL,
    findings_json TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (path, content_hash, rules_version)
);

CREATE TABLE IF NOT EXISTS toestub_suppressions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL,
    line INTEGER NOT NULL,
    rule_id TEXT NOT NULL,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_toestub_suppressions_path ON toestub_suppressions(path);
";
