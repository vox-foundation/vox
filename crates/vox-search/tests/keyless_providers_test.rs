use std::collections::HashMap;
use vox_search::arxiv::{ArXivClient, normalize_arxiv_url};
use vox_search::openalex::{OpenAlexClient, reconstruct_abstract};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_openalex_abstract_reconstruction_bounds() {
    let mut inverted = HashMap::new();
    inverted.insert("Rust".to_string(), vec![0]);
    inverted.insert("memory".to_string(), vec![1]);
    inverted.insert("safety.".to_string(), vec![2]);

    let reconstructed = reconstruct_abstract(&inverted, 500);
    assert_eq!(reconstructed, "Rust memory safety.");

    let truncated = reconstruct_abstract(&inverted, 6);
    assert!(truncated.ends_with("..."));

    let empty = reconstruct_abstract(&inverted, 0);
    assert_eq!(empty, "");

    let two_chars = reconstruct_abstract(&inverted, 2);
    assert_eq!(two_chars, "Ru");

    let three_chars = reconstruct_abstract(&inverted, 3);
    assert_eq!(three_chars, "...");
}

#[test]
fn test_arxiv_atom_parsing_isolated_entries() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title type="html">arXiv Query: search_query=all:Rust</title>
  <entry>
    <id>http://arxiv.org/abs/2206.05503v1</id>
    <title>Rust: Safety and Performance</title>
    <summary>A study on Rust memory safety without garbage collection.</summary>
    <link href="http://arxiv.org/abs/2206.05503v1" rel="alternate" type="text/html"/>
    <link title="pdf" href="http://arxiv.org/pdf/2206.05503v1" rel="related" type="application/pdf"/>
  </entry>
</feed>"#;

    let hits = ArXivClient::parse_atom_xml(xml, 5).expect("parse xml");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Rust: Safety and Performance");
    assert_eq!(
        hits[0].content,
        "A study on Rust memory safety without garbage collection."
    );
    assert_eq!(hits[0].url, "https://arxiv.org/abs/2206.05503");
    assert_eq!(hits[0].engine.as_deref(), Some("arxiv"));
}

#[test]
fn test_openalex_parse_search_json_url_priority() {
    // Priority 1: open_access.oa_url
    let json_oa = r#"{
        "results": [
            {
                "id": "https://openalex.org/W1",
                "doi": "https://doi.org/10.1234/w1",
                "title": "Paper 1",
                "abstract_inverted_index": { "Paper": [0], "one": [1] },
                "primary_location": { "landing_page_url": "https://publisher.example/w1" },
                "open_access": { "oa_url": "https://openaccess.example/w1.pdf" }
            }
        ]
    }"#;
    let hits = OpenAlexClient::parse_search_json(json_oa, 5).expect("parse json");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].url, "https://openaccess.example/w1.pdf");
    assert_eq!(hits[0].title, "Paper 1");
    assert_eq!(hits[0].content, "Paper one");
    assert_eq!(hits[0].engine.as_deref(), Some("openalex"));

    // Priority 2: primary_location.landing_page_url when oa_url is missing
    let json_landing = r#"{
        "results": [
            {
                "id": "https://openalex.org/W2",
                "doi": "https://doi.org/10.1234/w2",
                "title": "Paper 2",
                "primary_location": { "landing_page_url": "https://publisher.example/w2" }
            }
        ]
    }"#;
    let hits = OpenAlexClient::parse_search_json(json_landing, 5).expect("parse json");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].url, "https://publisher.example/w2");

    // Priority 3: doi when primary_location is missing
    let json_doi = r#"{
        "results": [
            {
                "id": "https://openalex.org/W3",
                "doi": "https://doi.org/10.1234/w3",
                "title": "Paper 3"
            }
        ]
    }"#;
    let hits = OpenAlexClient::parse_search_json(json_doi, 5).expect("parse json");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].url, "https://doi.org/10.1234/w3");

    // Priority 4: id when doi is missing
    let json_id = r#"{
        "results": [
            {
                "id": "https://openalex.org/W4",
                "title": "Paper 4"
            }
        ]
    }"#;
    let hits = OpenAlexClient::parse_search_json(json_id, 5).expect("parse json");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].url, "https://openalex.org/W4");
}

#[test]
fn test_arxiv_atom_parsing_entities_and_whitespace() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <id>http://arxiv.org/abs/2101.12345v2</id>
    <title>
      Fast &amp; Accurate: &lt;Machine&gt;
      Learning
    </title>
    <summary>
      Deep neural networks &quot;learn&quot;   representations.
      New line here.
    </summary>
  </entry>
</feed>"#;

    let hits = ArXivClient::parse_atom_xml(xml, 5).expect("parse xml");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Fast & Accurate: <Machine> Learning");
    assert_eq!(
        hits[0].content,
        "Deep neural networks \"learn\" representations. New line here."
    );
    assert_eq!(hits[0].url, "https://arxiv.org/abs/2101.12345");
}

#[test]
fn test_arxiv_url_normalization() {
    assert_eq!(
        normalize_arxiv_url("http://arxiv.org/abs/2206.05503v1"),
        "https://arxiv.org/abs/2206.05503"
    );
    assert_eq!(
        normalize_arxiv_url("https://arxiv.org/abs/2206.05503v12"),
        "https://arxiv.org/abs/2206.05503"
    );
    assert_eq!(
        normalize_arxiv_url("http://arxiv.org/pdf/2206.05503v1.pdf"),
        "https://arxiv.org/abs/2206.05503"
    );
    assert_eq!(
        normalize_arxiv_url("2206.05503v3"),
        "https://arxiv.org/abs/2206.05503"
    );
    assert_eq!(
        normalize_arxiv_url("https://arxiv.org/abs/math.PR/0001001v2"),
        "https://arxiv.org/abs/math.PR/0001001"
    );
    assert_eq!(
        normalize_arxiv_url("https://arxiv.org/abs/2206.05503"),
        "https://arxiv.org/abs/2206.05503"
    );
    assert_eq!(normalize_arxiv_url(""), "");
}

#[tokio::test]
async fn test_openalex_client_search_with_mock() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/works"))
        .and(query_param("search", "quantum computing"))
        .and(query_param("per-page", "3"))
        .and(query_param("mailto", "research@vox.computer"))
        .and(header("api-key", "sec_123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {
                    "id": "https://openalex.org/W99",
                    "title": "Quantum Supremacy",
                    "abstract_inverted_index": {
                        "Quantum": [0],
                        "advantage": [1],
                        "demonstrated.": [2]
                    },
                    "primary_location": {
                        "landing_page_url": "https://nature.example/articles/s41586"
                    }
                }
            ]
        })))
        .mount(&mock)
        .await;

    let hits = OpenAlexClient::search("quantum computing", 3, Some(&mock.uri()), Some("sec_123"))
        .await
        .expect("openalex search");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Quantum Supremacy");
    assert_eq!(hits[0].content, "Quantum advantage demonstrated.");
    assert_eq!(hits[0].url, "https://nature.example/articles/s41586");
    assert_eq!(hits[0].engine.as_deref(), Some("openalex"));
}

#[tokio::test]
async fn test_arxiv_client_search_with_mock() {
    let mock = MockServer::start().await;
    let xml_response = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title type="html">arXiv Query</title>
  <entry>
    <id>http://arxiv.org/abs/2301.00001v1</id>
    <title>Transformer Decoders at Scale</title>
    <summary>Scaling laws for language modeling architectures.</summary>
    <link href="http://arxiv.org/abs/2301.00001v1" rel="alternate"/>
  </entry>
</feed>"#;

    Mock::given(method("GET"))
        .and(path("/api/query"))
        .and(query_param("search_query", "all:transformers"))
        .and(query_param("max_results", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(xml_response, "application/atom+xml"))
        .mount(&mock)
        .await;

    let hits = ArXivClient::search("transformers", 2, Some(&mock.uri()))
        .await
        .expect("arxiv search");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Transformer Decoders at Scale");
    assert_eq!(
        hits[0].content,
        "Scaling laws for language modeling architectures."
    );
    assert_eq!(hits[0].url, "https://arxiv.org/abs/2301.00001");
    assert_eq!(hits[0].engine.as_deref(), Some("arxiv"));
}

#[tokio::test]
async fn test_search_blank_or_zero_limit_short_circuits() {
    let hits_openalex_blank = OpenAlexClient::search("   ", 5, None, None).await.unwrap();
    assert!(hits_openalex_blank.is_empty());
    let hits_openalex_zero = OpenAlexClient::search("test", 0, None, None).await.unwrap();
    assert!(hits_openalex_zero.is_empty());

    let hits_arxiv_blank = ArXivClient::search("   ", 5, None).await.unwrap();
    assert!(hits_arxiv_blank.is_empty());
    let hits_arxiv_zero = ArXivClient::search("test", 0, None).await.unwrap();
    assert!(hits_arxiv_zero.is_empty());
}
