use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use secrecy::SecretString;
use std::sync::Mutex;

use crate::backend::SecretBackend;
use crate::errors::SecretError;
use crate::spec::{SecretId, SecretSpec};

pub struct VoxCloudBackend {
    conn: Mutex<turso::Connection>,
    master_key: [u8; 32],
}

impl VoxCloudBackend {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Result<Self, SecretError> {
        let db_url = std::env::var("VOX_TURSO_URL")
            .unwrap_or_else(|_| "file:.vox/clavis_vault.db".to_string());
        let db_token = std::env::var("VOX_TURSO_TOKEN").unwrap_or_default();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;

        let conn: turso::Connection = rt.block_on(async {
            if db_url.starts_with("file:") {
                let db = turso::Builder::new_local(&db_url).build().await.map_err(|e: turso::Error| SecretError::BackendMisconfigured(e.to_string()))?;
                db.connect().map_err(|e: turso::Error| SecretError::BackendMisconfigured(e.to_string()))
            } else {
                let db = turso::sync::Builder::new_remote(":memory:")
                    .with_remote_url(&db_url)
                    .with_auth_token(&db_token)
                    .build()
                    .await
                    .map_err(|e: turso::Error| SecretError::BackendMisconfigured(e.to_string()))?;
                db.connect().await.map_err(|e: turso::Error| SecretError::BackendMisconfigured(e.to_string()))
            }
        })?;

        rt.block_on(async {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS vault_secrets (id TEXT PRIMARY KEY, ciphertext BLOB, nonce BLOB)",
                (),
            )
            .await
            .map_err(|e: turso::Error| SecretError::BackendMisconfigured(e.to_string()))
        })?;

        let entry = keyring::Entry::new("vox-clavis-vault", "master")
            .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;
        let password = entry
            .get_password()
            .unwrap_or_else(|_| "default_bootstrap_pwd".to_string());

        let mut hasher = blake3::Hasher::new();
        hasher.update(password.as_bytes());
        let master_key: [u8; 32] = hasher.finalize().into();

        Ok(Self {
            conn: Mutex::new(conn),
            master_key,
        })
    }

    pub fn write_secret(&self, key: &str, plaintext: &str) -> Result<(), SecretError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.master_key));
        use rand::RngCore;
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| SecretError::BackendUnavailable(format!("Encryption failed: {}", e)))?;

        let conn = self.conn.lock().unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;

        rt.block_on(async {
            conn.execute(
                "INSERT INTO vault_secrets (id, ciphertext, nonce) VALUES (?1, ?2, ?3) ON CONFLICT(id) DO UPDATE SET ciphertext=excluded.ciphertext, nonce=excluded.nonce",
                turso::params![key, ciphertext, nonce_bytes.to_vec()],
            )
            .await
            .map_err(|e: turso::Error| SecretError::BackendQueryFailed(e.to_string()))
        })?;
        
        Ok(())
    }
}

impl SecretBackend for VoxCloudBackend {
    fn resolve(
        &self,
        _id: SecretId,
        spec: SecretSpec,
    ) -> Result<Option<SecretString>, SecretError> {
        let conn = self.conn.lock().unwrap();
        let key = spec.backend_key.unwrap_or(spec.canonical_env);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SecretError::BackendMisconfigured(e.to_string()))?;

        let plaintext = rt.block_on(async {
            let mut stmt = conn
                .prepare("SELECT ciphertext, nonce FROM vault_secrets WHERE id = ?1")
                .await
                .map_err(|e: turso::Error| SecretError::BackendQueryFailed(e.to_string()))?;

            let mut rows = stmt
                .query(turso::params![key])
                .await
                .map_err(|e: turso::Error| SecretError::BackendQueryFailed(e.to_string()))?;

            if let Some(row) = rows
                .next()
                .await
                .map_err(|e: turso::Error| SecretError::BackendQueryFailed(e.to_string()))?
            {
                let ciphertext: Vec<u8> = row
                    .get(0)
                    .map_err(|e: turso::Error| SecretError::BackendQueryFailed(e.to_string()))?;
                let nonce_bytes: Vec<u8> = row
                    .get(1)
                    .map_err(|e: turso::Error| SecretError::BackendQueryFailed(e.to_string()))?;

                let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.master_key));
                let nonce = Nonce::from_slice(&nonce_bytes);

                let plaintext = cipher
                    .decrypt(nonce, ciphertext.as_ref())
                    .map_err(|e| SecretError::BackendQueryFailed(format!("Decryption failed: {}", e)))?;
                Ok(Some(plaintext))
            } else {
                Ok(None)
            }
        })?;

        if let Some(pt) = plaintext {
            let secret_str = String::from_utf8(pt)
                .map_err(|e| SecretError::BackendQueryFailed(e.to_string()))?;
            Ok(Some(SecretString::new(secret_str.into_boxed_str())))
        } else {
            Ok(None)
        }
    }
}
