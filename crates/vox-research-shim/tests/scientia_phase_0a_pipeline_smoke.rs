//! Phase 0a — `run_research` is callable; web hits flow through `vox-search` when scope allows.
//!
//! These tests run with no LLM endpoint configured (no `runtime` feature, no API key), so
//! synthesis honestly fails (Task 7 / D5: the template-answer fallback was removed). Each
//! test now asserts that honest failure instead of a template-backed "success" — the earlier
//! pipeline stages (session creation, event emission) still ran and are checked where they did.

use vox_research_shim::research::types::{ResearchQuery, ResearchScope};
use vox_research_shim::research::{
    BroadcastEmitter, ResearchConfig, run_research, run_research_with_context_and_session,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_research_without_llm_fails_honestly_not_with_a_template() {
    let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
        .await
        .expect("memory db");
    db.save_snippet(vox_db::SaveSnippetParams {
        language: "rust",
        title: "smoke test orchestrator research pipeline",
        code: "fn pipeline_smoke() {}",
        description: Some("smoke test orchestrator research pipeline evidence"),
        tags: Some("smoke"),
        author_id: None,
        source_ref: None,
        embedding_ref: None,
    })
    .await
    .expect("save snippet");

    let query = ResearchQuery {
        query: "smoke test orchestrator research pipeline".into(),
        scope: ResearchScope::Both,
        max_sources: 3,
        persist_to_docs: false,
        verify_claims: false,
        site_scope: None,
        domain_mode: Default::default(),
        waves: 1,
        lane: vox_search::policy::ResearchLane::Fast,
    };
    let config = ResearchConfig::default();

    // No LLM endpoint configured: synthesis honestly fails (D5 — no template fallback).
    let err = run_research(query, Some(&db), &config)
        .await
        .expect_err("synthesis has no LLM to call in this environment");
    assert!(err.to_string().contains("synthesis failed"), "{err}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_research_with_codex_persists_session_row() {
    let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
        .await
        .expect("memory db");
    db.save_snippet(vox_db::SaveSnippetParams {
        language: "rust",
        title: "session persistence smoke",
        code: "fn session_persistence_smoke() {}",
        description: Some("session persistence smoke snippet"),
        tags: Some("smoke"),
        author_id: None,
        source_ref: None,
        embedding_ref: None,
    })
    .await
    .expect("save snippet");

    let query = ResearchQuery {
        query: "session persistence smoke".into(),
        scope: ResearchScope::Local,
        max_sources: 3,
        persist_to_docs: false,
        verify_claims: false,
        site_scope: None,
        domain_mode: Default::default(),
        waves: 1,
        lane: vox_search::policy::ResearchLane::Fast,
    };
    let config = ResearchConfig::default();

    let session_id = db
        .create_research_session("test:session-persistence-smoke", &query.query)
        .await
        .expect("create session");

    // No LLM endpoint configured: synthesis honestly fails, but the session row created
    // before synthesis must still be findable and correctly marked failed (D5/D8).
    let err =
        run_research_with_context_and_session(query, None, Some(&db), &config, Some(session_id))
            .await
            .expect_err("synthesis has no LLM to call in this environment");
    assert!(err.to_string().contains("synthesis failed"), "{err}");

    let session = db
        .get_research_session(session_id)
        .await
        .expect("get session")
        .expect("session row");
    assert_eq!(session.query_text, "session persistence smoke");
    assert_eq!(session.status, "failed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_research_with_codex_persists_durable_artifact() {
    let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
        .await
        .expect("memory db");
    db.save_snippet(vox_db::SaveSnippetParams {
        language: "rust",
        title: "artifact persistence smoke",
        code: "fn artifact_persistence_smoke() {}",
        description: Some("artifact persistence smoke snippet"),
        tags: Some("smoke"),
        author_id: None,
        source_ref: None,
        embedding_ref: None,
    })
    .await
    .expect("save snippet");

    let query = ResearchQuery {
        query: "artifact persistence smoke".into(),
        scope: ResearchScope::Local,
        max_sources: 3,
        persist_to_docs: false,
        verify_claims: false,
        site_scope: None,
        domain_mode: Default::default(),
        waves: 1,
        lane: vox_search::policy::ResearchLane::Fast,
    };
    let config = ResearchConfig::default();

    let session_id = db
        .create_research_session("test:artifact-persistence-smoke", &query.query)
        .await
        .expect("create session");

    // No LLM endpoint configured: synthesis honestly fails before the artifact-persistence
    // stage runs, so no artifact must be written — a failed run must not leave a fabricated
    // durable artifact behind (D5/D8).
    let err =
        run_research_with_context_and_session(query, None, Some(&db), &config, Some(session_id))
            .await
            .expect_err("synthesis has no LLM to call in this environment");
    assert!(err.to_string().contains("synthesis failed"), "{err}");

    let artifact = db
        .get_research_artifact(session_id)
        .await
        .expect("get artifact query succeeds");
    assert!(
        artifact.is_none(),
        "a failed run must not persist a fabricated artifact"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_research_emits_scientia_events() {
    let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
        .await
        .expect("memory db");
    db.save_snippet(vox_db::SaveSnippetParams {
        language: "rust",
        title: "event emission smoke",
        code: "fn event_emission_smoke() {}",
        description: Some("event emission smoke snippet"),
        tags: Some("smoke"),
        author_id: None,
        source_ref: None,
        embedding_ref: None,
    })
    .await
    .expect("save snippet");

    // Capacity bumped from 16 (Task 15b): fixing the claim-span coordinate bug means
    // legitimate claims now survive extraction and each emits a `ClaimExtracted` event,
    // so the small buffer previously sized for the (mostly-dropped-claims) bug lags
    // before this test's single post-hoc `try_recv`.
    let (sender, mut receiver) = tokio::sync::broadcast::channel(256);
    let config = ResearchConfig {
        event_emitter: Some(std::sync::Arc::new(BroadcastEmitter::new(sender))),
        ..ResearchConfig::default()
    };
    let query = ResearchQuery {
        query: "event emission smoke".into(),
        scope: ResearchScope::Local,
        max_sources: 3,
        persist_to_docs: false,
        verify_claims: false,
        site_scope: None,
        domain_mode: Default::default(),
        waves: 1,
        lane: vox_search::policy::ResearchLane::Fast,
    };

    // No LLM endpoint configured: synthesis honestly fails, but the "research_started"
    // telemetry event fires well before synthesis and must still have been emitted.
    let err = run_research(query, Some(&db), &config)
        .await
        .expect_err("synthesis has no LLM to call in this environment");
    assert!(err.to_string().contains("synthesis failed"), "{err}");

    let first = receiver.try_recv().expect("at least one research event");
    assert!(matches!(
        first,
        vox_research_events::ResearchEvent::TelemetryObservation { .. }
    ));
}

#[tokio::test]
#[ignore = "requires live web (SearXNG/DDG/Tavily) and optional API keys — owner: orchestrator sunset: 2026-12-31"]
async fn run_research_live_web_may_return_sources() {
    let query = ResearchQuery {
        query: "Rust programming language official website".into(),
        scope: ResearchScope::Web,
        max_sources: 5,
        persist_to_docs: false,
        verify_claims: false,
        site_scope: None,
        domain_mode: Default::default(),
        waves: 1,
        lane: vox_search::policy::ResearchLane::Fast,
    };
    let config = ResearchConfig::default();

    let result = run_research(query, None, &config).await.expect("succeeds");
    assert!(
        !result.sources.is_empty(),
        "live web backends should return ≥1 hit when configured"
    );
}
