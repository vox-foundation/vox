//! Store-level checks for the KB findings inbox (baseline 96) and the legacy VoxKB readers.
//! Raw SQL here is `execute_batch` only (UPDATE/DROP/seed); rows are created through the typed
//! writer so the tiered-writer invariant (03-05) never needs a test allowlist entry.

use vox_db::kb_text::{ContentHash, FindingOrigin, FindingStatus, SanitizedText};
use vox_db::{ContextTrustTier, FindingTier, LEGACY_VOXKB_DDL, NewKbFinding, VoxDb};

fn finding(key: &str, origin: FindingOrigin) -> NewKbFinding {
    NewKbFinding {
        dedupe_key: key.into(),
        tier: FindingTier::Untrusted,
        origin,
        title: SanitizedText::new("Title"),
        summary: SanitizedText::new("A summary"),
        quotes: vec![SanitizedText::new("q1"), SanitizedText::new("q2")],
        source_url: Some(SanitizedText::new("https://example.org/a")),
        fetched_at_ms: 1_700_000_000_000,
        content_hash: ContentHash::from_blake3_hex(&"a".repeat(64)).unwrap(),
        snapshot_relpath: Some("kb/snapshots/a.md".into()),
        provenance: serde_json::json!({"query": "rust async"}),
    }
}

async fn ids(db: &VoxDb) -> Vec<String> {
    db.kb_findings_list(None, 100)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect()
}

#[tokio::test]
async fn kb_findings_round_trip_with_full_provenance() {
    let db = VoxDb::open_memory().await.unwrap();
    let id = db
        .insert_pending_finding(&finding("k1", FindingOrigin::WebSearch))
        .await
        .unwrap()
        .expect("inserted");
    let rows = db
        .kb_findings_list(Some(FindingStatus::Pending), 10)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.id, id);
    assert_eq!(r.status, FindingStatus::Pending);
    assert_eq!(r.trust_tier, ContextTrustTier::Untrusted);
    assert_eq!(r.origin, FindingOrigin::WebSearch);
    assert_eq!(r.summary, "A summary");
    assert_eq!(r.quotes, ["q1", "q2"]);
    assert_eq!(r.source_url.as_deref(), Some("https://example.org/a"));
    assert_eq!(r.fetched_at_ms, 1_700_000_000_000);
    assert!(r.content_hash.starts_with("blake3:"));
    assert_eq!(r.snapshot_relpath.as_deref(), Some("kb/snapshots/a.md"));
    assert_eq!(r.provenance["query"], "rust async");
    assert!(!r.mens_queued && r.reviewed_by.is_none());
}

#[tokio::test]
async fn kb_findings_advisory_tier_is_stored() {
    let db = VoxDb::open_memory().await.unwrap();
    let mut f = finding("adv", FindingOrigin::ResearchQuick);
    f.tier = FindingTier::Advisory;
    db.insert_pending_finding(&f).await.unwrap().unwrap();
    let rows = db.kb_findings_list(None, 10).await.unwrap();
    assert_eq!(rows[0].trust_tier, ContextTrustTier::Advisory);
}

#[tokio::test]
async fn kb_findings_duplicate_dedupe_key_is_a_no_op() {
    let db = VoxDb::open_memory().await.unwrap();
    let f = finding("same", FindingOrigin::WebSearch);
    assert!(db.insert_pending_finding(&f).await.unwrap().is_some());
    assert!(db.insert_pending_finding(&f).await.unwrap().is_none());
    assert_eq!(ids(&db).await.len(), 1);
}

#[tokio::test]
async fn kb_findings_list_orders_newest_first_then_id_ascending() {
    let db = VoxDb::open_memory().await.unwrap();
    for k in ["a", "b", "c", "d"] {
        db.insert_pending_finding(&finding(k, FindingOrigin::WebSearch))
            .await
            .unwrap();
    }
    let all = db.kb_findings_list(None, 10).await.unwrap();
    let newest = all[3].id.clone();
    db.connection()
        .execute_batch(format!(
            "UPDATE kb_findings SET created_at_ms = 1000; \
             UPDATE kb_findings SET created_at_ms = 2000 WHERE id = '{newest}';"
        ))
        .await
        .unwrap();
    let got = ids(&db).await;
    let mut tied: Vec<String> = got.iter().filter(|i| **i != newest).cloned().collect();
    tied.sort();
    assert_eq!(got[0], newest, "newest created_at_ms first");
    assert_eq!(got[1..], tied[..], "equal timestamps sort by ascending id");
    assert_eq!(
        db.kb_findings_list(None, 2).await.unwrap().len(),
        2,
        "limit applies"
    );
}

#[tokio::test]
async fn kb_findings_unqueued_reviewed_feeds_mens_once() {
    let db = VoxDb::open_memory().await.unwrap();
    for k in ["approved", "rejected", "no_reviewer", "pending"] {
        db.insert_pending_finding(&finding(k, FindingOrigin::WebSearch))
            .await
            .unwrap();
    }
    let by_key = |rows: &[vox_db::KbFindingRow], k: &str| {
        rows.iter().find(|r| r.dedupe_key == k).unwrap().id.clone()
    };
    let rows = db.kb_findings_list(None, 10).await.unwrap();
    let (a, r, n) = (
        by_key(&rows, "approved"),
        by_key(&rows, "rejected"),
        by_key(&rows, "no_reviewer"),
    );
    db.connection()
        .execute_batch(format!(
            "UPDATE kb_findings SET status='approved', reviewed_by='me', reviewed_at_ms=5 WHERE id='{a}'; \
             UPDATE kb_findings SET status='rejected', reviewed_by='me', reviewed_at_ms=5 WHERE id='{r}'; \
             UPDATE kb_findings SET status='approved' WHERE id='{n}';"
        ))
        .await
        .unwrap();
    let mut queue: Vec<String> = db
        .kb_findings_unqueued_reviewed(10)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect();
    queue.sort();
    let mut want = vec![a.clone(), r.clone()];
    want.sort();
    assert_eq!(queue, want);
    db.kb_findings_mark_mens_queued(&[a, r]).await.unwrap();
    assert!(
        db.kb_findings_unqueued_reviewed(10)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn kb_findings_counts_report_total_pending_and_origin() {
    let db = VoxDb::open_memory().await.unwrap();
    assert_eq!(db.kb_findings_counts().await.unwrap().total, 0);
    for (k, o) in [
        ("1", FindingOrigin::WebSearch),
        ("2", FindingOrigin::WebSearch),
        ("3", FindingOrigin::ResearchDeep),
    ] {
        db.insert_pending_finding(&finding(k, o)).await.unwrap();
    }
    let first = ids(&db).await.remove(0);
    db.connection()
        .execute_batch(format!(
            "UPDATE kb_findings SET status='approved' WHERE id='{first}';"
        ))
        .await
        .unwrap();
    let c = db.kb_findings_counts().await.unwrap();
    assert_eq!((c.total, c.pending), (3, 2));
    assert_eq!(c.by_origin.get("web_search"), Some(&2));
    assert_eq!(c.by_origin.values().sum::<i64>(), 3);
    assert_eq!(c.by_origin.get("research_deep"), Some(&1));
}

#[tokio::test]
async fn kb_findings_unknown_status_or_origin_errors_unknown_tier_is_untrusted() {
    let db = VoxDb::open_memory().await.unwrap();
    let id = db
        .insert_pending_finding(&finding("x", FindingOrigin::WebSearch))
        .await
        .unwrap()
        .unwrap();
    db.connection()
        .execute_batch(format!(
            "UPDATE kb_findings SET trust_tier='Trusted' WHERE id='{id}';"
        ))
        .await
        .unwrap();
    let rows = db.kb_findings_list(None, 10).await.unwrap();
    assert_eq!(
        rows[0].trust_tier,
        ContextTrustTier::Untrusted,
        "unknown tier fails closed"
    );
    db.connection()
        .execute_batch(format!(
            "UPDATE kb_findings SET status='bogus' WHERE id='{id}';"
        ))
        .await
        .unwrap();
    assert!(db.kb_findings_list(None, 10).await.is_err());
    db.connection()
        .execute_batch(format!(
            "UPDATE kb_findings SET status='pending', origin='bogus' WHERE id='{id}';"
        ))
        .await
        .unwrap();
    assert!(db.kb_findings_list(None, 10).await.is_err());
}

#[tokio::test]
async fn kb_findings_legacy_counts_and_paging() {
    let db = VoxDb::open_memory().await.unwrap();
    db.connection()
        .execute_batch(LEGACY_VOXKB_DDL)
        .await
        .unwrap();
    db.connection()
        .execute_batch(
            "INSERT INTO knowledge_bases (id, name, created_at_ms, updated_at_ms) VALUES ('b1','alpha',1,1);
             INSERT INTO kb_entries (id, kb_id, content, source_signal, source_ref, created_at_ms, accepted, mens_queued)
               VALUES ('e2','b1','second','manual','ref',20,0,1);
             INSERT INTO kb_entries (id, kb_id, content, source_signal, created_at_ms)
               VALUES ('e1','b1','first','manual',10);
             INSERT INTO kb_routing_rules (id, kb_id, rule_type, pattern, created_at_ms)
               VALUES ('r1','b1','keyword','x',1);",
        )
        .await
        .unwrap();
    let c = db.legacy_kb_counts().await.unwrap();
    assert_eq!(
        (c.knowledge_bases, c.kb_entries, c.kb_routing_rules),
        (1, 2, 1)
    );

    let page = db.legacy_kb_entries_page(None, 1).await.unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(
        (page[0].id.as_str(), page[0].kb_name.as_str()),
        ("e1", "alpha")
    );
    assert!(page[0].accepted && !page[0].mens_queued && page[0].source_ref.is_none());
    let next = db.legacy_kb_entries_page(Some("e1"), 10).await.unwrap();
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].id, "e2");
    assert_eq!(next[0].source_ref.as_deref(), Some("ref"));
    assert!(!next[0].accepted && next[0].mens_queued);
    assert!(
        db.legacy_kb_entries_page(Some("e2"), 10)
            .await
            .unwrap()
            .is_empty()
    );

    db.connection()
        .execute_batch(
            "DROP TABLE kb_routing_rules; DROP TABLE kb_entries; DROP TABLE knowledge_bases;",
        )
        .await
        .unwrap();
    let z = db.legacy_kb_counts().await.unwrap();
    assert_eq!(
        (z.knowledge_bases, z.kb_entries, z.kb_routing_rules),
        (0, 0, 0)
    );
    assert!(
        db.legacy_kb_entries_page(None, 10)
            .await
            .unwrap()
            .is_empty()
    );
}
