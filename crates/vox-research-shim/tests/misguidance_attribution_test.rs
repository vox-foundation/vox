use vox_research_shim::research::misguidance::correlate_diagnostic_to_citations;
use vox_research_shim::research::types::Citation;

#[test]
fn test_correlates_compile_error_to_culprit_citation() {
    let citations = vec![
        Citation {
            source_id: 1,
            url: "https://docs.rs/good/latest".into(),
            title: "Good docs".into(),
            snippet: "fn healthy_api()".into(),
            confidence: 0.9,
        },
        Citation {
            source_id: 2,
            url: "https://obsolete-blog.com/tips".into(),
            title: "Broken tips".into(),
            snippet: "call legacy_unstable_feature() in your code".into(),
            confidence: 0.6,
        },
    ];

    let error_log = "error[E0425]: cannot find function `legacy_unstable_feature` in module";
    let culprit = correlate_diagnostic_to_citations(error_log, &citations);
    assert_eq!(culprit.as_deref(), Some("https://obsolete-blog.com/tips"));
}

#[test]
fn test_returns_none_when_no_overlap() {
    let citations = vec![Citation {
        source_id: 1,
        url: "https://docs.rs/unrelated".into(),
        title: "Unrelated Crate".into(),
        snippet: "completely different text and tokens".into(),
        confidence: 0.9,
    }];

    let error_log = "error[E0425]: cannot find function `legacy_unstable_feature` in module";
    let culprit = correlate_diagnostic_to_citations(error_log, &citations);
    assert_eq!(culprit, None);
}

#[test]
fn test_selects_citation_with_highest_overlap() {
    let citations = vec![
        Citation {
            source_id: 1,
            url: "https://example.com/one-match".into(),
            title: "Article".into(),
            snippet: "legacy_unstable_feature is here".into(),
            confidence: 0.8,
        },
        Citation {
            source_id: 2,
            url: "https://example.com/two-matches".into(),
            title: "Deprecated module guide".into(),
            snippet: "legacy_unstable_feature function deprecated".into(),
            confidence: 0.8,
        },
    ];

    // diagnostic has: legacy_unstable_feature, function, module
    // citation 1 overlaps: legacy_unstable_feature (1)
    // citation 2 overlaps: legacy_unstable_feature, function, module (3)
    let error_log = "error[E0425]: cannot find function `legacy_unstable_feature` in module";
    let culprit = correlate_diagnostic_to_citations(error_log, &citations);
    assert_eq!(culprit.as_deref(), Some("https://example.com/two-matches"));
}

#[test]
fn test_short_tokens_under_4_chars_do_not_match() {
    let citations = vec![Citation {
        source_id: 1,
        url: "https://example.com/short".into(),
        title: "Bar".into(),
        snippet: "foo bar baz".into(),
        confidence: 0.8,
    }];

    let error_log = "error: foo bar baz";
    let culprit = correlate_diagnostic_to_citations(error_log, &citations);
    assert_eq!(culprit, None);
}
