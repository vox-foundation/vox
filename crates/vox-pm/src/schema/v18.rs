pub const SCHEMA_V18: &str = "
CREATE TABLE IF NOT EXISTS corpus_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fingerprint TEXT NOT NULL,
    generator_version TEXT NOT NULL,
    total_pairs INTEGER NOT NULL,
    pair_breakdown_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_corpus_snapshots_fp ON corpus_snapshots(fingerprint);
";
