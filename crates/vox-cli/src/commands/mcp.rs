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
            "Vox MCP server is not enabled in this build. Recompile with --features mcp-server."
        )
    }
}
