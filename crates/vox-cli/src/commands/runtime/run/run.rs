//! `vox run` — native (non-script) web-app mode runner.
//!
//! Used when the source file is detected as a web app (`@page` declarations).

use anyhow::Result;
use std::path::Path;
use vox_bounded_fs::read_utf8_path_capped;

/// Returns `true` if `file` should be executed as a standalone script rather
/// than a web-app dev server, using only the **`@page` substring** heuristic
/// on the first 8 KiB (no full parse). Prefer `Vox.toml` `[web] run_mode` when
/// that scan is insufficient (`vox_config::WebRunMode`).
pub fn is_script_file_by_page_heuristic(file: &Path) -> bool {
    // Read the first 8 KiB to look for @page — avoids full parse for detection.
    let Ok(head) = read_utf8_path_capped(file).map(|s| {
        let end = usize::min(8192, s.len());
        s[..end].to_string()
    }) else {
        // Unreadable file: do not route to script lane (avoid misrouting app builds on I/O errors).
        return false;
    };
    !head.contains("@page")
}

/// Script-shaped: declares `fn main()` and none of the surfaces the native lane boots.
/// Scans the first 8 KiB like the `@page` heuristic; false positives route to the native
/// lane, which is the safe direction.
pub fn is_script_shaped(head: &str) -> bool {
    let has_main = head.contains("fn main(");
    let service = [
        "@page",
        "\nroutes",
        "\ntable ",
        "\nserver ",
        "\nquery ",
        "\nmutation ",
        "\nactor ",
        "\nworkflow ",
        "\nactivity ",
    ];
    let h = format!("\n{head}");
    has_main && !service.iter().any(|s| h.contains(s))
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

#[cfg(test)]
mod is_script_shaped_tests {
    use super::is_script_shaped;

    #[test]
    fn script_shaped_requires_main_and_rejects_service_surfaces() {
        assert!(is_script_shaped("pub fn main() { print(1) }"));
        assert!(!is_script_shaped("table T { id: Id[T] }\npub fn main() {}"));
        assert!(!is_script_shaped("@page fn home() {}"));
    }
}
