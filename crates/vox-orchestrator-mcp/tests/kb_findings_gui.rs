//! GUI side doors (Ingest URL / Ingest Text / Publish) file pending Untrusted findings and
//! write neither `knowledge_nodes` rows nor files under `docs/src` (GROW-06, TRUST-02).

mod common;

use vox_db::kb_text::{FindingOrigin, FindingStatus};
use vox_db::{ContextTrustTier, VoxDb};
use vox_orchestrator_mcp::kb_tools::findings::file_gui_finding;

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
