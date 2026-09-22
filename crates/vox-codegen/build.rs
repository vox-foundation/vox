//! Build script for `vox-codegen`.
//!
//! Embeds the absolute path to the vox workspace root this crate was
//! compiled from (`VOX_COMPILER_REPO_ROOT`, read back via `env!()`), so
//! generated projects' `Cargo.toml` can point path dependencies at the
//! local vox checkout instead of a fragile `../../crates/...` relative
//! path that only resolves inside this repo. See `resolve_vox_repo_root`
//! in `src/codegen_rust/emit/mod.rs` and
//! `docs/src/architecture/generated-project-runtime-deps.md`.
fn main() {
    vox_build_meta::emit_repo_root();
    println!("cargo:rerun-if-changed=build.rs");
}
