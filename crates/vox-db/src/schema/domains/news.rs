pub const SCHEMA_NEWS: &'static str = "
-- Table of published news items to prevent duplicate syndication.
CREATE TABLE IF NOT EXISTS published_news (
    id TEXT PRIMARY KEY,
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
";
