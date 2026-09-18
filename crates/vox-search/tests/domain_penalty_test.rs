use vox_search::policy::SearchPolicy;
use vox_search::searxng::SearxngResult;
use vox_search::web_dispatcher::{WebSearchDispatcher, extract_registrable_domain};

#[test]
fn test_blacklisted_domain_results_are_pruned() {
    let mut policy = SearchPolicy::default();
    policy
        .blacklisted_domains
        .insert("malicious-docs.com".to_string());

    let mut results = vec![
        SearxngResult {
            url: "https://malicious-docs.com/guide".into(),
            title: "Bad guide".into(),
            content: "...".into(), // AMENDED #5: uses content, not snippet
            score: Some(1.0),
            engine: Some("searxng".into()),
        },
        SearxngResult {
            url: "https://docs.rs/tokio".into(),
            title: "Tokio docs".into(),
            content: "...".into(),
            score: Some(0.8),
            engine: Some("searxng".into()),
        },
    ];

    WebSearchDispatcher::filter_and_penalize_results(&mut results, &policy);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://docs.rs/tokio");
}

#[test]
fn test_domain_penalty_scales_score() {
    let mut policy = SearchPolicy::default();
    policy
        .domain_penalties
        .insert("unreliable-blog.org".to_string(), 0.5);

    let mut results = vec![
        SearxngResult {
            url: "https://www.unreliable-blog.org/article".into(),
            title: "Article".into(),
            content: "...".into(),
            score: Some(1.0),
            engine: Some("searxng".into()),
        },
        SearxngResult {
            url: "https://docs.rs/tokio".into(),
            title: "Tokio docs".into(),
            content: "...".into(),
            score: Some(0.8),
            engine: Some("searxng".into()),
        },
    ];

    WebSearchDispatcher::filter_and_penalize_results(&mut results, &policy);
    assert_eq!(results.len(), 2);
    let penalized = &results[0];
    assert!(
        (penalized.score.unwrap() - 0.5).abs() < 1e-6,
        "Expected score to be scaled by 0.5, got {:?}",
        penalized.score
    );
    let unpenalized = &results[1];
    assert!(
        (unpenalized.score.unwrap() - 0.8).abs() < 1e-6,
        "Expected unpenalized score to remain 0.8, got {:?}",
        unpenalized.score
    );
}

#[test]
fn test_extract_registrable_domain() {
    assert_eq!(
        extract_registrable_domain("https://www.example.com/path"),
        Some("example.com".to_string())
    );
    assert_eq!(
        extract_registrable_domain("http://sub.domain.org:8080/foo"),
        Some("sub.domain.org".to_string())
    );
    assert_eq!(
        extract_registrable_domain("https://MALICIOUS-DOCS.COM/guide"),
        Some("malicious-docs.com".to_string())
    );
}

#[test]
fn test_domain_penalty_clamping_floor() {
    let mut policy = SearchPolicy::default();
    policy
        .domain_penalties
        .insert("completely-broken.org".to_string(), 1.0);

    let mut results = vec![SearxngResult {
        url: "https://completely-broken.org/guide".into(),
        title: "Broken".into(),
        content: "...".into(),
        score: Some(1.0),
        engine: Some("searxng".into()),
    }];

    WebSearchDispatcher::filter_and_penalize_results(&mut results, &policy);
    assert_eq!(results.len(), 1);
    assert!((results[0].score.unwrap() - 0.05).abs() < 1e-6);
}
