//! Daemon-backed calls used by migrated `db`-class commands. Local mode reaches the daemon over
//! its TCP RPC (`orch.tool_call`, `orch.api_v2_get`); remote mode over the HTTP gateway
//! (`/v1/ws` `call_tool`, `/api/v2/*`). Both hit the same server handlers.

use serde::de::DeserializeOwned;
use serde_json::Value;
use vox_foundation::protocol::orch_daemon_method;
use vox_orchestrator::orch_daemon::OrchDaemonClient;

use super::backend::Backend;
use super::error::AxisError;
use crate::commands::daemon::PersistentDaemon;

/// Deadline for one remote tool call over plan 06's `/v1/ws` RPC.
const REMOTE_CALL_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

/// Codex user namespace every GUI preference has always been written under.
pub const GUI_USER_ID: &str = "local_user";

pub trait BackendCalls: Send + Sync {
    fn call_tool(
        &self,
        tool: &str,
        args: Value,
    ) -> impl std::future::Future<Output = Result<Value, String>> + Send;
    fn api_v2_get(
        &self,
        path_and_query: &str,
    ) -> impl std::future::Future<Output = Result<Value, String>> + Send;
}

async fn daemon_client(daemon: &PersistentDaemon) -> Result<OrchDaemonClient, String> {
    let addr = daemon.ensure().await?;
    Ok(match daemon.token().await {
        Some(token) => OrchDaemonClient::with_token(addr, token),
        None => OrchDaemonClient::new(addr),
    })
}

impl BackendCalls for Backend {
    async fn call_tool(&self, tool: &str, args: Value) -> Result<Value, String> {
        match self {
            Backend::Local(daemon) => daemon_client(daemon)
                .await?
                .call(
                    orch_daemon_method::TOOL_CALL,
                    serde_json::json!({ "name": tool, "args": args }),
                )
                .await
                .map_err(|e| format!("{tool}: {e}")),
            Backend::Remote(gw) => gw
                .call_daemon_method(
                    orch_daemon_method::TOOL_CALL,
                    serde_json::json!({ "name": tool, "args": args }),
                    REMOTE_CALL_DEADLINE,
                )
                .await
                .map_err(String::from),
        }
    }

    async fn api_v2_get(&self, path_and_query: &str) -> Result<Value, String> {
        match self {
            Backend::Local(daemon) => daemon_client(daemon)
                .await?
                .call(
                    orch_daemon_method::API_V2_GET,
                    serde_json::json!({ "path": path_and_query }),
                )
                .await
                .map_err(|e| format!("GET /api/v2{path_and_query}: {e}")),
            // Plan 06's `api_get` already unwraps the `{v,data|error}` envelope; re-wrap so
            // `api_data` sees one shape in both modes. Server-side errors stay `Ok(error envelope)`,
            // transport errors (unreachable, unauthorized, ...) stay `Err`, as in local mode.
            Backend::Remote(gw) => match gw.api_get(&format!("/api/v2{path_and_query}")).await {
                Ok(data) => Ok(serde_json::json!({ "v": 1, "data": data })),
                Err(AxisError::Remote { code, message }) => {
                    Ok(serde_json::json!({ "v": 1, "error": { "code": code, "message": message } }))
                }
                Err(e) => Err(String::from(e)),
            },
        }
    }
}

/// Unwrap a `ToolResult` envelope (`{success,data,error,meta}`) or a profile denial
/// (`{"error":{"code","message"}}`, 00-contracts §Wire shapes) into `T`.
pub fn tool_data<T: DeserializeOwned>(tool: &str, envelope: Value) -> Result<T, String> {
    if let Some(obj) = envelope.get("error").and_then(Value::as_object) {
        let code = obj.get("code").and_then(Value::as_str).unwrap_or("error");
        let msg = obj.get("message").and_then(Value::as_str).unwrap_or("");
        return Err(format!("{code}: {msg}"));
    }
    if envelope.get("success") == Some(&Value::Bool(true)) {
        let data = envelope.get("data").cloned().unwrap_or(Value::Null);
        return serde_json::from_value(data).map_err(|e| format!("{tool}: bad payload: {e}"));
    }
    let msg = envelope
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("tool failed");
    Err(format!("{tool}: {msg}"))
}

/// Unwrap the dashboard envelope (`{"v":1,"data":..}` / `{"v":1,"error":{code,message}}`).
pub fn api_data<T: DeserializeOwned>(envelope: Value) -> Result<T, String> {
    if let Some(err) = envelope.get("error") {
        let code = err.get("code").and_then(Value::as_str).unwrap_or("error");
        let msg = err.get("message").and_then(Value::as_str).unwrap_or("");
        return Err(format!("{code}: {msg}"));
    }
    let data = envelope.get("data").cloned().unwrap_or(Value::Null);
    serde_json::from_value(data).map_err(|e| format!("bad /api/v2 payload: {e}"))
}

/// `path?k=v&..` with RFC 3986 unreserved characters kept and everything else percent-encoded.
pub fn path_with_query(path: &str, pairs: &[(&str, Option<String>)]) -> String {
    fn enc(s: &str) -> String {
        s.bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    (b as char).to_string()
                }
                _ => format!("%{b:02X}"),
            })
            .collect()
    }
    let q: Vec<String> = pairs
        .iter()
        .filter_map(|(k, v)| v.as_ref().map(|v| format!("{k}={}", enc(v))))
        .collect();
    if q.is_empty() {
        path.to_string()
    } else {
        format!("{path}?{}", q.join("&"))
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use serde_json::Value;

    use super::BackendCalls;

    /// Records every call and answers from canned responses keyed `tool:<name>` / `get:<path>`.
    #[derive(Default)]
    pub struct FakeBackend {
        responses: Mutex<HashMap<String, Result<Value, String>>>,
        calls: Mutex<Vec<(String, Value)>>,
    }

    impl FakeBackend {
        pub fn new() -> Self {
            Self::default()
        }
        pub fn on_tool(self, tool: &str, resp: Result<Value, String>) -> Self {
            self.responses
                .lock()
                .unwrap()
                .insert(format!("tool:{tool}"), resp);
            self
        }
        pub fn on_get(self, path: &str, resp: Result<Value, String>) -> Self {
            self.responses
                .lock()
                .unwrap()
                .insert(format!("get:{path}"), resp);
            self
        }
        pub fn calls(&self) -> Vec<(String, Value)> {
            self.calls.lock().unwrap().clone()
        }
        fn answer(&self, key: String, args: Value) -> Result<Value, String> {
            self.calls.lock().unwrap().push((key.clone(), args));
            self.responses
                .lock()
                .unwrap()
                .get(&key)
                .cloned()
                .unwrap_or_else(|| Err(format!("FakeBackend: no response for {key}")))
        }
    }

    impl BackendCalls for FakeBackend {
        async fn call_tool(&self, tool: &str, args: Value) -> Result<Value, String> {
            self.answer(format!("tool:{tool}"), args)
        }
        async fn api_v2_get(&self, path_and_query: &str) -> Result<Value, String> {
            self.answer(format!("get:{path_and_query}"), Value::Null)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_data_unwraps_success_envelope_including_null() {
        let v: String = tool_data("t", json!({"success": true, "data": "x"})).unwrap();
        assert_eq!(v, "x");
        let none: Option<String> = tool_data("t", json!({"success": true, "data": null})).unwrap();
        assert_eq!(none, None);
    }

    #[test]
    fn tool_data_surfaces_tool_error_and_profile_denial() {
        let e = tool_data::<String>("t", json!({"success": false, "error": "boom"})).unwrap_err();
        assert_eq!(e, "t: boom");
        let denial =
            json!({"error": {"code": "tool_not_in_profile", "message": "tool t is not served"}});
        assert_eq!(
            tool_data::<String>("t", denial).unwrap_err(),
            "tool_not_in_profile: tool t is not served"
        );
    }

    #[test]
    fn api_data_unwraps_dashboard_envelope() {
        let v: Vec<u8> = api_data(json!({"v": 1, "data": [1, 2]})).unwrap();
        assert_eq!(v, vec![1, 2]);
        let e =
            api_data::<Vec<u8>>(json!({"v": 1, "error": {"code": "db_error", "message": "down"}}))
                .unwrap_err();
        assert_eq!(e, "db_error: down");
    }

    #[test]
    fn path_with_query_encodes_and_skips_none() {
        let p = path_with_query(
            "/x",
            &[
                ("a", Some("b c&d".into())),
                ("n", None),
                ("k", Some("1".into())),
            ],
        );
        assert_eq!(p, "/x?a=b%20c%26d&k=1");
        assert_eq!(path_with_query("/x", &[("n", None)]), "/x");
    }
}
