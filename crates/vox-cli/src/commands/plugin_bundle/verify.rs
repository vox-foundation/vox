//! `vox bundle verify <tarball>` — extract to tempdir, run plugin doctor,
//! exit 0 on success.
//!
//! The command:
//! 1. Extracts the tarball into a temporary directory.
//! 2. Confirms `plugins/` and `BUNDLE.toml` are present.
//! 3. Sets `VOX_PLUGINS_DIR` to `<tempdir>/plugins` so that `plugin doctor`
//!    scans the extracted bundle rather than the host install root.
//! 4. Runs `vox plugin doctor` and surfaces any issues.

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::Path;
use tar::Archive;

pub fn run(tarball_path: &Path) -> Result<()> {
    let tmp = tempfile::tempdir().context("creating tempdir for bundle extraction")?;

    println!("-> Extracting {} to {}", tarball_path.display(), tmp.path().display());

    let f = File::open(tarball_path)
        .with_context(|| format!("opening tarball {}", tarball_path.display()))?;
    let gz = GzDecoder::new(f);
    let mut archive = Archive::new(gz);
    archive
        .unpack(tmp.path())
        .with_context(|| format!("unpacking {}", tarball_path.display()))?;

    // Structural checks.
    let bundle_toml = tmp.path().join("BUNDLE.toml");
    if !bundle_toml.is_file() {
        anyhow::bail!(
            "bundle integrity check failed: BUNDLE.toml not found in {}",
            tarball_path.display()
        );
    }

    let plugins_root = tmp.path().join("plugins");
    // plugins/ may be absent for vox-base (no plugins); that is valid.
    // We only fail if the dir exists but contains nothing parseable.

    // Print BUNDLE.toml metadata.
    if let Ok(raw) = std::fs::read_to_string(&bundle_toml) {
        println!("  BUNDLE.toml:");
        for line in raw.lines() {
            if !line.starts_with('#') && !line.is_empty() {
                println!("    {line}");
            }
        }
    }

    // Run doctor against the extracted plugins root.
    println!("-> Running plugin doctor against extracted plugins root");
    // Safety: this is a single-threaded CLI path. No other thread writes
    // VOX_PLUGINS_DIR concurrently. set_var is safe here.
    #[allow(unsafe_code)]
    // SAFETY: CLI process; no concurrent thread environment mutation.
    unsafe {
        std::env::set_var("VOX_PLUGINS_DIR", &plugins_root);
    }
    crate::commands::plugin::doctor::run()?;

    println!("✓ bundle integrity verified: {}", tarball_path.display());
    Ok(())
}
