//! MENS local claim extraction parser with verbatim substring grounding.
//!
//! Extracts epistemic claim triplets `(subject, predicate, object)` from raw LLM outputs
//! and enforces substring grounding against source text to eliminate hallucinations.

use serde::{Deserialize, Serialize};

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

/// Parses claim triplets from raw LLM output and filters out any triplets
/// whose `evidence_snippet` is empty or not found as a verbatim substring in `source_text` (case-insensitive).
pub fn parse_and_ground_claim_triplets(raw_json: &str, source_text: &str) -> Vec<ClaimTriplet> {
    let raw_list = match parse_envelope(raw_json) {
        Some(list) => list,
        None => return Vec::new(),
    };

    let lower_source = source_text.to_lowercase();
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

            // Verbatim grounding verification (case-insensitive substring)
            if !lower_source.contains(&snippet.to_lowercase()) {
                return None; // Discard ungrounded hallucination
            }

            let confidence = raw.confidence.unwrap_or(0.90).clamp(0.0, 1.0);
            Some(ClaimTriplet {
                subject,
                predicate,
                object,
                confidence,
                evidence_snippet: snippet,
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
