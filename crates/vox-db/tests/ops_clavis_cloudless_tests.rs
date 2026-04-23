use vox_db::{DbConfig, UpsertAccountSecretCiphertextParams, VoxDb};

fn checksum(
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
    VoxDb::compute_account_secret_checksum(
        account_id,
        secret_id,
        ciphertext,
        nonce,
        cipher_version,
        dek_wrapped,
        kek_ref,
        kek_version,
        rotation_epoch,
        consistency_version,
    )
}

#[tokio::test]
async fn upsert_get_and_integrity_roundtrip() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let ciphertext = b"cipher:alpha".to_vec();
    let nonce = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let dek_wrapped = b"wrapped-dek".to_vec();
    let digest = checksum(
        "acct-1",
        "OPENROUTER_API_KEY",
        &ciphertext,
        &nonce,
        1,
        &dek_wrapped,
        "kek-primary",
        9,
        3,
        2,
    );
    db.upsert_account_secret_ciphertext(UpsertAccountSecretCiphertextParams {
        account_id: "acct-1",
        secret_id: "OPENROUTER_API_KEY",
        ciphertext: &ciphertext,
        nonce: &nonce,
        cipher_version: 1,
        dek_wrapped: &dek_wrapped,
        dek_wrap_alg: "AES-256-GCM",
        kek_ref: "kek-primary",
        kek_version: 9,
        aad_hash: Some("aad-hash"),
        updated_at_ms: 123_000,
        rotation_epoch: 3,
        rotated_at_ms: Some(100_000),
        consistency_origin: "canonical",
        consistency_version: 2,
        last_synced_at_ms: Some(122_000),
        checksum_blake3: &digest,
    })
    .await
    .expect("upsert");

    let row = db
        .get_account_secret_ciphertext("acct-1", "OPENROUTER_API_KEY")
        .await
        .expect("get")
        .expect("row exists");
    assert_eq!(row.kek_ref, "kek-primary");
    assert_eq!(row.kek_version, 9);
    assert_eq!(row.consistency_origin, "canonical");
    assert!(
        db.verify_account_secret_ciphertext_integrity("acct-1", "OPENROUTER_API_KEY")
            .await
            .expect("integrity")
    );
}

#[tokio::test]
async fn due_rotation_filter_and_account_scoping() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    for (account, secret, updated, epoch) in [
        ("acct-1", "OPENAI_API_KEY", 100_i64, 1_i64),
        ("acct-1", "ANTHROPIC_API_KEY", 200_i64, 2_i64),
        ("acct-2", "OPENROUTER_API_KEY", 300_i64, 4_i64),
    ] {
        let ciphertext = format!("cipher:{secret}").into_bytes();
        let nonce = vec![7; 12];
        let dek_wrapped = vec![9, 8, 7];
        let digest = checksum(
            account,
            secret,
            &ciphertext,
            &nonce,
            1,
            &dek_wrapped,
            "kek-main",
            1,
            epoch,
            1,
        );
        db.upsert_account_secret_ciphertext(UpsertAccountSecretCiphertextParams {
            account_id: account,
            secret_id: secret,
            ciphertext: &ciphertext,
            nonce: &nonce,
            cipher_version: 1,
            dek_wrapped: &dek_wrapped,
            dek_wrap_alg: "AES-256-GCM",
            kek_ref: "kek-main",
            kek_version: 1,
            aad_hash: None,
            updated_at_ms: updated,
            rotation_epoch: epoch,
            rotated_at_ms: None,
            consistency_origin: "canonical",
            consistency_version: 1,
            last_synced_at_ms: None,
            checksum_blake3: &digest,
        })
        .await
        .expect("upsert");
    }

    let acct1 = db
        .list_account_secret_ciphertexts_for_account("acct-1", 10)
        .await
        .expect("list");
    assert_eq!(acct1.len(), 2);
    assert_eq!(acct1[0].secret_id, "ANTHROPIC_API_KEY");

    let due = db
        .list_account_secret_ciphertexts_due_rotation(200, 10)
        .await
        .expect("due");
    assert_eq!(due.len(), 2);
    assert!(due.iter().all(|r| r.updated_at_ms <= 200));
}

#[tokio::test]
async fn backup_restore_rejects_corrupted_row() {
    let source = VoxDb::connect(DbConfig::Memory).await.expect("source");
    let target = VoxDb::connect(DbConfig::Memory).await.expect("target");
    let ciphertext = b"cipher:data".to_vec();
    let nonce = vec![3; 12];
    let dek_wrapped = vec![5, 6, 7, 8];
    let digest = checksum(
        "acct-backup",
        "VOX_MCP_HTTP_BEARER_TOKEN",
        &ciphertext,
        &nonce,
        1,
        &dek_wrapped,
        "kek-a",
        7,
        5,
        3,
    );
    source
        .upsert_account_secret_ciphertext(UpsertAccountSecretCiphertextParams {
            account_id: "acct-backup",
            secret_id: "VOX_MCP_HTTP_BEARER_TOKEN",
            ciphertext: &ciphertext,
            nonce: &nonce,
            cipher_version: 1,
            dek_wrapped: &dek_wrapped,
            dek_wrap_alg: "AES-256-GCM",
            kek_ref: "kek-a",
            kek_version: 7,
            aad_hash: None,
            updated_at_ms: 500,
            rotation_epoch: 5,
            rotated_at_ms: None,
            consistency_origin: "canonical",
            consistency_version: 3,
            last_synced_at_ms: Some(501),
            checksum_blake3: &digest,
        })
        .await
        .expect("seed row");

    let backup = source
        .export_account_secret_ciphertext_backup("acct-backup")
        .await
        .expect("export");
    target
        .import_account_secret_ciphertext_backup(&backup, true)
        .await
        .expect("restore");
    assert!(
        target
            .verify_account_secret_ciphertext_integrity("acct-backup", "VOX_MCP_HTTP_BEARER_TOKEN")
            .await
            .expect("integrity")
    );

    let mut corrupted = backup.clone();
    corrupted[0].ciphertext[0] ^= 0x01;
    let err = target
        .import_account_secret_ciphertext_backup(&corrupted, true)
        .await
        .expect_err("must reject checksum mismatch");
    assert!(err.to_string().contains("checksum mismatch during restore"));
}

#[tokio::test]
async fn revocation_delete_behaves_consistently_across_secret_material_kinds() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let cases = [
        "OPENROUTER_API_KEY",
        "VOX_OPENREVIEW_ACCESS_TOKEN",
        "VOX_OPENREVIEW_PASSWORD",
    ];
    for (idx, secret_id) in cases.iter().enumerate() {
        let ciphertext = format!("cipher:{secret_id}").into_bytes();
        let nonce = vec![idx as u8 + 1; 12];
        let dek_wrapped = vec![idx as u8 + 9; 8];
        let digest = checksum(
            "acct-revoke",
            secret_id,
            &ciphertext,
            &nonce,
            1,
            &dek_wrapped,
            "kek-revoke",
            2,
            idx as i64,
            1,
        );
        db.upsert_account_secret_ciphertext(UpsertAccountSecretCiphertextParams {
            account_id: "acct-revoke",
            secret_id,
            ciphertext: &ciphertext,
            nonce: &nonce,
            cipher_version: 1,
            dek_wrapped: &dek_wrapped,
            dek_wrap_alg: "AES-256-GCM",
            kek_ref: "kek-revoke",
            kek_version: 2,
            aad_hash: None,
            updated_at_ms: 800 + idx as i64,
            rotation_epoch: idx as i64,
            rotated_at_ms: None,
            consistency_origin: "canonical",
            consistency_version: 1,
            last_synced_at_ms: None,
            checksum_blake3: &digest,
        })
        .await
        .expect("insert");
    }

    let deleted = db
        .delete_account_secret_ciphertext("acct-revoke", "VOX_OPENREVIEW_ACCESS_TOKEN")
        .await
        .expect("delete");
    assert!(deleted >= 1);
    let revoked = db
        .get_account_secret_ciphertext("acct-revoke", "VOX_OPENREVIEW_ACCESS_TOKEN")
        .await
        .expect("get revoked");
    assert!(revoked.is_none());
    let still_present = db
        .list_account_secret_ciphertexts_for_account("acct-revoke", 10)
        .await
        .expect("list");
    assert_eq!(still_present.len(), 2);
}
