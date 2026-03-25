use anyhow::{Context, Result, anyhow};
use flate2::Compression;
use flate2::write::GzEncoder;
use sha2::{Digest, Sha256};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Builder;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

/// Supported release triples (must stay in sync with `vox-bootstrap` and `release-binaries.yml`).
pub const SUPPORTED_RELEASE_TARGETS: &[&str] = &[
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
];

pub(crate) fn validate_release_target(target: &str) -> Result<()> {
    if SUPPORTED_RELEASE_TARGETS.contains(&target) {
        Ok(())
    } else {
        Err(anyhow!(
            "unsupported release target `{target}`; supported: {}",
            SUPPORTED_RELEASE_TARGETS.join(", ")
        ))
    }
}

pub fn run(repo_root: &Path, target: &str, version: Option<&str>, out_dir: &Path) -> Result<()> {
    validate_release_target(target).context("release-build target")?;
    let artifact_version = version.unwrap_or(env!("CARGO_PKG_VERSION"));
    let out_dir_abs = resolve_out_dir(repo_root, out_dir);
    fs::create_dir_all(&out_dir_abs)
        .with_context(|| format!("create out dir {}", out_dir_abs.display()))?;

    let status = Command::new(super::cargo_bin())
        .current_dir(repo_root)
        .args([
            "build",
            "-p",
            "vox-cli",
            "--release",
            "--locked",
            "--target",
            target,
        ])
        .status()
        .context("spawn cargo build for release artifact")?;
    if !status.success() {
        return Err(anyhow!(
            "cargo build failed for target {target} with status {status}"
        ));
    }

    let bin_name = executable_name(target);
    let built_binary = repo_root
        .join("target")
        .join(target)
        .join("release")
        .join(bin_name);
    if !built_binary.is_file() {
        return Err(anyhow!(
            "built binary not found at {}",
            built_binary.display()
        ));
    }

    let artifact_name = artifact_filename(artifact_version, target);
    let artifact_path = out_dir_abs.join(&artifact_name);
    if is_windows_target(target) {
        package_zip(&built_binary, &artifact_path, bin_name)?;
    } else {
        package_tar_gz(&built_binary, &artifact_path, bin_name)?;
    }

    let digest = sha256_file(&artifact_path)?;
    let checksums = out_dir_abs.join("checksums.txt");
    fs::write(&checksums, checksum_line(&digest, &artifact_name))
        .with_context(|| format!("write checksum manifest {}", checksums.display()))?;

    println!("release-build complete");
    println!("  target: {target}");
    println!("  artifact: {}", artifact_path.display());
    println!("  checksums: {}", checksums.display());
    Ok(())
}

fn resolve_out_dir(repo_root: &Path, out_dir: &Path) -> PathBuf {
    if out_dir.is_absolute() {
        out_dir.to_path_buf()
    } else {
        repo_root.join(out_dir)
    }
}

fn is_windows_target(target: &str) -> bool {
    target.contains("windows")
}

fn executable_name(target: &str) -> &'static str {
    if is_windows_target(target) {
        "vox.exe"
    } else {
        "vox"
    }
}

fn artifact_extension(target: &str) -> &'static str {
    if is_windows_target(target) {
        "zip"
    } else {
        "tar.gz"
    }
}

fn artifact_filename(version: &str, target: &str) -> String {
    format!("vox-{version}-{target}.{}", artifact_extension(target))
}

/// Deterministic archive layout for CI: a single member at the archive root named `vox` or `vox.exe`.
fn package_tar_gz(binary_path: &Path, artifact_path: &Path, archive_name: &str) -> Result<()> {
    let artifact = File::create(artifact_path)
        .with_context(|| format!("create archive {}", artifact_path.display()))?;
    let encoder = GzEncoder::new(artifact, Compression::default());
    let mut tar = Builder::new(encoder);
    tar.append_path_with_name(binary_path, archive_name)
        .with_context(|| format!("add {} to tar", binary_path.display()))?;
    tar.finish().context("finish tar archive")?;
    Ok(())
}

fn package_zip(binary_path: &Path, artifact_path: &Path, archive_name: &str) -> Result<()> {
    let artifact = File::create(artifact_path)
        .with_context(|| format!("create archive {}", artifact_path.display()))?;
    let mut zip = zip::ZipWriter::new(artifact);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(archive_name, options)
        .context("start zip file entry")?;

    let mut src = File::open(binary_path)
        .with_context(|| format!("open binary for zipping {}", binary_path.display()))?;
    let mut buf = Vec::new();
    src.read_to_end(&mut buf)
        .with_context(|| format!("read binary {}", binary_path.display()))?;
    zip.write_all(&buf).context("write binary bytes to zip")?;
    zip.finish().context("finish zip archive")?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 8192];
    loop {
        let n = file
            .read(&mut buf)
            .with_context(|| format!("read {}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn checksum_line(sha256_hex: &str, filename: &str) -> String {
    format!("{sha256_hex}  {filename}\n")
}

#[cfg(test)]
mod tests {
    use super::{artifact_filename, checksum_line, executable_name, validate_release_target};

    #[test]
    fn unsupported_target_errors() {
        let err = validate_release_target("riscv64-unknown-linux-gnu").unwrap_err();
        assert!(
            err.to_string().contains("unsupported release target"),
            "{err}"
        );
    }

    #[test]
    fn executable_name_matches_target_family() {
        assert_eq!(executable_name("x86_64-pc-windows-msvc"), "vox.exe");
        assert_eq!(executable_name("x86_64-unknown-linux-gnu"), "vox");
        assert_eq!(executable_name("aarch64-apple-darwin"), "vox");
    }

    #[test]
    fn artifact_filename_contract_is_stable() {
        assert_eq!(
            artifact_filename("v1.2.3", "x86_64-pc-windows-msvc"),
            "vox-v1.2.3-x86_64-pc-windows-msvc.zip"
        );
        assert_eq!(
            artifact_filename("v1.2.3", "x86_64-unknown-linux-gnu"),
            "vox-v1.2.3-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert_eq!(
            artifact_filename("v1.2.3", "aarch64-apple-darwin"),
            "vox-v1.2.3-aarch64-apple-darwin.tar.gz"
        );
    }

    #[test]
    fn checksum_manifest_line_format() {
        let line = checksum_line("deadbeef", "vox-v1.2.3-x86_64-unknown-linux-gnu.tar.gz");
        assert_eq!(
            line,
            "deadbeef  vox-v1.2.3-x86_64-unknown-linux-gnu.tar.gz\n"
        );
    }
}
