//! Integration test: verifies `vox mobile <args>` dispatches to the
//! `vox-mobile` plugin binary on PATH.

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Locate the cargo target directory containing the test binaries.
/// `CARGO_MANIFEST_DIR` points at `crates/vox-mobile`. We walk up looking
/// for a `target/debug` (or `target/release`) sibling.
fn locate_target_bin_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest.ancestors() {
        for profile in &["debug", "release"] {
            let candidate = ancestor.join("target").join(profile);
            if candidate.exists() {
                return candidate;
            }
        }
    }
    panic!("could not locate target/debug or target/release");
}

#[test]
fn vox_dispatches_mobile_subcommand_to_plugin() {
    let bin_dir = locate_target_bin_dir();

    let vox_exe = if cfg!(windows) {
        bin_dir.join("vox.exe")
    } else {
        bin_dir.join("vox")
    };
    let vox_mobile_exe = if cfg!(windows) {
        bin_dir.join("vox-mobile.exe")
    } else {
        bin_dir.join("vox-mobile")
    };

    if !vox_exe.exists() {
        panic!(
            "expected vox binary at {} - run `cargo build -p vox` first",
            vox_exe.display()
        );
    }
    if !vox_mobile_exe.exists() {
        panic!(
            "expected vox-mobile binary at {} - run `cargo build -p vox-mobile` first",
            vox_mobile_exe.display()
        );
    }

    let path = env::var("PATH").unwrap_or_default();
    let separator = if cfg!(windows) { ';' } else { ':' };
    let new_path = format!("{}{}{}", bin_dir.display(), separator, path);

    let output = Command::new(&vox_exe)
        .env("PATH", new_path)
        .arg("mobile")
        .arg("--help")
        .output()
        .expect("failed to spawn vox");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "vox mobile --help failed: status={:?}\nstdout: {}\nstderr: {}",
        output.status,
        stdout,
        stderr
    );
    assert!(
        stdout.contains("doctor"),
        "expected dispatched help to mention `doctor`; got:\n{}",
        stdout
    );
}
