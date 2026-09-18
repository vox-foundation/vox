//! Phase 0a — `run_research` is callable; web hits flow through `vox-search` when scope allows.

use vox_research_shim::research::types::{ResearchQuery, ResearchScope};
use vox_research_shim::research::{BroadcastEmitter, ResearchConfig, run_research};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_research_returns_coherent_metadata() {
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

    let result = run_research(query, Some(&db), &config)
        .await
        .expect("succeeds");

    assert!(
        result.research_metadata.subquery_count >= 1,
        "planner emits at least one subquery"
    );
    assert!(
        result.research_metadata.source_count == result.sources.len(),
        "metadata source_count tracks sources vec"
    );
    assert!(result.research_metadata.claim_verdicts.is_empty());
    assert!(
        result.citations.len() <= result.sources.len(),
        "citations are capped from sources"
    );
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

    let result = run_research(query, Some(&db), &config)
        .await
        .expect("succeeds");

    assert!(result.research_metadata.session_id > 0);
    let session = db
        .get_research_session(result.research_metadata.session_id)
        .await
        .expect("get session")
        .expect("session row");
    assert_eq!(session.query_text, "session persistence smoke");
    assert_eq!(session.status, "completed");
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

    let result = run_research(query, Some(&db), &config)
        .await
        .expect("succeeds");

    let artifact = db
        .get_research_artifact(result.research_metadata.session_id)
        .await
        .expect("get artifact")
        .expect("artifact row");
    assert!(artifact.artifact_json.contains("\"schema_version\":1"));
    assert!(
        artifact
            .artifact_json
            .contains("artifact persistence smoke")
    );
    assert!(artifact.report_markdown.contains("# Research Report"));
    assert!(artifact.report_markdown.contains("## Sources"));
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

    let (sender, mut receiver) = tokio::sync::broadcast::channel(16);
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

    let _ = run_research(query, Some(&db), &config)
        .await
        .expect("succeeds");

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
