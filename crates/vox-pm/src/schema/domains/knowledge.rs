//! Arca SQL: Knowledge graph, embeddings, and RAG search.
pub const SCHEMA_KNOWLEDGE: &str = "
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

CREATE TABLE IF NOT EXISTS embeddings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_type TEXT,
    source_id TEXT NOT NULL,
    dim INTEGER NOT NULL,
    vector BLOB NOT NULL,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS search_documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_uri TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    mime_type TEXT NOT NULL DEFAULT '',
    content_hash TEXT NOT NULL DEFAULT '',
    ingested_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_search_documents_uri ON search_documents(source_uri);
CREATE INDEX IF NOT EXISTS idx_search_documents_hash ON search_documents(content_hash);

CREATE TABLE IF NOT EXISTS search_document_chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL REFERENCES search_documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    body_text TEXT NOT NULL,
    embedding_ref TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(document_id, chunk_index)
);

CREATE TABLE IF NOT EXISTS search_indexing_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_kind TEXT NOT NULL,
    target_uri TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'queued',
    detail_json TEXT,
    error_text TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_knowledge_edges_src ON knowledge_edges(src_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_edges_dst ON knowledge_edges(dst_id);
CREATE INDEX IF NOT EXISTS idx_embeddings_source ON embeddings(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_search_chunks_doc ON search_document_chunks(document_id);
CREATE INDEX IF NOT EXISTS idx_search_jobs_status ON search_indexing_jobs(status);
";
