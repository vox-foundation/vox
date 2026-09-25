//! `vox mcp` — spawns the **`vox-mcp`** executable (separate crate) with stdio inherited.

use anyhow::Result;

/// Run the native in-process MCP server (stdio) if the `mcp-server` feature is enabled.
pub async fn run() -> Result<()> {
    #[cfg(feature = "mcp-server")]
    {
        crate::commands::mcp_server::run_stdio_server_blocking().await?;
        Ok(())
    }

    #[cfg(not(feature = "mcp-server"))]
    {
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
        let err = run()
            .await
            .expect_err("run() must fail when mcp-server is off");
        assert!(
            err.to_string()
                .contains("cargo build -p vox-cli --features mcp-server"),
            "error should name the exact rebuild command; got: {err}"
        );
    }
}
