use aho_corasick::{AhoCorasick, MatchKind};
use serde_json::Value;

const MIN_REDACT_LEN: usize = 8; // don't redact tiny tokens that cause false positives
const MIN_CLAVIS_SCRUB_LEN: usize = 6; // minimum UTF-8 byte length for Clavis exact-value scrubbing

fn is_scrubbable_secret_spec(spec: &crate::spec::SecretSpec) -> bool {
    let meta = spec.id.metadata();
    if !crate::spec::is_user_facing_secret(spec.id)
        || crate::spec::taxonomy_class_for(spec.id).is_config_only()
        || meta.taxonomy_class.is_config_only()
        || matches!(
            meta.material_kind,
            crate::spec::SecretMaterialKind::ConfigValue
                | crate::spec::SecretMaterialKind::EndpointUrl
                | crate::spec::SecretMaterialKind::Username
                | crate::spec::SecretMaterialKind::DelegationRef
        )
    {
        return false;
    }
    let env = spec.canonical_env;
    !env.ends_with("_URL")
        && !env.ends_with("_EMAIL")
        && !env.ends_with("_USER_AGENT")
        && !env.ends_with("_PATH")
}

/// Scrub caller-provided secret needles (`>= 6` UTF-8 bytes after trimming) from `text`
/// using a `LeftmostLongest` Aho-Corasick automaton, replacing matches with `"[REDACTED:CLAVIS]"`.
#[must_use]
pub fn scrub_secret_needles(text: &str, needles: &[&str]) -> String {
    if text.len() < MIN_CLAVIS_SCRUB_LEN {
        return text.to_string();
    }
    let mut filtered: Vec<&str> = Vec::new();
    for needle in needles {
        let trimmed = needle.trim();
        if trimmed.len() >= MIN_CLAVIS_SCRUB_LEN && !filtered.contains(&trimmed) {
            filtered.push(trimmed);
        }
    }
    if filtered.is_empty() {
        return text.to_string();
    }
    let Ok(ac) = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .build(&filtered)
    else {
        return text.to_string();
    };
    let replacements: Vec<&str> =
        std::iter::repeat_n("[REDACTED:CLAVIS]", filtered.len()).collect();
    ac.replace_all(text, &replacements)
}

/// Scrub resolved user-facing Clavis secret material (`>= 6` UTF-8 bytes, non-config,
/// non-URL, non-username, non-delegation-ref) from arbitrary UTF-8 text.
#[must_use]
pub fn scrub_resolved_secrets(text: &str) -> String {
    if text.len() < MIN_CLAVIS_SCRUB_LEN {
        return text.to_string();
    }
    let mut needles: Vec<String> = Vec::new();
    for spec in crate::all_specs() {
        if !is_scrubbable_secret_spec(spec) {
            continue;
        }
        let resolved = crate::resolve_secret(spec.id);
        if let Some(val) = resolved.expose() {
            let trimmed = val.trim();
            if trimmed.len() >= MIN_CLAVIS_SCRUB_LEN && !needles.iter().any(|n| n == trimmed) {
                needles.push(trimmed.to_string());
            }
        }
    }
    if needles.is_empty() {
        return text.to_string();
    }
    let refs: Vec<&str> = needles.iter().map(String::as_str).collect();
    scrub_secret_needles(text, &refs)
}

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
    let replacements: Vec<&str> = std::iter::repeat_n("[REDACTED]", non_empty.len()).collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_secret_needles_leftmost_longest_and_min_len() {
        let out = scrub_secret_needles(
            "prefix mysecret_extended_token middle mysecret suffix ok",
            &["mysecret", "mysecret_extended_token", "tiny", "12345"],
        );
        assert_eq!(
            out,
            "prefix [REDACTED:CLAVIS] middle [REDACTED:CLAVIS] suffix ok"
        );
    }

    #[test]
    fn scrub_secret_needles_handles_empty_and_short_inputs() {
        assert_eq!(scrub_secret_needles("", &["mysecret"]), "");
        assert_eq!(scrub_secret_needles("short", &["mysecret"]), "short");
        assert_eq!(
            scrub_secret_needles("nothing matches here", &[]),
            "nothing matches here"
        );
        // Exact 6-byte UTF-8 threshold is scrubbed; 5-byte is ignored.
        assert_eq!(
            scrub_secret_needles("val=abcdef and val=abcde", &["abcdef", "abcde"]),
            "val=[REDACTED:CLAVIS] and val=abcde"
        );
    }

    #[test]
    fn scrub_resolved_secrets_handles_empty_and_runs_cleanly() {
        let _guard = crate::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        assert_eq!(scrub_resolved_secrets(""), "");
        assert_eq!(scrub_resolved_secrets("   "), "   ");
        let sample = "normal log line without secrets";
        let scrubbed = scrub_resolved_secrets(sample);
        assert!(!scrubbed.is_empty());
    }

    #[test]
    fn scrubbable_spec_filter_excludes_config_url_username_delegation() {
        assert!(is_scrubbable_secret_spec(
            crate::SecretId::GeminiApiKey.spec()
        ));
        assert!(is_scrubbable_secret_spec(
            crate::SecretId::OpenaiApiKey.spec()
        ));
        assert!(is_scrubbable_secret_spec(
            crate::SecretId::ForgeToken.spec()
        ));
        assert!(!is_scrubbable_secret_spec(
            crate::SecretId::GeminiTuningTemperature.spec()
        ));
        assert!(!is_scrubbable_secret_spec(crate::SecretId::VoxModel.spec()));
        assert!(!is_scrubbable_secret_spec(crate::SecretId::VoxDbUrl.spec()));

        let mut saw_included = false;
        let mut saw_excluded_config = false;
        for spec in crate::all_specs() {
            let meta = spec.id.metadata();
            let allowed = is_scrubbable_secret_spec(spec);
            if !crate::spec::is_user_facing_secret(spec.id)
                || crate::spec::taxonomy_class_for(spec.id).is_config_only()
                || meta.taxonomy_class.is_config_only()
                || matches!(
                    meta.material_kind,
                    crate::spec::SecretMaterialKind::ConfigValue
                        | crate::spec::SecretMaterialKind::EndpointUrl
                        | crate::spec::SecretMaterialKind::Username
                        | crate::spec::SecretMaterialKind::DelegationRef
                )
            {
                assert!(
                    !allowed,
                    "config/url/username spec must be skipped: {:?}",
                    spec.id
                );
                saw_excluded_config = true;
            } else if allowed {
                saw_included = true;
            }
        }
        assert!(saw_included);
        assert!(saw_excluded_config);
    }
}
