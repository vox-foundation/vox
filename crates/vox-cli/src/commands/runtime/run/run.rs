//! `vox run` — native (non-script) web-app mode runner.
//!
//! Used when the source file is detected as a web app (`@page` declarations).

use anyhow::Result;
use std::path::Path;

/// Returns `true` if `file` should be executed as a standalone script rather
/// than a web-app dev server, using only the **`@page` substring** heuristic
/// on the first 8 KiB (no full parse). Prefer `Vox.toml` `[web] run_mode` when
/// that scan is insufficient (`vox_config::WebRunMode`).
pub fn is_script_file_by_page_heuristic(file: &Path) -> bool {
    // Read the first 8 KiB to look for @page — avoids full parse for detection.
    let Ok(head) = crate::commands::ci::bounded_read::read_utf8_path_capped(file).map(|s| {
        let end = usize::min(8192, s.len());
        s[..end].to_string()
    }) else {
        // Unreadable file: do not route to script lane (avoid misrouting app builds on I/O errors).
        return false;
    };
    !head.contains("@page")
}

/// Run a web-app Vox source file in dev-server mode (non-script path).
///
/// Delegates to `vox-compilerd` daemon's `run` method.
pub async fn run(
    file: &Path,
    _args: &[String],
    _sandbox: bool,
    _trust_class: Option<&str>,
    open: bool,
) -> Result<()> {
    crate::dispatch::call_daemon(
        "vox-compilerd",
        "run",
        serde_json::json!({ "file": file, "open": open }),
        open,
    )
    .await?;
    Ok(())
}
