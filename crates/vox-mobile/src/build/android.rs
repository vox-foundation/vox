//! Android build path: cargo-ndk per ABI.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;
use vox_pm::manifest::AndroidConfig;

pub fn build(project_dir: &Path, android: &AndroidConfig, release: bool) -> Result<()> {
    if android.abis.is_empty() {
        bail!("[mobile.android].abis is empty; nothing to build");
    }
    let out_root = project_dir.join("target/mobile/android");
    std::fs::create_dir_all(&out_root)
        .with_context(|| format!("creating {}", out_root.display()))?;

    for abi in &android.abis {
        eprintln!("[vox-mobile] building Android {abi}");
        let mut cmd = Command::new("cargo-ndk");
        cmd.current_dir(project_dir)
            .arg("--target")
            .arg(abi)
            .arg("--platform")
            .arg(android.min_sdk.unwrap_or(26).to_string())
            .arg("--output-dir")
            .arg(out_root.join(abi))
            .arg("--")
            .arg("build");
        if release {
            cmd.arg("--release");
        }
        let status = cmd
            .status()
            .with_context(|| format!("invoking cargo-ndk for {abi}"))?;
        if !status.success() {
            bail!(
                "cargo-ndk build failed for ABI {abi}: exit {}",
                status.code().unwrap_or(-1)
            );
        }
    }
    eprintln!(
        "[vox-mobile] Android build complete: {}",
        out_root.display()
    );
    Ok(())
}
