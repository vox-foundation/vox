//! End-to-end: build noop-code (assumed pre-built), copy artifact + manifest
//! to a tempdir, discover, load, exercise the trait object.

use std::path::PathBuf;
use vox_plugin_host::{Loader, discover};

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/vox-plugin-host -> crates/
    p.pop(); // crates/ -> repo root
    p
}

fn dylib_filename(crate_name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}.dll", crate_name.replace('-', "_"))
    } else if cfg!(target_os = "macos") {
        format!("lib{}.dylib", crate_name.replace('-', "_"))
    } else {
        format!("lib{}.so", crate_name.replace('-', "_"))
    }
}

fn built_dylib(crate_name: &str) -> PathBuf {
    // Try debug then release (cargo test usually uses debug).
    let root = workspace_root();
    let filename = dylib_filename(crate_name);
    for profile in ["debug", "release"] {
        let p = root.join("target").join(profile).join(&filename);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "build {crate_name} first: `cargo build --manifest-path crates/vox-plugin-host/tests/fixtures/noop-code/Cargo.toml`. Looked for {filename} in target/debug and target/release.",
    );
}

#[test]
fn end_to_end_load_noop_code() {
    let dylib_src = built_dylib("vox-plugin-noop-code");

    let tmp = tempfile::tempdir().expect("tempdir");
    let plugin_dir = tmp.path().join("noop-code").join("0.1.0");
    std::fs::create_dir_all(&plugin_dir).expect("mkdir");

    let manifest_src = workspace_root()
        .join("crates")
        .join("vox-plugin-host")
        .join("tests")
        .join("fixtures")
        .join("noop-code")
        .join("Plugin.toml");
    std::fs::copy(&manifest_src, plugin_dir.join("Plugin.toml")).expect("copy manifest");
    let dylib_dest = plugin_dir.join(dylib_src.file_name().unwrap());
    std::fs::copy(&dylib_src, &dylib_dest).expect("copy dylib");

    let registry = discover(tmp.path()).expect("discover");
    assert!(
        registry.has("noop-code"),
        "expected noop-code in registry, got {:?}",
        registry.list_ids()
    );

    let loaded = Loader::load("noop-code", "0.1.0", &dylib_dest).expect("load");
    assert_eq!(loaded.plugin.id().as_str(), "noop-code");
    let _ = loaded.plugin.shutdown();
}
