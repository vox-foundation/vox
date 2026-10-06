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
    ingested_at TEXT NOT NULL DEFAULT (datetime('now')),
    corpus TEXT NOT NULL DEFAULT '',
    rel_path TEXT,
    frontmatter_json TEXT,
    indexed_commit TEXT,
    index_error TEXT
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
    chunk_hash TEXT,
    chunk_kind TEXT NOT NULL DEFAULT 'section',
    heading_path TEXT NOT NULL DEFAULT '',
    anchor TEXT,
    token_estimate INTEGER,
    UNIQUE(document_id, chunk_index)
);

-- KB Library index manifest (baseline 95). One row per corpus. Indexes on the baseline-95
-- columns of search_documents/search_document_chunks are created in store/open.rs AFTER the
-- guarded ALTERs (a v94 database lacks those columns when this text runs).
CREATE TABLE IF NOT EXISTS kb_index_state (
    corpus TEXT PRIMARY KEY,
    corpus_root TEXT NOT NULL,
    corpus_commit TEXT,
    indexed_at TEXT NOT NULL,
    doc_count INTEGER NOT NULL,
    chunk_count INTEGER NOT NULL,
    card_count INTEGER NOT NULL,
    chunker_version TEXT NOT NULL,
    lexical_backend TEXT NOT NULL,
    trust_tier TEXT NOT NULL,
    embed_model_id TEXT,
    embed_revision TEXT,
    embed_dim INTEGER,
    errors_json TEXT NOT NULL DEFAULT '[]',
    duration_ms INTEGER NOT NULL
);

-- search_indexing_jobs: quarantined (DEAD, Task 4) — see domains/quarantine.rs.

CREATE INDEX IF NOT EXISTS idx_knowledge_edges_src ON knowledge_edges(src_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_edges_dst ON knowledge_edges(dst_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_edges_src_rel ON knowledge_edges(src_id, relation, dst_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_edges_dst_rel ON knowledge_edges(dst_id, relation, src_id);
CREATE INDEX IF NOT EXISTS idx_embeddings_source ON embeddings(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_embeddings_source_created ON embeddings(source_type, created_at);
CREATE INDEX IF NOT EXISTS idx_search_chunks_doc ON search_document_chunks(document_id);

-- KB findings inbox (baseline 96, KB Phase 3). Every captured finding lands `pending`; the
-- trust_tier default is the fail-closed guard for raw SQL (Turso rejects CHECK and triggers, so
-- validity is enforced by the Rust types in store/ops_memory/kb_findings.rs).
CREATE TABLE IF NOT EXISTS kb_findings (
    id TEXT PRIMARY KEY,
    dedupe_key TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'pending',
    trust_tier TEXT NOT NULL DEFAULT 'untrusted',
    origin TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL,
    quotes_json TEXT NOT NULL DEFAULT '[]',
    source_url TEXT,
    fetched_at_ms INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    snapshot_relpath TEXT,
    provenance_json TEXT NOT NULL DEFAULT '{}',
    review_note TEXT,
    reviewed_by TEXT,
    reviewed_at_ms INTEGER,
    mens_queued INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_kb_findings_status ON kb_findings(status, created_at_ms);

-- Web Cache ----------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS web_cache (
    url_hash TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    etag TEXT,
    last_modified TEXT,
    status_code INTEGER NOT NULL,
    content_type TEXT NOT NULL,
    raw_body BLOB NOT NULL,
    extracted_markdown TEXT NOT NULL,
    fetched_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_web_cache_url ON web_cache(url);
CREATE INDEX IF NOT EXISTS idx_web_cache_fetched ON web_cache(fetched_at_ms);

-- Provider Quota Usage -----------------------------------------------------------
CREATE TABLE IF NOT EXISTS provider_quota_usage (
    provider TEXT NOT NULL,
    period_key TEXT NOT NULL,
    units_spent INTEGER NOT NULL DEFAULT 0,
    units_limit INTEGER NOT NULL DEFAULT 1000,
    last_synced_at TEXT NOT NULL,
    PRIMARY KEY (provider, period_key)
);
";

/// Legacy VoxKB tables, retired in favour of `kb_findings` (KB Phase 3). Its own fragment so the
/// quarantine move (03-12) is a one-line change and legacy-seeding tests can create the tables
/// through `vox_db::LEGACY_VOXKB_DDL` regardless of where the baseline keeps them.
pub const SCHEMA_LEGACY_VOXKB: &str = "
-- Knowledge Base tables (VoxKB) ---------------------------------------------------
CREATE TABLE IF NOT EXISTS knowledge_bases (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    entry_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS kb_entries (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL REFERENCES knowledge_bases(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    source_signal TEXT NOT NULL,
    source_ref TEXT,
    routing_confidence REAL NOT NULL DEFAULT 1.0,
    tags TEXT NOT NULL DEFAULT '[]',
    created_at_ms INTEGER NOT NULL,
    last_accessed_at_ms INTEGER,
    access_count INTEGER NOT NULL DEFAULT 0,
    accepted INTEGER NOT NULL DEFAULT 1,
    mens_queued INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS kb_routing_rules (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL REFERENCES knowledge_bases(id) ON DELETE CASCADE,
    rule_type TEXT NOT NULL,
    pattern TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_kb_entries_kb_id ON kb_entries(kb_id);
CREATE INDEX IF NOT EXISTS idx_kb_entries_source_signal ON kb_entries(source_signal);
CREATE INDEX IF NOT EXISTS idx_kb_entries_accepted ON kb_entries(accepted);
CREATE INDEX IF NOT EXISTS idx_kb_entries_mens_queued ON kb_entries(mens_queued, accepted);
CREATE INDEX IF NOT EXISTS idx_kb_routing_rules_kb_id ON kb_routing_rules(kb_id);
";
