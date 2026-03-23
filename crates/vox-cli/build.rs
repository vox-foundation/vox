//! Build script for `vox-cli` to inject versioning and build metadata.
//!
//! This script captures the Git commit count and short hash to provide
//! human-readable build information via the `vox --version` command.
use std::process::Command;

fn main() {
    // Emit the git commit count as VOX_BUILD_NUMBER.
    // Every merged commit increments this automatically — no manual maintenance needed.
    let build_number = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_string());

    println!("cargo:rustc-env=VOX_BUILD_NUMBER={build_number}");

    // Emit the short git hash for diagnostics (optional, never version-bumped).
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=VOX_GIT_HASH={git_hash}");

    // Windows default stack (~1 MiB) overflows clap help generation for the large `Cli` enum.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        if target_env == "gnu" {
            println!("cargo:rustc-link-arg=-Wl,--stack,8388608");
        } else {
            println!("cargo:rustc-link-arg=/STACK:8388608");
        }
    }

    // Re-run when build script changes. Avoid .git paths when index may be corrupted.
    println!("cargo:rerun-if-changed=build.rs");
}
