/// V8: Codex reactivity + schema lineage (forward-only; see ADR 004 / `docs/src/architecture/codex-vnext-schema.md`).
pub const SCHEMA_V8: &str = "
CREATE TABLE IF NOT EXISTS codex_schema_lineage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    baseline_id TEXT NOT NULL,
    schema_digest TEXT NOT NULL,
    provenance TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS codex_change_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic TEXT NOT NULL,
    entity_kind TEXT,
    entity_id TEXT,
    change_kind TEXT NOT NULL,
    payload_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_codex_change_log_topic ON codex_change_log(topic);
CREATE INDEX IF NOT EXISTS idx_codex_change_log_created ON codex_change_log(created_at);

CREATE TABLE IF NOT EXISTS codex_subscriptions (
    id TEXT PRIMARY KEY,
    topic TEXT NOT NULL,
    filter_json TEXT,
    client_hint TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_codex_subscriptions_topic ON codex_subscriptions(topic);

CREATE TABLE IF NOT EXISTS codex_query_snapshots (
    id TEXT PRIMARY KEY,
    query_name TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    digest TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_codex_query_snapshots_name ON codex_query_snapshots(query_name);

CREATE TABLE IF NOT EXISTS codex_projection_versions (
    projection_name TEXT NOT NULL,
    version INTEGER NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (projection_name, version)
);
";
