use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use super::GatewayState;

pub(super) fn is_origin_allowed(
    public_eval_enabled: bool,
    path: &str,
    headers: &HeaderMap,
) -> bool {
    // Exempt the public eval sandbox from loopback restrictions for specific paths
    if public_eval_enabled && (path == "/v1/eval" || path == "/health") {
        return true;
    }

    let is_upgrade = headers
        .get(axum::http::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);

    let origin = headers.get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get(axum::http::header::HOST).and_then(|v| v.to_str().ok()))
        .unwrap_or("");

    if !origin.is_empty() {
        // Strip the optional scheme so the same check works for both `Origin`
        // (`http://localhost:3000`) and `Host` (`localhost:8080`) headers.
        let host_part = origin
            .strip_prefix("http://")
            .or_else(|| origin.strip_prefix("https://"))
            .unwrap_or(origin);

        if !is_loopback_host(host_part) {
            return false;
        }
    } else if is_upgrade {
        // Strict WS upgrade check: if no Origin or Host is provided during a WebSocket upgrade, deny it.
        return false;
    }

    true
}

/// Returns true iff `host` is exactly a loopback name, optionally followed by
/// `:<port>` or `/<path>`. Prevents subdomain spoofing like `localhost.evil.com`
/// or `127.0.0.1.attacker.org` from passing a naive `starts_with` check.
fn is_loopback_host(host: &str) -> bool {
    for prefix in ["127.0.0.1", "localhost", "[::1]"] {
        if host == prefix {
            return true;
        }
        if let Some(rest) = host.strip_prefix(prefix) {
            // Only ':' (port) or '/' (path) may follow a real loopback host.
            if rest.starts_with(':') || rest.starts_with('/') {
                return true;
            }
        }
    }
    false
}

pub(super) async fn check_origin_allowlist(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    req: Request,
    next: Next,
) -> Response {
    if !is_origin_allowed(state.public_eval_enabled, req.uri().path(), &headers) {
        let body = Json(json!({"error": "origin_denied"}));
        return (StatusCode::FORBIDDEN, body).into_response();
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_origin_allowed_loopback() {
        let mut headers = HeaderMap::new();
        headers.insert("origin", HeaderValue::from_static("http://localhost:3000"));
        assert!(is_origin_allowed(false, "/v1/info", &headers));

        let mut headers = HeaderMap::new();
        headers.insert("origin", HeaderValue::from_static("https://127.0.0.1:8080"));
        assert!(is_origin_allowed(false, "/v1/info", &headers));

        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("localhost:8080"));
        assert!(is_origin_allowed(false, "/v1/info", &headers));
    }

    #[test]
    fn test_origin_denied_external() {
        let mut headers = HeaderMap::new();
        headers.insert("origin", HeaderValue::from_static("https://malicious.com"));
        assert!(!is_origin_allowed(false, "/v1/info", &headers));
    }

    #[test]
    fn test_public_eval_exemptions() {
        let mut headers = HeaderMap::new();
        headers.insert("origin", HeaderValue::from_static("https://some-remote.com"));
        
        // Allowed if public_eval_enabled and path is /v1/eval or /health
        assert!(is_origin_allowed(true, "/v1/eval", &headers));
        assert!(is_origin_allowed(true, "/health", &headers));
        
        // Not allowed for other endpoints even if public_eval is true
        assert!(!is_origin_allowed(true, "/v1/info", &headers));
    }

    #[test]
    fn test_origin_denied_localhost_subdomain_spoof() {
        // Subdomain that *contains* "localhost" must not pass the loopback gate.
        let mut headers = HeaderMap::new();
        headers.insert("origin", HeaderValue::from_static("http://localhost.evil.com"));
        assert!(!is_origin_allowed(false, "/v1/info", &headers));

        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("localhost.attacker.org"));
        assert!(!is_origin_allowed(false, "/v1/info", &headers));

        // Same for 127.0.0.1 prefix attacks.
        let mut headers = HeaderMap::new();
        headers.insert("origin", HeaderValue::from_static("http://127.0.0.1.evil.com"));
        assert!(!is_origin_allowed(false, "/v1/info", &headers));
    }

    #[test]
    fn test_ws_upgrade_missing_origin() {
        let mut headers = HeaderMap::new();
        headers.insert("upgrade", HeaderValue::from_static("websocket"));
        // No origin or host provided
        assert!(!is_origin_allowed(false, "/v1/ws", &headers));
        
        let mut headers2 = HeaderMap::new();
        headers2.insert("upgrade", HeaderValue::from_static("websocket"));
        headers2.insert("origin", HeaderValue::from_static("http://localhost"));
        assert!(is_origin_allowed(false, "/v1/ws", &headers2));
    }
}
