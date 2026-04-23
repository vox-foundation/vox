//! Arca SQL: Clavis Cloudless encrypted account-secret records.
pub const SCHEMA_CLAVIS_CLOUDLESS: &str = r#"
CREATE TABLE IF NOT EXISTS clavis_account_secrets (
    account_id TEXT NOT NULL,
    secret_id TEXT NOT NULL,
    ciphertext BLOB NOT NULL,
    nonce BLOB NOT NULL,
    cipher_version INTEGER NOT NULL DEFAULT 1,
    dek_wrapped BLOB NOT NULL,
    dek_wrap_alg TEXT NOT NULL DEFAULT 'AES-256-GCM',
    kek_ref TEXT NOT NULL,
    kek_version INTEGER NOT NULL,
    aad_hash TEXT,
    updated_at_ms INTEGER NOT NULL,
    rotation_epoch INTEGER NOT NULL DEFAULT 0,
    rotated_at_ms INTEGER,
    consistency_origin TEXT NOT NULL DEFAULT 'canonical',
    consistency_version INTEGER NOT NULL DEFAULT 1,
    last_synced_at_ms INTEGER,
    checksum_blake3 TEXT NOT NULL,
    PRIMARY KEY (account_id, secret_id)
);

CREATE INDEX IF NOT EXISTS idx_clavis_account_secrets_account_updated
    ON clavis_account_secrets(account_id, updated_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_clavis_account_secrets_kek
    ON clavis_account_secrets(kek_ref, kek_version);
"#;
