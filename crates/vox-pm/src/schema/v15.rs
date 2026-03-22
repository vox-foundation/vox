/// V15: **search / RAG document** rows, **chunk** storage, and **indexing job** queue (Pack 09 DDL).
///
/// Hybrid retrieval logic remains in `vox-db` (`retrieval.rs`); this migration only provides
/// relational storage for ingest pipelines and dashboard status.
pub const SCHEMA_V15: &str = "
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

CREATE INDEX IF NOT EXISTS idx_search_chunks_doc ON search_document_chunks(document_id);

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

CREATE INDEX IF NOT EXISTS idx_search_jobs_status ON search_indexing_jobs(status);
";
