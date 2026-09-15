use vox_search::term_density_reranker::{
    CandidatePassage, rerank_passages, score_passage_term_density,
};

#[test]
fn test_score_passage_term_density_scoring() {
    let query_terms = &["metal", "candle", "unified", "memory"];
    let relevant_passage = "Using candle-metal with Unified Memory allows zero-copy tensor sharing between CPU and GPU.";
    let irrelevant_passage = "A recipe for cooking pasta in boiling water with salt.";

    let score_high = score_passage_term_density(query_terms, relevant_passage);
    let score_low = score_passage_term_density(query_terms, irrelevant_passage);

    assert!(score_high > score_low);
    assert!(score_high > 0.5);
    assert_eq!(score_low, 0.0);

    // Empty queries test
    assert_eq!(score_passage_term_density(&[], relevant_passage), 0.0);
    assert_eq!(score_passage_term_density(&["a"], relevant_passage), 0.0);
}

#[test]
fn test_rerank_passages() {
    let passages = vec![
        CandidatePassage {
            id: "p1".to_string(),
            text: "General overview of programming languages.".to_string(),
            base_score: 0.9,
        },
        CandidatePassage {
            id: "p2".to_string(),
            text: "Rust memory management and zero-copy safety.".to_string(),
            base_score: 0.4,
        },
        CandidatePassage {
            id: "p3".to_string(),
            text: "Baking bread with yeast and flour.".to_string(),
            base_score: 0.1,
        },
    ];

    let query = "Rust zero-copy memory";
    let reranked = rerank_passages(query, &passages, 2);

    assert_eq!(reranked.len(), 2);
    // p2 should be boosted to top because of high term density despite lower base_score
    assert_eq!(reranked[0].id, "p2");
    assert_eq!(reranked[1].id, "p1");
}

#[test]
fn test_candidate_passage_serde() {
    let passage = CandidatePassage {
        id: "p1".to_string(),
        text: "Passage test".to_string(),
        base_score: 0.85,
    };
    let json = serde_json::to_string(&passage).expect("serialization should succeed");
    let deserialized: CandidatePassage =
        serde_json::from_str(&json).expect("deserialization should succeed");
    assert_eq!(passage, deserialized);
}
