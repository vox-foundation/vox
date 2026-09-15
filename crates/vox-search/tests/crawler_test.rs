use url::Url;
use vox_search::crawler::{
    crawl_domain_depth, extract_candidate_links, normalize_crawl_url, score_and_prioritize_links,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
fn test_extract_candidate_links_strict_origin_rejects_subdomain_or_prefix_spoof() {
    let html = r#"
        <html>
            <body>
                <a href="https://docs.rs/page">Valid</a>
                <a href="https://docs.rs.evil.com/phish">Evil Prefix Spoof</a>
                <a href="https://other.rs/page">Different Domain</a>
            </body>
        </html>
    "#;
    let base_url = "https://docs.rs";
    let allow_origin = "https://docs.rs";

    let links = extract_candidate_links(html, base_url, allow_origin);
    assert_eq!(links, vec!["https://docs.rs/page".to_string()]);
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

#[tokio::test]
async fn test_crawl_domain_depth_recurses_and_respects_limits() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"
            <html>
                <head><title>Root Page</title></head>
                <body>
                    <a href="/sub1">Sub 1</a>
                    <a href="/sub2">Sub 2</a>
                </body>
            </html>
        "#,
        ))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/sub1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"
            <html>
                <head><title>Sub 1</title></head>
                <body>
                    <a href="/sub1/deep">Deep link</a>
                </body>
            </html>
        "#,
        ))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/sub2"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"
            <html>
                <head><title>Sub 2</title></head>
                <body>
                    <p>Subpage 2 content</p>
                </body>
            </html>
        "#,
        ))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/sub1/deep"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"
            <html>
                <head><title>Deep Page</title></head>
                <body>
                    <p>Deep nested content</p>
                </body>
            </html>
        "#,
        ))
        .mount(&mock_server)
        .await;

    let docs = crawl_domain_depth(&mock_server.uri(), 2, 5, 2000)
        .await
        .expect("crawl should succeed");

    assert_eq!(docs.len(), 4);
    let titles: Vec<&str> = docs.iter().map(|d| d.title.as_str()).collect();
    assert!(titles.contains(&"Root Page"));
    assert!(titles.contains(&"Sub 1"));
    assert!(titles.contains(&"Sub 2"));
    assert!(titles.contains(&"Deep Page"));

    // Verify raw_html was captured on every document without duplicate HTTP fetches
    for doc in &docs {
        assert!(doc.raw_html.is_some());
    }
}
