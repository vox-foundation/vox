//! `extract_terminal_return::<T>(journal)` walks a completed durable workflow
//! journal back to its `WorkflowCompleted` event and deserializes the recorded
//! `return_value` into the caller's T. Generated workflow bodies call this at
//! the end of `interpret_workflow_durable` to surface the typed return value.
//!
//! The journal today is `Vec<serde_json::Value>` (see `workflow::run`). Each
//! event is a JSON object discriminated by its `"event"` string field. The
//! terminal `WorkflowCompleted` event carries a `"return_value"` field whose
//! shape is workflow-specific.

use serde::de::DeserializeOwned;
use serde_json::Value;

/// Errors returned by [`extract_terminal_return`].
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    /// The journal contained no `WorkflowCompleted` event — the workflow did
    /// not terminate cleanly (interrupted, panicked, or still running).
    #[error("no WorkflowCompleted event in journal — workflow did not terminate cleanly")]
    NoTerminal,
    /// The terminal `WorkflowCompleted` event had no `return_value` field.
    /// Codegen produced by an earlier compiler version may emit this; bump and
    /// recompile the workflow.
    #[error("WorkflowCompleted event has no return_value field")]
    MissingReturnValue,
    /// The recorded `return_value` could not be deserialized into the caller's
    /// requested type `T`.
    #[error("terminal return value did not deserialize to expected type: {0}")]
    TypeMismatch(#[from] serde_json::Error),
    /// A tagged journal envelope has no serde-facing representation.
    #[error("cannot project journal envelope into serde return type: {0}")]
    UnprojectableEnvelope(String),
}

/// Map a journal-encoded `return_value` into JSON that `T: DeserializeOwned`
/// can read. `{"__vox":"Ok","value":"tx"}` becomes the serde shape of
/// `Ok("tx")` (`{"Ok":"tx"}` for a Rust `Result`).
pub fn journal_value_to_serde(value: &Value) -> Result<Value, ExtractError> {
    match value.get("__vox").and_then(Value::as_str) {
        Some("Ok") => Ok(serde_json::json!({"Ok": journal_value_to_serde(
            value.get("value").unwrap_or(&Value::Null)
        )?})),
        Some("Err") => Ok(serde_json::json!({"Err": journal_value_to_serde(
            value.get("value").unwrap_or(&Value::Null)
        )?})),
        Some("Some") => Ok(journal_value_to_serde(
            value.get("value").unwrap_or(&Value::Null),
        )?),
        Some("None") => Ok(Value::Null),
        Some("Decimal") => Ok(value.get("value").cloned().unwrap_or(Value::Null)),
        Some("Tuple") => Ok(value
            .get("items")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]))),
        Some("Object") => Ok(value
            .get("fields")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}))),
        Some("Tagged") => Ok(value.clone()),
        Some(other) => Err(ExtractError::UnprojectableEnvelope(other.to_string())),
        None => Ok(value.clone()),
    }
}

/// Extract the typed return value from a completed durable workflow journal.
///
/// Scans backwards for the terminal `WorkflowCompleted` event and deserializes
/// its `return_value` field into `T`. Returns [`ExtractError`] if the event is
/// missing, the field is absent, or the value does not match `T`.
pub fn extract_terminal_return<T: DeserializeOwned>(journal: &[Value]) -> Result<T, ExtractError> {
    let completed = journal
        .iter()
        .rev()
        .find(|e| e.get("event").and_then(Value::as_str) == Some("WorkflowCompleted"))
        .ok_or(ExtractError::NoTerminal)?;
    let return_value = completed
        .get("return_value")
        .ok_or(ExtractError::MissingReturnValue)?;
    let serde_facing = journal_value_to_serde(return_value)?;
    Ok(serde_json::from_value(serde_facing)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_ok_envelope_as_rust_result() {
        let journal = vec![json!({
            "event": "WorkflowCompleted",
            "return_value": {"__vox": "Ok", "value": "tx"}
        })];
        let got: Result<String, String> = extract_terminal_return(&journal).expect("extracts");
        assert_eq!(got, Ok("tx".into()));
    }

    #[test]
    fn extracts_err_envelope_as_rust_result() {
        let journal = vec![json!({
            "event": "WorkflowCompleted",
            "return_value": {"__vox": "Err", "value": "declined"}
        })];
        let got: Result<String, String> = extract_terminal_return(&journal).expect("extracts");
        assert_eq!(got, Err("declined".into()));
    }
}
