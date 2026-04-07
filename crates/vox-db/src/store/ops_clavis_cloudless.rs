use crate::{VoxDb, store::StoreError, store::types::*};
use turso::params;

impl VoxDb {
    /// Upsert one encrypted account-secret row for Clavis Cloudless persistence.
    pub async fn upsert_account_secret_ciphertext(
        &self,
        p: UpsertAccountSecretCiphertextParams<'_>,
    ) -> Result<(), StoreError> {
        if p.cipher_version <= 0 {
            return Err(StoreError::Db(
                "clavis_account_secrets.cipher_version must be > 0".into(),
            ));
        }
        if p.kek_version <= 0 {
            return Err(StoreError::Db(
                "clavis_account_secrets.kek_version must be > 0".into(),
            ));
        }
        if p.consistency_version <= 0 {
            return Err(StoreError::Db(
                "clavis_account_secrets.consistency_version must be > 0".into(),
            ));
        }
        let account_id = p.account_id.to_string();
        let secret_id = p.secret_id.to_string();
        let ciphertext = p.ciphertext.to_vec();
        let nonce = p.nonce.to_vec();
        let cipher_version = p.cipher_version;
        let dek_wrapped = p.dek_wrapped.to_vec();
        let dek_wrap_alg = p.dek_wrap_alg.to_string();
        let kek_ref = p.kek_ref.to_string();
        let kek_version = p.kek_version;
        let aad_hash = p.aad_hash.map(str::to_string);
        let updated_at_ms = p.updated_at_ms;
        let rotation_epoch = p.rotation_epoch;
        let rotated_at_ms = p.rotated_at_ms;
        let consistency_origin = p.consistency_origin.to_string();
        let consistency_version = p.consistency_version;
        let last_synced_at_ms = p.last_synced_at_ms;
        let checksum_blake3 = p.checksum_blake3.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO clavis_account_secrets (
                        account_id, secret_id, ciphertext, nonce, cipher_version,
                        dek_wrapped, dek_wrap_alg, kek_ref, kek_version, aad_hash,
                        updated_at_ms, rotation_epoch, rotated_at_ms,
                        consistency_origin, consistency_version, last_synced_at_ms, checksum_blake3
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
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
                        last_synced_at_ms = excluded.last_synced_at_ms,
                        checksum_blake3 = excluded.checksum_blake3",
                    params![
                        account_id,
                        secret_id,
                        ciphertext,
                        nonce,
                        cipher_version,
                        dek_wrapped,
                        dek_wrap_alg,
                        kek_ref,
                        kek_version,
                        aad_hash,
                        updated_at_ms,
                        rotation_epoch,
                        rotated_at_ms,
                        consistency_origin,
                        consistency_version,
                        last_synced_at_ms,
                        checksum_blake3,
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Fetch one encrypted account-secret row by `(account_id, secret_id)`.
    pub async fn get_account_secret_ciphertext(
        &self,
        account_id: &str,
        secret_id: &str,
    ) -> Result<Option<AccountSecretCiphertextRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT account_id, secret_id, ciphertext, nonce, cipher_version,
                        dek_wrapped, dek_wrap_alg, kek_ref, kek_version, aad_hash,
                        updated_at_ms, rotation_epoch, rotated_at_ms,
                        consistency_origin, consistency_version, last_synced_at_ms, checksum_blake3
                 FROM clavis_account_secrets
                 WHERE account_id = ?1 AND secret_id = ?2",
                (account_id.to_string(), secret_id.to_string()),
            )
            .await?;
        let Some(r) = rows.first() else {
            return Ok(None);
        };
        Ok(Some(AccountSecretCiphertextRow {
            account_id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
            secret_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
            ciphertext: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
            nonce: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            cipher_version: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
            dek_wrapped: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
            dek_wrap_alg: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
            kek_ref: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
            kek_version: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
            aad_hash: r.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
            updated_at_ms: r.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
            rotation_epoch: r.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
            rotated_at_ms: r.get(12).map_err(|e| StoreError::Db(e.to_string()))?,
            consistency_origin: r.get(13).map_err(|e| StoreError::Db(e.to_string()))?,
            consistency_version: r.get(14).map_err(|e| StoreError::Db(e.to_string()))?,
            last_synced_at_ms: r.get(15).map_err(|e| StoreError::Db(e.to_string()))?,
            checksum_blake3: r.get(16).map_err(|e| StoreError::Db(e.to_string()))?,
        }))
    }

    /// List encrypted account-secret rows for one account.
    pub async fn list_account_secret_ciphertexts_for_account(
        &self,
        account_id: &str,
        limit: i64,
    ) -> Result<Vec<AccountSecretCiphertextRow>, StoreError> {
        let lim = limit.clamp(1, 5_000);
        let rows = self
            .query_all(
                "SELECT account_id, secret_id, ciphertext, nonce, cipher_version,
                        dek_wrapped, dek_wrap_alg, kek_ref, kek_version, aad_hash,
                        updated_at_ms, rotation_epoch, rotated_at_ms,
                        consistency_origin, consistency_version, last_synced_at_ms, checksum_blake3
                 FROM clavis_account_secrets
                 WHERE account_id = ?1
                 ORDER BY updated_at_ms DESC, secret_id ASC
                 LIMIT ?2",
                (account_id.to_string(), lim),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(AccountSecretCiphertextRow {
                    account_id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    secret_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    ciphertext: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    nonce: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    cipher_version: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                    dek_wrapped: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                    dek_wrap_alg: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                    kek_ref: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                    kek_version: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
                    aad_hash: r.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
                    updated_at_ms: r.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
                    rotation_epoch: r.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
                    rotated_at_ms: r.get(12).map_err(|e| StoreError::Db(e.to_string()))?,
                    consistency_origin: r.get(13).map_err(|e| StoreError::Db(e.to_string()))?,
                    consistency_version: r.get(14).map_err(|e| StoreError::Db(e.to_string()))?,
                    last_synced_at_ms: r.get(15).map_err(|e| StoreError::Db(e.to_string()))?,
                    checksum_blake3: r.get(16).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }

    /// Delete one encrypted account-secret row by `(account_id, secret_id)`.
    pub async fn delete_account_secret_ciphertext(
        &self,
        account_id: &str,
        secret_id: &str,
    ) -> Result<u64, StoreError> {
        let account_id = account_id.to_string();
        let secret_id = secret_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let deleted = conn
                    .execute(
                        "DELETE FROM clavis_account_secrets WHERE account_id = ?1 AND secret_id = ?2",
                        (account_id, secret_id),
                    )
                    .await?;
                Ok::<u64, StoreError>(deleted)
            })
            .await
    }

    /// List rows that should rotate before `updated_before_ms` (stale key material).
    pub async fn list_account_secret_ciphertexts_due_rotation(
        &self,
        updated_before_ms: i64,
        limit: i64,
    ) -> Result<Vec<AccountSecretCiphertextRow>, StoreError> {
        let lim = limit.clamp(1, 5_000);
        let rows = self
            .query_all(
                "SELECT account_id, secret_id, ciphertext, nonce, cipher_version,
                        dek_wrapped, dek_wrap_alg, kek_ref, kek_version, aad_hash,
                        updated_at_ms, rotation_epoch, rotated_at_ms,
                        consistency_origin, consistency_version, last_synced_at_ms, checksum_blake3
                 FROM clavis_account_secrets
                 WHERE updated_at_ms <= ?1
                 ORDER BY updated_at_ms ASC, account_id ASC, secret_id ASC
                 LIMIT ?2",
                (updated_before_ms, lim),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(AccountSecretCiphertextRow {
                    account_id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    secret_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    ciphertext: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    nonce: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    cipher_version: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                    dek_wrapped: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                    dek_wrap_alg: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                    kek_ref: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                    kek_version: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
                    aad_hash: r.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
                    updated_at_ms: r.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
                    rotation_epoch: r.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
                    rotated_at_ms: r.get(12).map_err(|e| StoreError::Db(e.to_string()))?,
                    consistency_origin: r.get(13).map_err(|e| StoreError::Db(e.to_string()))?,
                    consistency_version: r.get(14).map_err(|e| StoreError::Db(e.to_string()))?,
                    last_synced_at_ms: r.get(15).map_err(|e| StoreError::Db(e.to_string()))?,
                    checksum_blake3: r.get(16).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }

    /// Export all encrypted rows for one account in stable order (backup source).
    pub async fn export_account_secret_ciphertext_backup(
        &self,
        account_id: &str,
    ) -> Result<Vec<AccountSecretCiphertextRow>, StoreError> {
        self.list_account_secret_ciphertexts_for_account(account_id, 50_000)
            .await
    }

    /// Import encrypted rows (restore target), optionally verifying embedded checksums.
    pub async fn import_account_secret_ciphertext_backup(
        &self,
        rows: &[AccountSecretCiphertextRow],
        verify_checksums: bool,
    ) -> Result<(), StoreError> {
        for row in rows {
            if verify_checksums && !Self::verify_account_secret_ciphertext_checksum(row) {
                return Err(StoreError::Db(format!(
                    "checksum mismatch during restore for account_id={} secret_id={}",
                    row.account_id, row.secret_id
                )));
            }
            self.upsert_account_secret_ciphertext(UpsertAccountSecretCiphertextParams {
                account_id: &row.account_id,
                secret_id: &row.secret_id,
                ciphertext: &row.ciphertext,
                nonce: &row.nonce,
                cipher_version: row.cipher_version,
                dek_wrapped: &row.dek_wrapped,
                dek_wrap_alg: &row.dek_wrap_alg,
                kek_ref: &row.kek_ref,
                kek_version: row.kek_version,
                aad_hash: row.aad_hash.as_deref(),
                updated_at_ms: row.updated_at_ms,
                rotation_epoch: row.rotation_epoch,
                rotated_at_ms: row.rotated_at_ms,
                consistency_origin: &row.consistency_origin,
                consistency_version: row.consistency_version,
                last_synced_at_ms: row.last_synced_at_ms,
                checksum_blake3: &row.checksum_blake3,
            })
            .await?;
        }
        Ok(())
    }

    /// Check whether one persisted row's checksum matches its deterministic digest payload.
    pub async fn verify_account_secret_ciphertext_integrity(
        &self,
        account_id: &str,
        secret_id: &str,
    ) -> Result<bool, StoreError> {
        let Some(row) = self
            .get_account_secret_ciphertext(account_id, secret_id)
            .await?
        else {
            return Ok(false);
        };
        Ok(Self::verify_account_secret_ciphertext_checksum(&row))
    }

    #[must_use]
    pub fn verify_account_secret_ciphertext_checksum(row: &AccountSecretCiphertextRow) -> bool {
        let expected = Self::compute_account_secret_checksum(
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
        );
        expected == row.checksum_blake3
    }

    #[must_use]
    pub fn compute_account_secret_checksum(
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
}
