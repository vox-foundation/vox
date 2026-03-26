use thiserror::Error;

/// Store operation failure (Turso, not-found, or serialization).
#[derive(Error, Debug)]
pub enum StoreError {
    /// Generic database-layer message.
    #[error("Database error: {0}")]
    Db(String),
    /// Underlying Turso / libSQL error.
    #[error(transparent)]
    Turso(#[from] turso::Error),
    /// Local filesystem I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Requested row or binding was missing.
    #[error("Not found: {0}")]
    NotFound(String),
    /// Stable identity columns would change for an existing natural key (`idempotency_key` / adapter id).
    #[error("upsert_identity_mismatch: {0}")]
    UpsertIdentityMismatch(String),
    /// JSON or other serialization failed.
    #[error("Serialization error: {0}")]
    Serialization(String),
    /// Invalid UTF-8 in blob payload.
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    /// Database used the historical multi-row `schema_version` chain; baseline-V1 only supports version 1.
    #[error(
        "legacy Arca schema chain detected (schema_version max={max_version}): export with `vox codex export-legacy`, initialize a fresh Codex database, then `vox codex import-legacy`"
    )]
    LegacySchemaChain {
        /// Highest `schema_version.version` present before baseline migration.
        max_version: i64,
    },
}
