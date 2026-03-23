//! JSON / JSONL validation helpers for strict tool and API outputs.

use serde_json::Value;

/// Failure reason for structured validation (used by serve prompt shaping).
#[derive(Debug, Clone)]
pub enum StructuredFailReason {
    /// `serde_json` could not parse the trimmed buffer as one value.
    InvalidJson {
        /// Parser error text.
        error: String,
    },
    /// One JSONL row failed to parse as JSON.
    JsonlLineInvalid {
        /// 1-based line index in the JSONL buffer.
        line_no: usize,
        /// Parser error text.
        error: String,
    },
    /// Parsed JSON did not satisfy the minimal schema contract.
    SchemaMismatch {
        /// Human-readable mismatch detail.
        detail: String,
    },
}

impl std::fmt::Display for StructuredFailReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StructuredFailReason::InvalidJson { error } => {
                write!(f, "invalid JSON: {error}")
            }
            StructuredFailReason::JsonlLineInvalid { line_no, error } => {
                write!(f, "JSONL line {line_no}: {error}")
            }
            StructuredFailReason::SchemaMismatch { detail } => {
                write!(f, "schema mismatch: {detail}")
            }
        }
    }
}

impl std::error::Error for StructuredFailReason {}

/// Wrap user text so the model emits a single JSON object only.
pub fn strict_json_prompt(user: &str) -> String {
    format!(
        "{user}\n\nRespond with a single valid JSON object only. No markdown fences, no commentary."
    )
}

/// Ensure `text` trims to a single JSON value (object/array/primitive).
pub fn validate_strict_json(text: &str) -> Result<(), StructuredFailReason> {
    let t = text.trim();
    serde_json::from_str::<Value>(t).map_err(|e| StructuredFailReason::InvalidJson {
        error: e.to_string(),
    })?;
    Ok(())
}

/// Validate every non-empty line in `text` as JSON.
pub fn validate_jsonl(text: &str) -> Result<(), StructuredFailReason> {
    for (i, line) in text.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        serde_json::from_str::<Value>(line.trim()).map_err(|e| {
            StructuredFailReason::JsonlLineInvalid {
                line_no: i + 1,
                error: e.to_string(),
            }
        })?;
    }
    Ok(())
}

/// Minimal schema check: ensure every key in `schema` object exists in `value` with compatible type.
pub fn validate_against_schema(value: &Value, schema: &Value) -> Result<(), StructuredFailReason> {
    let Some(obj) = schema.as_object() else {
        return Ok(());
    };
    let Value::Object(vmap) = value else {
        return Err(StructuredFailReason::SchemaMismatch {
            detail: "root must be a JSON object".into(),
        });
    };
    for (k, spec) in obj {
        let Some(found) = vmap.get(k) else {
            return Err(StructuredFailReason::SchemaMismatch {
                detail: format!("missing key `{k}`"),
            });
        };
        if let Some(st) = spec.as_str() {
            match st {
                "string" if !found.is_string() => {
                    return Err(StructuredFailReason::SchemaMismatch {
                        detail: format!("key `{k}` must be string"),
                    });
                }
                "number" if !found.is_number() => {
                    return Err(StructuredFailReason::SchemaMismatch {
                        detail: format!("key `{k}` must be number"),
                    });
                }
                "object" if !found.is_object() => {
                    return Err(StructuredFailReason::SchemaMismatch {
                        detail: format!("key `{k}` must be object"),
                    });
                }
                "array" if !found.is_array() => {
                    return Err(StructuredFailReason::SchemaMismatch {
                        detail: format!("key `{k}` must be array"),
                    });
                }
                _ => {}
            }
        }
    }
    Ok(())
}
