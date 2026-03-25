pub const SCHEMA_NEWS: &'static str = "
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
";
