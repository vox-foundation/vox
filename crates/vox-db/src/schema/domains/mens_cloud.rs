/// Mens cloud GPU dispatch domain DDL.
///
/// Tables:
/// - `training_throughput_profiles` — measured ms/step from real runs (local + cloud)
/// - `cloud_dispatch_log` — audit trail for every cloud GPU job
/// - `local_train_log` — local GPU training sessions (4080 Super etc.) for parity tracking
pub const SCHEMA_POPULI_CLOUD: &str = r#"
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
"#;
