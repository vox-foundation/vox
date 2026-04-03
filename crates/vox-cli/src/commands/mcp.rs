//! `vox mcp` — spawns the **`vox-mcp`** executable (separate crate) with stdio inherited.

use anyhow::{Context, Result};
use std::process::Command;

/// Run `vox-mcp` from `PATH` until the child process exits.
pub fn run() -> Result<()> {
    // Start the vox-mcp binary, connecting its stdio (JSON-RPC) to the CLI's stdio.
    // The MCP client (e.g. VS Code extension) will communicate with vox-mcp through this process.

    let mcp_path = crate::process_supervision::resolve_managed_binary_path("vox-mcp");
    let mut child = Command::new(&mcp_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to spawn vox-mcp binary at '{}'", mcp_path.display()))?;

    let status = child.wait()?;

    if !status.success() {
        anyhow::bail!("vox-mcp exited with status: {}", status);
    }

    Ok(())
}
