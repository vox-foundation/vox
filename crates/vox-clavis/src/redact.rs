use aho_corasick::{AhoCorasick, MatchKind};
use serde_json::Value;

const MIN_REDACT_LEN: usize = 8; // don't redact tiny tokens that cause false positives

/// Recursively scrub all known secret values from a JSON `Value`.
/// `patterns` is a slice of plaintext secret values from the caller.
/// The caller must obtain these from `resolved.expose()` and is responsible
/// for not retaining them beyond this call's scope.
///
/// Returns a new `Value` with all occurrences replaced by `"[REDACTED]"`.
///
/// # Panics
/// Does not panic. If AhoCorasick construction fails (empty patterns or
/// pattern too long), returns the input unchanged.
pub fn redact_secrets_from_value(value: &Value, patterns: &[&str]) -> Value {
    let non_empty: Vec<&str> = patterns
        .iter()
        .filter(|p| p.len() >= MIN_REDACT_LEN) // don't redact 1-2 char patterns
        .copied()
        .collect();
    if non_empty.is_empty() {
        return value.clone();
    }
    let replacements: Vec<&str> = std::iter::repeat("[REDACTED]")
        .take(non_empty.len())
        .collect();
    let Ok(ac) = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostFirst)
        .build(&non_empty)
    else {
        return value.clone();
    };
    scrub_value_recursive(value, &ac, &replacements)
}

/// Check if a string contains any of the provided known-secret patterns.
/// Used for the audit-log safety check (C1 fix).
pub fn contains_secret_material(text: &str, patterns: &[&str]) -> bool {
    let non_empty: Vec<&str> = patterns
        .iter()
        .filter(|p| p.len() >= MIN_REDACT_LEN)
        .copied()
        .collect();
    if non_empty.is_empty() {
        return false;
    }
    if let Ok(ac) = AhoCorasick::new(&non_empty) {
        ac.is_match(text)
    } else {
        false
    }
}

fn scrub_value_recursive(value: &Value, ac: &AhoCorasick, replacements: &[&str]) -> Value {
    match value {
        Value::String(s) => Value::String(ac.replace_all(s, replacements)),
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| scrub_value_recursive(v, ac, replacements))
                .collect(),
        ),
        Value::Object(obj) => Value::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), scrub_value_recursive(v, ac, replacements)))
                .collect(),
        ),
        other => other.clone(),
    }
}
