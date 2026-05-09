//! Build script for `vox-cli`.
//!
//! Delegates version metadata injection to `vox-build-meta`, then handles
//! CLI-specific concerns (Windows stack size, registry rerun trigger).
fn main() {
    vox_build_meta::emit();

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

    println!("cargo:rerun-if-changed=build.rs");
    // `command_contract` embeds this file; rebuild CLI when registry changes.
    println!("cargo:rerun-if-changed=../../contracts/cli/command-registry.yaml");
}
