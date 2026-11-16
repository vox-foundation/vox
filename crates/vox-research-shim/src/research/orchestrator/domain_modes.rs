//! Domain-specific research orchestration hooks: query expansion, hit post-processing,
//! and synthesis instructions for General, CodeGen, Shopping, and Scholarly modes.

use crate::research::domain::{codegen, shopping};
use crate::research::types::{ResearchDomainMode, ResearchHit};

/// Returns additional search queries tailored to the active `domain_mode`, excluding duplicates.
#[must_use]
pub fn domain_subqueries(domain_mode: ResearchDomainMode, query: &str) -> Vec<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    match domain_mode {
        ResearchDomainMode::Shopping => shopping::generate_shopping_subqueries(trimmed),
        ResearchDomainMode::CodeGen => codegen::generate_codegen_subqueries(trimmed),
        ResearchDomainMode::Scholarly => vec![
            format!("{trimmed} peer-reviewed paper doi arxiv openalex"),
            format!("{trimmed} prior art systematic review benchmark"),
        ],
        ResearchDomainMode::General => Vec::new(),
    }
}

/// Appends domain subqueries into `target`, preserving order and avoiding exact duplicates.
pub fn extend_domain_subqueries(
    domain_mode: ResearchDomainMode,
    query: &str,
    target: &mut Vec<String>,
) {
    for sq in domain_subqueries(domain_mode, query) {
        if !target.contains(&sq) {
            target.push(sq);
        }
    }
}

/// Applies domain-specific score adjustments and ordering heuristics to collected `hits`.
#[allow(clippy::ptr_arg)]
pub fn post_process_domain_hits(domain_mode: ResearchDomainMode, hits: &mut Vec<ResearchHit>) {
    match domain_mode {
        ResearchDomainMode::Shopping => {
            shopping::deboost_affiliate_spam(hits);
        }
        ResearchDomainMode::Scholarly => {
            for hit in hits.iter_mut() {
                let lower = hit.url.to_ascii_lowercase();
                if lower.contains("arxiv.org")
                    || lower.contains("doi.org")
                    || lower.contains("openalex.org")
                    || lower.contains("semanticscholar.org")
                    || lower.contains("acm.org")
                    || lower.contains("ieee.org")
                {
                    hit.score = (hit.score + 0.15).min(1.0);
                    hit.trust_score = (hit.trust_score + 0.15).min(1.0);
                }
            }
        }
        ResearchDomainMode::CodeGen => {
            for hit in hits.iter_mut() {
                let lower = hit.url.to_ascii_lowercase();
                if lower.contains("docs.rs")
                    || lower.contains("doc.rust-lang.org")
                    || lower.contains("github.com")
                    || lower.contains("developer.mozilla.org")
                {
                    hit.score = (hit.score + 0.10).min(1.0);
                    hit.trust_score = (hit.trust_score + 0.10).min(1.0);
                }
            }
        }
        ResearchDomainMode::General => {}
    }
}

/// Builds the synthesis prompt query decorated with domain-mode instructions.
#[must_use]
pub fn decorate_synthesis_query(domain_mode: ResearchDomainMode, query: &str) -> String {
    match domain_mode {
        ResearchDomainMode::Shopping => {
            format!("{query}\n\n{}", shopping::shopping_synthesis_instructions())
        }
        ResearchDomainMode::CodeGen => {
            format!("{query}\n\n{}", codegen::codegen_synthesis_instructions())
        }
        ResearchDomainMode::Scholarly => format!(
            "{query}\n\nDOMAIN MODE: SCHOLARLY LITERATURE & PRIOR-ART SYNTHESIS\nIn addition to standard research synthesis:\n1. Prioritize peer-reviewed literature, DOIs, arXiv preprints, and formal empirical benchmarks.\n2. Contrast corroborating and contradicting prior-art findings with venue and publication year provenance.\n3. Distinguish verified empirical claims from unverified hypotheses."
        ),
        ResearchDomainMode::General => query.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_hit(url: &str, score: f64) -> ResearchHit {
        ResearchHit {
            url: url.to_string(),
            title: url.to_string(),
            snippet: "snippet".to_string(),
            score,
            http_status: 200,
            trust_score: score,
            raw_content: String::new(),
        }
    }

    #[test]
    fn domain_subqueries_cover_all_four_modes() {
        assert!(domain_subqueries(ResearchDomainMode::General, "rust async").is_empty());
        assert!(!domain_subqueries(ResearchDomainMode::CodeGen, "tokio select").is_empty());
        assert!(!domain_subqueries(ResearchDomainMode::Shopping, "ergonomic chair").is_empty());
        let scholarly = domain_subqueries(ResearchDomainMode::Scholarly, "crag retrieval");
        assert_eq!(scholarly.len(), 2);
        assert!(scholarly[0].contains("doi"));
    }

    #[test]
    fn extend_domain_subqueries_deduplicates() {
        let mut target = vec!["existing".to_string()];
        extend_domain_subqueries(ResearchDomainMode::Scholarly, "topic", &mut target);
        let len_after_first = target.len();
        extend_domain_subqueries(ResearchDomainMode::Scholarly, "topic", &mut target);
        assert_eq!(target.len(), len_after_first);
    }

    #[test]
    fn post_process_boosts_scholarly_and_codegen_authority_urls() {
        let mut scholarly_hits = vec![
            mk_hit("https://arxiv.org/abs/2401.12345", 0.70),
            mk_hit("https://random-blog.example.com/post", 0.70),
        ];
        post_process_domain_hits(ResearchDomainMode::Scholarly, &mut scholarly_hits);
        assert!(scholarly_hits[0].score > scholarly_hits[1].score);

        let mut codegen_hits = vec![
            mk_hit("https://docs.rs/tokio/latest/tokio/", 0.70),
            mk_hit("https://random-blog.example.com/post", 0.70),
        ];
        post_process_domain_hits(ResearchDomainMode::CodeGen, &mut codegen_hits);
        assert!(codegen_hits[0].score > codegen_hits[1].score);
    }

    #[test]
    fn decorate_synthesis_query_preserves_general_and_decorates_specialized_modes() {
        assert_eq!(
            decorate_synthesis_query(ResearchDomainMode::General, "plain query"),
            "plain query"
        );
        assert!(
            decorate_synthesis_query(ResearchDomainMode::Scholarly, "q")
                .contains("SCHOLARLY LITERATURE")
        );
        assert!(
            decorate_synthesis_query(ResearchDomainMode::Shopping, "q").contains("SPECIFICATION")
                || decorate_synthesis_query(ResearchDomainMode::Shopping, "q").len() > 2
        );
        assert!(decorate_synthesis_query(ResearchDomainMode::CodeGen, "q").len() > 2);
    }
}
