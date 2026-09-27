//! Build script for `vox-codegen`.
//!
//! Embeds the absolute path to the vox workspace root this crate was
//! compiled from (`VOX_COMPILER_REPO_ROOT`, read back via `env!()`), so
//! generated projects' `Cargo.toml` can point path dependencies at the
//! local vox checkout instead of a fragile `../../crates/...` relative
//! path that only resolves inside this repo. See `resolve_vox_repo_root`
//! in `src/codegen_rust/emit/mod.rs` and
//! `docs/src/architecture/generated-project-runtime-deps.md`.
// vox:defactored-from vox-build-meta 2026-09-22 (avoids a vox-codegen -> vox-build-meta edge)
fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("cargo sets CARGO_MANIFEST_DIR for build scripts");
    // <repo_root>/crates/vox-codegen -> <repo_root>
    let dir = std::path::Path::new(&manifest_dir);
    let root = dir
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap_or(dir);
    println!("cargo:rustc-env=VOX_COMPILER_REPO_ROOT={}", root.display());
    println!("cargo:rerun-if-changed=build.rs");
}
