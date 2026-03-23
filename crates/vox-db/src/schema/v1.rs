/// V1: Original schema — objects, names, causal.
pub const SCHEMA_V1: &str = "
CREATE TABLE IF NOT EXISTS objects (
    hash TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    data BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS names (
    namespace TEXT NOT NULL,
    name TEXT NOT NULL,
    hash TEXT NOT NULL REFERENCES objects(hash),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (namespace, name)
);

CREATE TABLE IF NOT EXISTS causal (
    hash TEXT NOT NULL REFERENCES objects(hash),
    parent_hash TEXT NOT NULL REFERENCES objects(hash),
    PRIMARY KEY (hash, parent_hash)
);

CREATE INDEX IF NOT EXISTS idx_names_hash ON names(hash);
CREATE INDEX IF NOT EXISTS idx_causal_parent ON causal(parent_hash);
";
