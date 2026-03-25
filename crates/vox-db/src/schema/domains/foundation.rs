//! Arca SQL: users, preferences, and usage limits (identity + billing).
pub const SCHEMA_FOUNDATION: &str = "
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    email TEXT,
    avatar_url TEXT,
    role TEXT NOT NULL DEFAULT 'user',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS user_preferences (
    user_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, key)
);

CREATE INDEX IF NOT EXISTS idx_user_preferences_user ON user_preferences(user_id);

CREATE TABLE IF NOT EXISTS usage_limit_definitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    metric_key TEXT NOT NULL,
    scope_kind TEXT NOT NULL,
    scope_id TEXT NOT NULL DEFAULT '',
    period_kind TEXT NOT NULL,
    limit_value INTEGER NOT NULL,
    enforcement TEXT NOT NULL DEFAULT 'hard',
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(metric_key, scope_kind, scope_id, period_kind)
);

CREATE INDEX IF NOT EXISTS idx_usage_limit_defs_lookup
    ON usage_limit_definitions(metric_key, scope_kind, scope_id);

CREATE TABLE IF NOT EXISTS usage_counter_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    metric_key TEXT NOT NULL,
    scope_kind TEXT NOT NULL,
    scope_id TEXT NOT NULL DEFAULT '',
    period_start TEXT NOT NULL,
    amount INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(metric_key, scope_kind, scope_id, period_start)
);

CREATE INDEX IF NOT EXISTS idx_usage_counters_lookup
    ON usage_counter_snapshots(metric_key, scope_kind, scope_id, period_start);
";
