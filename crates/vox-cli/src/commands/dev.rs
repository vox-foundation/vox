//! `vox dev` — watch mode backed by **`vox-compilerd`** (JSON-RPC over stdio).
//!
//! Spawns the daemon (sibling to `vox` or on `PATH`), sends a `dev` request, and streams
//! logs/progress until EOF or Ctrl+C. For an in-repo daemon, build `vox-compilerd` and ensure
//! it is next to `vox` or on `PATH`.

use anyhow::Result;
use std::path::Path;

/// Start dev/watch for `file`, writing artifacts under `out_dir`, binding `port` (default 3000).
///
/// Long-lived: runs until the daemon exits or the user presses Ctrl+C (forwards interrupt to child).
pub async fn run(file: &Path, out_dir: &Path, port: Option<u16>, open: bool) -> Result<()> {
    crate::dispatch::call_daemon_streaming(
        "vox-compilerd",
        "dev",
        serde_json::json!({
            "file": file.display().to_string(),
            "out_dir": out_dir.display().to_string(),
            "port": port.unwrap_or(3000),
            "open": open,
        }),
        open,
    )
    .await
}
