//! Task 8e fix round 1: hermetic `VOX_HOME` (and disabled live web research)
//! for every `vox-orchestrator-mcp` integration-test binary.
//!
//! `crates/vox-orchestrator-mcp/src/lib.rs`'s `#[cfg(test)]`-gated ctor
//! (`hermetic_test_env::set_hermetic_vox_home`) only compiles into the
//! crate's *unit*-test binary (`cargo test -p vox-orchestrator-mcp --lib`).
//! Every file directly under `tests/` is its own separate binary that links
//! `vox-orchestrator-mcp` as an ordinary (non-`cfg(test)`) rlib — so that
//! code never reaches these binaries, and several of them (the ones that
//! build a `ServerState` via `ServerState::new_full(load_config())`) fell
//! back to reading the developer's real `~/.vox/config.toml`.
//!
//! Rust's `tests/common/mod.rs` naming convention is special-cased by cargo:
//! unlike a bare `tests/common.rs`, this file does **not** become its own
//! test binary — it is only a module other integration-test files can pull
//! in with `mod common;`. Each integration-test file in this crate does
//! exactly that (nothing else — the `#[ctor]`/`#[dtor]` below run on their
//! own).
//!
//! `#[ctor]` functions run before `main`, i.e. before this binary's test
//! harness starts any `#[test]`/`#[tokio::test]` thread, so a single
//! `mod common;` line is sufficient per binary — no per-test call needed,
//! and (unlike a `std::sync::Once` invoked from inside a test body) there is
//! no race with some other test in the same binary reaching
//! `vox_config::toml_config::load_user_config()` first.
//!
//! `vox_home_is_hermetic` below is compiled into (and runs in) every binary
//! that does `mod common;`, proving per-binary coverage: it fails loudly if
//! `dot_vox_user_dir()` ever resolves under the real `$HOME/.vox` again.

/// The directory this binary's `VOX_HOME` points at. Deterministic from the
/// process id alone (stable for the process's whole lifetime), so the
/// matching `#[dtor]` below can remove it without any shared state.
fn vox_home_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "vox-orchestrator-mcp-inttest-home-{}",
        std::process::id()
    ))
}

/// Also sets `VOX_SEARCH_WEB_RESEARCH_DISABLED` (Task 8e fix round 1): a
/// `ServerState::new_full(load_config())`-driven integration test can reach
/// `chat_message`'s autonomous-retrieval preamble exactly like a unit test
/// can, so it needs the same test-hermeticity kill switch — see
/// `vox_search::policy::SearchPolicy::from_env` and
/// `crates/vox-orchestrator-mcp/src/lib.rs`'s `hermetic_test_env` doc
/// comment for the full mechanism.
#[ctor::ctor(unsafe)]
#[allow(unsafe_code)] // SAFETY: ctors run pre-main, single-threaded, before any test thread.
fn set_hermetic_vox_home() {
    let dir = vox_home_dir();
    let _ = std::fs::create_dir_all(&dir);
    // SAFETY: see the fn-level comment above.
    unsafe {
        std::env::set_var("VOX_HOME", &dir);
        std::env::set_var("VOX_SEARCH_WEB_RESEARCH_DISABLED", "1");
    }
}

/// Removes the per-process temp `VOX_HOME` at process exit (Task 8e fix
/// round 1) — mirrors `hermetic_test_env::remove_hermetic_vox_home` in
/// `src/lib.rs`; see that fn's doc comment for why this is the standalone
/// `dtor` crate rather than `ctor`'s own attribute (`ctor` 1.x dropped its
/// 0.x `#[dtor]` companion).
#[dtor::dtor]
#[allow(unsafe_code)]
fn remove_hermetic_vox_home() {
    let _ = std::fs::remove_dir_all(vox_home_dir());
}

/// Compiled into every integration-test binary that does `mod common;` —
/// proves, per binary, that this crate's config resolution is not reading
/// the developer's real `~/.vox`. Mutation-guarded in the Task 8e fix-round
/// report: commenting out `set_hermetic_vox_home`'s body reproduces a
/// failure here (and in the crate's other hermeticity-dependent tests).
#[test]
fn vox_home_is_hermetic() {
    let resolved = vox_config::paths::dot_vox_user_dir();
    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        let real_dot_vox = home.join(".vox");
        assert!(
            resolved != real_dot_vox,
            "dot_vox_user_dir() resolved to the developer's real ~/.vox ({resolved:?}) — \
             this integration-test binary is not hermetic (the `mod common;` ctor did not \
             run, or VOX_HOME was cleared after it ran)"
        );
    }
}
