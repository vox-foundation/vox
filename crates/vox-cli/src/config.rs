//! Centralized config and env overrides for Vox CLI.

/// Default HTTP port for run/dev/bundle servers. Override with `VOX_PORT`.
pub const DEFAULT_PORT: u16 = 3000;

/// Resolve server port: `VOX_PORT` env, else [`DEFAULT_PORT`].
pub fn default_port() -> u16 {
    std::env::var("VOX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

/// Sets `VOX_PORT` for the rest of this process (used by `vox-compilerd` before `run` / `dev`).
///
/// # Safety
///
/// `std::env::set_var` is `unsafe` in Rust 2024+ because concurrent environment access is UB.
/// Callers must ensure no other thread reads or writes the process environment at the same time.
/// The `vox-compilerd` binary handles one stdio request at a time on the main task, which satisfies this.
#[allow(unsafe_code)]
pub fn set_process_vox_port(port: u16) {
    // SAFETY: `vox-compilerd` does not spawn concurrent env access; the thin `vox` client does not
    // call this (it relies on `default_port()` / user shell env).
    unsafe {
        std::env::set_var("VOX_PORT", port.to_string());
    }
}

/// Set an environment variable for the current process (Rust 2024: `set_var` is `unsafe`).
///
/// # Safety
///
/// Call only when no other thread concurrently reads or writes the process environment
/// (typical: main thread during CLI startup before spawning workers).
#[allow(unsafe_code)]
#[allow(unsafe_op_in_unsafe_fn)]
pub(crate) unsafe fn set_process_env(key: &str, value: &str) {
    std::env::set_var(key, value);
}
