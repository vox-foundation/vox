//! Hermetic test home for vox-actor-runtime integration tests (Task 13), the
//! twin of `src/lib.rs` `hermetic_test_env`: `VOX_HOME` points at an empty
//! per-process temp dir before any test runs, so no test reads the developer's
//! real `~/.vox/config.toml` (which may pin `VOX_MODEL_FORCE`). Pulled in with
//! `mod common;`. The live `openrouter_free_floor_smoke` deliberately omits it.

fn vox_home_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "vox-actor-runtime-inttest-home-{}",
        std::process::id()
    ))
}

#[ctor::ctor(unsafe)]
#[allow(unsafe_code)] // SAFETY: ctors run pre-main, single-threaded, before any test thread.
fn set_hermetic_vox_home() {
    let dir = vox_home_dir();
    let _ = std::fs::create_dir_all(&dir);
    unsafe { std::env::set_var("VOX_HOME", &dir) };
}

#[dtor::dtor]
fn remove_hermetic_vox_home() {
    let _ = std::fs::remove_dir_all(vox_home_dir());
}
