//! vox-build-meta's feature-probing build script has been retired. Optional
//! capabilities are now installable plugins managed by `vox plugin install`.
//!
//! See: docs/src/architecture/plugin-system-redesign-2026.md

fn main() {
    // Keep the env var defined so existing call sites compile, but always
    // emit the empty list. Real capability discovery now goes through
    // vox-plugin-host (lands in SP2).
    println!("cargo:rustc-env=VOX_BUILD_FEATURES=[]");
}
