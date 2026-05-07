//! Manifest-driven Cargo.toml emission for the mobile / server / client targets.
//!
//! This is a parallel path to the HIR-driven full-app emitter in
//! `vox_compiler::codegen_rust`. For Phase 1 of the vox-mobile plugin, only the
//! mobile target is exercised; other targets fall through to a baseline shape.
//!
//! Note on location: this facade was originally specified to live in
//! `vox_compiler::codegen`, but adding a `vox-pm` dependency to `vox-compiler`
//! creates a cargo dependency cycle (`vox-compiler -> vox-pm -> vox-db ->
//! vox-compiler`). It lives here in `vox-pm` instead, which already owns
//! `VoxManifest` and has no inbound cycle to `vox-compiler`.

use crate::manifest::VoxManifest;

/// Emit a Cargo.toml string for the given manifest, branching on the
/// `[build] target` field. Used by the vox-mobile build pipeline (and
/// subject to future expansion when the server/fullstack/client paths
/// also adopt manifest-driven emission).
pub fn cargo_toml_for_manifest(manifest: &VoxManifest) -> String {
    let target = manifest
        .build
        .as_ref()
        .and_then(|b| b.target.as_deref())
        .unwrap_or("fullstack");

    let name = &manifest.package.name;
    let version = if manifest.package.version.is_empty() {
        "0.1.0"
    } else {
        manifest.package.version.as_str()
    };

    let mut out = String::new();
    out.push_str(&format!(
        "[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2021\"\n\n"
    ));

    match target {
        "mobile" => {
            out.push_str("[lib]\n");
            out.push_str(r#"crate-type = ["cdylib", "staticlib"]"#);
            out.push('\n');
            out.push('\n');
            out.push_str("[dependencies]\n");
            out.push_str("vox-runtime = { workspace = true }\n");
            out.push_str("vox-oratio = { workspace = true, features = [\"stt-sherpa\"] }\n");
            out.push_str("vox-crypto = { workspace = true }\n");
            out.push_str("vox-db = { workspace = true }\n");
            out.push('\n');
            out.push_str("[target.'cfg(target_os = \"android\")'.dependencies]\n");
            out.push_str("jni = \"0.21\"\n");
        }
        _ => {
            // Default / server / client / fullstack: baseline shape — a binary crate
            // with the runtime as its only dep. Future tasks may expand this.
            out.push_str("[dependencies]\nvox-runtime = { workspace = true }\n");
        }
    }

    out
}
