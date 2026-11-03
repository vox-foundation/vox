use vox_db::VoxDb;
use vox_db::web_cache::{CachedWebArtifact, hash_url};

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
    let item = retrieved.unwrap();
    assert_eq!(item.url, artifact.url);
    assert_eq!(item.url_hash, hash_url(&artifact.url));
    assert_eq!(item.etag, artifact.etag);
    assert_eq!(item.last_modified, artifact.last_modified);
    assert_eq!(item.status_code, 200);
    assert_eq!(item.content_type, artifact.content_type);
    assert_eq!(item.raw_body, artifact.raw_body);
    assert_eq!(item.extracted_markdown, artifact.extracted_markdown);
    assert_eq!(item.fetched_at_ms, 1000);

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

#[tokio::test]
async fn test_web_cache_miss_returns_none() {
    let db = VoxDb::in_memory().await.expect("in-memory db init");
    let result = db
        .get_cached_web_artifact("https://example.com/nonexistent")
        .await
        .expect("get non-existent cache");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_web_cache_trailing_slash_normalization() {
    let db = VoxDb::in_memory().await.expect("in-memory db init");

    let artifact = CachedWebArtifact {
        url_hash: String::new(),
        url: "https://example.com/docs/api/".to_string(),
        etag: Some("\"etag123\"".to_string()),
        last_modified: None,
        status_code: 200,
        content_type: "text/html".to_string(),
        raw_body: b"API Documentation".to_vec(),
        extracted_markdown: "# API Documentation".to_string(),
        fetched_at_ms: 5000,
    };

    db.put_cached_web_artifact(&artifact)
        .await
        .expect("put cache with trailing slash");

    // Retrieve using url without trailing slash
    let retrieved_no_slash = db
        .get_cached_web_artifact("https://example.com/docs/api")
        .await
        .expect("get cache without trailing slash");
    assert!(retrieved_no_slash.is_some());
    assert_eq!(retrieved_no_slash.unwrap().fetched_at_ms, 5000);

    // Retrieve using url with trailing slash
    let retrieved_slash = db
        .get_cached_web_artifact("https://example.com/docs/api/")
        .await
        .expect("get cache with trailing slash");
    assert!(retrieved_slash.is_some());
    assert_eq!(retrieved_slash.unwrap().fetched_at_ms, 5000);
}

#[tokio::test]
async fn test_web_cache_optional_fields_none() {
    let db = VoxDb::in_memory().await.expect("in-memory db init");

    let artifact = CachedWebArtifact {
        url_hash: String::new(),
        url: "https://example.com/bare".to_string(),
        etag: None,
        last_modified: None,
        status_code: 204,
        content_type: "application/json".to_string(),
        raw_body: vec![],
        extracted_markdown: String::new(),
        fetched_at_ms: 12345,
    };

    db.put_cached_web_artifact(&artifact)
        .await
        .expect("put cache with None fields");

    let retrieved = db
        .get_cached_web_artifact(&artifact.url)
        .await
        .expect("get cache");
    assert!(retrieved.is_some());
    let item = retrieved.unwrap();
    assert!(item.etag.is_none());
    assert!(item.last_modified.is_none());
    assert_eq!(item.status_code, 204);
    assert_eq!(item.raw_body, Vec::<u8>::new());
    assert_eq!(item.extracted_markdown, "");
    assert_eq!(item.fetched_at_ms, 12345);
}

#[tokio::test]
async fn test_web_cache_update_existing_row() {
    let db = VoxDb::in_memory().await.expect("in-memory db init");

    let mut artifact = CachedWebArtifact {
        url_hash: String::new(),
        url: "https://example.com/mutable".to_string(),
        etag: Some("\"v1\"".to_string()),
        last_modified: Some("Mon, 01 Jan 2026 00:00:00 GMT".to_string()),
        status_code: 200,
        content_type: "text/plain".to_string(),
        raw_body: b"Version 1".to_vec(),
        extracted_markdown: "Version 1".to_string(),
        fetched_at_ms: 1000,
    };

    db.put_cached_web_artifact(&artifact)
        .await
        .expect("initial put");

    // Overwrite with Version 2
    artifact.etag = Some("\"v2\"".to_string());
    artifact.raw_body = b"Version 2 Updated".to_vec();
    artifact.extracted_markdown = "Version 2 Updated".to_string();
    artifact.fetched_at_ms = 3000;

    db.put_cached_web_artifact(&artifact)
        .await
        .expect("second put / update");

    let retrieved = db
        .get_cached_web_artifact(&artifact.url)
        .await
        .expect("get cache after update");
    assert!(retrieved.is_some());
    let item = retrieved.unwrap();
    assert_eq!(item.etag, Some("\"v2\"".to_string()));
    assert_eq!(item.raw_body, b"Version 2 Updated".to_vec());
    assert_eq!(item.extracted_markdown, "Version 2 Updated");
    assert_eq!(item.fetched_at_ms, 3000);
}

#[test]
fn test_cached_web_artifact_serde_roundtrip() {
    let artifact = CachedWebArtifact {
        url_hash: "abcd1234".to_string(),
        url: "https://example.com/data".to_string(),
        etag: Some("\"xyz\"".to_string()),
        last_modified: None,
        status_code: 200,
        content_type: "application/json".to_string(),
        raw_body: b"{\"hello\": \"world\"}".to_vec(),
        extracted_markdown: "```json\n{\"hello\": \"world\"}\n```".to_string(),
        fetched_at_ms: 9999,
    };

    let serialized = serde_json::to_string(&artifact).expect("serialize artifact");
    let deserialized: CachedWebArtifact =
        serde_json::from_str(&serialized).expect("deserialize artifact");
    assert_eq!(artifact, deserialized);
}
