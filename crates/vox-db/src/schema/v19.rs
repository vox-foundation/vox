pub const SCHEMA_V19: &str = r#"
CREATE TABLE IF NOT EXISTS chat_transcripts (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    model_used TEXT,
    tokens INTEGER,
    context_files TEXT NOT NULL, -- JSON array of file paths
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    repository_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_chat_transcripts_session ON chat_transcripts(session_id);
"#;
