use vox_db::{DbConfig, VoxDb};

#[tokio::test]
async fn news_publish_approvals_require_two_distinct_approvers() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();

    db.record_news_approval("2026-03-25-example", "alice")
        .await
        .unwrap();
    assert_eq!(
        db.count_news_approvers("2026-03-25-example").await.unwrap(),
        1
    );
    assert!(
        !db.has_dual_news_approval("2026-03-25-example")
            .await
            .unwrap()
    );

    db.record_news_approval("2026-03-25-example", "bob")
        .await
        .unwrap();
    assert!(
        db.has_dual_news_approval("2026-03-25-example")
            .await
            .unwrap()
    );

    // Same approver again does not increase distinct count
    db.record_news_approval("2026-03-25-example", "alice")
        .await
        .unwrap();
    assert_eq!(
        db.count_news_approvers("2026-03-25-example").await.unwrap(),
        2
    );
}

#[tokio::test]
async fn mark_news_published_column_order_matches_github_twitter_oc() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    db.mark_news_published("x", Some("gh"), Some("tw"), Some("oc"))
        .await
        .unwrap();
    let rows = db
        .query_all(
            "SELECT github_release_id, twitter_tweet_id, opencollective_update_id FROM published_news WHERE id = 'x'",
            (),
        )
        .await
        .unwrap();
    let r = rows.first().unwrap();
    let gh: String = r.get(0).unwrap();
    let tw: String = r.get(1).unwrap();
    let oc: String = r.get(2).unwrap();
    assert_eq!(gh, "gh");
    assert_eq!(tw, "tw");
    assert_eq!(oc, "oc");
}
