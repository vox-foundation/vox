use vox_search::mens_research_subagent::{
    GroundingQuality, build_local_claim_extraction_prompt, evaluate_span_grounding,
    negation_parity_matches, parse_and_ground_claim_triplets, token_sliding_window_overlap,
};

#[test]
fn test_parse_and_ground_claim_triplets_discards_hallucinations() {
    let source = "In Tokio 1.0, delay_for was renamed to sleep for naming consistency.";
    let json_text = r#"
    {
        "claims": [
            {
                "subject": "tokio::time::sleep",
                "predicate": "replaces",
                "object": "tokio::time::delay_for",
                "confidence": 0.95,
                "evidence_snippet": "delay_for was renamed to sleep"
            },
            {
                "subject": "tokio",
                "predicate": "supports",
                "object": "hallucinated_feature",
                "confidence": 0.80,
                "evidence_snippet": "this snippet does not exist in source text"
            }
        ]
    }
    "#;

    let claims = parse_and_ground_claim_triplets(json_text, source);
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].subject, "tokio::time::sleep");
    assert_eq!(claims[0].predicate, "replaces");
    assert_eq!(claims[0].object, "tokio::time::delay_for");
    assert!((claims[0].confidence - 0.95).abs() < f64::EPSILON);
    assert_eq!(claims[0].evidence_snippet, "delay_for was renamed to sleep");
}

#[test]
fn test_parse_and_ground_claim_triplets_empty_snippet_rejected() {
    let source = "In Tokio 1.0, delay_for was renamed to sleep for naming consistency.";
    let json_text = r#"
    {
        "claims": [
            {
                "subject": "tokio::time::sleep",
                "predicate": "replaces",
                "object": "tokio::time::delay_for",
                "confidence": 0.95,
                "evidence_snippet": ""
            },
            {
                "subject": "tokio::time::sleep",
                "predicate": "replaces",
                "object": "tokio::time::delay_for",
                "confidence": 0.95,
                "evidence_snippet": "   "
            },
            {
                "subject": "tokio::time::sleep",
                "predicate": "replaces",
                "object": "tokio::time::delay_for",
                "confidence": 0.95
            }
        ]
    }
    "#;

    let claims = parse_and_ground_claim_triplets(json_text, source);
    assert!(
        claims.is_empty(),
        "Empty or missing snippets must be rejected"
    );
}

#[test]
fn test_parse_and_ground_claim_triplets_markdown_fenced_json() {
    let source = "Vox compiler uses incremental compilation with content-addressed cache.";
    let json_text = r#"Here is the extracted information:
```json
{
    "claims": [
        {
            "subject": "vox compiler",
            "predicate": "uses",
            "object": "incremental compilation",
            "confidence": 0.98,
            "evidence_snippet": "incremental compilation"
        }
    ]
}
```
Hope this is helpful!
"#;

    let claims = parse_and_ground_claim_triplets(json_text, source);
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].subject, "vox compiler");
    assert_eq!(claims[0].predicate, "uses");
    assert_eq!(claims[0].object, "incremental compilation");
    assert_eq!(claims[0].evidence_snippet, "incremental compilation");
}

#[test]
fn test_parse_and_ground_claim_triplets_array_vs_object_shape() {
    let source = "Rust 2024 edition introduces new prelude items.";
    let object_json = r#"
    {
        "claims": [
            {
                "subject": "Rust 2024",
                "predicate": "introduces",
                "object": "new prelude items",
                "confidence": 0.9,
                "evidence_snippet": "Rust 2024 edition introduces new prelude items"
            }
        ]
    }
    "#;
    let array_json = r#"
    [
        {
            "subject": "Rust 2024",
            "predicate": "introduces",
            "object": "new prelude items",
            "confidence": 0.9,
            "evidence_snippet": "new prelude items"
        }
    ]
    "#;

    let from_obj = parse_and_ground_claim_triplets(object_json, source);
    let from_arr = parse_and_ground_claim_triplets(array_json, source);

    assert_eq!(from_obj.len(), 1);
    assert_eq!(from_arr.len(), 1);
    assert_eq!(from_obj[0].subject, "Rust 2024");
    assert_eq!(from_arr[0].subject, "Rust 2024");
    assert_eq!(from_obj[0].predicate, "introduces");
    assert_eq!(from_arr[0].predicate, "introduces");
}

#[test]
fn test_build_local_claim_extraction_prompt() {
    let prompt = build_local_claim_extraction_prompt("Some sample evidence text");
    assert!(prompt.contains("Some sample evidence text"));
    assert!(prompt.contains("evidence_snippet"));
    assert!(prompt.contains("epistemic triplets"));
}

#[test]
fn test_graduated_grounding_normalized_whitespace_and_punctuation() {
    let source = "In benchmarks, connection pooling reduced p99 latency by 35%—an unprecedented improvement.";
    let raw_json = r#"{"claims": [{"subject": "connection pooling", "predicate": "reduced", "object": "p99 latency by 35%", "confidence": 0.95, "evidence_snippet": "connection pooling reduced p99 latency by 35% - an unprecedented improvement"}]}"#;

    let claims = parse_and_ground_claim_triplets(raw_json, source);
    assert_eq!(
        claims.len(),
        1,
        "Normalized punctuation/dashes must not be discarded"
    );
    assert!(matches!(
        claims[0].grounding,
        GroundingQuality::NormalizedSpan { .. } | GroundingQuality::VerbatimExact
    ));
}

#[test]
fn test_graduated_grounding_coreference_high_token_overlap() {
    let source = "SQLite added JSONB in version 3.45. It provides 3x faster reads.";
    let raw_json = r#"{"claims": [{"subject": "SQLite JSONB", "predicate": "provides", "object": "3x faster reads", "confidence": 0.90, "evidence_snippet": "SQLite JSONB provides 3x faster reads"}]}"#;

    let claims = parse_and_ground_claim_triplets(raw_json, source);
    assert_eq!(
        claims.len(),
        1,
        "Coreference-expanded snippet must be preserved"
    );
    if let GroundingQuality::NormalizedSpan { overlap_ratio } = claims[0].grounding {
        assert!(
            overlap_ratio >= 0.80,
            "Overlap ratio must meet or exceed 0.80"
        );
    } else {
        panic!("Expected NormalizedSpan grounding");
    }
}

#[test]
fn test_negation_inversion_hard_rejected() {
    let source = "The microbenchmark did not reduce latency across threads.";
    // Snippet inverts claim by omitting "not"
    let snippet = "The microbenchmark did reduce latency across threads.";
    assert!(
        !negation_parity_matches(snippet, source),
        "Negation parity must detect flipped polarity"
    );
    assert_eq!(
        evaluate_span_grounding(snippet, source),
        None,
        "Inverted negation must be rejected"
    );
}

#[test]
fn test_scattered_tokens_across_long_document_rejected() {
    let source = "Tokio is an async runtime. SQLite provides embedded relational storage. Linux supports epoll.";
    // Snippet combines words scattered across separate sentences
    let snippet = "Tokio provides embedded relational epoll";
    assert_eq!(
        evaluate_span_grounding(snippet, source),
        None,
        "Scattered unwindowed words must not match"
    );
}

#[test]
fn test_ungrounded_hallucination_discarded() {
    let source = "Rust compiler version 1.85 stabilized async closures.";
    let raw_json = r#"{"claims": [{"subject": "Go runtime", "predicate": "introduced", "object": "generics", "confidence": 0.90, "evidence_snippet": "Go 1.18 added full generics support"}]}"#;

    let claims = parse_and_ground_claim_triplets(raw_json, source);
    assert!(
        claims.is_empty(),
        "Completely hallucinated claim must be discarded"
    );
}

#[test]
fn test_token_sliding_window_overlap_direct() {
    let source = "sqlite added jsonb in version 3 45 it provides 3x faster reads";
    let snippet = "sqlite jsonb provides 3x faster reads";
    let overlap = token_sliding_window_overlap(snippet, source);
    assert!(overlap.is_some());
    let ratio = overlap.unwrap();
    assert!(ratio > 0.80);
}
