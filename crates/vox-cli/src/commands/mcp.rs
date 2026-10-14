//! `vox mcp` — spawns the **`vox-mcp`** executable (separate crate) with stdio inherited.

use anyhow::Result;

/// Run the native in-process MCP server (stdio) if the `mcp-server` feature is enabled.
pub async fn run(profile: Option<String>) -> Result<()> {
    #[cfg(feature = "mcp-server")]
    {
        let profile = crate::commands::profile_arg::resolve_cli_profile(profile.as_deref())?;
        crate::commands::mcp_server::run_stdio_server_blocking_with_profile(profile).await?;
        Ok(())
    }

    #[cfg(not(feature = "mcp-server"))]
    {
        let _ = profile;
        anyhow::bail!(
            "Vox MCP server is not enabled in this build. Rebuild with: cargo build -p vox-cli --features mcp-server"
        )
    }
}

#[cfg(all(test, not(feature = "mcp-server")))]
mod tests {
    use super::*;

    /// When `mcp-server` is off, `vox mcp` must fail closed (non-zero exit via
    /// `main`'s `anyhow::Result` return) with the exact rebuild command, not a
    /// silent success — see the 2026-09-21 feature-gated-command-honesty fix.
    #[tokio::test]
    async fn run_without_mcp_server_feature_errors_with_rebuild_instruction() {
        let err = run(None)
            .await
            .expect_err("run() must fail when mcp-server is off");
        assert!(
            err.to_string()
                .contains("cargo build -p vox-cli --features mcp-server"),
            "error should name the exact rebuild command; got: {err}"
        );
    }
}

#[cfg(all(test, feature = "mcp-server"))]
mod profile_tests {
    #[tokio::test]
    async fn unknown_profile_fails_before_starting_the_server() {
        let err = super::run(Some("nope".to_string()))
            .await
            .expect_err("unknown profile");
        assert_eq!(err.to_string(), "profile_unknown: nope");
    }
}
