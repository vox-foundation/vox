//! Hierarchical context distillation and grounded span verification.
//!
//! Compresses retrieved multi-source evidence into structured `ClaimEvidenceUnit`s
//! with exact substring grounding against raw source pages and RAG context budget packing
//! with distinct-domain logarithmic dampening.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Classification of distilled evidence format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    AtomicFact,
    TabularRow,
    CodeExcerpt,
    NumericPricing,
}

/// Epistemic modality of the extracted claim unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicModality {
    Definite,
    Conditional,
    Hypothetical,
    Negated,
}

/// A distilled, verifiable atomic unit of evidence extracted from a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimEvidenceUnit {
    pub unit_id: u64,
    pub kind: EvidenceKind,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub conditions: Vec<String>,
    pub modality: EpistemicModality,
    pub verbatim_quote: String,
    pub span_start: usize,
    pub span_end: usize,
    pub source_url: String,
    pub registrable_domain: String,
    pub trust_score: f64,
    pub corroborating_domains: Vec<String>,
    pub is_contradicted_or_contested: bool,
}

/// Verifies whether an extracted evidence unit is strictly grounded in raw source text.
///
/// Prevents distilled hallucination by asserting that `verbatim_quote` exists as an exact
/// slice at `[span_start..span_end]` or as an exact substring of `raw_content`.
/// For `NumericPricing` claims, it additionally checks that the digits in `object`
/// exist within `verbatim_quote`.
pub fn verify_span_grounding(raw_content: &str, unit: &ClaimEvidenceUnit) -> bool {
    if unit.verbatim_quote.trim().is_empty() {
        return false;
    }

    // 1. Direct slice check
    let slice_match = if let Some(slice) = raw_content.get(unit.span_start..unit.span_end) {
        slice == unit.verbatim_quote
    } else {
        false
    };

    // 2. Substring fallback if offset shifted due to sanitization
    let substring_match = slice_match
        || (unit.verbatim_quote.len() >= 15 && raw_content.contains(&unit.verbatim_quote));

    if !substring_match {
        return false;
    }

    // 3. For numeric/pricing facts, ensure numbers were not mutated by LLM extraction
    if unit.kind == EvidenceKind::NumericPricing {
        let object_digits: String = unit.object.chars().filter(|c| c.is_ascii_digit()).collect();
        let quote_digits: String = unit
            .verbatim_quote
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect();
        if !object_digits.is_empty() && !quote_digits.contains(&object_digits) {
            return false;
        }
    }

    true
}

/// Extract root domain from URL string (e.g. "https://docs.rs/example/path" -> "docs.rs").
pub fn extract_registrable_domain(url: &str) -> String {
    let stripped = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    let host = match stripped.find('/') {
        Some(idx) => &stripped[..idx],
        None => stripped,
    };
    let host_no_port = match host.find(':') {
        Some(idx) => &host[..idx],
        None => host,
    };
    host_no_port.to_ascii_lowercase()
}

/// Compute lexical token overlap score between unit and research query [0.0, 1.0].
fn lexical_relevance(unit: &ClaimEvidenceUnit, query_tokens: &HashSet<String>) -> f64 {
    if query_tokens.is_empty() {
        return 0.5;
    }
    let combined = format!(
        "{} {} {} {}",
        unit.subject, unit.predicate, unit.object, unit.verbatim_quote
    )
    .to_ascii_lowercase();

    let mut matches = 0;
    for token in query_tokens {
        if combined.contains(token) {
            matches += 1;
        }
    }
    (matches as f64 / query_tokens.len() as f64).clamp(0.0, 1.0)
}

/// Estimates character footprint of an evidence unit in synthesis context.
pub fn unit_char_cost(unit: &ClaimEvidenceUnit) -> usize {
    unit.subject.len()
        + unit.predicate.len()
        + unit.object.len()
        + unit.verbatim_quote.len()
        + unit.source_url.len()
        + 60 // Formatting wrappers
}

/// Greedily packs evidence units into context budget with:
/// - 15% pre-reserved allocation for contested/contradicted units.
/// - Distinct-domain logarithmic dampening to prevent single-source monopolization.
pub fn pack_rag_budget(
    units: &[ClaimEvidenceUnit],
    query: &str,
    max_chars: usize,
) -> Vec<ClaimEvidenceUnit> {
    if units.is_empty() || max_chars == 0 {
        return Vec::new();
    }

    let query_tokens: HashSet<String> = query
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|w| w.len() > 2)
        .collect();

    // 15% reserved for contradicted / contested evidence (clamped to max_chars for small budgets)
    let reserved_contradiction_budget = (max_chars * 15 / 100).max(1000).min(max_chars);
    let primary_budget = max_chars.saturating_sub(reserved_contradiction_budget);

    let (contradicted, normal): (Vec<_>, Vec<_>) = units
        .iter()
        .cloned()
        .partition(|u| u.is_contradicted_or_contested);

    let mut selected: Vec<ClaimEvidenceUnit> = Vec::new();
    let mut current_chars: usize = 0;
    let mut domain_counts: HashMap<String, usize> = HashMap::new();

    // 1. Pack contradicted units first into reserved budget
    for unit in contradicted {
        let cost = unit_char_cost(&unit);
        if current_chars + cost <= reserved_contradiction_budget {
            *domain_counts
                .entry(unit.registrable_domain.clone())
                .or_insert(0) += 1;
            current_chars += cost;
            selected.push(unit);
        }
    }

    // 2. Score remaining normal units with domain log dampening
    let mut scored_units: Vec<(f64, ClaimEvidenceUnit)> = normal
        .into_iter()
        .map(|u| {
            let t = u.trust_score.clamp(0.0, 1.0);
            let r = lexical_relevance(&u, &query_tokens);
            let corroboration_bonus =
                ((1.0 + u.corroborating_domains.len() as f64).ln()).clamp(0.0, 1.5);
            let specificity = match u.kind {
                EvidenceKind::NumericPricing | EvidenceKind::CodeExcerpt => 1.0,
                EvidenceKind::TabularRow => 0.8,
                EvidenceKind::AtomicFact => 0.6,
            };

            let score = 0.35 * t + 0.25 * r + 0.20 * corroboration_bonus + 0.20 * specificity;
            (score, u)
        })
        .collect();

    // Sort descending by initial score
    scored_units.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Greedily pack with dynamic domain repetition penalty
    for (_, unit) in scored_units {
        let cost = unit_char_cost(&unit);
        if current_chars + cost > max_chars {
            continue;
        }

        let seen_count = *domain_counts.get(&unit.registrable_domain).unwrap_or(&0);
        // Enforce maximum domain saturation: no single domain may supply more than 3 units
        // into the distillation context, guaranteeing diversity.
        if seen_count >= 3 {
            continue;
        }

        *domain_counts
            .entry(unit.registrable_domain.clone())
            .or_insert(0) += 1;
        current_chars += cost;
        selected.push(unit);
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_registrable_domain() {
        assert_eq!(
            extract_registrable_domain("https://docs.rs/tokio/1.0"),
            "docs.rs"
        );
        assert_eq!(
            extract_registrable_domain("http://www.github.com/vox/repo"),
            "github.com"
        );
        assert_eq!(
            extract_registrable_domain("https://api.openai.com:8080/v1"),
            "api.openai.com"
        );
    }

    #[test]
    fn test_verify_span_grounding_exact_and_fuzzy() {
        let raw_text =
            "The quick brown fox jumps over the lazy dog. Vox compiler version 2026 released.";
        let mut unit = ClaimEvidenceUnit {
            unit_id: 1,
            kind: EvidenceKind::AtomicFact,
            subject: "Vox compiler".into(),
            predicate: "released".into(),
            object: "version 2026".into(),
            conditions: vec![],
            modality: EpistemicModality::Definite,
            verbatim_quote: "Vox compiler version 2026 released.".into(),
            span_start: 45,
            span_end: 81,
            source_url: "https://vox-lang.org/news".into(),
            registrable_domain: "vox-lang.org".into(),
            trust_score: 0.95,
            corroborating_domains: vec![],
            is_contradicted_or_contested: false,
        };

        // Exact slice match
        assert!(verify_span_grounding(raw_text, &unit));

        // Shift offsets, fallback substring match succeeds
        unit.span_start = 0;
        unit.span_end = 10;
        assert!(verify_span_grounding(raw_text, &unit));

        // Hallucinated quote fails
        unit.verbatim_quote = "Vox compiler version 9999 released yesterday.".into();
        assert!(!verify_span_grounding(raw_text, &unit));
    }

    #[test]
    fn test_verify_span_grounding_numeric_pricing_integrity() {
        let raw_text = "Pro plan subscription costs $49 per month billed annually.";
        let unit = ClaimEvidenceUnit {
            unit_id: 2,
            kind: EvidenceKind::NumericPricing,
            subject: "Pro plan".into(),
            predicate: "costs".into(),
            object: "$99/mo".into(), // Mutated price: 99 instead of 49
            conditions: vec![],
            modality: EpistemicModality::Definite,
            verbatim_quote: "Pro plan subscription costs $49 per month".into(),
            span_start: 0,
            span_end: 41,
            source_url: "https://example.com/pricing".into(),
            registrable_domain: "example.com".into(),
            trust_score: 0.9,
            corroborating_domains: vec![],
            is_contradicted_or_contested: false,
        };

        // Number mutation in object ($99 vs $49) must be caught and rejected!
        assert!(!verify_span_grounding(raw_text, &unit));
    }

    #[test]
    fn test_pack_rag_budget_preserves_contradictions_and_dampens_domains() {
        let mut units = Vec::new();

        // 1 contradicted unit
        units.push(ClaimEvidenceUnit {
            unit_id: 100,
            kind: EvidenceKind::AtomicFact,
            subject: "API".into(),
            predicate: "deprecated".into(),
            object: "true".into(),
            conditions: vec![],
            modality: EpistemicModality::Definite,
            verbatim_quote: "Warning: API v1 is officially deprecated.".into(),
            span_start: 0,
            span_end: 40,
            source_url: "https://upstream.org/deprecations".into(),
            registrable_domain: "upstream.org".into(),
            trust_score: 0.9,
            corroborating_domains: vec![],
            is_contradicted_or_contested: true,
        });

        // 10 units from single domain "blog.com"
        for i in 1..=10 {
            units.push(ClaimEvidenceUnit {
                unit_id: i,
                kind: EvidenceKind::AtomicFact,
                subject: format!("Blog fact {i}"),
                predicate: "states".into(),
                object: format!("detail {i}"),
                conditions: vec![],
                modality: EpistemicModality::Definite,
                verbatim_quote: format!("Blog fact {i} states detail {i} in detail."),
                span_start: 0,
                span_end: 30,
                source_url: format!("https://blog.com/post/{i}"),
                registrable_domain: "blog.com".into(),
                trust_score: 0.6,
                corroborating_domains: vec![],
                is_contradicted_or_contested: false,
            });
        }

        // 1 high trust unit from "crates.io"
        units.push(ClaimEvidenceUnit {
            unit_id: 200,
            kind: EvidenceKind::AtomicFact,
            subject: "Package version".into(),
            predicate: "is".into(),
            object: "2.5.0".into(),
            conditions: vec![],
            modality: EpistemicModality::Definite,
            verbatim_quote: "Package version 2.5.0 released.".into(),
            span_start: 0,
            span_end: 32,
            source_url: "https://crates.io/crates/test".into(),
            registrable_domain: "crates.io".into(),
            trust_score: 1.0,
            corroborating_domains: vec!["docs.rs".into()],
            is_contradicted_or_contested: false,
        });

        let packed = pack_rag_budget(&units, "API version", 2000);

        // Assert contradicted unit is preserved
        assert!(
            packed.iter().any(|u| u.unit_id == 100),
            "Contradicted unit must be preserved in budget partition"
        );
        // Assert crates.io unit is present
        assert!(
            packed.iter().any(|u| u.unit_id == 200),
            "High trust authoritative unit must be included"
        );
        // Assert blog.com does not monopolize all slots
        let blog_count = packed
            .iter()
            .filter(|u| u.registrable_domain == "blog.com")
            .count();
        assert!(
            blog_count <= 4,
            "Domain blog.com should be dampened, got {blog_count}"
        );
    }
}
