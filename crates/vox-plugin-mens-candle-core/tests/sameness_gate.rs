//! Sameness gate for the Metal/CUDA plugin trees (task E2).
//!
//! Before this crate existed, `vox-plugin-mens-candle-metal/src` and
//! `vox-plugin-mens-candle-cuda/src` had 25 file pairs that were byte-for-byte
//! identical — up to 720 lines each — because a change to one lane routinely
//! never got ported to the other. This test walks both `src/` trees and fails
//! if any file with the same relative path is byte-identical to its
//! counterpart in the other plugin: that is exactly the duplication this
//! crate exists to hold instead, and a new byte-identical pair means the
//! problem is silently regrowing rather than being folded here.
//!
//! This intentionally does NOT check for identical *content* moved into this
//! crate and merely re-exported by both plugins (that's the fix, not the
//! bug) — it only compares the plugins' own `src/` trees against each other.

use std::fs;
use std::path::{Path, PathBuf};

fn plugin_src(name: &str) -> PathBuf {
    // CARGO_MANIFEST_DIR is this crate's own directory
    // (.../crates/vox-plugin-mens-candle-core); the sibling plugin crates
    // live one level up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ parent directory")
        .join(name)
        .join("src")
}

fn walk_files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|e| panic!("read_dir({root:?}): {e}")) {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            walk_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn no_file_is_byte_identical_between_the_metal_and_cuda_plugin_trees() {
    let metal_root = plugin_src("vox-plugin-mens-candle-metal");
    let cuda_root = plugin_src("vox-plugin-mens-candle-cuda");
    assert!(metal_root.is_dir(), "expected {metal_root:?} to exist");
    assert!(cuda_root.is_dir(), "expected {cuda_root:?} to exist");

    let mut metal_files = Vec::new();
    walk_files(&metal_root, &mut metal_files);

    let mut identical_pairs = Vec::new();
    for metal_file in &metal_files {
        let rel = metal_file
            .strip_prefix(&metal_root)
            .expect("file under metal_root");
        let cuda_file = cuda_root.join(rel);
        if !cuda_file.is_file() {
            continue;
        }
        let metal_bytes = fs::read(metal_file).expect("read metal file");
        let cuda_bytes = fs::read(&cuda_file).expect("read cuda file");
        if metal_bytes == cuda_bytes {
            identical_pairs.push(rel.display().to_string());
        }
    }

    assert!(
        identical_pairs.is_empty(),
        "these files are byte-identical between vox-plugin-mens-candle-metal/src \
         and vox-plugin-mens-candle-cuda/src — fold them into \
         vox-plugin-mens-candle-core instead of letting the duplication regrow: {identical_pairs:#?}"
    );
}
