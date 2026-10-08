//! GUI side doors (Ingest URL / Ingest Text / Publish) file pending Untrusted findings and
//! write neither `knowledge_nodes` rows nor files under `docs/src` (GROW-06, TRUST-02).

mod common;

use vox_db::kb_text::{FindingOrigin, FindingStatus};
use vox_db::{ContextTrustTier, VoxDb};
use vox_orchestrator_mcp::kb_tools::findings::{file_gui_finding, file_publish_draft};

#[tokio::test]
async fn gui_inbox_ingest_url_writes_no_knowledge_node() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let url = "https://example.com/post";
    let md = "# Post\n\nSome scraped page markdown.";
    let call = || {
        file_gui_finding(
            &db,
            tmp.path(),
            FindingOrigin::GuiIngestUrl,
            "Post",
            md,
            Some(url),
            serde_json::json!({"text_density": 0.5}),
        )
    };

    let first = call().await.unwrap();
    assert_eq!(first.status, "pending");
    assert!(first.finding_id.is_some());

    let rows = db.kb_findings_list(None, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, FindingStatus::Pending);
    assert_eq!(rows[0].trust_tier, ContextTrustTier::Untrusted);
    assert_eq!(rows[0].origin, FindingOrigin::GuiIngestUrl);
    assert_eq!(rows[0].source_url.as_deref(), Some(url));
    let snap = rows[0].snapshot_relpath.as_deref().unwrap();
    assert_eq!(std::fs::read_to_string(tmp.path().join(snap)).unwrap(), md);
    assert!(
        db.list_knowledge_nodes(None, 0, 10)
            .await
            .unwrap()
            .is_empty(),
        "side door must not write knowledge_nodes"
    );

    let second = call().await.unwrap();
    assert_eq!(second.status, "duplicate");
    assert_eq!(second.finding_id, None);
    assert_eq!(db.kb_findings_list(None, 10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn gui_inbox_empty_text_is_rejected_and_stores_nothing() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let r = file_gui_finding(
        &db,
        tmp.path(),
        FindingOrigin::GuiIngestUrl,
        "t",
        "  \n",
        Some("https://example.com"),
        serde_json::json!({}),
    )
    .await;
    assert!(r.is_err());
    assert!(db.kb_findings_list(None, 10).await.unwrap().is_empty());
}

#[tokio::test]
async fn gui_inbox_ingest_text_files_untrusted_pending_finding() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let r = file_gui_finding(
        &db,
        tmp.path(),
        FindingOrigin::GuiIngestText,
        "Notes",
        "pasted text body",
        None,
        serde_json::json!({"ingest_type": "text"}),
    )
    .await
    .unwrap();
    assert_eq!(r.status, "pending");
    let rows = db.kb_findings_list(None, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].origin, FindingOrigin::GuiIngestText);
    assert_eq!(rows[0].trust_tier, ContextTrustTier::Untrusted);
    assert_eq!(rows[0].status, FindingStatus::Pending);
    assert!(
        db.list_knowledge_nodes(None, 0, 10)
            .await
            .unwrap()
            .is_empty()
    );
}

fn docs_src_exists(root: &std::path::Path) -> bool {
    root.join("docs").join("src").exists()
}

#[tokio::test]
async fn gui_inbox_publish_writes_no_docs_file() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let r = file_publish_draft(&db, tmp.path(), 7, "my-topic", "# Draft\n\nbody text")
        .await
        .unwrap();
    assert_eq!(r.status, "pending");
    assert_eq!(r.proposed_path, "docs/src/architecture/my-topic-2026.md");
    let rows = db.kb_findings_list(None, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].origin, FindingOrigin::PublishResearchDoc);
    assert_eq!(rows[0].trust_tier, ContextTrustTier::Untrusted);
    assert_eq!(
        rows[0].provenance["proposed_path"],
        "docs/src/architecture/my-topic-2026.md"
    );
    assert_eq!(rows[0].provenance["session_id"], 7);
    assert!(
        !docs_src_exists(tmp.path()),
        "publish must not write docs/src"
    );
    assert!(
        db.list_knowledge_nodes(None, 0, 10)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn gui_inbox_publish_does_not_double_the_suffix_and_reports_duplicates() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let first = file_publish_draft(&db, tmp.path(), 1, "topic-2026.md", "draft")
        .await
        .unwrap();
    assert_eq!(first.proposed_path, "docs/src/architecture/topic-2026.md");
    let again = file_publish_draft(&db, tmp.path(), 1, "topic-2026.md", "draft")
        .await
        .unwrap();
    assert_eq!(again.status, "duplicate");
    assert_eq!(again.finding_id, None);
}

#[tokio::test]
async fn gui_inbox_publish_rejects_traversal_slugs() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    for slug in ["a/b", "a\\b", "..", "x..y", ".hidden", ""] {
        let r = file_publish_draft(&db, tmp.path(), 1, slug, "draft").await;
        assert!(r.is_err(), "slug {slug:?} must be rejected");
    }
    assert!(db.kb_findings_list(None, 10).await.unwrap().is_empty());
    assert!(!docs_src_exists(tmp.path()));
}
