//! MENS local claim extraction parser with verbatim substring grounding.
//!
//! Extracts epistemic claim triplets `(subject, predicate, object)` from raw LLM outputs
//! and enforces substring grounding against source text to eliminate hallucinations.

use serde::{Deserialize, Serialize};

/// Grounding status of an extracted epistemic claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GroundingQuality {
    /// Exact verbatim substring found in source text.
    VerbatimExact,
    /// High token-overlap in normalized sliding window (handles punctuation, whitespace, pronoun resolution).
    NormalizedSpan { overlap_ratio: f64 },
}

/// Epistemic claim triplet extracted from evidence text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimTriplet {
    /// Entity or concept acting as the claim's subject.
    pub subject: String,
    /// Predicate or relationship between subject and object (normalized to lowercase).
    pub predicate: String,
    /// Entity, attribute, or concept acting as the claim's object.
    pub object: String,
    /// Confidence score in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Verbatim substring from the source document supporting this claim.
    pub evidence_snippet: String,
    pub source_url: Option<String>,
    pub grounding: GroundingQuality,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TripletEnvelope {
    ClaimsObject { claims: Vec<RawTriplet> },
    TripletsObject { triplets: Vec<RawTriplet> },
    Array(Vec<RawTriplet>),
}

#[derive(Deserialize)]
struct RawTriplet {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: Option<f64>,
    pub evidence_snippet: Option<String>,
}

fn strip_markdown_fences(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(start) = trimmed.find("```json") {
        let rest = &trimmed[start + "```json".len()..];
        if let Some(end) = rest.find("```") {
            return rest[..end].trim();
        }
    }
    if let Some(start) = trimmed.find("```") {
        let rest = &trimmed[start + "```".len()..];
        if let Some(end) = rest.find("```") {
            return rest[..end].trim();
        }
    }
    trimmed
}

fn extract_json_slice(text: &str) -> Option<&str> {
    let stripped = strip_markdown_fences(text);
    let start = stripped.find(|c| c == '{' || c == '[')?;
    let end = stripped.rfind(|c| c == '}' || c == ']')?;
    if start <= end {
        Some(&stripped[start..=end])
    } else {
        None
    }
}

fn parse_envelope(raw_json: &str) -> Option<Vec<RawTriplet>> {
    let stripped = strip_markdown_fences(raw_json);

    // First attempt: try between first '{'/'[' and last '}'/']'
    if let Some(slice) = extract_json_slice(stripped) {
        if let Ok(env) = serde_json::from_str::<TripletEnvelope>(slice) {
            return Some(match env {
                TripletEnvelope::ClaimsObject { claims } => claims,
                TripletEnvelope::TripletsObject { triplets } => triplets,
                TripletEnvelope::Array(arr) => arr,
            });
        }
    }

    // Second attempt: parse first valid JSON value from start
    if let Some(start) = stripped.find(|c| c == '{' || c == '[') {
        let mut de =
            serde_json::Deserializer::from_str(&stripped[start..]).into_iter::<TripletEnvelope>();
        if let Some(Ok(env)) = de.next() {
            return Some(match env {
                TripletEnvelope::ClaimsObject { claims } => claims,
                TripletEnvelope::TripletsObject { triplets } => triplets,
                TripletEnvelope::Array(arr) => arr,
            });
        }
    }

    None
}

pub const TOKEN_OVERLAP_THRESHOLD: f64 = 0.80;
const NEGATION_WORDS: &[&str] = &[
    "not", "no", "never", "none", "neither", "nor", "cannot", "can't", "won't", "didn't",
    "doesn't", "isn't", "aren't", "without",
];

pub fn normalize_for_matching(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for c in text.chars() {
        let mapped = match c {
            '—' | '–' => '-',
            '“' | '”' | '"' => '"',
            '‘' | '’' | '\'' => '\'',
            c if c.is_whitespace() => ' ',
            c if c.is_alphanumeric() || c == '-' || c == '\'' || c == '"' => c.to_ascii_lowercase(),
            _ => ' ',
        };
        if mapped == ' ' {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(mapped);
            prev_space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

pub fn negation_parity_matches(snippet: &str, candidate_window: &str) -> bool {
    let count_neg = |text: &str| {
        text.split_whitespace()
            .filter(|w| {
                let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
                NEGATION_WORDS
                    .iter()
                    .any(|nw| nw.eq_ignore_ascii_case(clean))
            })
            .count()
    };
    (count_neg(snippet) % 2) == (count_neg(candidate_window) % 2)
}

pub fn token_sliding_window_overlap(snippet: &str, source: &str) -> Option<f64> {
    let snip_tokens: Vec<&str> = snippet.split_whitespace().collect();
    if snip_tokens.is_empty() {
        return None;
    }
    let src_tokens: Vec<&str> = source.split_whitespace().collect();
    if src_tokens.is_empty() {
        return None;
    }

    let snip_set: std::collections::HashSet<&str> = snip_tokens.iter().copied().collect();
    let window_size = snip_tokens.len() + 4; // Bounded window: N + 4 tokens
    let mut best_ratio = 0.0;

    for window in src_tokens.windows(window_size.min(src_tokens.len())) {
        let win_str = window.join(" ");
        if !negation_parity_matches(snippet, &win_str) {
            continue;
        }
        let common = snip_set.iter().filter(|t| window.contains(t)).count();
        let ratio = common as f64 / snip_set.len() as f64;
        if ratio > best_ratio {
            best_ratio = ratio;
            if (best_ratio - 1.0).abs() < f64::EPSILON {
                break;
            }
        }
    }

    if best_ratio >= TOKEN_OVERLAP_THRESHOLD {
        Some(best_ratio)
    } else {
        None
    }
}

pub fn evaluate_span_grounding(snippet: &str, source_text: &str) -> Option<GroundingQuality> {
    let raw_snippet = snippet.trim();
    if raw_snippet.is_empty() {
        return None;
    }

    if source_text
        .to_lowercase()
        .contains(&raw_snippet.to_lowercase())
    {
        return Some(GroundingQuality::VerbatimExact);
    }

    let norm_source = normalize_for_matching(source_text);
    let norm_snippet = normalize_for_matching(raw_snippet);

    if norm_source.contains(&norm_snippet) {
        return Some(GroundingQuality::NormalizedSpan { overlap_ratio: 1.0 });
    }

    token_sliding_window_overlap(&norm_snippet, &norm_source)
        .map(|overlap_ratio| GroundingQuality::NormalizedSpan { overlap_ratio })
}

/// Parses claim triplets from raw LLM output and filters out any triplets
/// whose `evidence_snippet` cannot be grounded against `source_text`.
pub fn parse_and_ground_claim_triplets(raw_json: &str, source_text: &str) -> Vec<ClaimTriplet> {
    parse_and_ground_claim_triplets_with_source(raw_json, source_text, None)
}

/// Parses claim triplets from raw LLM output and filters out any triplets
/// whose `evidence_snippet` cannot be grounded against `source_text`, propagating
/// the optional `source_url`.
pub fn parse_and_ground_claim_triplets_with_source(
    raw_json: &str,
    source_text: &str,
    source_url: Option<&str>,
) -> Vec<ClaimTriplet> {
    let raw_list = match parse_envelope(raw_json) {
        Some(list) => list,
        None => return Vec::new(),
    };

    raw_list
        .into_iter()
        .filter_map(|raw| {
            let subject = raw.subject.trim().to_string();
            let predicate = raw.predicate.trim().to_lowercase();
            let object = raw.object.trim().to_string();
            let snippet = raw.evidence_snippet.unwrap_or_default().trim().to_string();

            if subject.is_empty() || predicate.is_empty() || object.is_empty() || snippet.is_empty()
            {
                return None;
            }

            let grounding = evaluate_span_grounding(&snippet, source_text)?;
            let confidence = raw.confidence.unwrap_or(0.90).clamp(0.0, 1.0);

            Some(ClaimTriplet {
                subject,
                predicate,
                object,
                confidence,
                evidence_snippet: snippet,
                source_url: source_url.map(ToString::to_string),
                grounding,
            })
        })
        .collect()
}

/// Builds an extraction prompt prompting an LLM to extract verbatim-grounded claim triplets.
pub fn build_local_claim_extraction_prompt(evidence: &str) -> String {
    format!(
        "You are an epistemic claim extraction agent. Extract atomic epistemic triplets from the following text.\n\
         Every evidence_snippet MUST be a verbatim quote from the text.\n\
         Output ONLY valid JSON with format:\n\
         {{\"claims\": [{{\"subject\": \"...\", \"predicate\": \"...\", \"object\": \"...\", \"confidence\": 0.95, \"evidence_snippet\": \"...\"}}]}}\n\n\
         Text:\n{evidence}"
    )
}
