use vox_corpus::external_review_replay::{
    ExternalReviewReplayRow, build_repeated_error_features, validate_external_review_rows,
};
use vox_db::{
    store::types::{ExternalReviewFindingParams, ExternalReviewRunParams},
    DbConfig, VoxDb,
};

#[tokio::test]
async fn ingest_to_export_dataset_smoke() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let run_id = db
        .insert_external_review_run(ExternalReviewRunParams {
            provider: "coderabbit",
            repository_id: "owner/repo",
            owner: "owner",
            repo: "repo",
            pr_number: 1,
            commit_sha: None,
            trigger_kind: "review",
            idempotency_key: Some("e2e-1"),
            item_count: 1,
            metadata_json: None,
        })
        .await
        .expect("insert run");
    db.upsert_external_review_finding(ExternalReviewFindingParams {
        run_id,
        provider: "coderabbit",
        repository_id: "owner/repo",
        pr_number: 1,
        finding_identity: "finding-1",
        thread_identity: Some("thread-1"),
        source_comment_id: Some(1),
        placement_kind: "inline",
        line_anchor_state: "current",
        file_path: Some("src/main.rs"),
        line_start: Some(12),
        line_end: Some(12),
        category: "logic",
        anti_pattern_id: Some("review/logic-bug"),
        severity: "warning",
        title: "Avoid unwrap",
        details: "unwrap can panic; use context-aware handling",
        suggested_fix: Some("match value { ... }"),
        extraction_confidence: Some(0.8),
        source_payload_hash: "hash-1",
        fingerprint: "fp-1",
        status: "unverified",
    })
    .await
    .expect("insert finding");

    let findings = db
        .list_external_review_findings_for_training_window("owner/repo", 10)
        .await
        .expect("list findings");
    assert_eq!(findings.len(), 1);

    let rows = vec![ExternalReviewReplayRow {
        prompt: format!("Review finding: {}", findings[0].title),
        response: findings[0].details.clone(),
        category: findings[0].category.clone(),
        severity: findings[0].severity.clone(),
        placement_kind: findings[0].placement_kind.clone(),
        source_id: findings[0].finding_identity.clone(),
        repository_id: findings[0].repository_id.clone(),
        pr_number: findings[0].pr_number,
        file_path: findings[0].file_path.clone(),
        line_start: findings[0].line_start,
        correctness_state: findings[0].status.clone(),
        sample_kind: "review_fix_pairs".to_string(),
    }];
    validate_external_review_rows(&rows).expect("row validation");
}

#[test]
fn regression_harness_repeated_error_delta_smoke() {
    let baseline = vec![
        ExternalReviewReplayRow {
            prompt: "p1".to_string(),
            response: "r1".to_string(),
            category: "logic".to_string(),
            severity: "warning".to_string(),
            placement_kind: "inline".to_string(),
            source_id: "b1".to_string(),
            repository_id: "owner/repo".to_string(),
            pr_number: 1,
            file_path: Some("src/a.rs".to_string()),
            line_start: Some(10),
            correctness_state: "confirmed_true".to_string(),
            sample_kind: "review_fix_pairs".to_string(),
        },
        ExternalReviewReplayRow {
            prompt: "p2".to_string(),
            response: "r2".to_string(),
            category: "logic".to_string(),
            severity: "warning".to_string(),
            placement_kind: "inline".to_string(),
            source_id: "b2".to_string(),
            repository_id: "owner/repo".to_string(),
            pr_number: 2,
            file_path: Some("src/a.rs".to_string()),
            line_start: Some(11),
            correctness_state: "confirmed_true".to_string(),
            sample_kind: "review_fix_pairs".to_string(),
        },
    ];
    let improved = vec![ExternalReviewReplayRow {
        prompt: "p3".to_string(),
        response: "r3".to_string(),
        category: "logic".to_string(),
        severity: "warning".to_string(),
        placement_kind: "inline".to_string(),
        source_id: "i1".to_string(),
        repository_id: "owner/repo".to_string(),
        pr_number: 3,
        file_path: Some("src/a.rs".to_string()),
        line_start: Some(12),
        correctness_state: "confirmed_true".to_string(),
        sample_kind: "review_fix_pairs".to_string(),
    }];

    let baseline_features = build_repeated_error_features(&baseline);
    let improved_features = build_repeated_error_features(&improved);
    let baseline_repeat = baseline_features[0]["category_repeat_count"]
        .as_u64()
        .unwrap_or_default();
    let improved_repeat = improved_features[0]["category_repeat_count"]
        .as_u64()
        .unwrap_or_default();
    assert!(baseline_repeat > improved_repeat);
}

