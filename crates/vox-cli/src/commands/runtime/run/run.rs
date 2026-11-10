//! `vox run` — native (non-script) web-app mode runner.
//!
//! Used when the source file is detected as a web app (`@page` declarations).

use anyhow::Result;
use std::path::Path;
use vox_bounded_fs::read_utf8_path_capped;

pub use vox_cli_core::fs_utils::{SERVICE_SURFACE_MARKERS, has_service_surface, is_script_shaped};

/// Returns `true` if `file` should be executed as a standalone script rather
/// than a web-app dev server, using the **service-surface** heuristic (see
/// `has_service_surface`) on the first 8 KiB (no full parse). Prefer
/// `Vox.toml` `[web] run_mode` when that scan is insufficient
/// (`vox_config::WebRunMode`).
pub fn is_script_file_by_page_heuristic(file: &Path) -> bool {
    let Ok(head) = read_utf8_path_capped(file).map(|s| {
        let end = usize::min(8192, s.len());
        s[..end].to_string()
    }) else {
        return false;
    };
    !has_service_surface(&head)
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

#[cfg(test)]
mod page_heuristic_app_lane_tests {
    use super::is_script_file_by_page_heuristic;

    // Regression test for: `vox init my-app && cd my-app && vox run src/main.vox`
    // failing with "No fn main() found". The scaffolded starter template
    // (table/server/component/routes, no `@page`, no `fn main()`) was routed
    // to the script lane by a heuristic that only recognized `@page` as an
    // app-lane signal, so it always looked script-shaped for a whole class
    // of valid app files.
    #[test]
    fn scaffolded_default_app_template_stays_on_app_lane() {
        let dir = tempfile::tempdir().unwrap();
        vox_project_scaffold::scaffold_vox_project_at(dir.path(), "my-app", "application", None)
            .expect("scaffold vox init's default project");
        let main_file = dir.path().join("src/main.vox");
        assert!(
            main_file.is_file(),
            "scaffold should have written src/main.vox"
        );
        assert!(
            !is_script_file_by_page_heuristic(&main_file),
            "vox init's default full-stack template declares table/server/component/routes \
             and must route to the app lane, not the script lane"
        );
    }
}
