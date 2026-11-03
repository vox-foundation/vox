//! 4-Tier Dynamic Escalation Classifier for autonomous research requests.
//!
//! Evaluates incoming research queries across 4 progressive tiers:
//! - Tier 0: Instant VoxDB FTS5 memory lookup (<50ms, 0 LLM tokens) with Jaccard >= 0.85 and volatility freshness gates.
//! - Tier 1: Shallow web lookup (<1.5s, single query) for short factual questions.
//! - Tier 2: Standard single-wave deep research with claim verification.
//! - Tier 3: Multi-wave autonomous deep research with contradiction isolation and empirical sandboxing.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Target tier determined by research query classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchTriageTier {
    InstantMemory {
        session_id: i64,
        cached_query: String,
        snippet: String,
        similarity: f64,
    },
    ShallowWeb {
        query: String,
    },
    StandardDeep {
        query: String,
        domain_mode: String,
    },
    MultiWaveAutonomous {
        query: String,
        domain_mode: String,
        max_waves: usize,
    },
}

/// Tokenize text into lowercased alphanumeric word set for similarity calculation.
pub fn tokenize_to_set(text: &str) -> HashSet<String> {
    text.split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|w| w.len() > 1)
        .collect()
}

/// Compute token Jaccard similarity between two strings in [0.0, 1.0].
pub fn compute_jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a = tokenize_to_set(a);
    let set_b = tokenize_to_set(b);
    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }
    if set_a.is_empty() || set_b.is_empty() {
        return 0.0;
    }
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    intersection as f64 / union as f64
}

/// Returns true if query touches rapidly changing information (pricing, releases, dates).
pub fn is_volatile_query(query: &str) -> bool {
    let lower = query.to_ascii_lowercase();
    const VOLATILE_TERMS: &[&str] = &[
        "latest",
        "today",
        "yesterday",
        "current",
        "version",
        "pricing",
        "cost",
        "release",
        "changelog",
        "breaking change",
    ];
    VOLATILE_TERMS.iter().any(|&term| lower.contains(term))
}

/// Returns true if query contains comparative or in-depth keywords that warrant multi-wave analysis.
pub fn is_comparative_or_deep(query: &str) -> bool {
    let lower = query.to_ascii_lowercase();
    const COMPARATIVE_TERMS: &[&str] = &[
        "compare",
        " vs ",
        " vs. ",
        "versus",
        "tradeoff",
        "trade-off",
        "benchmark",
        "in-depth",
        "comprehensive",
        "deep dive",
        "pros and cons",
    ];
    COMPARATIVE_TERMS.iter().any(|&term| lower.contains(term))
}

/// Returns true if query is a succinct factual question suitable for shallow lookup.
pub fn is_shallow_factual(query: &str) -> bool {
    let trimmed = query.trim();
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() >= 14 || words.is_empty() {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    const FACTUAL_PREFIXES: &[&str] = &[
        "what is",
        "who is",
        "when was",
        "where is",
        "default port",
        "define ",
        "meaning of",
    ];
    FACTUAL_PREFIXES.iter().any(|&p| lower.starts_with(p))
}

/// Classifies an incoming research request into an appropriate execution tier.
pub async fn classify_research_request(
    query: &str,
    domain_mode: Option<&str>,
    explicit_deep: bool,
    explicit_waves: Option<usize>,
    db: Option<&vox_db::VoxDb>,
) -> ResearchTriageTier {
    let mode = domain_mode.unwrap_or("general");

    // 1. Explicit multi-wave or domain triggers
    if explicit_deep || explicit_waves.unwrap_or(1) > 1 {
        return ResearchTriageTier::MultiWaveAutonomous {
            query: query.to_string(),
            domain_mode: mode.to_string(),
            max_waves: explicit_waves.unwrap_or(3).max(2),
        };
    }

    if mode == "codegen" || mode == "shopping" {
        return ResearchTriageTier::MultiWaveAutonomous {
            query: query.to_string(),
            domain_mode: mode.to_string(),
            max_waves: 3,
        };
    }

    if is_comparative_or_deep(query) {
        return ResearchTriageTier::MultiWaveAutonomous {
            query: query.to_string(),
            domain_mode: mode.to_string(),
            max_waves: 2,
        };
    }

    // 2. Tier 0: Instant VoxDB FTS5 memory lookup
    if let Some(voxdb) = db {
        if let Ok(hits) = voxdb.search_research_artifacts(query, 5).await {
            let now_ms = chrono::Utc::now().timestamp_millis();
            for hit in hits {
                let similarity = compute_jaccard_similarity(query, &hit.query_text);
                if similarity >= 0.85 {
                    let age_ms = now_ms.saturating_sub(hit.created_at_ms);
                    let max_age_ms = if is_volatile_query(query) {
                        24 * 3600 * 1000 // 24 hours for volatile queries
                    } else {
                        14 * 24 * 3600 * 1000 // 14 days for stable architectural queries
                    };

                    if age_ms <= max_age_ms {
                        return ResearchTriageTier::InstantMemory {
                            session_id: hit.session_id,
                            cached_query: hit.query_text,
                            snippet: hit.snippet,
                            similarity,
                        };
                    }
                }
            }
        }
    }

    // 3. Tier 1: Shallow web lookup
    if is_shallow_factual(query) {
        return ResearchTriageTier::ShallowWeb {
            query: query.to_string(),
        };
    }

    // 4. Tier 2: Standard Deep Research
    ResearchTriageTier::StandardDeep {
        query: query.to_string(),
        domain_mode: mode.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_similarity_calculation() {
        assert_eq!(
            compute_jaccard_similarity("what is rust", "what is rust"),
            1.0
        );
        assert_eq!(
            compute_jaccard_similarity("tokio async runtime", "async runtime tokio"),
            1.0
        );
        let sim = compute_jaccard_similarity("best rust database sqlite", "rust database postgres");
        assert!(sim > 0.3 && sim < 0.8);
    }

    #[test]
    fn test_volatile_and_comparative_classification() {
        assert!(is_volatile_query("What is the latest version of tokio?"));
        assert!(is_volatile_query("pricing for claude 3.5 sonnet"));
        assert!(!is_volatile_query(
            "How does the Raft consensus algorithm work?"
        ));

        assert!(is_comparative_or_deep(
            "Compare SQLite vs RocksDB for embedded storage"
        ));
        assert!(is_comparative_or_deep("Postgres vs MySQL tradeoffs"));
        assert!(!is_comparative_or_deep("What is SQLite?"));
    }

    #[test]
    fn test_shallow_factual_classification() {
        assert!(is_shallow_factual("what is DNS?"));
        assert!(is_shallow_factual("default port for postgresql"));
        assert!(!is_shallow_factual(
            "what is DNS and how does root name server recursive resolution work across all continents?"
        )); // >= 14 words
    }

    #[tokio::test]
    async fn test_classify_escalation_rules() {
        // Comparative triggers MultiWaveAutonomous
        let t1 = classify_research_request(
            "Compare React vs Vue for dashboard",
            None,
            false,
            None,
            None,
        )
        .await;
        assert!(matches!(
            t1,
            ResearchTriageTier::MultiWaveAutonomous { max_waves: 2, .. }
        ));

        // CodeGen triggers MultiWaveAutonomous
        let t2 = classify_research_request(
            "tokio tcp listener syntax",
            Some("codegen"),
            false,
            None,
            None,
        )
        .await;
        assert!(matches!(
            t2,
            ResearchTriageTier::MultiWaveAutonomous { max_waves: 3, .. }
        ));

        // Shallow factual triggers ShallowWeb
        let t3 = classify_research_request("what is vox?", None, false, None, None).await;
        assert!(matches!(t3, ResearchTriageTier::ShallowWeb { .. }));

        // General query triggers StandardDeep
        let t4 = classify_research_request(
            "Explain distributed transaction coordination algorithms in database systems",
            None,
            false,
            None,
            None,
        )
        .await;
        assert!(matches!(t4, ResearchTriageTier::StandardDeep { .. }));
    }
}
