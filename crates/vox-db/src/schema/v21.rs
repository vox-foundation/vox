/// V21: **actor_state** — KV store for Actor persistent state (migrated from standalone `vox-runtime` `state.db`).
pub const SCHEMA_V21: &str = "
CREATE TABLE IF NOT EXISTS actor_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
";
