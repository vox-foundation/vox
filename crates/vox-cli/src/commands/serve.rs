//! `vox serve` — run `vox-orchestrator-d` in the foreground with the HTTP gateway enabled and no
//! stdio MCP transport, scoped to a capability profile.

use anyhow::{Context, Result};
use std::process::Stdio;
use vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::{
    ping_existing_daemon, ping_profile_matches, profile_mismatch_error,
};
use vox_cli_core::daemon_ipc::process_supervision::resolve_managed_binary_path;
use vox_secrets::SecretId;

const DEFAULT_DAEMON_ADDR: &str = "127.0.0.1:9745";

pub async fn run(
    profile_flag: Option<&str>,
    network: &crate::commands::serve_network::ServeNetworkArgs,
) -> Result<()> {
    let profile = crate::commands::profile_arg::resolve_cli_profile(profile_flag)?;
    let profile_id = profile.id_string();

    if !gateway_auth_configured() {
        anyhow::bail!(
            "vox serve enables the HTTP gateway, which requires VOX_MCP_HTTP_BEARER_TOKEN or VOX_MCP_HTTP_READ_BEARER_TOKEN (or VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED=1 for local testing)"
        );
    }
    if let Some(client) = ping_existing_daemon().await {
        let ping = client.ping().await.unwrap_or_default();
        anyhow::bail!("{}", existing_daemon_refusal(&ping, &profile_id));
    }

    let socket = vox_secrets::resolve_secret(SecretId::VoxOrchestratorDaemonSocket)
        .expose()
        .filter(|s| s.contains(':'))
        .map(str::to_string);
    let daemon_bin = resolve_managed_binary_path("vox-orchestrator-d");
    let shown = daemon_bin.display().to_string();
    let mut cmd = std::process::Command::new(&daemon_bin);
    for (key, value) in serve_child_env(&profile_id, socket) {
        cmd.env(key, value);
    }
    for (key, value) in network.gateway_env()? {
        cmd.env(key, value);
    }
    cmd.stdin(Stdio::null());
    eprintln!("vox serve: profile `{profile_id}` — starting {shown}");
    let status = tokio::task::spawn_blocking(move || cmd.status())
        .await
        .context("joining vox-orchestrator-d")?
        .with_context(|| format!("failed to start {shown}"))?;
    if !status.success() {
        anyhow::bail!("vox-orchestrator-d exited with {status}");
    }
    Ok(())
}

fn serve_child_env(profile_id: &str, socket: Option<String>) -> Vec<(&'static str, String)> {
    vec![
        ("VOX_PROFILE", profile_id.to_string()),
        ("VOX_MCP_HTTP_ENABLED", "1".to_string()),
        (
            "VOX_ORCHESTRATOR_DAEMON_SOCKET",
            socket.unwrap_or_else(|| DEFAULT_DAEMON_ADDR.to_string()),
        ),
    ]
}

fn existing_daemon_refusal(ping: &serde_json::Value, requested: &str) -> String {
    if ping_profile_matches(ping, requested) {
        format!(
            "a vox-orchestrator-d serving profile `{requested}` is already running; vox serve will not start a second one"
        )
    } else {
        profile_mismatch_error(ping, requested)
    }
}

fn truthy(v: &str) -> bool {
    let t = v.trim();
    t == "1"
        || t.eq_ignore_ascii_case("true")
        || t.eq_ignore_ascii_case("yes")
        || t.eq_ignore_ascii_case("on")
}

fn gateway_auth_configured() -> bool {
    let present = |id: SecretId| {
        vox_secrets::resolve_secret(id)
            .expose()
            .is_some_and(|s| !s.trim().is_empty())
    };
    present(SecretId::VoxMcpHttpBearerToken)
        || present(SecretId::VoxMcpHttpReadBearerToken)
        || vox_secrets::resolve_secret(SecretId::VoxMcpHttpAllowUnauthenticated)
            .expose()
            .is_some_and(truthy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_env_sets_profile_gateway_and_default_socket() {
        let env = serve_child_env("knowledge,models", None);
        assert!(env.contains(&("VOX_PROFILE", "knowledge,models".to_string())));
        assert!(env.contains(&("VOX_MCP_HTTP_ENABLED", "1".to_string())));
        assert!(env.contains(&(
            "VOX_ORCHESTRATOR_DAEMON_SOCKET",
            "127.0.0.1:9745".to_string()
        )));
    }

    #[test]
    fn child_env_keeps_configured_socket() {
        let env = serve_child_env("dev", Some("127.0.0.1:9800".to_string()));
        assert!(env.contains(&(
            "VOX_ORCHESTRATOR_DAEMON_SOCKET",
            "127.0.0.1:9800".to_string()
        )));
    }

    #[test]
    fn refusal_for_same_profile_says_already_running() {
        let msg = existing_daemon_refusal(&serde_json::json!({"profile": "mesh"}), "mesh");
        assert!(msg.contains("already running"), "{msg}");
    }

    #[test]
    fn refusal_for_other_profile_is_profile_mismatch() {
        let msg = existing_daemon_refusal(&serde_json::json!({"profile": "mesh"}), "models");
        assert!(msg.starts_with("profile_mismatch:"), "{msg}");
    }

    #[test]
    fn truthy_matches_gateway_bool_parsing() {
        for v in ["1", "true", "YES", " on "] {
            assert!(truthy(v), "{v}");
        }
        for v in ["0", "false", "", "off"] {
            assert!(!truthy(v), "{v}");
        }
    }
}
