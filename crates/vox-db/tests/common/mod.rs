//! Shared helpers for opt-in network integration tests in `tests/*.rs`.

/// Remote URL + token when `gate_env` is exactly `1`, using the same precedence as
/// `DbConfig` remote env resolution (`VOX_DB_*` → `VOX_TURSO_*` → `TURSO_*`).
pub fn remote_creds(gate_env: &str) -> Option<(String, String)> {
    if std::env::var(gate_env).ok().as_deref() != Some("1") {
        return None;
    }
    let try_pair = |url_k: &str, tok_k: &str| {
        let url = std::env::var(url_k).ok()?;
        let tok = std::env::var(tok_k).ok()?;
        let url = url.trim().to_string();
        let tok = tok.trim().to_string();
        if url.is_empty() || tok.is_empty() {
            return None;
        }
        Some((url, tok))
    };
    try_pair("VOX_DB_URL", "VOX_DB_TOKEN")
        .or_else(|| try_pair("VOX_TURSO_URL", "VOX_TURSO_TOKEN"))
        .or_else(|| try_pair("TURSO_URL", "TURSO_AUTH_TOKEN"))
}
