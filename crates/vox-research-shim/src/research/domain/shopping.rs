//! First-party Shopping domain engine for product research.
//!
//! Generates targeted subqueries for technical specifications, price tracking,
//! and authentic community complaints (e.g. Reddit, RTINGS) while de-boosting
//! SEO affiliate spam.

use crate::research::types::ResearchHit;

/// Substrings and domains that typically identify SEO affiliate spam farms.
pub const AFFILIATE_SEO_PATTERNS: &[&str] = &[
    "best-10-",
    "top-rated-gear",
    "bestreviews",
    "thebest10",
    "top10",
    "buyersguide",
    "affiliate",
    "expert-review-rankings",
];

/// Generate targeted shopping subqueries for specs, price, common defects, and lab benchmarks.
pub fn generate_shopping_subqueries(product: &str) -> Vec<String> {
    vec![
        format!("{product} technical specifications teardown battery display measurements"),
        format!("{product} msrp current price discounts deals sales"),
        format!("{product} common problems issues failure rate complaints site:reddit.com"),
        format!(
            "{product} review laboratory measurements site:rtings.com OR site:notebookcheck.net"
        ),
    ]
}

/// Detect if a URL or host appears to be affiliate SEO spam.
pub fn is_affiliate_seo_spam(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    AFFILIATE_SEO_PATTERNS
        .iter()
        .any(|pattern| lower.contains(pattern))
}

/// De-boost trust score and relevance for hits identified as affiliate SEO spam.
pub fn deboost_affiliate_spam(hits: &mut [ResearchHit]) {
    for hit in hits.iter_mut() {
        if is_affiliate_seo_spam(&hit.url) {
            hit.trust_score *= 0.3;
            hit.score *= 0.5;
        }
    }
}

/// Synthesis instructions for generating a product comparison matrix.
pub fn shopping_synthesis_instructions() -> &'static str {
    "DOMAIN MODE: SHOPPING & PRODUCT RESEARCH\nIn addition to standard research synthesis:\n1. Output a Markdown comparison matrix table with the following exact columns:\n   | Product | Price / MSRP | Key Pros | Critical Flaws / Common Complaints | Verdict |\n2. Highlight empirical lab measurements (e.g. battery life, frequency response, thermal throttling) from reputable lab reviewers.\n3. Cross-reference Reddit community feedback for persistent build quality issues, software bugs, or customer support problems.\n4. Filter out affiliate marketing bias and provide a balanced, evidence-backed verdict."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affiliate_detection() {
        assert!(is_affiliate_seo_spam(
            "https://top-rated-gear.com/best-headphones"
        ));
        assert!(is_affiliate_seo_spam("https://thebest10.org/laptops"));
        assert!(!is_affiliate_seo_spam(
            "https://rtings.com/headphones/reviews/sony/wh-1000xm5"
        ));
        assert!(!is_affiliate_seo_spam("https://reddit.com/r/headphones"));
    }

    #[test]
    fn test_deboost_affiliate_spam() {
        let mut hits = vec![
            ResearchHit {
                url: "https://top-rated-gear.com/review".into(),
                title: "Top Gear".into(),
                snippet: "The best product".into(),
                score: 1.0,
                http_status: 200,
                trust_score: 1.0,
                raw_content: String::new(),
            },
            ResearchHit {
                url: "https://rtings.com/review".into(),
                title: "RTINGS Review".into(),
                snippet: "Lab testing results".into(),
                score: 1.0,
                http_status: 200,
                trust_score: 1.0,
                raw_content: String::new(),
            },
        ];

        deboost_affiliate_spam(&mut hits);
        assert!(hits[0].trust_score < 0.5);
        assert!(hits[0].score < 0.8);
        assert!((hits[1].trust_score - 1.0).abs() < f64::EPSILON);
    }
}
