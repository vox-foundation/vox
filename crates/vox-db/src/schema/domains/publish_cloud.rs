//! Arca SQL: Populi cloud + news + publication (adjacent manifest fragments merged; DDL byte-compatible).
pub const SCHEMA_PUBLISH_CLOUD: &str = r#"
CREATE TABLE IF NOT EXISTS training_throughput_profiles (
    gpu_name     TEXT    NOT NULL,
    seq_len      INTEGER NOT NULL,
    batch_size   INTEGER NOT NULL,
    ms_per_step  REAL    NOT NULL,
    sample_count INTEGER NOT NULL DEFAULT 0,
    last_updated TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    PRIMARY KEY (gpu_name, seq_len, batch_size)
);

CREATE TABLE IF NOT EXISTS cloud_dispatch_log (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id              TEXT    NOT NULL UNIQUE,
    provider            TEXT    NOT NULL,
    offer_id            TEXT    NOT NULL DEFAULT '',
    gpu_name            TEXT    NOT NULL DEFAULT '',
    vram_mb             INTEGER NOT NULL DEFAULT 0,
    price_per_hr_usd    REAL    NOT NULL DEFAULT 0.0,
    estimated_cost      REAL,
    actual_cost         REAL,
    job_kind            TEXT    NOT NULL DEFAULT 'train',
    status              TEXT    NOT NULL DEFAULT 'running',
    -- Phase timing (seconds) — filled in by vox mens status --cloud updates
    setup_secs          REAL,
    download_secs       REAL,
    train_secs          REAL,
    upload_secs         REAL,
    -- Efficiency tracking
    total_steps         INTEGER,
    total_tokens        INTEGER,
    tokens_per_dollar   REAL,
    -- Termination audit
    termination_reason  TEXT,
    created_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    completed_at        TEXT,
    notes               TEXT
);

CREATE INDEX IF NOT EXISTS idx_cloud_dispatch_status
    ON cloud_dispatch_log(status, created_at);
CREATE INDEX IF NOT EXISTS idx_cloud_dispatch_provider
    ON cloud_dispatch_log(provider, job_kind);
CREATE INDEX IF NOT EXISTS idx_throughput_gpu
    ON training_throughput_profiles(gpu_name);

-- Local training log — tracks 4080 Super (and other local GPUs) for cost and
-- efficiency parity with cloud training. All fields mirror cloud_dispatch_log.
CREATE TABLE IF NOT EXISTS local_train_log (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    gpu_name      TEXT    NOT NULL,
    model_id      TEXT    NOT NULL,
    preset        TEXT    NOT NULL DEFAULT 'auto',
    wall_secs     REAL    NOT NULL,
    total_steps   INTEGER NOT NULL DEFAULT 0,
    total_tokens  INTEGER NOT NULL DEFAULT 0,
    ms_per_step   REAL,
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_local_train_gpu
    ON local_train_log(gpu_name, created_at);

-- Table of published news items to prevent duplicate syndication.
CREATE TABLE IF NOT EXISTS published_news (
    news_id TEXT PRIMARY KEY,
    published_at_ms INTEGER NOT NULL,
    github_release_id TEXT,
    twitter_tweet_id TEXT,
    opencollective_update_id TEXT
);

-- Two-person approval: distinct approver identities per news id (filename stem).
CREATE TABLE IF NOT EXISTS news_publish_approvals (
    news_id TEXT NOT NULL,
    approver TEXT NOT NULL,
    approved_at_ms INTEGER NOT NULL,
    PRIMARY KEY (news_id, approver)
);

-- Digest-bound approvals (v2): approvals are tied to immutable content hash.
CREATE TABLE IF NOT EXISTS news_publish_approvals_v2 (
    news_id TEXT NOT NULL,
    content_sha3_256 TEXT NOT NULL,
    approver TEXT NOT NULL,
    approved_at_ms INTEGER NOT NULL,
    PRIMARY KEY (news_id, content_sha3_256, approver)
);

CREATE TABLE IF NOT EXISTS news_publish_attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    news_id TEXT NOT NULL,
    content_sha3_256 TEXT NOT NULL,
    attempted_at_ms INTEGER NOT NULL,
    result_json TEXT NOT NULL
);

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

CREATE TABLE IF NOT EXISTS publication_media_assets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    publication_id TEXT NOT NULL,
    asset_ref TEXT NOT NULL,
    media_type TEXT NOT NULL,
    storage_uri TEXT,
    status TEXT NOT NULL,
    metadata_json TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    UNIQUE(publication_id, asset_ref)
);

CREATE INDEX IF NOT EXISTS idx_publication_media_assets_pub_status
    ON publication_media_assets(publication_id, status);

CREATE TABLE IF NOT EXISTS publication_status_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    publication_id TEXT NOT NULL,
    status TEXT NOT NULL,
    detail_json TEXT,
    recorded_at_ms INTEGER NOT NULL
);
"#;
