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
