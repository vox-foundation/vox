use vox_db::{DbConfig, VoxDb};
use vox_db_types::{MisguidanceReporter, RecordMisguidanceParams, ResearchDefectClass};

#[tokio::test]
async fn test_record_misguidance_updates_reputation_with_decay() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let params = RecordMisguidanceParams {
        session_id: None,
        defect_class: ResearchDefectClass::FailsToRun,
        culprit_url: Some("https://bad-docs.example.com/api".into()),
        culprit_domain: "bad-docs.example.com".into(),
        claim_id: None,
        research_query: "How to use example API".into(),
        misleading_excerpt: Some("Call api.v1.old_method()".into()),
        generated_code_snippet: Some("example::old_method();".into()),
        failure_diagnostic: Some("error[E0425]: cannot find function `old_method`".into()),
        correction_diff: None,
        reporter: MisguidanceReporter::Compiler,
        domain_penalty: 0.2,
    };

    let id = db
        .record_research_misguidance(&params)
        .await
        .expect("record");
    assert!(id > 0);

    let penalties = db.get_domain_penalties().await.expect("penalties");
    assert_eq!(penalties.get("bad-docs.example.com").copied(), Some(0.2));
}

#[tokio::test]
async fn test_list_research_misguidance() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let session_id = db
        .create_research_session("test_session_key", "Quantum query")
        .await
        .expect("create session");

    let params1 = RecordMisguidanceParams {
        session_id: Some(session_id),
        defect_class: ResearchDefectClass::HallucinatedApi,
        culprit_url: Some("https://hallucinated.example.com".into()),
        culprit_domain: "hallucinated.example.com".into(),
        claim_id: Some(42),
        research_query: "Quantum query".into(),
        misleading_excerpt: Some("Quantum.teleport()".into()),
        generated_code_snippet: Some("quantum.teleport();".into()),
        failure_diagnostic: Some("No method teleport".into()),
        correction_diff: Some("- teleport()\n+ travel()".into()),
        reporter: MisguidanceReporter::Linter,
        domain_penalty: 0.3,
    };

    let params2 = RecordMisguidanceParams {
        session_id: None,
        defect_class: ResearchDefectClass::InelegantCode,
        culprit_url: None,
        culprit_domain: "verbose-docs.example.com".into(),
        claim_id: None,
        research_query: "Looping construct".into(),
        misleading_excerpt: None,
        generated_code_snippet: None,
        failure_diagnostic: None,
        correction_diff: None,
        reporter: MisguidanceReporter::PonytailAudit,
        domain_penalty: 0.1,
    };

    let id1 = db
        .record_research_misguidance(&params1)
        .await
        .expect("record 1");
    let _id2 = db
        .record_research_misguidance(&params2)
        .await
        .expect("record 2");

    let all_records = db
        .list_research_misguidance(None, 10)
        .await
        .expect("list all");
    assert_eq!(all_records.len(), 2);

    let session_records = db
        .list_research_misguidance(Some(session_id), 10)
        .await
        .expect("list session");
    assert_eq!(session_records.len(), 1);
    assert_eq!(session_records[0].id, id1);
    assert_eq!(
        session_records[0].defect_class,
        ResearchDefectClass::HallucinatedApi
    );
    assert_eq!(session_records[0].reporter, MisguidanceReporter::Linter);
    assert_eq!(
        session_records[0].culprit_domain,
        "hallucinated.example.com"
    );
    assert_eq!(session_records[0].claim_id, Some(42));
    assert_eq!(
        session_records[0].correction_diff.as_deref(),
        Some("- teleport()\n+ travel()")
    );

    let other_session_records = db
        .list_research_misguidance(Some(99999), 10)
        .await
        .expect("list empty");
    assert!(other_session_records.is_empty());
}

#[tokio::test]
async fn test_domain_penalties_and_blacklisting() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");

    let base_params = RecordMisguidanceParams {
        session_id: None,
        defect_class: ResearchDefectClass::FailsToRun,
        culprit_url: Some("https://malicious-or-broken.example.com/api".into()),
        culprit_domain: "malicious-or-broken.example.com".into(),
        claim_id: None,
        research_query: "Bad library".into(),
        misleading_excerpt: None,
        generated_code_snippet: None,
        failure_diagnostic: None,
        correction_diff: None,
        reporter: MisguidanceReporter::Compiler,
        domain_penalty: 0.25,
    };

    // Record 4 incidents
    for _ in 0..4 {
        db.record_research_misguidance(&base_params)
            .await
            .expect("record");
    }

    // After 4 incidents, not blacklisted yet (requires >= 5 incidents AND penalty >= 1.0)
    let blacklisted = db.get_blacklisted_domains().await.expect("blacklisted");
    assert!(!blacklisted.contains("malicious-or-broken.example.com"));

    // 5th incident: penalty reaches >= 1.0 (approx 0.25 * 5 = 1.25) and count = 5
    db.record_research_misguidance(&base_params)
        .await
        .expect("record 5th");

    let blacklisted_after = db
        .get_blacklisted_domains()
        .await
        .expect("blacklisted after");
    assert!(blacklisted_after.contains("malicious-or-broken.example.com"));

    let penalties = db.get_domain_penalties().await.expect("penalties");
    let penalty = penalties
        .get("malicious-or-broken.example.com")
        .copied()
        .unwrap();
    assert!(penalty >= 1.0);
    assert!(penalty <= 2.0); // capped at 2.0
}

#[tokio::test]
async fn test_exponential_decay_30_day_calculation() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let thirty_days_ms = 30 * 86_400_000;
    let seeded_time = now - thirty_days_ms;

    // Seed domain reputation with penalty = 1.0, 30 days ago
    db.connection()
        .execute(
            "INSERT INTO research_domain_reputation (domain, incident_count, penalty_score, last_incident_at_ms, is_blacklisted, updated_at_ms) \
             VALUES ('decay.example.com', 1, 1.0, ?1, 0, ?1)",
            turso::params![seeded_time],
        )
        .await
        .expect("seed");

    // Record new misguidance with penalty = 0.1
    let params = RecordMisguidanceParams {
        session_id: None,
        defect_class: ResearchDefectClass::InelegantCode,
        culprit_url: None,
        culprit_domain: "decay.example.com".into(),
        claim_id: None,
        research_query: "Query".into(),
        misleading_excerpt: None,
        generated_code_snippet: None,
        failure_diagnostic: None,
        correction_diff: None,
        reporter: MisguidanceReporter::User,
        domain_penalty: 0.1,
    };

    db.record_research_misguidance(&params)
        .await
        .expect("record");

    let penalties = db.get_domain_penalties().await.expect("penalties");
    let penalty = penalties.get("decay.example.com").copied().unwrap();

    // Expected: 1.0 * exp(-30/30) + 0.1 = e^(-1) + 0.1 ≈ 0.367879 + 0.1 = 0.467879
    let expected = (-1.0f64).exp() + 0.1;
    assert!(
        (penalty - expected).abs() < 0.01,
        "penalty {penalty} should be close to expected {expected}"
    );
}
