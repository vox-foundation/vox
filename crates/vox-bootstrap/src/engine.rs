//! Bootstrap evaluation: probe host toolchain; optional `--apply` runs low-risk heals.

use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Cursor, Read, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tar::Archive;
use zip::ZipArchive;

use crate::report::{BootstrapItem, BootstrapReport};

/// CLI-driven options for probing (and optionally fixing) the host toolchain.
#[derive(Debug, Clone)]
pub struct BootstrapOptions {
    /// Include dev probes (`rustfmt`, `clippy`).
    pub dev: bool,
    /// Treat LLVM/Clang as required on Windows (Turso / aegis native builds).
    pub install_clang: bool,
    /// Run safe heals (`rustup component add`, etc.).
    pub apply: bool,
    /// Install the vox CLI via cargo after successful checks.
    pub install: bool,
    /// Skip release-binary install and force source install.
    pub source_only: bool,
    /// Optional release version/tag override (`v1.2.3`).
    pub version: Option<String>,
}

fn platform_str() -> String {
    if cfg!(target_os = "windows") {
        "windows".into()
    } else if cfg!(target_os = "linux") {
        "linux".into()
    } else if cfg!(target_os = "macos") {
        "macos".into()
    } else {
        "other".into()
    }
}

fn run_cmd(bin: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(if detail.is_empty() {
            format!("exit code {}", out.status)
        } else {
            detail
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Evaluate all probes. Mutates the system only when `opts.apply` runs component installs.
#[must_use]
pub fn evaluate(opts: BootstrapOptions) -> BootstrapReport {
    let mut items = Vec::new();

    let rustc_ok = run_cmd("rustc", &["--version"]);
    items.push(BootstrapItem {
        id: "rustc",
        description: "Rust compiler (`rustc --version`)",
        required: true,
        ok: rustc_ok.is_ok(),
        detail: rustc_ok.unwrap_or_else(|e| e),
        heal_command: Some("https://rustup.rs/ — then open a new shell".to_string()),
    });

    let cargo_ok = run_cmd("cargo", &["--version"]);
    items.push(BootstrapItem {
        id: "cargo",
        description: "Cargo (`cargo --version`)",
        required: true,
        ok: cargo_ok.is_ok(),
        detail: cargo_ok.unwrap_or_else(|e| e),
        heal_command: Some("rustup default stable (or reinstall from rustup.rs)".to_string()),
    });

    if opts.dev {
        let fmt_ok = run_cmd("rustfmt", &["--version"]);
        let fmt_heal = "rustup component add rustfmt";
        if opts.apply && fmt_ok.is_err() {
            let _ = Command::new("rustup")
                .args(["component", "add", "rustfmt"])
                .status();
        }
        let fmt_ok_after = run_cmd("rustfmt", &["--version"]);
        items.push(BootstrapItem {
            id: "rustfmt",
            description: "rustfmt (`rustfmt --version`)",
            required: false,
            ok: fmt_ok_after.is_ok(),
            detail: fmt_ok_after.unwrap_or_else(|e| e),
            heal_command: Some(fmt_heal.to_string()),
        });

        let clippy_out = Command::new("cargo").args(["clippy", "--version"]).output();
        let (ok_before, detail_before) = match &clippy_out {
            Ok(o) if o.status.success() => {
                (true, String::from_utf8_lossy(&o.stdout).trim().to_string())
            }
            Ok(o) => (false, String::from_utf8_lossy(&o.stderr).trim().to_string()),
            Err(e) => (false, e.to_string()),
        };
        if opts.apply && !ok_before {
            let _ = Command::new("rustup")
                .args(["component", "add", "clippy"])
                .status();
        }
        let clippy_after = Command::new("cargo").args(["clippy", "--version"]).output();
        let (ok, detail) = match clippy_after {
            Ok(o) if o.status.success() => {
                (true, String::from_utf8_lossy(&o.stdout).trim().to_string())
            }
            Ok(_) => (false, detail_before.clone()),
            Err(e) => (false, e.to_string()),
        };
        items.push(BootstrapItem {
            id: "clippy",
            description: "Clippy (`cargo clippy --version`)",
            required: false,
            ok,
            detail,
            heal_command: Some("rustup component add clippy".to_string()),
        });
    }

    if cfg!(target_os = "windows") {
        let clang =
            run_cmd("clang-cl", &["--version"]).or_else(|_| run_cmd("clang", &["--version"]));
        items.push(BootstrapItem {
            id: "turso_clang",
            description: "LLVM/Clang for Turso aegis (`clang-cl` or `clang` on PATH)",
            required: opts.install_clang,
            ok: clang.is_ok(),
            detail: clang.unwrap_or_else(|e| e),
            heal_command: Some(
                "winget install -e LLVM.LLVM — then add LLVM\\bin to PATH and restart the shell"
                    .to_string(),
            ),
        });
    }

    BootstrapReport {
        platform: platform_str(),
        items,
    }
}

/// Print a human-readable report; returns process exit code (`0` = all required probes passed).
pub fn run_and_print(opts: BootstrapOptions, w: &mut impl Write) -> io::Result<i32> {
    let report = evaluate(opts.clone());
    writeln!(w, "Platform: {}", report.platform)?;
    for item in &report.items {
        let status = if item.ok { "OK" } else { "FAIL" };
        writeln!(w, "  [{status}] {} — {}", item.description, item.detail)?;
        if !item.ok
            && let Some(ref h) = item.heal_command
        {
            writeln!(w, "       hint: {h}")?;
        }
    }
    let ok = report.required_ok();
    if ok && opts.install {
        writeln!(w, "\nDependencies met. Installing vox-cli...")?;
        if opts.source_only {
            install_from_source(w)?;
        } else {
            match install_from_binary(opts.version.as_deref(), w) {
                Ok(()) => {}
                Err(e) => {
                    writeln!(w, "Binary install unavailable: {e}")?;
                    writeln!(
                        w,
                        "Falling back to source install (`cargo install --path crates/vox-cli`)..."
                    )?;
                    install_from_source(w)?;
                }
            }
        }
    }
    Ok(if ok { 0 } else { 1 })
}

fn resolve_vox_repo_root() -> io::Result<PathBuf> {
    if let Ok(p) = std::env::var("VOX_REPO_ROOT") {
        let pb = PathBuf::from(p.trim());
        if pb.join("crates/vox-cli/Cargo.toml").is_file() {
            return Ok(pb);
        }
        return Err(io::Error::other(format!(
            "VOX_REPO_ROOT does not contain crates/vox-cli/Cargo.toml ({})",
            pb.display()
        )));
    }
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("crates/vox-cli/Cargo.toml").is_file() && dir.join("Cargo.toml").is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err(io::Error::other(
                "could not find Vox repository root (expected crates/vox-cli/Cargo.toml). \
                 Run from a clone of the repo, or set VOX_REPO_ROOT to the repo root.",
            ));
        }
    }
}

fn install_from_source(w: &mut impl Write) -> io::Result<()> {
    let repo_root = resolve_vox_repo_root()?;
    writeln!(
        w,
        "  Installing from source using repo root {}",
        repo_root.display()
    )?;
    let status = Command::new("cargo")
        .current_dir(&repo_root)
        .args(["install", "--path", "crates/vox-cli"])
        .status()?;
    if !status.success() {
        writeln!(w, "Failed to install vox-cli from source")?;
        return Err(io::Error::other("source install command failed"));
    }
    writeln!(w, "vox-cli installed from source successfully.")?;
    Ok(())
}

fn install_from_binary(version: Option<&str>, w: &mut impl Write) -> io::Result<()> {
    let target = host_target_triple()
        .ok_or_else(|| io::Error::other("unsupported host target for binary installer"))?;
    let ext = if target.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    };
    writeln!(
        w,
        "  Attempting release binary install for target `{target}`..."
    )?;

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(io::Error::other)?;
    let tag = resolve_release_tag(&client, version)?;
    let asset_name = format!("vox-{tag}-{target}.{ext}");
    let (asset_url, checksums_url) = if version.is_some() {
        release_download_urls(&tag, &asset_name)
    } else {
        latest_download_urls(&asset_name)
    };

    let asset_bytes = http_get_bytes(&client, &asset_url)?;
    let checksums = http_get_text(&client, &checksums_url)?;
    verify_checksum(&asset_bytes, &checksums, &asset_name)?;

    let install_dir = install_bin_dir()?;
    fs::create_dir_all(&install_dir)?;
    let dst = install_dir.join(if target.contains("windows") {
        "vox.exe"
    } else {
        "vox"
    });
    extract_binary(&asset_bytes, target, &dst)?;
    writeln!(w, "  Installed binary to {}", dst.display())?;
    Ok(())
}

fn resolve_release_tag(client: &Client, version: Option<&str>) -> io::Result<String> {
    if let Some(v) = version {
        return Ok(if v.starts_with('v') {
            v.to_string()
        } else {
            format!("v{v}")
        });
    }
    let url = "https://api.github.com/repos/vox-foundation/vox/releases/latest";
    let resp = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "vox-bootstrap")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .map_err(io::Error::other)?;
    if !resp.status().is_success() {
        return Err(io::Error::other(format!(
            "GitHub API GET {url} failed: {}",
            resp.status()
        )));
    }
    let val: serde_json::Value = resp.json().map_err(io::Error::other)?;
    let tag = val
        .get("tag_name")
        .and_then(|t| t.as_str())
        .ok_or_else(|| io::Error::other("GitHub releases/latest: missing tag_name"))?;
    Ok(tag.to_string())
}

fn release_download_urls(tag: &str, asset_name: &str) -> (String, String) {
    let base = "https://github.com/vox-foundation/vox/releases";
    (
        format!("{base}/download/{tag}/{asset_name}"),
        format!("{base}/download/{tag}/checksums.txt"),
    )
}

fn latest_download_urls(asset_name: &str) -> (String, String) {
    let base = "https://github.com/vox-foundation/vox/releases";
    (
        format!("{base}/latest/download/{asset_name}"),
        format!("{base}/latest/download/checksums.txt"),
    )
}

fn http_get_bytes(client: &Client, url: &str) -> io::Result<Vec<u8>> {
    let resp = client
        .get(url)
        .header("User-Agent", "vox-bootstrap")
        .send()
        .map_err(io::Error::other)?;
    if !resp.status().is_success() {
        return Err(io::Error::other(format!(
            "download failed ({url}): {}",
            resp.status()
        )));
    }
    resp.bytes().map(|b| b.to_vec()).map_err(io::Error::other)
}

fn http_get_text(client: &Client, url: &str) -> io::Result<String> {
    let resp = client
        .get(url)
        .header("User-Agent", "vox-bootstrap")
        .send()
        .map_err(io::Error::other)?;
    if !resp.status().is_success() {
        return Err(io::Error::other(format!(
            "download failed ({url}): {}",
            resp.status()
        )));
    }
    resp.text().map_err(io::Error::other)
}

fn sample_checksum_basenames(checksums_txt: &str, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in checksums_txt.lines() {
        let mut parts = line.split_whitespace();
        if parts.next().is_none() {
            continue;
        }
        if let Some(file) = parts.next() {
            let base = Path::new(file)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(file);
            out.push(base.to_string());
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

fn verify_checksum(asset_bytes: &[u8], checksums_txt: &str, asset_name: &str) -> io::Result<()> {
    let expected = checksum_for_asset(checksums_txt, asset_name).ok_or_else(|| {
        let sample = sample_checksum_basenames(checksums_txt, 5);
        io::Error::other(format!(
            "checksum entry not found for {asset_name} in checksums.txt (first basenames in file: {sample:?})"
        ))
    })?;
    let actual = sha256_hex(asset_bytes);
    if actual != expected {
        return Err(io::Error::other(format!(
            "checksum mismatch for {asset_name} (expected {expected}, got {actual})"
        )));
    }
    Ok(())
}

fn checksum_for_asset(checksums_txt: &str, asset_name: &str) -> Option<String> {
    for line in checksums_txt.lines() {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let file = parts.next()?;
        let file_name = Path::new(file)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(file);
        if file_name == asset_name {
            return Some(hash.to_lowercase());
        }
    }
    None
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn extract_binary(archive: &[u8], target: &str, destination: &Path) -> io::Result<()> {
    if target.contains("windows") {
        extract_zip_binary(archive, destination)
    } else {
        extract_tar_binary(archive, destination)
    }
}

fn extract_zip_binary(archive: &[u8], destination: &Path) -> io::Result<()> {
    let cursor = Cursor::new(archive);
    let mut zip = ZipArchive::new(cursor).map_err(io::Error::other)?;
    let mut file = zip
        .by_name("vox.exe")
        .map_err(|e| io::Error::other(format!("vox.exe not found in zip: {e}")))?;
    write_reader_to_path(&mut file, destination)?;
    Ok(())
}

fn extract_tar_binary(archive: &[u8], destination: &Path) -> io::Result<()> {
    let cursor = Cursor::new(archive);
    let decoder = GzDecoder::new(cursor);
    let mut tar = Archive::new(decoder);
    for entry in tar.entries().map_err(io::Error::other)? {
        let mut entry = entry.map_err(io::Error::other)?;
        let path = entry.path().map_err(io::Error::other)?;
        let is_vox = path
            .file_name()
            .and_then(|f| f.to_str())
            .map(|f| f == "vox")
            .unwrap_or(false);
        if is_vox {
            write_reader_to_path(&mut entry, destination)?;
            return Ok(());
        }
    }
    Err(io::Error::other("vox binary not found in tar.gz"))
}

fn write_reader_to_path(reader: &mut impl Read, destination: &Path) -> io::Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::other("install destination has no parent directory"))?;
    let name = destination
        .file_name()
        .ok_or_else(|| io::Error::other("install destination has no file name"))?;
    let tmp = parent.join(format!(".{}.vox-install-tmp", name.to_string_lossy()));

    let result = (|| -> io::Result<()> {
        let mut out = fs::File::create(&tmp)?;
        io::copy(reader, &mut out)?;
        drop(out);
        #[cfg(unix)]
        {
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&tmp, perms)?;
        }
        if destination.exists() {
            fs::remove_file(destination)?;
        }
        fs::rename(&tmp, destination)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn install_bin_dir() -> io::Result<PathBuf> {
    if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
        return Ok(PathBuf::from(cargo_home).join("bin"));
    }
    if cfg!(target_os = "windows") {
        let user =
            std::env::var("USERPROFILE").map_err(|_| io::Error::other("USERPROFILE is not set"))?;
        Ok(PathBuf::from(user).join(".cargo").join("bin"))
    } else {
        let home = std::env::var("HOME").map_err(|_| io::Error::other("HOME is not set"))?;
        Ok(PathBuf::from(home).join(".cargo").join("bin"))
    }
}

fn host_target_triple() -> Option<&'static str> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return Some("x86_64-unknown-linux-gnu");
    }
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        return Some("x86_64-pc-windows-msvc");
    }
    if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        return Some("x86_64-apple-darwin");
    }
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Some("aarch64-apple-darwin");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{checksum_for_asset, latest_download_urls, release_download_urls, sha256_hex};

    #[test]
    fn release_urls_support_latest_and_tagged() {
        let (latest_asset, latest_checksums) =
            latest_download_urls("vox-v1.2.3-x86_64-unknown-linux-gnu.tar.gz");
        assert!(latest_asset.contains("/latest/download/"));
        assert!(latest_checksums.ends_with("/latest/download/checksums.txt"));

        let (tagged_asset, tagged_checksums) =
            release_download_urls("v1.2.3", "vox-v1.2.3-x86_64-unknown-linux-gnu.tar.gz");
        assert!(tagged_asset.contains("/download/v1.2.3/"));
        assert!(tagged_checksums.contains("/download/v1.2.3/checksums.txt"));
    }

    #[test]
    fn checksum_lookup_accepts_path_prefix() {
        let txt =
            "abc123  release-x86_64-unknown-linux-gnu/vox-v1.2.3-x86_64-unknown-linux-gnu.tar.gz\n";
        let found = checksum_for_asset(txt, "vox-v1.2.3-x86_64-unknown-linux-gnu.tar.gz");
        assert_eq!(found.as_deref(), Some("abc123"));
    }

    #[test]
    fn sha256_hex_has_expected_length() {
        let h = sha256_hex(b"vox");
        assert_eq!(h.len(), 64);
    }
}
