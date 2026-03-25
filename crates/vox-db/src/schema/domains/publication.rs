pub const SCHEMA_PUBLICATION: &str = "
CREATE TABLE IF NOT EXISTS publication_manifests (
    publication_id TEXT PRIMARY KEY,
    content_type TEXT NOT NULL,
    source_ref TEXT,
    title TEXT NOT NULL,
    author TEXT NOT NULL,
    abstract_text TEXT,
    body_markdown TEXT NOT NULL,
    citations_json TEXT,
    metadata_json TEXT,
    content_sha3_256 TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    state TEXT NOT NULL DEFAULT 'draft',
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_publication_manifests_type_state
    ON publication_manifests(content_type, state);
CREATE INDEX IF NOT EXISTS idx_publication_manifests_digest
    ON publication_manifests(content_sha3_256);

CREATE TABLE IF NOT EXISTS publication_approvals (
    publication_id TEXT NOT NULL,
    content_sha3_256 TEXT NOT NULL,
    approver TEXT NOT NULL,
    approved_at_ms INTEGER NOT NULL,
    PRIMARY KEY (publication_id, content_sha3_256, approver)
);

CREATE TABLE IF NOT EXISTS publication_attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    publication_id TEXT NOT NULL,
    content_sha3_256 TEXT NOT NULL,
    channel TEXT NOT NULL,
    attempted_at_ms INTEGER NOT NULL,
    outcome_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_publication_attempts_pub_attempted
    ON publication_attempts(publication_id, attempted_at_ms);

CREATE TABLE IF NOT EXISTS scholarly_submissions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    publication_id TEXT NOT NULL,
    content_sha3_256 TEXT NOT NULL,
    adapter TEXT NOT NULL,
    external_submission_id TEXT NOT NULL,
    status TEXT NOT NULL,
    submitted_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    response_fingerprint TEXT,
    metadata_json TEXT,
    UNIQUE(adapter, external_submission_id)
);

CREATE INDEX IF NOT EXISTS idx_scholarly_submissions_pub_status
    ON scholarly_submissions(publication_id, status);

CREATE TABLE IF NOT EXISTS publication_status_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    publication_id TEXT NOT NULL,
    status TEXT NOT NULL,
    detail_json TEXT,
    recorded_at_ms INTEGER NOT NULL
);
";
