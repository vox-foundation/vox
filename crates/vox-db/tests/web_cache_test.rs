use vox_db::VoxDb;
use vox_db::web_cache::CachedWebArtifact;

#[tokio::test]
async fn test_web_cache_put_get_and_touch() {
    let db = VoxDb::in_memory().await.expect("in-memory db init");

    let artifact = CachedWebArtifact {
        url_hash: String::new(),
        url: "https://docs.rs/tokio/latest/tokio/time/fn.sleep.html".to_string(),
        etag: Some("\"33a64df5\"".to_string()),
        last_modified: Some("Wed, 21 Oct 2025 07:28:00 GMT".to_string()),
        status_code: 200,
        content_type: "text/html; charset=utf-8".to_string(),
        raw_body: b"<html><body>sleep doc</body></html>".to_vec(),
        extracted_markdown: "# sleep".to_string(),
        fetched_at_ms: 1000,
    };

    db.put_cached_web_artifact(&artifact)
        .await
        .expect("put cache");

    let retrieved = db
        .get_cached_web_artifact(&artifact.url)
        .await
        .expect("get cache");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().fetched_at_ms, 1000);

    // Touch on HTTP 304 Not Modified
    db.touch_cached_web_artifact(&artifact.url, 2000)
        .await
        .expect("touch cache");
    let touched = db
        .get_cached_web_artifact(&artifact.url)
        .await
        .expect("get cache");
    assert_eq!(touched.unwrap().fetched_at_ms, 2000);
}
