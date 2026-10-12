//! Retrieval honors stored trust tiers (TRUST-01, D-03, D-05) and sanitizes at the injection
//! boundary (TRUST-05): `kb_tiers_*` cover tier propagation, the chat-preamble filter, the
//! derived envelope tier and the findings prohibition; `kb_injection_*` replay the red-team set
//! through the retrieval bundle, the numbered research sources and the Phase 2 KB tools.

mod common;

use std::sync::Arc;

use serde_json::json;
use vox_db::kb_text::{ContentHash, FindingOrigin, SanitizedText};
use vox_db::{ContextTrustTier, FindingTier, NewKbFinding, VoxDb};
use vox_orchestrator_mcp::ServerState;
use vox_orchestrator_mcp::memory_tools::{
    RetrievalBundle, RetrievalTriggerMode, run_retrieval_bundle,
};

const REDTEAM: &str = include_str!("../../../contracts/kb/redteam/cases.v1.jsonl");

fn redteam_cases() -> Vec<serde_json::Value> {
    REDTEAM
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("red-team line"))
        .collect()
}

fn str_list(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .expect("array")
        .iter()
        .map(|s| s.as_str().expect("str").to_string())
        .collect()
}

/// Seeds the three corpora the preamble reads, each carrying the term `zephyrine`:
/// a Trusted Library-style chunk, a legacy web-ingest chunk (column default: untrusted),
/// KG nodes of type fact / external_research / research_cache, and one APPROVED finding.
async fn seeded_state() -> ServerState {
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let tid = db
        .upsert_search_document(
            "library:docs/src/t.md",
            "Trusted Doc",
            "text/markdown",
            "h1",
            ContextTrustTier::Trusted,
        )
        .await
        .unwrap();
    db.replace_search_document_chunks(tid, &["zephyrine trustedalpha".to_string()])
        .await
        .unwrap();
    db.connection()
        .execute_batch(
            "INSERT INTO search_documents (id, source_uri, title) VALUES (900, 'web-ingest:x', 'Web Doc'); \
             INSERT INTO search_document_chunks (document_id, chunk_index, body_text) \
               VALUES (900, 0, 'zephyrine webbeta');",
        )
        .await
        .unwrap();
    for (id, ty, word) in [
        ("n-fact", "fact", "kgfactword"),
        ("n-ext", "external_research", "kgextword"),
        ("n-cache", "research_cache", "kgcacheword"),
    ] {
        db.upsert_knowledge_node(id, id, &format!("zephyrine {word}"), Some(ty), None, None)
            .await
            .unwrap();
    }
    db.insert_pending_finding(&NewKbFinding {
        dedupe_key: "kb-tiers-finding".into(),
        tier: FindingTier::Untrusted,
        origin: FindingOrigin::WebSearch,
        title: SanitizedText::new("finding"),
        summary: SanitizedText::new("zephyrine-finding-marker"),
        quotes: vec![],
        source_url: None,
        fetched_at_ms: 1,
        content_hash: ContentHash::from_blake3_hex(&"a".repeat(64)).unwrap(),
        snapshot_relpath: None,
        provenance: json!({}),
    })
    .await
    .unwrap()
    .expect("finding inserted");
    db.connection()
        .execute_batch(
            "UPDATE kb_findings SET status = 'approved', reviewed_by = 'test', reviewed_at_ms = 2;",
        )
        .await
        .unwrap();
    ServerState::new_test().await.with_db_initialized(db).await
}

async fn bundle(state: &ServerState, trigger: RetrievalTriggerMode) -> RetrievalBundle {
    run_retrieval_bundle(state, "zephyrine", trigger, 20, None, true)
        .await
        .expect("bundle")
}

fn every_line(b: &RetrievalBundle) -> Vec<&String> {
    b.memory_lines
        .iter()
        .chain(&b.knowledge_lines)
        .chain(&b.chunk_lines)
        .chain(&b.repo_lines)
        .chain(&b.rrf_fused_lines)
        .collect()
}

fn envelope_tier(b: &RetrievalBundle) -> ContextTrustTier {
    b.evidence
        .to_context_envelope("repo", None)
        .trust
        .trust_tier
}

fn has(lines: &[String], needle: &str) -> bool {
    lines.iter().any(|l| l.contains(needle))
}

#[tokio::test(flavor = "multi_thread")]
async fn kb_tiers_preamble_drops_untrusted_and_labels_the_rest() {
    let state = seeded_state().await;
    let b = bundle(&state, RetrievalTriggerMode::AutoChatPreamble).await;
    let trusted = b
        .chunk_lines
        .iter()
        .find(|l| l.contains("trustedalpha"))
        .unwrap_or_else(|| panic!("trusted chunk missing: {:?}", b.chunk_lines));
    assert!(trusted.starts_with("[tier:trusted] "), "{trusted}");
    assert!(
        !has(&b.chunk_lines, "webbeta"),
        "legacy web-ingest chunk reached the preamble: {:?}",
        b.chunk_lines
    );
    let fact = b
        .knowledge_lines
        .iter()
        .find(|l| l.contains("kgfactword"))
        .unwrap_or_else(|| panic!("fact node missing: {:?}", b.knowledge_lines));
    assert!(fact.starts_with("[tier:advisory] "), "{fact}");
    assert!(
        !has(&b.knowledge_lines, "kgextword"),
        "external_research node reached the preamble: {:?}",
        b.knowledge_lines
    );
    assert_eq!(envelope_tier(&b), ContextTrustTier::Advisory);
}

#[tokio::test(flavor = "multi_thread")]
async fn kb_tiers_explicit_query_keeps_untrusted_labelled() {
    let state = seeded_state().await;
    let b = bundle(&state, RetrievalTriggerMode::ExplicitToolQuery).await;
    let web = b
        .chunk_lines
        .iter()
        .find(|l| l.contains("webbeta"))
        .unwrap_or_else(|| panic!("web-ingest chunk missing: {:?}", b.chunk_lines));
    assert!(web.starts_with("[tier:untrusted] "), "{web}");
    let ext = b
        .knowledge_lines
        .iter()
        .find(|l| l.contains("kgextword"))
        .unwrap_or_else(|| panic!("external_research node missing: {:?}", b.knowledge_lines));
    assert!(ext.starts_with("[tier:untrusted] "), "{ext}");
    assert_eq!(envelope_tier(&b), ContextTrustTier::Untrusted);
}

#[tokio::test(flavor = "multi_thread")]
async fn kb_tiers_cache_nodes_and_findings_are_never_retrieved() {
    let state = seeded_state().await;
    for trigger in [
        RetrievalTriggerMode::AutoChatPreamble,
        RetrievalTriggerMode::ExplicitToolQuery,
    ] {
        let b = bundle(&state, trigger).await;
        for marker in ["kgcacheword", "zephyrine-finding-marker"] {
            for l in every_line(&b) {
                assert!(!l.contains(marker), "{trigger:?}: {marker} leaked: {l}");
            }
        }
        // Non-vacuous: the same bundle does carry the other seeded rows.
        assert!(has(&b.chunk_lines, "trustedalpha"), "{trigger:?}");
        assert!(has(&b.knowledge_lines, "kgfactword"), "{trigger:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn kb_tiers_findings_never_retrieved() {
    let state = seeded_state().await;
    let counts = state
        .db
        .as_ref()
        .unwrap()
        .kb_findings_counts()
        .await
        .unwrap();
    assert_eq!(counts.total, 1, "the approved finding must exist");
    for trigger in [
        RetrievalTriggerMode::AutoChatPreamble,
        RetrievalTriggerMode::ExplicitToolQuery,
    ] {
        let b = bundle(&state, trigger).await;
        assert!(
            every_line(&b)
                .iter()
                .all(|l| !l.contains("zephyrine-finding-marker")),
            "{trigger:?}: an approved finding reached retrieval"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn kb_tiers_empty_bundle_is_untrusted() {
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let state = ServerState::new_test().await.with_db_initialized(db).await;
    let b = run_retrieval_bundle(
        &state,
        "qqxnothingmatchesthis",
        RetrievalTriggerMode::AutoChatPreamble,
        5,
        None,
        true,
    )
    .await
    .unwrap();
    assert_eq!(
        b.evidence.chunk_hit_count + b.evidence.knowledge_hit_count,
        0
    );
    assert_eq!(envelope_tier(&b), ContextTrustTier::Untrusted);
}

#[tokio::test(flavor = "multi_thread")]
async fn kb_tiers_preamble_preserves_rank_order_of_survivors() {
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    // Newest created_at ranks first (chunk readers ORDER BY created_at DESC):
    // trusted-first, untrusted, trusted-last.
    let a = db
        .upsert_search_document(
            "library:a.md",
            "A",
            "text/markdown",
            "ha",
            ContextTrustTier::Trusted,
        )
        .await
        .unwrap();
    let c = db
        .upsert_search_document(
            "library:c.md",
            "C",
            "text/markdown",
            "hc",
            ContextTrustTier::Trusted,
        )
        .await
        .unwrap();
    db.connection()
        .execute_batch(&format!(
            "INSERT INTO search_documents (id, source_uri, title) VALUES (901, 'web-ingest:mid', 'Mid'); \
             INSERT INTO search_document_chunks (document_id, chunk_index, body_text, created_at) VALUES \
               ({a}, 0, 'zephyrine rankfirst', '2026-01-03 00:00:00'), \
               (901, 0, 'zephyrine rankmiddle', '2026-01-02 00:00:00'), \
               ({c}, 0, 'zephyrine ranklast', '2026-01-01 00:00:00');"
        ))
        .await
        .unwrap();
    let state = ServerState::new_test().await.with_db_initialized(db).await;

    let all = bundle(&state, RetrievalTriggerMode::ExplicitToolQuery).await;
    let pos = |b: &RetrievalBundle, w: &str| b.chunk_lines.iter().position(|l| l.contains(w));
    assert!(
        pos(&all, "rankfirst") < pos(&all, "rankmiddle")
            && pos(&all, "rankmiddle") < pos(&all, "ranklast")
            && pos(&all, "rankfirst").is_some()
            && pos(&all, "ranklast").is_some(),
        "explicit order: {:?}",
        all.chunk_lines
    );

    let pre = bundle(&state, RetrievalTriggerMode::AutoChatPreamble).await;
    assert!(pos(&pre, "rankmiddle").is_none(), "{:?}", pre.chunk_lines);
    assert!(
        pos(&pre, "rankfirst").is_some() && pos(&pre, "rankfirst") < pos(&pre, "ranklast"),
        "survivors lost their relative order: {:?}",
        pre.chunk_lines
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn kb_injection_bundle_lines_are_sanitized_for_every_redteam_case() {
    let cases = redteam_cases();
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let id = db
        .upsert_search_document(
            "library:docs/src/rt.md",
            "RT",
            "text/markdown",
            "hrt",
            ContextTrustTier::Trusted,
        )
        .await
        .unwrap();
    let bodies: Vec<String> = cases
        .iter()
        .map(|c| {
            format!(
                "zephyrine {} {}",
                c["id"].as_str().unwrap(),
                c["input"].as_str().unwrap()
            )
        })
        .collect();
    db.replace_search_document_chunks(id, &bodies)
        .await
        .unwrap();
    let state = ServerState::new_test().await.with_db_initialized(db).await;
    let b = run_retrieval_bundle(
        &state,
        "zephyrine",
        RetrievalTriggerMode::ExplicitToolQuery,
        50,
        None,
        true,
    )
    .await
    .unwrap();
    let mut checked = 0;
    for c in &cases {
        let cid = c["id"].as_str().unwrap();
        let line = b
            .chunk_lines
            .iter()
            .find(|l| l.contains(cid))
            .unwrap_or_else(|| panic!("case {cid} not retrieved: {:?}", b.chunk_lines));
        for bad in str_list(&c["must_not_contain"]) {
            assert!(
                !line.contains(&bad),
                "case {cid}: {bad:?} survived in {line:?}"
            );
        }
        for good in str_list(&c["must_contain"]) {
            assert!(
                line.contains(&good),
                "case {cid}: lost {good:?} in {line:?}"
            );
        }
        checked += 1;
    }
    assert!(checked >= 10, "red-team set shrank to {checked}");
}

#[test]
fn kb_injection_research_sources_are_sanitized_for_every_redteam_case() {
    use vox_orchestrator_mcp::chat_tools::chat::research_turn::sources_from_hits;
    use vox_search::memory_hybrid::HybridSearchHit;
    let mut checked = 0;
    for c in redteam_cases() {
        let cid = c["id"].as_str().unwrap().to_string();
        let input = c["input"].as_str().unwrap().to_string();
        let hit = HybridSearchHit {
            path: "https://evil.example/page".into(),
            title: input.clone(),
            content_snippet: input,
            score: 0.5,
            provenance: vec!["engine:searxng".into()],
            potential_contradiction: false,
        };
        let s = sources_from_hits(&[hit], 8);
        assert_eq!(s.len(), 1, "case {cid}");
        for field in [&s[0].title, &s[0].snippet] {
            for bad in str_list(&c["must_not_contain"]) {
                assert!(
                    !field.contains(&bad),
                    "case {cid}: {bad:?} survived in {field:?}"
                );
            }
            for good in str_list(&c["must_contain"]) {
                assert!(
                    field.contains(&good),
                    "case {cid}: lost {good:?} in {field:?}"
                );
            }
        }
        checked += 1;
    }
    assert!(checked >= 10, "red-team set shrank to {checked}");
}

/// D-07 as scoped by D-07a (WR-10): a fact the model stores through `memory_store` is an
/// Untrusted graph row that never reaches the preamble; the MEMORY.md memory line still does.
#[tokio::test(flavor = "multi_thread")]
async fn kb_tiers_agent_fact_from_memory_store_never_reaches_the_preamble() {
    use vox_orchestrator_mcp::memory_tools::{MemoryStoreParams, memory_store};

    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let mut state = ServerState::new_test()
        .await
        .with_db_initialized(db.clone())
        .await;
    let dir = tempfile::tempdir().unwrap();
    state.orchestrator_config.memory =
        vox_orchestrator::MemoryConfig::for_account("kbt", dir.path());
    let out = memory_store(
        &state,
        MemoryStoreParams {
            agent_id: 1,
            key: "kb-agent-fact".into(),
            value: "zephyragent marker text".into(),
            relations: Some(vec!["zephyrrel".into()]),
            media_url: None,
            media_type: None,
        },
    )
    .await;
    assert!(out.contains("Stored"), "memory_store failed: {out}");

    // persist_fact writes the graph rows in a spawned task: wait for both to exist.
    let mut present = false;
    for _ in 0..50 {
        let ids: Vec<String> = db
            .list_knowledge_nodes(None, 0, 100)
            .await
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        if ["kb-agent-fact", "zephyrrel"]
            .iter()
            .all(|w| ids.iter().any(|i| i == w))
        {
            present = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(present, "persist_fact graph rows never appeared");
    let tiers = db
        .trust_tiers_for_knowledge_nodes(&["kb-agent-fact".into(), "zephyrrel".into()])
        .await
        .unwrap();
    assert_eq!(tiers["kb-agent-fact"], ContextTrustTier::Untrusted);
    assert_eq!(tiers["zephyrrel"], ContextTrustTier::Untrusted);

    let explicit = run_retrieval_bundle(
        &state,
        "zephyragent",
        RetrievalTriggerMode::ExplicitToolQuery,
        20,
        None,
        true,
    )
    .await
    .expect("explicit bundle");
    let line = explicit
        .knowledge_lines
        .iter()
        .find(|l| l.contains("[node:kb-agent-fact]"))
        .unwrap_or_else(|| panic!("agent fact missing: {:?}", explicit.knowledge_lines));
    assert!(line.starts_with("[tier:untrusted] "), "{line}");
    assert_eq!(
        explicit.evidence.min_trust_tier,
        Some(ContextTrustTier::Untrusted)
    );

    let pre = run_retrieval_bundle(
        &state,
        "zephyragent",
        RetrievalTriggerMode::AutoChatPreamble,
        20,
        None,
        true,
    )
    .await
    .expect("preamble bundle");
    for l in every_line(&pre) {
        assert!(
            !l.contains("[node:kb-agent-fact]") && !l.contains("[node:zephyrrel]"),
            "agent graph row reached the preamble: {l}"
        );
    }
    assert!(
        !has(&pre.knowledge_lines, "zephyragent"),
        "{:?}",
        pre.knowledge_lines
    );
    // D-07a: the memory leg is not re-tiered in Phase 3; the MEMORY.md line stays.
    assert!(
        has(&pre.memory_lines, "zephyragent"),
        "memory line dropped (D-07a pins it): {:?}",
        pre.memory_lines
    );
}
