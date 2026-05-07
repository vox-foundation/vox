//! End-to-end: load the deliberately-mismatched bad-abi dylib, assert the
//! loader returns AbiMismatch and the plugin_abi field is the bad value.

use std::path::PathBuf;
use vox_plugin_host::{errors::LoadError, Loader};

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
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
    let root = workspace_root();
    let filename = dylib_filename(crate_name);
    for profile in ["debug", "release"] {
        let p = root.join("target").join(profile).join(&filename);
        if p.exists() {
            return p;
        }
    }
    panic!("build {crate_name} first: `cargo build -p {crate_name}`");
}

#[test]
fn rejects_mismatched_abi() {
    let dylib = built_dylib("vox-plugin-noop-code-bad-abi");
    let result = Loader::load("noop-bad-abi", "0.1.0", &dylib);
    match result {
        Err(LoadError::AbiMismatch(e)) => {
            assert_eq!(e.plugin_abi, 999_999);
            assert_eq!(e.host_abi, 10);
        }
        Ok(_) => panic!("expected AbiMismatch, got Ok"),
        Err(other) => panic!("expected AbiMismatch, got {other:?}"),
    }
}
