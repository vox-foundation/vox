/// V17: **conversation versioning** and **edges**, **research session** rows (structured counterpart to
/// schemaless `research_metrics.session_id` TEXT), and **topic evolution** audit events.
///
/// SQL is `execute_batch`-safe (no row-returning statements).
pub const SCHEMA_V17: &str = "
CREATE TABLE IF NOT EXISTS research_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_key TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    repository_id TEXT NOT NULL DEFAULT '',
    config_json TEXT,
    summary_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_research_sessions_repo_created ON research_sessions(repository_id, created_at);
CREATE INDEX IF NOT EXISTS idx_research_sessions_status ON research_sessions(status);

CREATE TABLE IF NOT EXISTS conversation_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    version_index INTEGER NOT NULL,
    label TEXT NOT NULL DEFAULT '',
    snapshot_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(conversation_id, version_index)
);

CREATE INDEX IF NOT EXISTS idx_conversation_versions_conv ON conversation_versions(conversation_id);

CREATE TABLE IF NOT EXISTS conversation_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    to_conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    edge_kind TEXT NOT NULL DEFAULT 'related',
    weight REAL NOT NULL DEFAULT 1.0,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_conversation_edges_from ON conversation_edges(from_conversation_id);
CREATE INDEX IF NOT EXISTS idx_conversation_edges_to ON conversation_edges(to_conversation_id);

CREATE TABLE IF NOT EXISTS topic_evolution_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic_id INTEGER NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    event_kind TEXT NOT NULL,
    prior_label TEXT,
    new_label TEXT,
    detail_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_topic_evolution_topic_created ON topic_evolution_events(topic_id, created_at);
";
