/// V2: Extended schema — metadata, packages, execution log,
/// scheduled functions, and component registry.
pub const SCHEMA_V2: &str = "
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Typed metadata for any object (key-value with JSON values)
CREATE TABLE IF NOT EXISTS metadata (
    hash TEXT NOT NULL REFERENCES objects(hash),
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (hash, key)
);

-- Package registry
CREATE TABLE IF NOT EXISTS packages (
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    hash TEXT NOT NULL REFERENCES objects(hash),
    description TEXT,
    author TEXT,
    license TEXT,
    published_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (name, version)
);

-- Package dependency graph
CREATE TABLE IF NOT EXISTS package_deps (
    package_name TEXT NOT NULL,
    package_version TEXT NOT NULL,
    dep_name TEXT NOT NULL,
    dep_version_req TEXT NOT NULL,
    PRIMARY KEY (package_name, package_version, dep_name),
    FOREIGN KEY (package_name, package_version) REFERENCES packages(name, version)
);

-- Workflow/activity execution log (append-only)
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

-- Scheduled functions (durable scheduling)
CREATE TABLE IF NOT EXISTS scheduled (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    function_hash TEXT NOT NULL REFERENCES objects(hash),
    args BLOB,
    run_at TEXT NOT NULL,
    cron_expr TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Component registry (skills, workflows, packages)
CREATE TABLE IF NOT EXISTS components (
    name TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    schema_hash TEXT REFERENCES objects(hash),
    description TEXT,
    version TEXT NOT NULL DEFAULT '0.1.0',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_metadata_hash ON metadata(hash);
CREATE INDEX IF NOT EXISTS idx_packages_hash ON packages(hash);
CREATE INDEX IF NOT EXISTS idx_exec_log_workflow ON execution_log(workflow_id);
CREATE INDEX IF NOT EXISTS idx_exec_log_status ON execution_log(status);
CREATE INDEX IF NOT EXISTS idx_scheduled_run_at ON scheduled(run_at);
CREATE INDEX IF NOT EXISTS idx_scheduled_status ON scheduled(status);
CREATE INDEX IF NOT EXISTS idx_components_namespace ON components(namespace);
";
