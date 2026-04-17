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

CREATE TABLE IF NOT EXISTS node_identities (
    node_id        TEXT PRIMARY KEY,               -- BLAKE3(pubkey)[0..16] hex
    pubkey_hex     TEXT NOT NULL UNIQUE,            -- Ed25519 verifying key, hex
    label          TEXT,                            -- user-friendly name
    account_id     TEXT,                            -- FK to users.id (nullable until linked)
    created_at     TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_node_identities_account ON node_identities(account_id);

CREATE TABLE IF NOT EXISTS node_trust_grants (
    granting_node_id  TEXT NOT NULL,   -- the node granting trust
    trusted_node_id   TEXT NOT NULL,   -- the node being trusted  
    granted_at        TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (granting_node_id, trusted_node_id)
);

CREATE TABLE IF NOT EXISTS user_preferences (
    user_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, key)
);

CREATE INDEX IF NOT EXISTS idx_user_preferences_user ON user_preferences(user_id);

CREATE TABLE IF NOT EXISTS account_config (
    account_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (account_id, key)
);

CREATE INDEX IF NOT EXISTS idx_account_config_account ON account_config(account_id);

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

CREATE TABLE IF NOT EXISTS anomalous_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    severity INTEGER NOT NULL DEFAULT 1,
    payload_json TEXT NOT NULL DEFAULT '{}',
    recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_anomalous_events_node ON anomalous_events(node_id);
";
