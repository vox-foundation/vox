//! Legacy VoxKB rows are counted, then copied into the findings inbox as pending Untrusted
//! findings, without deleting anything (RET-03, D-03).

mod common;

use std::sync::Arc;
use std::time::Duration;

use vox_db::kb_text::{FindingOrigin, FindingStatus};
use vox_db::{ContextTrustTier, LegacyKbCounts, VoxDb};
use vox_orchestrator_mcp::ServerState;
use vox_orchestrator_mcp::kb_tools::findings::{LegacyKbReport, migrate_legacy_kb_entries};

/// One KB, two entries (the second whitespace-only), one routing rule.
async fn seeded_db() -> VoxDb {
    let db = VoxDb::open_memory().await.unwrap();
    db.connection()
        .execute_batch(format!(
            "{} \
             INSERT INTO knowledge_bases (id, name, description, created_at_ms, updated_at_ms, entry_count) \
               VALUES ('kb1', 'research', '', 1, 1, 2); \
             INSERT INTO kb_entries (id, kb_id, content, source_signal, source_ref, routing_confidence, \
                                     created_at_ms, accepted, mens_queued) \
               VALUES ('e1', 'kb1', 'Legacy fact about widgets.', 'chat', 'sess-1', 0.75, 42, 1, 1); \
             INSERT INTO kb_entries (id, kb_id, content, source_signal, created_at_ms) \
               VALUES ('e2', 'kb1', '   \n ', 'chat', 43); \
             INSERT INTO kb_routing_rules (id, kb_id, rule_type, pattern, created_at_ms) \
               VALUES ('r1', 'kb1', 'keyword', 'widget', 1);",
            vox_db::LEGACY_VOXKB_DDL
        ))
        .await
        .unwrap();
    db
}

fn counts(kbs: i64, entries: i64, rules: i64) -> LegacyKbCounts {
    LegacyKbCounts {
        knowledge_bases: kbs,
        kb_entries: entries,
        kb_routing_rules: rules,
    }
}

#[tokio::test]
async fn legacy_kb_counts_then_migrates_non_empty_rows_as_pending_untrusted() {
    let db = seeded_db().await;
    let tmp = tempfile::tempdir().unwrap();
    let r = migrate_legacy_kb_entries(&db, tmp.path()).await.unwrap();
    assert_eq!(
        r,
        LegacyKbReport {
            knowledge_bases: 1,
            kb_entries: 2,
            kb_routing_rules: 1,
            migrated: 1,
            skipped_empty: 1,
            already_present: 0,
        }
    );
    let rows = db.kb_findings_list(None, 10).await.unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    let f = &rows[0];
    assert_eq!(f.status, FindingStatus::Pending);
    assert_eq!(f.trust_tier, ContextTrustTier::Untrusted);
    assert_eq!(f.origin, FindingOrigin::LegacyKbEntry);
    assert!(f.reviewed_by.is_none());
    assert!(f.summary.contains("widgets"));
    let p = &f.provenance;
    assert_eq!(p["legacy_id"], "e1");
    assert_eq!(p["kb_name"], "research");
    assert_eq!(p["source_signal"], "chat");
    assert_eq!(p["source_ref"], "sess-1");
    assert_eq!(p["routing_confidence"], 0.75);
    assert_eq!(p["accepted"], true);
    assert_eq!(p["mens_queued"], true);
    assert_eq!(p["created_at_ms"], 42);
}

#[tokio::test]
async fn legacy_kb_rows_survive_migration() {
    let db = seeded_db().await;
    let tmp = tempfile::tempdir().unwrap();
    migrate_legacy_kb_entries(&db, tmp.path()).await.unwrap();
    assert_eq!(db.legacy_kb_counts().await.unwrap(), counts(1, 2, 1));
    let page = db.legacy_kb_entries_page(None, 10).await.unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(page[0].content, "Legacy fact about widgets.");
}

#[tokio::test]
async fn legacy_kb_migration_is_idempotent() {
    let db = seeded_db().await;
    let tmp = tempfile::tempdir().unwrap();
    migrate_legacy_kb_entries(&db, tmp.path()).await.unwrap();
    let r = migrate_legacy_kb_entries(&db, tmp.path()).await.unwrap();
    assert_eq!((r.migrated, r.already_present, r.skipped_empty), (0, 1, 1));
    assert_eq!(db.kb_findings_list(None, 10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn legacy_kb_absent_tables_report_zeros_without_error() {
    let db = seeded_db().await;
    db.connection()
        .execute_batch(
            "DROP TABLE kb_routing_rules; DROP TABLE kb_entries; DROP TABLE knowledge_bases;"
                .to_string(),
        )
        .await
        .unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let r = migrate_legacy_kb_entries(&db, tmp.path()).await.unwrap();
    assert_eq!(r, LegacyKbReport::default());
    assert!(db.kb_findings_list(None, 10).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn legacy_kb_attach_migrates_in_background() {
    let db = Arc::new(seeded_db().await);
    let tmp = tempfile::tempdir().unwrap();
    let mut state = ServerState::new_test().await;
    state.repository.root = tmp.path().to_path_buf();
    let _state = state.with_db_initialized(db.clone()).await;
    let mut found = vec![];
    for _ in 0..50 {
        found = db.kb_findings_list(None, 10).await.unwrap();
        if !found.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].origin, FindingOrigin::LegacyKbEntry);
}
