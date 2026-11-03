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

    // Seed domain reputation with penalty = 0.4, 30 days ago
    db.connection()
        .execute(
            "INSERT INTO research_domain_reputation (domain, incident_count, penalty_score, last_incident_at_ms, is_blacklisted, updated_at_ms) \
             VALUES ('decay.example.com', 1, 0.4, ?1, 0, ?1)",
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

    // At 30 days (1 half-life), penalty decays by exactly 50%: 0.4 * 0.5 = 0.2.
    // With new penalty of 0.1, expected is 0.2 + 0.1 = 0.3.
    let expected = 0.4 * 0.5 + 0.1;
    assert!(
        (penalty - expected).abs() < 0.001,
        "penalty {penalty} should be close to expected {expected}"
    );
}

#[test]
fn test_sanitize_telemetry_string_redactions() {
    use vox_db::research_pipeline::sanitize_telemetry_string;

    // Bearer token
    let s = sanitize_telemetry_string("Authorization: Bearer my-secret-token.123_456-jwt");
    assert_eq!(s, "Authorization: Bearer [REDACTED]");

    // API keys
    let s = sanitize_telemetry_string(
        "Errors with tvly-12345abcdef and sk_live_987654321 and ghp_0123456789abcdefghijklmnopqrstuv",
    );
    assert!(!s.contains("tvly-12345abcdef"));
    assert!(!s.contains("sk_live_987654321"));
    assert!(!s.contains("ghp_0123456789abcdefghijklmnopqrstuv"));
    assert!(s.contains("[REDACTED]"));

    // Sensitive env vars
    let s = sanitize_telemetry_string(
        "Failed with TAVILY_API_KEY=tvly_secret and OPENAI_API_KEY=\"sk-proj-xyz\" and APP_SECRET=supersecret123",
    );
    assert!(!s.contains("tvly_secret"));
    assert!(!s.contains("sk-proj-xyz"));
    assert!(!s.contains("supersecret123"));
    assert!(s.contains("TAVILY_API_KEY=[REDACTED]"));
    assert!(s.contains("OPENAI_API_KEY=[REDACTED]"));
    assert!(s.contains("APP_SECRET=[REDACTED]"));
}

#[tokio::test]
async fn test_record_misguidance_sanitizes_excerpt_and_diagnostic() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let params = RecordMisguidanceParams {
        session_id: None,
        defect_class: ResearchDefectClass::FailsToRun,
        culprit_url: Some("https://example.com/api".into()),
        culprit_domain: "example.com".into(),
        claim_id: None,
        research_query: "Query".into(),
        misleading_excerpt: Some("header: Bearer secret-token-xyz".into()),
        generated_code_snippet: Some("api.call();".into()),
        failure_diagnostic: Some(
            "Runtime error: TAVILY_API_KEY=tvly_secret_key_123 failed with sk_live_abc123".into(),
        ),
        correction_diff: None,
        reporter: MisguidanceReporter::Compiler,
        domain_penalty: 0.1,
    };

    let id = db
        .record_research_misguidance(&params)
        .await
        .expect("record");
    let records = db.list_research_misguidance(None, 10).await.expect("list");
    let rec = records.iter().find(|r| r.id == id).expect("found");

    assert_eq!(
        rec.misleading_excerpt.as_deref(),
        Some("header: Bearer [REDACTED]")
    );
    let diag = rec.failure_diagnostic.as_deref().unwrap();
    assert!(!diag.contains("tvly_secret_key_123"));
    assert!(!diag.contains("sk_live_abc123"));
    assert!(diag.contains("[REDACTED]"));
}

#[tokio::test]
async fn test_resolve_research_misguidance() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let params = RecordMisguidanceParams {
        session_id: None,
        defect_class: ResearchDefectClass::HallucinatedApi,
        culprit_url: Some("https://example.com/docs".into()),
        culprit_domain: "example.com".into(),
        claim_id: None,
        research_query: "Query".into(),
        misleading_excerpt: Some("Old method".into()),
        generated_code_snippet: None,
        failure_diagnostic: None,
        correction_diff: None,
        reporter: MisguidanceReporter::User,
        domain_penalty: 0.1,
    };

    let id = db
        .record_research_misguidance(&params)
        .await
        .expect("record");

    let records = db.list_research_misguidance(None, 10).await.expect("list");
    let rec = records.iter().find(|r| r.id == id).expect("found");
    assert_eq!(rec.status, "open");
    assert_eq!(rec.resolved_at_ms, None);
    assert_eq!(rec.correction_diff, None);

    let diff = "--- old\n+++ new";
    db.resolve_research_misguidance(id, diff)
        .await
        .expect("resolve");

    let updated_records = db.list_research_misguidance(None, 10).await.expect("list");
    let updated = updated_records.iter().find(|r| r.id == id).expect("found");
    assert_eq!(updated.status, "resolved");
    assert!(updated.resolved_at_ms.is_some());
    assert_eq!(updated.correction_diff.as_deref(), Some(diff));
}
