//! Arca SQL: Conversations, topics, and versions.
pub const SCHEMA_CONVERSATIONS: &str = "
CREATE TABLE IF NOT EXISTS conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    title TEXT NOT NULL DEFAULT '',
    code_version TEXT,
    repository_id TEXT,
    external_session_id TEXT,
    thread_id TEXT,
    origin_surface TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS conversation_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content_text TEXT NOT NULL DEFAULT '',
    payload_json TEXT,
    external_turn_id TEXT,
    model_used TEXT,
    token_count INTEGER,
    context_files_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS conversation_tool_calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_message_id INTEGER NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL DEFAULT 0,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL DEFAULT '{}',
    result_json TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    error_text TEXT,
    started_at_ms INTEGER NOT NULL DEFAULT 0,
    finished_at_ms INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS topics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    slug TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS conversation_topics (
    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    topic_id INTEGER NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    weight REAL NOT NULL DEFAULT 1.0,
    PRIMARY KEY (conversation_id, topic_id)
);

CREATE TABLE IF NOT EXISTS conversation_message_topics (
    conversation_message_id INTEGER NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    topic_id INTEGER NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    PRIMARY KEY (conversation_message_id, topic_id)
);

CREATE TABLE IF NOT EXISTS conversation_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    version_index INTEGER NOT NULL,
    label TEXT NOT NULL DEFAULT '',
    snapshot_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(conversation_id, version_index)
);

CREATE TABLE IF NOT EXISTS conversation_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    to_conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    edge_kind TEXT NOT NULL DEFAULT 'related',
    weight REAL NOT NULL DEFAULT 1.0,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS topic_evolution_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic_id INTEGER NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    event_kind TEXT NOT NULL,
    prior_label TEXT,
    new_label TEXT,
    detail_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_conversations_user ON conversations(user_id);
CREATE INDEX IF NOT EXISTS idx_conversations_updated ON conversations(updated_at);
CREATE INDEX IF NOT EXISTS idx_conversations_repository ON conversations(repository_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_repo_ext_session ON conversations(repository_id, external_session_id)
    WHERE repository_id IS NOT NULL AND external_session_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_conversation_messages_conv ON conversation_messages(conversation_id);
CREATE INDEX IF NOT EXISTS idx_conversation_messages_created ON conversation_messages(conversation_id, created_at);
CREATE INDEX IF NOT EXISTS idx_conversation_messages_external_turn ON conversation_messages(external_turn_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversation_tool_calls_msg_ord ON conversation_tool_calls(conversation_message_id, ordinal);
CREATE INDEX IF NOT EXISTS idx_conversation_tool_calls_tool ON conversation_tool_calls(tool_name);
CREATE INDEX IF NOT EXISTS idx_conversation_tool_calls_status ON conversation_tool_calls(status);
CREATE INDEX IF NOT EXISTS idx_topics_label ON topics(label);
CREATE INDEX IF NOT EXISTS idx_conversation_topics_topic ON conversation_topics(topic_id);
CREATE INDEX IF NOT EXISTS idx_conversation_message_topics_topic ON conversation_message_topics(topic_id);
CREATE INDEX IF NOT EXISTS idx_conversation_versions_conv ON conversation_versions(conversation_id);
CREATE INDEX IF NOT EXISTS idx_conversation_edges_from ON conversation_edges(from_conversation_id);
CREATE INDEX IF NOT EXISTS idx_conversation_edges_to ON conversation_edges(to_conversation_id);
CREATE INDEX IF NOT EXISTS idx_topic_evolution_topic_created ON topic_evolution_events(topic_id, created_at);
";
