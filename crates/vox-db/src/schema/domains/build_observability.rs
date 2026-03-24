/// Build observability domain DDL — tables for `build_run`, `build_crate_sample`, `build_warning`.
pub const SCHEMA_BUILD_OBSERVABILITY: &str = r#"
CREATE TABLE IF NOT EXISTS build_run (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    repository_id    TEXT    NOT NULL,
    run_name         TEXT,
    rustc_version    TEXT,
    profile          TEXT    NOT NULL DEFAULT 'dev',
    total_ms         INTEGER NOT NULL,
    crate_count      INTEGER NOT NULL DEFAULT 0,
    fresh_count      INTEGER NOT NULL DEFAULT 0,
    dep_fingerprint  TEXT,
    recorded_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE TABLE IF NOT EXISTS build_crate_sample (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id     INTEGER NOT NULL REFERENCES build_run(id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    version    TEXT,
    elapsed_ms INTEGER,
    fresh      INTEGER NOT NULL DEFAULT 0,
    features   TEXT
);

CREATE TABLE IF NOT EXISTS build_warning (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id     INTEGER NOT NULL REFERENCES build_run(id) ON DELETE CASCADE,
    crate_name TEXT    NOT NULL,
    level      TEXT    NOT NULL DEFAULT 'warning',
    code       TEXT,
    message    TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_build_run_repo       ON build_run(repository_id, recorded_at);
CREATE INDEX IF NOT EXISTS idx_build_crate_run      ON build_crate_sample(run_id, name);
CREATE INDEX IF NOT EXISTS idx_build_warning_run    ON build_warning(run_id, crate_name);
CREATE INDEX IF NOT EXISTS idx_build_warning_code   ON build_warning(code);
"#;
