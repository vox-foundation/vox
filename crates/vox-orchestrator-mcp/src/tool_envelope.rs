//! MCP `ToolResult` envelope success helpers.
//!
//! Tools routinely return `Ok(ToolResult::err(...).to_json())`. Telemetry,
//! ChatHop, and Ludus must treat those as failures — never bare `Result::is_ok()`.

/// Returns true when JSON looks like `ToolResult` with `success: false` (MCP `is_error` signal).
pub fn tool_json_envelope_is_error(json: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get("success").and_then(|s| s.as_bool()))
        == Some(false)
}

/// True when dispatch returned `Ok(payload)` **and** the payload is not a
/// `ToolResult` error envelope. Prefer this over `Result::is_ok()` for hops,
/// Ludus, and `mcp_tool_call` telemetry.
pub fn tool_dispatch_call_succeeded(result: &Result<String, impl std::fmt::Display>) -> bool {
    match result {
        Ok(payload) => !tool_json_envelope_is_error(payload),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_error_when_success_false() {
        assert!(tool_json_envelope_is_error(
            r#"{"success":false,"error":"nope"}"#
        ));
    }

    #[test]
    fn envelope_ok_when_success_true() {
        assert!(!tool_json_envelope_is_error(
            r#"{"success":true,"data":"x"}"#
        ));
    }

    #[test]
    fn envelope_ok_when_not_tool_result_shape() {
        assert!(!tool_json_envelope_is_error("not json"));
        assert!(!tool_json_envelope_is_error(r#"{"foo":1}"#));
    }

    #[test]
    fn dispatch_call_succeeded_false_for_envelope_error_ok() {
        let envelope = r#"{"success":false,"error":"budget denied"}"#.to_string();
        let result: Result<String, anyhow::Error> = Ok(envelope);
        assert!(!tool_dispatch_call_succeeded(&result));
    }

    #[test]
    fn dispatch_call_succeeded_true_for_envelope_ok() {
        let envelope = r#"{"success":true,"data":"pong"}"#.to_string();
        let result: Result<String, anyhow::Error> = Ok(envelope);
        assert!(tool_dispatch_call_succeeded(&result));
    }

    #[test]
    fn dispatch_call_succeeded_false_for_rust_err() {
        let result: Result<String, anyhow::Error> = Err(anyhow::anyhow!("boom"));
        assert!(!tool_dispatch_call_succeeded(&result));
    }
}
