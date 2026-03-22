/// V13: **usage limits** (policy rows) and **counter snapshots** per billing window.
///
/// `scope_kind` is typically `user`, `tenant`, or `global`. `scope_id` is empty string when not
/// applicable. `period_kind` examples: `daily`, `monthly`, `none`. `enforcement`: `hard` or `soft`.
pub const SCHEMA_V13: &str = "
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
