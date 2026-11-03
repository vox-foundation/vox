//! Capability-profiles plan 04: the gateway's network settings are resolvable through Clavis.

#[test]
fn mcp_http_network_settings_are_registered() {
    for env in [
        "VOX_MCP_HTTP_TLS_CERT",
        "VOX_MCP_HTTP_TLS_KEY",
        "VOX_MCP_HTTP_BEHIND_TUNNEL",
    ] {
        assert!(
            vox_secrets::all_specs()
                .iter()
                .any(|s| s.canonical_env == env),
            "{env} has no SecretSpec"
        );
    }
}
