use url::Url;
use vox_search::crawler::{
    extract_candidate_links, normalize_crawl_url, score_and_prioritize_links,
};

#[test]
fn test_normalize_crawl_url_strips_fragments_and_tracking_queries() {
    let raw = Url::parse("https://docs.rs/tokio/1.38.0/tokio/sync/struct.Mutex.html?utm_source=feed&ref=tracker#method.lock").unwrap();
    let normalized = normalize_crawl_url(&raw);
    assert_eq!(
        normalized.as_str(),
        "https://docs.rs/tokio/1.38.0/tokio/sync/struct.Mutex.html"
    );
}

#[test]
fn test_extract_candidate_links_preserves_api_items() {
    let html = r#"
        <html>
            <body>
                <a href="/tokio/sync/struct.Mutex.html">Mutex</a>
                <a href="/tokio/time/fn.sleep.html">sleep</a>
                <a href="https://external.com/ad">External</a>
                <a href="/tokio/asset.png">Image</a>
            </body>
        </html>
    "#;
    let base_url = "https://docs.rs/tokio/1.38.0";
    let allow_origin = "https://docs.rs";

    let links = extract_candidate_links(html, base_url, allow_origin);
    assert_eq!(links.len(), 2);
    assert!(links.contains(&"https://docs.rs/tokio/sync/struct.Mutex.html".to_string()));
    assert!(links.contains(&"https://docs.rs/tokio/time/fn.sleep.html".to_string()));
}

#[test]
fn test_score_and_prioritize_links_orders_api_higher_than_generic() {
    let links = vec![
        "https://docs.rs/tokio/about".to_string(),
        "https://docs.rs/tokio/sync/struct.Mutex.html".to_string(),
        "https://docs.rs/tokio/guide".to_string(),
    ];
    let scored = score_and_prioritize_links(&links);
    assert_eq!(scored[0].0, "https://docs.rs/tokio/sync/struct.Mutex.html");
    assert!(scored[0].1 > scored[1].1);
}
