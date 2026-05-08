//! iOS build path: cargo build per arch + xcodebuild -create-xcframework.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;
use vox_pm::manifest::IosConfig;

pub fn build(project_dir: &Path, ios: &IosConfig, release: bool) -> Result<()> {
    if !cfg!(target_os = "macos") {
        bail!(
            "iOS builds require macOS; current OS is {}",
            std::env::consts::OS
        );
    }
    if ios.archs.is_empty() {
        bail!("[mobile.ios].archs is empty; nothing to build");
    }

    let out_root = project_dir.join("target/mobile/ios");
    std::fs::create_dir_all(&out_root)
        .with_context(|| format!("creating {}", out_root.display()))?;

    // The fixture's package name is `hello_mobile`, but a real app would use its own.
    // We discover the staticlib by reading the Cargo.toml's package name. This is invariant
    // across arches, so hoist it out of the loop.
    let crate_name = read_crate_name(project_dir)?;
    let lib_filename = format!("lib{crate_name}.a");
    let profile = if release { "release" } else { "debug" };
    let mut staticlib_paths: Vec<std::path::PathBuf> = Vec::new();
    for arch in &ios.archs {
        eprintln!("[vox-mobile] building iOS {arch}");
        let mut cmd = Command::new("cargo");
        cmd.current_dir(project_dir)
            .arg("build")
            .arg("--target")
            .arg(arch)
            .arg("--lib");
        if release {
            cmd.arg("--release");
        }
        let status = cmd
            .status()
            .with_context(|| format!("invoking cargo build for {arch}"))?;
        if !status.success() {
            bail!(
                "cargo build failed for arch {arch}: exit {}",
                status.code().unwrap_or(-1)
            );
        }
        let staticlib = project_dir
            .join("target")
            .join(arch)
            .join(profile)
            .join(&lib_filename);
        if !staticlib.exists() {
            bail!(
                "expected staticlib {} after cargo build; not found",
                staticlib.display()
            );
        }
        staticlib_paths.push(staticlib);
    }

    // Assemble the XCFramework. Note: out_root holds the assembled XCFramework;
    // per-arch staticlibs stay in cargo's target/<arch>/<profile>/ tree (asymmetric
    // with the Android path, where out_root holds per-arch artifacts directly).
    let xcf_filename = format!("{crate_name}.xcframework");
    let xcf_path = out_root.join(&xcf_filename);
    if xcf_path.exists() {
        std::fs::remove_dir_all(&xcf_path).context("clearing previous XCFramework")?;
    }
    let mut cmd = Command::new("xcodebuild");
    cmd.arg("-create-xcframework");
    for lib in &staticlib_paths {
        cmd.arg("-library").arg(lib);
    }
    cmd.arg("-output").arg(&xcf_path);
    let status = cmd
        .status()
        .context("invoking xcodebuild -create-xcframework")?;
    if !status.success() {
        bail!(
            "xcodebuild -create-xcframework failed: exit {}",
            status.code().unwrap_or(-1)
        );
    }
    eprintln!(
        "[vox-mobile] iOS build complete: {}",
        xcf_path.display()
    );
    Ok(())
}

/// Read the `[package].name` from the project's Cargo.toml.
/// (We read Cargo.toml here, not Vox.toml, because cargo-emitted libs use the Cargo package name.)
fn read_crate_name(project_dir: &Path) -> Result<String> {
    let cargo_toml = project_dir.join("Cargo.toml");
    let src = std::fs::read_to_string(&cargo_toml)
        .with_context(|| format!("reading {}", cargo_toml.display()))?;
    let parsed: toml::Value = toml::from_str(&src)
        .with_context(|| format!("parsing {}", cargo_toml.display()))?;
    let name = parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| anyhow::anyhow!("Cargo.toml missing [package].name"))?;
    Ok(name.to_string())
}
