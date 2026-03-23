/// V14: **topics** and many-to-many links to conversations and individual messages.
pub const SCHEMA_V14: &str = "
CREATE TABLE IF NOT EXISTS topics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    slug TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_topics_label ON topics(label);

CREATE TABLE IF NOT EXISTS conversation_topics (
    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    topic_id INTEGER NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    weight REAL NOT NULL DEFAULT 1.0,
    PRIMARY KEY (conversation_id, topic_id)
);

CREATE INDEX IF NOT EXISTS idx_conversation_topics_topic ON conversation_topics(topic_id);

CREATE TABLE IF NOT EXISTS conversation_message_topics (
    conversation_message_id INTEGER NOT NULL REFERENCES conversation_messages(id) ON DELETE CASCADE,
    topic_id INTEGER NOT NULL REFERENCES topics(id) ON DELETE CASCADE,
    PRIMARY KEY (conversation_message_id, topic_id)
);

CREATE INDEX IF NOT EXISTS idx_conversation_message_topics_topic ON conversation_message_topics(topic_id);
";
