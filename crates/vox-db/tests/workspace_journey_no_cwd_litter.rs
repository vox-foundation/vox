//! Regression test for the 2026-09-21 cwd-litter bug: `vox term` / `vox shell repl` /
//! `vox init` (via the shared telemetry-DB open at CLI startup) writing `.vox/store.db`
//! into a fresh, non-project directory. See `crates/vox-db/src/workspace_journey_store.rs`.
//!
//! This test actually `chdir`s into a fresh, non-project tempdir before calling the CLI's
//! entrypoint (`connect_workspace_journey_optional`, no explicit start-dir override) — the
//! repository-discovery walk-up falls back to `std::env::current_dir()` when no `Vox.toml`
//! is found on the way to the filesystem root, so process cwd (not a passed-in path) is what
//! actually drives this bug in production, and is what must be simulated here.

#![allow(unsafe_code)] // test-only env mutation (set_var/remove_var are unsafe on Rust 2024)

use vox_db::{DbConnectSurface, connect_workspace_journey_optional};

/// A fresh, non-git, non-Vox directory must not get a `.vox/` store littered into it —
/// only a real project (git repo or `Vox.toml`) earns a project-local `.vox/store.db`.
#[tokio::test(flavor = "multi_thread")]
async fn non_project_dir_gets_no_dot_vox_store() {
    let scratch = tempfile::tempdir().expect("scratch tempdir");
    let user_data = tempfile::tempdir().expect("user data tempdir");
    let original_cwd = std::env::current_dir().expect("original cwd");

    // Isolate the canonical fallback to a throwaway dir too, so this test never touches
    // the real $HOME/.vox and can assert the fallback path was actually exercised.
    // SAFETY: test-only process, single test in this binary, no concurrent env/cwd readers.
    unsafe {
        std::env::set_var("VOX_DATA_DIR", user_data.path());
    }
    std::env::set_current_dir(scratch.path()).expect("chdir into scratch");

    let _ = connect_workspace_journey_optional(DbConnectSurface::CliWorkspace, true).await;

    std::env::set_current_dir(&original_cwd).expect("restore cwd");
    unsafe {
        std::env::remove_var("VOX_DATA_DIR");
    }

    assert!(
        !scratch.path().join(".vox").exists(),
        ".vox/ must not be created under a non-project directory"
    );
}
