//! Scientia-domain daemon IPC fallbacks when canonical `vox.db` holds a Turso `flock`.

use std::io::Write;

use anyhow::Result;
use serde_json::{Value, json};

use super::formatters::format_retrieval_status_proxy;
use super::{api_v2_get, api_v2_get_with_client};

fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            use std::fmt::Write as _;
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

pub(crate) fn scientia_claims_api_path(publication_id: &str) -> String {
    let encoded = encode_uri_component(publication_id);
    format!("/scientia/publications/{encoded}/claims")
}

pub async fn scientia_dashboard_via_daemon() -> Result<()> {
    let data = api_v2_get("/scientia/queue").await?;
    let rendered = serde_json::to_string_pretty(&data)?;
    writeln!(std::io::stdout(), "{rendered}")?;
    Ok(())
}

pub async fn scientia_cost_via_daemon() -> Result<()> {
    let data = api_v2_get("/scientia/cost").await?;
    let rendered = serde_json::to_string_pretty(&data)?;
    writeln!(std::io::stdout(), "{rendered}")?;
    Ok(())
}

pub async fn scientia_claims_via_daemon(publication_id: &str) -> Result<Value> {
    api_v2_get(&scientia_claims_api_path(publication_id)).await
}

pub async fn retrieval_status_via_daemon() -> Result<()> {
    let client = crate::daemon_tool::daemon_client().await?;
    let engine_status = api_v2_get_with_client(&client, "/research/engine-status").await?;
    let repo_index = crate::daemon_tool::tool_call(&client, "vox_repo_index_status", json!({}))
        .await
        .ok();
    let report = format_retrieval_status_proxy(&engine_status, repo_index.as_ref());
    writeln!(std::io::stdout(), "{report}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scientia_claims_api_path_percent_encodes_reserved_segments() {
        assert_eq!(
            scientia_claims_api_path("pub-42_v1.0"),
            "/scientia/publications/pub-42_v1.0/claims"
        );
        assert_eq!(
            scientia_claims_api_path("../a?x=1#frag"),
            "/scientia/publications/..%2Fa%3Fx%3D1%23frag/claims"
        );
        assert_eq!(
            scientia_claims_api_path("my pub"),
            "/scientia/publications/my%20pub/claims"
        );
    }
}
