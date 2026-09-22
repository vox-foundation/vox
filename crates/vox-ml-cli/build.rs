//! Build script for `vox-ml-cli`: embeds the short git hash as `VOX_GIT_HASH`
//! so `vox` can detect a stale sidecar (see `vox_cli_core::ml_cli_handshake`).
// vox:defactored-from vox-build-meta 2026-09-22 (git-hash half of `emit`;
// avoids a new vox-ml-cli -> vox-build-meta crate edge).
fn main() {
    // vox-arch-check: allow git-exec
    let hash = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=VOX_GIT_HASH={hash}");
    // Same invalidation as vox-build-meta: rerun when the branch tip changes.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");
}
