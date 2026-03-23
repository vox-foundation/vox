//! Shared Cargo `target/` directories for script-mode builds (native vs WASI lanes).

use std::path::PathBuf;

/// Build lane for script compilation — native host binary vs `wasm32-wasip1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildLane {
    ScriptNative,
    ScriptWasi,
}

/// Process-local tag used when coordinating lock files (diagnostics / future file locks).
#[must_use]
pub fn lane_isolation() -> u64 {
    u64::from(std::process::id())
}

/// Resolve the shared Cargo `target` directory for script builds under `~/.vox/`.
#[must_use]
pub fn resolve_target_dir(lane: BuildLane, _workspace_label: &str, _isolation: u64) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let subdir = match lane {
        BuildLane::ScriptNative => "script-target",
        BuildLane::ScriptWasi => "script-target-wasi",
    };
    home.join(".vox").join(subdir)
}
