use vox_db::{DbConfig, PublicationManifestParams, VoxDb};

#[tokio::test]
async fn publication_manifest_upsert_and_dual_approval_are_digest_bound() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id: "paper-001",
        content_type: "scientia",
        source_ref: Some("docs/research/paper-001.md"),
        title: "Paper 001",
        author: "alice",
        abstract_text: Some("abstract"),
        body_markdown: "# body",
        citations_json: None,
        metadata_json: None,
        content_sha3_256: "digest-a",
        state: "draft",
    })
    .await
    .unwrap();

    db.record_publication_approval_for_digest("paper-001", "digest-a", "alice")
        .await
        .unwrap();
    assert!(
        !db.has_dual_publication_approval_for_digest("paper-001", "digest-a")
            .await
            .unwrap()
    );
    db.record_publication_approval_for_digest("paper-001", "digest-a", "bob")
        .await
        .unwrap();
    assert!(
        db.has_dual_publication_approval_for_digest("paper-001", "digest-a")
            .await
            .unwrap()
    );

    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id: "paper-001",
        content_type: "scientia",
        source_ref: Some("docs/research/paper-001.md"),
        title: "Paper 001 v2",
        author: "alice",
        abstract_text: Some("abstract"),
        body_markdown: "# body v2",
        citations_json: None,
        metadata_json: None,
        content_sha3_256: "digest-b",
        state: "draft",
    })
    .await
    .unwrap();
    assert!(
        !db.has_dual_publication_approval_for_digest("paper-001", "digest-b")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn scholarly_submission_upsert_tracks_status() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id: "paper-002",
        content_type: "scientia",
        source_ref: None,
        title: "Paper 002",
        author: "alice",
        abstract_text: None,
        body_markdown: "# body",
        citations_json: None,
        metadata_json: None,
        content_sha3_256: "digest-x",
        state: "approved",
    })
    .await
    .unwrap();

    db.upsert_scholarly_submission(
        "paper-002",
        "digest-x",
        "local_ledger",
        "local-abc",
        "submitted",
        Some("fp-1"),
        None,
    )
    .await
    .unwrap();
    db.upsert_scholarly_submission(
        "paper-002",
        "digest-x",
        "local_ledger",
        "local-abc",
        "accepted",
        Some("fp-2"),
        None,
    )
    .await
    .unwrap();

    let rows = db.list_scholarly_submissions("paper-002").await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, "accepted");
    let manifest = db
        .get_publication_manifest("paper-002")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(manifest.state, "accepted");
}
