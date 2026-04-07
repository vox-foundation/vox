use std::sync::Mutex;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;
use secrecy::SecretString;
use turso::params;

use crate::backend::SecretBackend;
use crate::errors::SecretError;
use crate::spec::{SecretId, SecretSpec};

const WRAP_NONCE_LEN: usize = 12;

#[derive(Debug, Clone)]
pub struct CloudlessSecretRecord {
    pub account_id: String,
    pub secret_id: String,
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub cipher_version: i64,
    pub dek_wrapped: Vec<u8>,
    pub kek_ref: String,
    pub kek_version: i64,
    pub aad_hash: Option<String>,
    pub updated_at_ms: i64,
    pub rotation_epoch: i64,
    pub rotated_at_ms: Option<i64>,
    pub consistency_origin: String,
    pub consistency_version: i64,
    pub checksum_blake3: String,
}

pub struct VoxCloudBackend {
    conn: Mutex<turso::Connection>,
    master_key: [u8; 32],
    account_id: String,
    kek_ref: String,
    kek_version: i64,
}

impl VoxCloudBackend {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Result<Self, SecretError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;
        let conn = rt.block_on(open_cloudless_connection())?;
        rt.block_on(ensure_schema(&conn))?;
        let account_id = std::env::var("VOX_ACCOUNT_ID")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "default-account".to_string());
        let kek_ref = std::env::var("VOX_CLAVIS_KEK_REF")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "local-master".to_string());
        let kek_version = std::env::var("VOX_CLAVIS_KEK_VERSION")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(1);
        Ok(Self {
            conn: Mutex::new(conn),
            master_key: derive_master_key()?,
            account_id,
            kek_ref,
            kek_version,
        })
    }

    pub fn write_secret(&self, key: &str, plaintext: &str) -> Result<(), SecretError> {
        self.write_secret_for_account(
            &self.account_id,
            key,
            plaintext,
            &self.kek_ref,
            self.kek_version,
            "canonical",
            1,
            None,
        )
    }

    pub fn write_secret_for_account(
        &self,
        account_id: &str,
        secret_id: &str,
        plaintext: &str,
        kek_ref: &str,
        kek_version: i64,
        consistency_origin: &str,
        consistency_version: i64,
        aad_hash: Option<&str>,
    ) -> Result<(), SecretError> {
        let mut dek = [0_u8; 32];
        rand::thread_rng().fill_bytes(&mut dek);
        let mut nonce_bytes = [0_u8; WRAP_NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let ciphertext = encrypt_aes_gcm(&dek, &nonce_bytes, plaintext.as_bytes())?;
        let dek_wrapped = self.wrap_dek(&dek, kek_ref, kek_version)?;
        let checksum = compute_account_secret_checksum(
            account_id,
            secret_id,
            &ciphertext,
            &nonce_bytes,
            1,
            &dek_wrapped,
            kek_ref,
            kek_version,
            0,
            consistency_version,
        );
        let conn = self.conn.lock().expect("vox vault mutex");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;
        rt.block_on(async {
            conn.execute(
                "INSERT INTO clavis_account_secrets (
                    account_id, secret_id, ciphertext, nonce, cipher_version, dek_wrapped, dek_wrap_alg,
                    kek_ref, kek_version, aad_hash, updated_at_ms, rotation_epoch, rotated_at_ms,
                    consistency_origin, consistency_version, last_synced_at_ms, checksum_blake3
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'AES-256-GCM', ?7, ?8, ?9, ?10, ?11, NULL, ?12, ?13, NULL, ?14)
                 ON CONFLICT(account_id, secret_id) DO UPDATE SET
                    ciphertext = excluded.ciphertext,
                    nonce = excluded.nonce,
                    cipher_version = excluded.cipher_version,
                    dek_wrapped = excluded.dek_wrapped,
                    dek_wrap_alg = excluded.dek_wrap_alg,
                    kek_ref = excluded.kek_ref,
                    kek_version = excluded.kek_version,
                    aad_hash = excluded.aad_hash,
                    updated_at_ms = excluded.updated_at_ms,
                    rotation_epoch = excluded.rotation_epoch,
                    rotated_at_ms = excluded.rotated_at_ms,
                    consistency_origin = excluded.consistency_origin,
                    consistency_version = excluded.consistency_version,
                    checksum_blake3 = excluded.checksum_blake3",
                params![
                    account_id,
                    secret_id,
                    ciphertext,
                    nonce_bytes.to_vec(),
                    1_i64,
                    dek_wrapped,
                    kek_ref,
                    kek_version,
                    aad_hash.map(str::to_string),
                    now_ms(),
                    0_i64,
                    consistency_origin,
                    consistency_version,
                    checksum
                ],
            )
            .await
            .map(|_| ())
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))
        })
    }

    pub fn rewrap_secret(
        &self,
        secret_id: &str,
        new_kek_ref: &str,
        new_kek_version: i64,
    ) -> Result<bool, SecretError> {
        self.rewrap_secret_for_account(&self.account_id, secret_id, new_kek_ref, new_kek_version)
    }

    pub fn rewrap_secret_for_account(
        &self,
        account_id: &str,
        secret_id: &str,
        new_kek_ref: &str,
        new_kek_version: i64,
    ) -> Result<bool, SecretError> {
        let Some(existing) = self.get_row(account_id, secret_id)? else {
            return Ok(false);
        };
        if !verify_record_checksum(&existing) {
            return Err(SecretError::BackendQueryFailed(format!(
                "checksum mismatch for account_id={account_id} secret_id={secret_id}"
            )));
        }
        let dek = self.unwrap_dek(&existing.dek_wrapped, &existing.kek_ref, existing.kek_version)?;
        let new_wrapped = self.wrap_dek(&dek, new_kek_ref, new_kek_version)?;
        let checksum = compute_account_secret_checksum(
            &existing.account_id,
            &existing.secret_id,
            &existing.ciphertext,
            &existing.nonce,
            existing.cipher_version,
            &new_wrapped,
            new_kek_ref,
            new_kek_version,
            existing.rotation_epoch + 1,
            existing.consistency_version,
        );
        let conn = self.conn.lock().expect("vox vault mutex");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;
        rt.block_on(async {
            conn.execute(
                "UPDATE clavis_account_secrets
                 SET dek_wrapped = ?1,
                     kek_ref = ?2,
                     kek_version = ?3,
                     rotation_epoch = ?4,
                     rotated_at_ms = ?5,
                     updated_at_ms = ?5,
                     checksum_blake3 = ?6
                 WHERE account_id = ?7 AND secret_id = ?8",
                params![
                    new_wrapped,
                    new_kek_ref,
                    new_kek_version,
                    existing.rotation_epoch + 1,
                    now_ms(),
                    checksum,
                    account_id,
                    secret_id
                ],
            )
            .await
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))
        })?;
        Ok(true)
    }

    pub fn export_account_backup(
        &self,
        account_id: &str,
    ) -> Result<Vec<CloudlessSecretRecord>, SecretError> {
        let conn = self.conn.lock().expect("vox vault mutex");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;
        rt.block_on(async {
            let mut rows = conn
                .query(
                    "SELECT account_id, secret_id, ciphertext, nonce, cipher_version, dek_wrapped,
                            kek_ref, kek_version, aad_hash, updated_at_ms, rotation_epoch,
                            rotated_at_ms, consistency_origin, consistency_version, checksum_blake3
                     FROM clavis_account_secrets
                     WHERE account_id = ?1
                     ORDER BY updated_at_ms DESC, secret_id ASC",
                    params![account_id],
                )
                .await
                .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?;
            let mut out = Vec::new();
            while let Some(row) = rows
                .next()
                .await
                .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?
            {
                out.push(row_to_record(row)?);
            }
            Ok(out)
        })
    }

    pub fn import_account_backup(
        &self,
        rows: &[CloudlessSecretRecord],
        verify_checksums: bool,
    ) -> Result<(), SecretError> {
        for row in rows {
            if verify_checksums && !verify_record_checksum(row) {
                return Err(SecretError::BackendQueryFailed(format!(
                    "checksum mismatch during restore for account_id={} secret_id={}",
                    row.account_id, row.secret_id
                )));
            }
            let conn = self.conn.lock().expect("vox vault mutex");
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;
            rt.block_on(async {
                conn.execute(
                    "INSERT INTO clavis_account_secrets (
                        account_id, secret_id, ciphertext, nonce, cipher_version, dek_wrapped, dek_wrap_alg,
                        kek_ref, kek_version, aad_hash, updated_at_ms, rotation_epoch, rotated_at_ms,
                        consistency_origin, consistency_version, last_synced_at_ms, checksum_blake3
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'AES-256-GCM', ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, NULL, ?15)
                    ON CONFLICT(account_id, secret_id) DO UPDATE SET
                        ciphertext = excluded.ciphertext,
                        nonce = excluded.nonce,
                        cipher_version = excluded.cipher_version,
                        dek_wrapped = excluded.dek_wrapped,
                        dek_wrap_alg = excluded.dek_wrap_alg,
                        kek_ref = excluded.kek_ref,
                        kek_version = excluded.kek_version,
                        aad_hash = excluded.aad_hash,
                        updated_at_ms = excluded.updated_at_ms,
                        rotation_epoch = excluded.rotation_epoch,
                        rotated_at_ms = excluded.rotated_at_ms,
                        consistency_origin = excluded.consistency_origin,
                        consistency_version = excluded.consistency_version,
                        checksum_blake3 = excluded.checksum_blake3",
                    params![
                        row.account_id.clone(),
                        row.secret_id.clone(),
                        row.ciphertext.clone(),
                        row.nonce.clone(),
                        row.cipher_version,
                        row.dek_wrapped.clone(),
                        row.kek_ref.clone(),
                        row.kek_version,
                        row.aad_hash.clone(),
                        row.updated_at_ms,
                        row.rotation_epoch,
                        row.rotated_at_ms,
                        row.consistency_origin.clone(),
                        row.consistency_version,
                        row.checksum_blake3.clone(),
                    ],
                )
                .await
                .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))
            })?;
        }
        Ok(())
    }

    fn get_row(
        &self,
        account_id: &str,
        secret_id: &str,
    ) -> Result<Option<CloudlessSecretRecord>, SecretError> {
        let conn = self.conn.lock().expect("vox vault mutex");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;
        rt.block_on(async {
            let mut stmt = conn
                .prepare(
                    "SELECT account_id, secret_id, ciphertext, nonce, cipher_version, dek_wrapped,
                            kek_ref, kek_version, aad_hash, updated_at_ms, rotation_epoch,
                            rotated_at_ms, consistency_origin, consistency_version, checksum_blake3
                     FROM clavis_account_secrets
                     WHERE account_id = ?1 AND secret_id = ?2",
                )
                .await
                .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?;
            let mut rows = stmt
                .query(params![account_id, secret_id])
                .await
                .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?;
            if let Some(row) = rows
                .next()
                .await
                .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?
            {
                return row_to_record(row).map(Some);
            }
            Ok(None)
        })
    }

    fn wrap_dek(&self, dek: &[u8; 32], kek_ref: &str, kek_version: i64) -> Result<Vec<u8>, SecretError> {
        let kek = derive_kek(&self.master_key, kek_ref, kek_version);
        let mut wrap_nonce = [0_u8; WRAP_NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut wrap_nonce);
        let wrapped = encrypt_aes_gcm(&kek, &wrap_nonce, dek)?;
        let mut out = Vec::with_capacity(WRAP_NONCE_LEN + wrapped.len());
        out.extend_from_slice(&wrap_nonce);
        out.extend_from_slice(&wrapped);
        Ok(out)
    }

    fn unwrap_dek(
        &self,
        wrapped: &[u8],
        kek_ref: &str,
        kek_version: i64,
    ) -> Result<[u8; 32], SecretError> {
        if wrapped.len() <= WRAP_NONCE_LEN {
            return Err(SecretError::BackendQueryFailed(
                "wrapped DEK payload is too short".to_string(),
            ));
        }
        let wrap_nonce = &wrapped[..WRAP_NONCE_LEN];
        let wrapped_ct = &wrapped[WRAP_NONCE_LEN..];
        let kek = derive_kek(&self.master_key, kek_ref, kek_version);
        let dek_vec = decrypt_aes_gcm(&kek, wrap_nonce, wrapped_ct)?;
        let dek: [u8; 32] = dek_vec
            .as_slice()
            .try_into()
            .map_err(|_| SecretError::BackendQueryFailed("unwrapped DEK is not 32 bytes".into()))?;
        Ok(dek)
    }
}

impl SecretBackend for VoxCloudBackend {
    fn resolve(
        &self,
        _id: SecretId,
        spec: SecretSpec,
    ) -> Result<Option<SecretString>, SecretError> {
        let key = spec.backend_key.unwrap_or(spec.canonical_env);
        let Some(row) = self.get_row(&self.account_id, key)? else {
            return Ok(None);
        };
        if !verify_record_checksum(&row) {
            return Err(SecretError::BackendQueryFailed(format!(
                "checksum mismatch for account_id={} secret_id={}",
                row.account_id, row.secret_id
            )));
        }
        let dek = self.unwrap_dek(&row.dek_wrapped, &row.kek_ref, row.kek_version)?;
        let plaintext = decrypt_aes_gcm(&dek, &row.nonce, &row.ciphertext)?;
        let secret_str =
            String::from_utf8(plaintext).map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?;
        Ok(Some(SecretString::new(secret_str.into_boxed_str())))
    }
}

async fn open_cloudless_connection() -> Result<turso::Connection, SecretError> {
    let db_url = std::env::var("VOX_TURSO_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "file:.vox/clavis_vault.db".to_string());
    if db_url.starts_with("file:") {
        let db = turso::Builder::new_local(&db_url)
            .build()
            .await
            .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;
        return db
            .connect()
            .map_err(|e| SecretError::BackendMisconfigured(e.to_string()));
    }
    let token = std::env::var("VOX_TURSO_TOKEN").unwrap_or_default();
    let db = turso::sync::Builder::new_remote(":memory:")
        .with_remote_url(&db_url)
        .with_auth_token(token)
        .build()
        .await
        .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;
    db.connect()
        .await
        .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))
}

async fn ensure_schema(conn: &turso::Connection) -> Result<(), SecretError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clavis_account_secrets (
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
            ON clavis_account_secrets(kek_ref, kek_version);",
    )
    .await
    .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))
}

fn row_to_record(row: turso::Row) -> Result<CloudlessSecretRecord, SecretError> {
    Ok(CloudlessSecretRecord {
        account_id: row
            .get(0)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        secret_id: row
            .get(1)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        ciphertext: row
            .get(2)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        nonce: row
            .get(3)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        cipher_version: row
            .get(4)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        dek_wrapped: row
            .get(5)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        kek_ref: row
            .get(6)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        kek_version: row
            .get(7)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        aad_hash: row
            .get(8)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        updated_at_ms: row
            .get(9)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        rotation_epoch: row
            .get(10)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        rotated_at_ms: row
            .get(11)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        consistency_origin: row
            .get(12)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        consistency_version: row
            .get(13)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
        checksum_blake3: row
            .get(14)
            .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?,
    })
}

fn verify_record_checksum(row: &CloudlessSecretRecord) -> bool {
    compute_account_secret_checksum(
        &row.account_id,
        &row.secret_id,
        &row.ciphertext,
        &row.nonce,
        row.cipher_version,
        &row.dek_wrapped,
        &row.kek_ref,
        row.kek_version,
        row.rotation_epoch,
        row.consistency_version,
    ) == row.checksum_blake3
}

fn compute_account_secret_checksum(
    account_id: &str,
    secret_id: &str,
    ciphertext: &[u8],
    nonce: &[u8],
    cipher_version: i64,
    dek_wrapped: &[u8],
    kek_ref: &str,
    kek_version: i64,
    rotation_epoch: i64,
    consistency_version: i64,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(account_id.as_bytes());
    hasher.update(&[0x1f]);
    hasher.update(secret_id.as_bytes());
    hasher.update(&[0x1f]);
    hasher.update(ciphertext);
    hasher.update(&[0x1f]);
    hasher.update(nonce);
    hasher.update(&cipher_version.to_le_bytes());
    hasher.update(dek_wrapped);
    hasher.update(kek_ref.as_bytes());
    hasher.update(&kek_version.to_le_bytes());
    hasher.update(&rotation_epoch.to_le_bytes());
    hasher.update(&consistency_version.to_le_bytes());
    hasher.finalize().to_hex().to_string()
}

fn derive_master_key() -> Result<[u8; 32], SecretError> {
    let entry = keyring::Entry::new("vox-clavis-vault", "master")
        .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;
    let password = match entry.get_password() {
        Ok(value) if !value.is_empty() => value,
        _ => {
            let mut bootstrap = [0_u8; 32];
            rand::thread_rng().fill_bytes(&mut bootstrap);
            let generated = bootstrap
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            entry.set_password(&generated).map_err(|e| {
                SecretError::BackendMisconfigured(format!(
                    "failed to initialize keyring master key: {e}"
                ))
            })?;
            generated
        }
    };
    let mut hasher = blake3::Hasher::new();
    hasher.update(password.as_bytes());
    Ok(hasher.finalize().into())
}

fn derive_kek(master_key: &[u8; 32], kek_ref: &str, kek_version: i64) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(master_key);
    hasher.update(kek_ref.as_bytes());
    hasher.update(&kek_version.to_le_bytes());
    hasher.finalize().into()
}

fn encrypt_aes_gcm(key_bytes: &[u8; 32], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, SecretError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key_bytes));
    let nonce = Nonce::from_slice(nonce);
    cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| SecretError::BackendUnavailable(format!("encryption failed: {e}")))
}

fn decrypt_aes_gcm(
    key_bytes: &[u8; 32],
    nonce: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, SecretError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key_bytes));
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| SecretError::BackendQueryFailed(format!("decryption failed: {e}")))
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
