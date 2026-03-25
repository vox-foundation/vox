use std::fs;
use std::sync::Arc;

use vox_db::{DbConfig, VoxDb};
use vox_orchestrator::{Orchestrator, OrchestratorConfig, services::news::NewsService};
use vox_publisher::types::UnifiedNewsItem;

fn write_news_file(dir: &std::path::Path, id: &str) -> std::path::PathBuf {
    let path = dir.join(format!("{id}.md"));
    let content = r#"---
title: "News title"
author: "Agent"
published_at: "2026-03-25T00:00:00Z"
tags: ["news"]
syndication:
  rss: true
  dry_run: false
---
# Body

Hello world.
"#;
    fs::write(&path, content).expect("write news file");
    path
}

fn build_config(
    news_dir: &std::path::Path,
    feed_path: &std::path::Path,
    armed: bool,
) -> OrchestratorConfig {
    let mut cfg = OrchestratorConfig::for_testing();
    cfg.news.enabled = true;
    cfg.news.dry_run = false;
    cfg.news.publish_armed = armed;
    cfg.news.scan_recursive = false;
    cfg.news.news_dir = news_dir.to_string_lossy().to_string();
    cfg.news.rss_feed_path = Some(feed_path.to_string_lossy().to_string());
    cfg.news.site_base_url = Some("https://example.org".to_string());
    cfg
}

#[tokio::test]
async fn news_tick_blocks_when_not_armed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let news_dir = tmp.path().join("news");
    fs::create_dir_all(&news_dir).expect("mkdir");
    let id = "2026-03-25-not-armed";
    write_news_file(&news_dir, id);
    let feed_path = tmp.path().join("feed.xml");

    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let orch = Orchestrator::new(build_config(&news_dir, &feed_path, false)).with_db(db.clone());
    NewsService::tick(&orch).await.expect("tick");
    let published = db.is_news_published(id).await.expect("query");
    assert!(!published);
}

#[tokio::test]
async fn news_tick_publishes_when_armed_and_digest_has_dual_approval() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let news_dir = tmp.path().join("news");
    fs::create_dir_all(&news_dir).expect("mkdir");
    let id = "2026-03-25-armed";
    let path = write_news_file(&news_dir, id);
    let feed_path = tmp.path().join("feed.xml");

    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let content = fs::read_to_string(path).expect("read");
    let item = UnifiedNewsItem::parse(&content, id).expect("parse");
    let digest = item.content_sha3_256();
    db.record_news_approval_for_digest(id, &digest, "alice")
        .await
        .expect("approve alice");
    db.record_news_approval_for_digest(id, &digest, "bob")
        .await
        .expect("approve bob");
    db.record_publication_approval_for_digest(id, &digest, "alice")
        .await
        .expect("approve publication alice");
    db.record_publication_approval_for_digest(id, &digest, "bob")
        .await
        .expect("approve publication bob");

    let orch = Orchestrator::new(build_config(&news_dir, &feed_path, true)).with_db(db.clone());
    NewsService::tick(&orch).await.expect("tick");

    let published = db.is_news_published(id).await.expect("query");
    assert!(published);
}

#[tokio::test]
async fn news_tick_blocks_when_worthiness_enforced_and_below_floor() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let news_dir = tmp.path().join("news");
    fs::create_dir_all(&news_dir).expect("mkdir");
    let id = "2026-03-25-worthiness-block";
    let path = write_news_file(&news_dir, id);
    let feed_path = tmp.path().join("feed.xml");

    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let content = fs::read_to_string(path).expect("read");
    let item = UnifiedNewsItem::parse(&content, id).expect("parse");
    let digest = item.content_sha3_256();
    db.record_news_approval_for_digest(id, &digest, "alice")
        .await
        .expect("approve alice");
    db.record_news_approval_for_digest(id, &digest, "bob")
        .await
        .expect("approve bob");
    db.record_publication_approval_for_digest(id, &digest, "alice")
        .await
        .expect("approve publication alice");
    db.record_publication_approval_for_digest(id, &digest, "bob")
        .await
        .expect("approve publication bob");

    let mut cfg = build_config(&news_dir, &feed_path, true);
    cfg.news.worthiness_enforce = true;
    cfg.news.worthiness_score_min = Some(0.99);
    let orch = Orchestrator::new(cfg).with_db(db.clone());
    NewsService::tick(&orch).await.expect("tick");

    let published = db.is_news_published(id).await.expect("query");
    assert!(!published);
}
