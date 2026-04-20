//! `vox mcp` — spawns the **`vox-mcp`** executable (separate crate) with stdio inherited.

use anyhow::Result;

/// Run the native in-process MCP server (stdio) if the `mcp-server` feature is enabled.
pub fn run() -> Result<()> {
    #[cfg(feature = "mcp-server")]
    {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        rt.block_on(crate::commands::mcp_server::run_stdio_server_blocking())?;
        Ok(())
    }

    #[cfg(not(feature = "mcp-server"))]
    {
        anyhow::bail!(
            "Vox MCP server is not enabled in this build. Recompile with --features mcp-server."
        )
    }
}
