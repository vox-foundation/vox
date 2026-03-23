/// V3: users, preferences, memory, knowledge graph, embeddings, behavior, RLHF, marketplace primitives.
pub const SCHEMA_V3: &str = "
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

CREATE TABLE IF NOT EXISTS memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    memory_type TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata TEXT,
    importance REAL NOT NULL DEFAULT 1.0,
    vcs_snapshot_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_memories_agent ON memories(agent_id);
CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(memory_type);
CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at);

CREATE TABLE IF NOT EXISTS knowledge_nodes (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    content TEXT,
    node_type TEXT NOT NULL DEFAULT 'concept',
    media_url TEXT,
    media_type TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS knowledge_edges (
    src_id TEXT NOT NULL,
    dst_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (src_id, dst_id, relation)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_edges_src ON knowledge_edges(src_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_edges_dst ON knowledge_edges(dst_id);

CREATE TABLE IF NOT EXISTS embeddings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_type TEXT,
    source_id TEXT NOT NULL,
    dim INTEGER NOT NULL,
    vector BLOB NOT NULL,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_embeddings_source ON embeddings(source_type, source_id);

CREATE TABLE IF NOT EXISTS behavior_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    context TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_behavior_user ON behavior_events(user_id);
CREATE INDEX IF NOT EXISTS idx_behavior_type ON behavior_events(event_type);

CREATE TABLE IF NOT EXISTS learned_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    pattern_type TEXT NOT NULL,
    category TEXT NOT NULL,
    description TEXT NOT NULL,
    confidence REAL NOT NULL,
    vcs_snapshot_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_learned_patterns_user ON learned_patterns(user_id);
CREATE INDEX IF NOT EXISTS idx_learned_patterns_category ON learned_patterns(user_id, category);

CREATE TABLE IF NOT EXISTS llm_interactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    user_id TEXT,
    prompt TEXT NOT NULL,
    response TEXT NOT NULL,
    model_version TEXT NOT NULL,
    latency_ms INTEGER,
    token_count INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_llm_interactions_session ON llm_interactions(session_id);

CREATE TABLE IF NOT EXISTS llm_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    interaction_id INTEGER NOT NULL,
    user_id TEXT,
    rating INTEGER,
    feedback_type TEXT NOT NULL,
    correction_text TEXT,
    preferred_response TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_llm_feedback_interaction ON llm_feedback(interaction_id);

CREATE TABLE IF NOT EXISTS snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    language TEXT NOT NULL,
    title TEXT NOT NULL,
    code TEXT NOT NULL,
    description TEXT,
    tags TEXT,
    author_id TEXT,
    source_ref TEXT,
    embedding_ref TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_snippets_lang ON snippets(language);
CREATE INDEX IF NOT EXISTS idx_snippets_title ON snippets(title);

CREATE TABLE IF NOT EXISTS artifacts (
    id TEXT PRIMARY KEY,
    artifact_type TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    author_id TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    version TEXT NOT NULL,
    tags TEXT,
    status TEXT NOT NULL DEFAULT 'public',
    downloads INTEGER NOT NULL DEFAULT 0,
    avg_rating REAL NOT NULL DEFAULT 0.0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_artifacts_type ON artifacts(artifact_type);
CREATE INDEX IF NOT EXISTS idx_artifacts_name ON artifacts(name);

CREATE TABLE IF NOT EXISTS artifact_reviews (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_id TEXT NOT NULL,
    reviewer_id TEXT NOT NULL,
    status TEXT NOT NULL,
    comment TEXT,
    rating INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_artifact_reviews_target ON artifact_reviews(artifact_id);

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    system_prompt TEXT,
    tools TEXT,
    model_config TEXT,
    owner_id TEXT,
    version TEXT NOT NULL,
    is_public INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_agents_name ON agents(name);

CREATE TABLE IF NOT EXISTS skill_manifests (
    id TEXT NOT NULL,
    version TEXT NOT NULL,
    manifest_json TEXT NOT NULL,
    skill_md TEXT NOT NULL,
    published_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (id, version)
);

CREATE INDEX IF NOT EXISTS idx_skill_manifests_id ON skill_manifests(id);

CREATE TABLE IF NOT EXISTS db_snapshots (
    id INTEGER PRIMARY KEY,
    agent_id TEXT NOT NULL,
    description TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS research_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    metric_type TEXT NOT NULL,
    metric_value REAL,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_research_metrics_session ON research_metrics(session_id, metric_type);

CREATE TABLE IF NOT EXISTS eval_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL UNIQUE,
    model_path TEXT,
    format_validity REAL,
    safety_rejection_rate REAL,
    quality_proxy REAL,
    skills_discovered INTEGER,
    workflows_discovered INTEGER,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS builder_sessions (
    id TEXT PRIMARY KEY,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS session_turns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_session_turns_session ON session_turns(session_id);

CREATE TABLE IF NOT EXISTS typed_stream_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stream_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_typed_stream_events_stream ON typed_stream_events(stream_id);

CREATE TABLE IF NOT EXISTS populi_reviews (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    target_id TEXT NOT NULL,
    review_kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_populi_reviews_target ON populi_reviews(target_id);
";
