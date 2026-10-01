//! Structural-only allowlist for `research_trial.*` metric rows (TELEM-01).
//!
//! Every key a research-trial producer may write is listed here; anything else —
//! including content such as queries, answers, URLs, titles, or snippets — is rejected
//! before the row reaches `research_metrics`. Mirrored by
//! `contracts/telemetry/research-trial.v1.schema.json`.

use crate::types::TelemetryError;

/// Metric-type prefix that opts a row into allowlist enforcement.
pub const RESEARCH_TRIAL_METRIC_PREFIX: &str = "research_trial.";

/// Maximum length of any string value in a research-trial row.
pub const RESEARCH_TRIAL_STRING_MAX_CHARS: usize = 128;

const IDENTITY_KEYS: &[&str] = &["campaign_id", "run_id", "arm_id", "attempt", "replicate_id"];

/// Allowed keys per research-trial metric type (identity keys are allowed everywhere).
pub const RESEARCH_TRIAL_ALLOWLIST: &[(&str, &[&str])] = &[
    (
        "research_trial.campaign",
        &[
            "prereg_digest",
            "arm_count",
            "cost_cap_usd_micros",
            "status",
            "reason_code",
        ],
    ),
    (
        "research_trial.run",
        &[
            "config_hash",
            "query_hash",
            "build_version",
            "status",
            "duration_ms",
            "served_from_cache",
            "evidence_manifest_hash",
            "tokens_in",
            "tokens_out",
            "cost_usd_micros",
            "llm_calls",
            "tool_calls",
            "retry_of_run_id",
        ],
    ),
    (
        "research_trial.stage",
        &[
            "stage",
            "seq",
            "status",
            "started_at_ms",
            "duration_ms",
            "reason_code",
        ],
    ),
    (
        "research_trial.model_call",
        &[
            "stage",
            "requested_model_id",
            "resolved_model_id",
            "provider_id",
            "prompt_tokens",
            "completion_tokens",
            "cache_read_tokens",
            "latency_ms",
            "ttft_ms",
            "ttft_source",
            "cost_usd_micros",
            "cost_source",
            "error_class",
            "request_hash",
            "response_hash",
        ],
    ),
    (
        "research_trial.tool_call",
        &[
            "stage",
            "tool_id",
            "status",
            "elapsed_ms",
            "result_count",
            "response_hash",
        ],
    ),
];

fn allowed_keys(metric_type: &str) -> Option<&'static [&'static str]> {
    RESEARCH_TRIAL_ALLOWLIST
        .iter()
        .find(|(t, _)| *t == metric_type)
        .map(|(_, keys)| *keys)
}

fn structural_string(s: &str) -> bool {
    s.len() <= RESEARCH_TRIAL_STRING_MAX_CHARS
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | ':' | '+'))
}

fn reject(msg: String) -> Result<(), TelemetryError> {
    Err(TelemetryError::Validation(format!("research_trial: {msg}")))
}

/// Validate a `research_trial.*` row: known metric type, flat JSON object, allowlisted keys,
/// scalar values, and bounded structural strings only.
pub fn validate_research_trial_metadata(
    metric_type: &str,
    metadata_json: Option<&str>,
) -> Result<(), TelemetryError> {
    let Some(keys) = allowed_keys(metric_type) else {
        return reject(format!("unknown metric_type {metric_type:?}"));
    };
    let Some(raw) = metadata_json else {
        return reject(format!("{metric_type} requires metadata_json"));
    };
    let value: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => return reject(format!("metadata_json is not valid JSON: {e}")),
    };
    let Some(obj) = value.as_object() else {
        return reject("metadata_json must be a JSON object".into());
    };
    for (key, val) in obj {
        if !IDENTITY_KEYS.contains(&key.as_str()) && !keys.contains(&key.as_str()) {
            return reject(format!("key {key:?} is not allowlisted for {metric_type}"));
        }
        match val {
            serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
            }
            serde_json::Value::String(s) if structural_string(s) => {}
            serde_json::Value::String(_) => {
                return reject(format!(
                    "value of {key:?} is not a bounded structural string"
                ));
            }
            _ => return reject(format!("value of {key:?} must be a scalar")),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_never_contains_content_keys() {
        const CONTENT: &[&str] = &[
            "query", "answer", "url", "title", "snippet", "excerpt", "prompt", "response", "text",
            "content",
        ];
        for (metric_type, keys) in RESEARCH_TRIAL_ALLOWLIST {
            for key in keys.iter().chain(IDENTITY_KEYS) {
                assert!(
                    !CONTENT.contains(key),
                    "{metric_type} allowlists content key {key}"
                );
            }
        }
    }

    #[test]
    fn contract_schema_matches_allowlist() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../contracts/telemetry/research-trial.v1.schema.json"
        ))
        .expect("schema parses");
        let defs = schema["$defs"].as_object().expect("$defs object");
        assert_eq!(defs.len(), RESEARCH_TRIAL_ALLOWLIST.len());
        for (metric_type, keys) in RESEARCH_TRIAL_ALLOWLIST {
            let def = &defs[*metric_type];
            assert_eq!(def["additionalProperties"], serde_json::Value::Bool(false));
            let mut props: Vec<&str> = def["properties"]
                .as_object()
                .expect("properties")
                .keys()
                .map(String::as_str)
                .collect();
            props.sort_unstable();
            let mut expected: Vec<&str> = keys.iter().chain(IDENTITY_KEYS).copied().collect();
            expected.sort_unstable();
            assert_eq!(props, expected, "schema drift for {metric_type}");
        }
    }
}
