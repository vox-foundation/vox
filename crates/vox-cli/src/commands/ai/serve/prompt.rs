//! Prompt shaping and structured output validation.

/// P015: Returns true if s is a prefix of valid JSON (can be extended to complete JSON).
#[cfg(feature = "execution-api")]
#[allow(dead_code)]
pub fn is_valid_json_prefix(s: &str) -> bool {
    let s = s.trim_start();
    if s.is_empty() {
        return true;
    }
    if serde_json::from_str::<serde_json::Value>(s).is_ok() {
        return true;
    }
    let msg = serde_json::from_str::<serde_json::Value>(s)
        .unwrap_err()
        .to_string();
    msg.contains("EOF") || msg.contains("unexpected end of")
}

/// P015: Mask logits for invalid JSON tokens. Sets logits to NEG_INFINITY for tokens that would break JSON.
#[cfg(feature = "execution-api")]
#[allow(dead_code)]
pub fn mask_logits_for_json(logits: &mut [f32], current_text: &str) {
    use vox_populi::tensor::data::{VOCAB_SIZE, VoxTokenizer};
    for id in 0..VOCAB_SIZE.min(logits.len()) {
        let token_str = VoxTokenizer::decode(&[id as u32]);
        let extended = format!("{}{}", current_text, token_str);
        if !is_valid_json_prefix(&extended) {
            logits[id] = f32::NEG_INFINITY;
        }
    }
}

/// Shape user prompt for output_mode (strict_json, jsonl_records, tool_args_json).
#[cfg(feature = "execution-api")]
pub fn prompt_for_output_mode(user_prompt: &str, output_mode: Option<&str>) -> String {
    match output_mode.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some("strict_json") | Some("jsonl_records") | Some("tool_args_json") => {
            vox_corpus::corpus::structured_eval::strict_json_prompt(user_prompt)
        }
        _ => user_prompt.to_string(),
    }
}

/// P017 minimal_edit: Returns validation error for targeted repair prompts.
#[cfg(feature = "execution-api")]
pub fn validate_structured_output_with_reason(
    text: &str,
    output_mode: Option<&str>,
    schema: Option<&serde_json::Value>,
) -> Result<(), vox_corpus::corpus::structured_eval::StructuredFailReason> {
    use vox_corpus::corpus::structured_eval::{
        StructuredFailReason, validate_against_schema, validate_jsonl, validate_strict_json,
    };
    let base = match output_mode {
        Some("jsonl_records") => validate_jsonl(text),
        Some("strict_json") | Some("tool_args_json") => validate_strict_json(text),
        _ => return Ok(()),
    };
    base?;
    if let Some(s) = schema {
        if output_mode == Some("jsonl_records") {
            for (i, line) in text
                .trim()
                .lines()
                .filter(|l| !l.trim().is_empty())
                .enumerate()
            {
                let parsed =
                    serde_json::from_str::<serde_json::Value>(line.trim()).map_err(|e| {
                        StructuredFailReason::JsonlLineInvalid {
                            line_no: i + 1,
                            error: e.to_string(),
                        }
                    })?;
                validate_against_schema(&parsed, s)?;
            }
        } else if output_mode.is_some() {
            let parsed = serde_json::from_str::<serde_json::Value>(text.trim()).map_err(|e| {
                StructuredFailReason::InvalidJson {
                    error: e.to_string(),
                }
            })?;
            validate_against_schema(&parsed, s)?;
        }
    }
    Ok(())
}

/// P016: Validate structured output (JSON/JSONL + optional schema). Used by tests and retry logic.
#[cfg(feature = "execution-api")]
pub fn validate_structured_output(
    text: &str,
    output_mode: Option<&str>,
    schema: Option<&serde_json::Value>,
) -> bool {
    validate_structured_output_with_reason(text, output_mode, schema).is_ok()
}
