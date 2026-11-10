//! `[python]` discovery, `uv` sync, and the lock hash recorded in `vox.lock`.
//!
//! The BLAKE3 digest is `vox_crypto::secure_hash`. This module does not shell out.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

pub const PY_ENV_MISSING: &str = "py-env: python not configured (add [python] to Vox.toml)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonConfig {
    pub root: PathBuf,
    pub version: Option<String>,
    pub packages: Vec<String>,
    pub extras: Vec<String>,
    pub max_fetch_mb: u64,
    pub timeout_s: u64,
}

/// BLAKE3 of `bytes`, lowercase hex. Same helper `vox py sync` stores in `vox.lock`.
pub fn blake3_hex(bytes: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(bytes))
}

/// Walk upward from `start` for `Vox.toml`. `Ok(None)` when no manifest or no `[python]`.
pub fn discover_python(start: &Path) -> Result<Option<PythonConfig>> {
    let mut dir = if start.is_file() {
        start
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = dir.join("Vox.toml");
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate)
                .with_context(|| format!("read {}", candidate.display()))?;
            let value: toml::Value = text
                .parse()
                .with_context(|| format!("parse {}", candidate.display()))?;
            let Some(section) = value.get("python") else {
                return Ok(None);
            };
            let section = section
                .as_table()
                .context("py-env: [python] must be a table")?;
            return Ok(Some(parse_section(dir, section)?));
        }
        if !dir.pop() {
            return Ok(None);
        }
    }
}

fn parse_section(
    root: PathBuf,
    section: &toml::map::Map<String, toml::Value>,
) -> Result<PythonConfig> {
    let version = match section.get("version") {
        None => None,
        Some(toml::Value::String(s)) => Some(s.clone()),
        Some(_) => bail!("py-env: [python].version must be a string"),
    };
    Ok(PythonConfig {
        root,
        version,
        packages: string_list(section, "packages")?,
        extras: string_list(section, "extras")?,
        max_fetch_mb: int_field(section, "max_fetch_mb", 256)?,
        timeout_s: int_field(section, "timeout_s", 300)?,
    })
}

fn string_list(section: &toml::map::Map<String, toml::Value>, key: &str) -> Result<Vec<String>> {
    match section.get(key) {
        None => Ok(Vec::new()),
        Some(toml::Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .with_context(|| format!("py-env: [python].{key} entries must be strings"))
            })
            .collect(),
        Some(_) => bail!("py-env: [python].{key} must be an array of strings"),
    }
}

fn int_field(
    section: &toml::map::Map<String, toml::Value>,
    key: &str,
    default: u64,
) -> Result<u64> {
    match section.get(key) {
        None => Ok(default),
        Some(toml::Value::Integer(n)) if *n >= 0 => Ok(*n as u64),
        Some(_) => bail!("py-env: [python].{key} must be a non-negative integer"),
    }
}

/// Create `.vox/python`, run `uv lock` and `uv sync --frozen`, record the lock hash.
pub fn sync_project(start: &Path) -> Result<String> {
    let Some(config) = discover_python(start)? else {
        bail!(PY_ENV_MISSING);
    };
    if which::which("uv").is_err() {
        bail!("py-env: uv not found on PATH");
    }
    let dir = config.root.join(".vox").join("python");
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    std::fs::write(dir.join("pyproject.toml"), render_pyproject(&config)?)
        .with_context(|| format!("write {}", dir.join("pyproject.toml").display()))?;
    run_uv(&dir, &["lock"])?;
    run_uv(&dir, &["sync", "--frozen"])?;
    let uv_lock = std::fs::read(dir.join("uv.lock"))
        .with_context(|| format!("read {}", dir.join("uv.lock").display()))?;
    let hash = blake3_hex(&uv_lock);
    record_uv_lock_hash(&config.root, &hash)?;
    Ok(hash)
}

fn render_pyproject(config: &PythonConfig) -> Result<String> {
    let mut deps = String::new();
    for spec in config.packages.iter().chain(config.extras.iter()) {
        deps.push_str("  ");
        deps.push_str(&toml_basic_string(spec)?);
        deps.push_str(",\n");
    }
    let requires = match &config.version {
        Some(version) => {
            let spec = if version.starts_with(|c: char| c.is_ascii_digit()) {
                format!(">={version}")
            } else {
                version.clone()
            };
            format!("requires-python = {}\n", toml_basic_string(&spec)?)
        }
        None => String::new(),
    };
    Ok(format!(
        "[project]\nname = \"vox-python\"\nversion = \"0.0.0\"\n{requires}dependencies = [\n{deps}]\n"
    ))
}

fn toml_basic_string(value: &str) -> Result<String> {
    if value
        .chars()
        .any(|c| c == '"' || c == '\\' || c.is_control())
    {
        bail!("py-env: value {value:?} cannot be written as a TOML string");
    }
    Ok(format!("\"{value}\""))
}

fn run_uv(dir: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("uv")
        .arg("--directory")
        .arg(dir)
        .args(args)
        .output()
        .with_context(|| "py-env: failed to spawn uv")?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("py-env: uv {} failed: {}", args.join(" "), stderr.trim())
}

/// Insert or replace `python_uv_lock_hash` in `vox.lock` using the lockfile's line shape.
pub fn record_uv_lock_hash(project_root: &Path, hash: &str) -> Result<()> {
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("py-env: uv.lock hash must be 64 hex characters");
    }
    let path = project_root.join("vox.lock");
    let mut text = if path.is_file() {
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?
    } else {
        "# This file is auto-generated by VoxPM. Do not edit manually.\n\nversion = 1\n".to_string()
    };
    let line = format!("python_uv_lock_hash = \"{hash}\"");
    if let Some(start) = text.find("python_uv_lock_hash") {
        let end = text[start..]
            .find('\n')
            .map(|i| start + i)
            .unwrap_or(text.len());
        text.replace_range(start..end, &line);
    } else if let Some(vpos) = text.find("version = ") {
        let nl = text[vpos..]
            .find('\n')
            .map(|i| vpos + i)
            .unwrap_or(text.len());
        text.insert_str(nl, &format!("\n{line}"));
    } else {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&line);
        text.push('\n');
    }
    std::fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn read_recorded_hash(lock_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(lock_path).ok()?;
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("python_uv_lock_hash") else {
            continue;
        };
        let rest = rest.trim().strip_prefix('=')?.trim();
        let rest = rest.strip_prefix('"')?.strip_suffix('"')?;
        return Some(rest.to_string());
    }
    None
}

pub fn venv_python(root: &Path) -> Option<PathBuf> {
    let unix = root
        .join(".vox")
        .join("python")
        .join(".venv")
        .join("bin")
        .join("python");
    let windows = root
        .join(".vox")
        .join("python")
        .join(".venv")
        .join("Scripts")
        .join("python.exe");
    if unix.is_file() {
        Some(unix)
    } else if windows.is_file() {
        Some(windows)
    } else {
        None
    }
}

fn python_version(bin: &Path) -> Option<String> {
    let output = Command::new(bin).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let text = if text.trim().is_empty() {
        String::from_utf8_lossy(&output.stderr).into_owned()
    } else {
        text.into_owned()
    };
    // `Python 3.12.10`
    text.split_whitespace().nth(1).map(str::to_string)
}

/// Status text for `vox py doctor`, including the unsandboxed-worker warning.
pub fn doctor_report(start: &Path) -> Result<String> {
    let Some(config) = discover_python(start)? else {
        bail!(PY_ENV_MISSING);
    };
    let uv = which::which("uv")
        .ok()
        .map(|p| format!("uv: {}", p.display()))
        .unwrap_or_else(|| "uv: missing".to_string());
    let python = venv_python(&config.root)
        .and_then(|bin| python_version(&bin))
        .map(|v| format!("python: {v}"))
        .unwrap_or_else(|| "python: missing".to_string());
    let specs: Vec<&str> = config
        .packages
        .iter()
        .chain(config.extras.iter())
        .map(String::as_str)
        .collect();
    let packages = if specs.is_empty() {
        "(none)".to_string()
    } else {
        specs.join(", ")
    };
    let lock = lock_status(&config.root);
    Ok(format!(
        "{uv}\n{python}\npackages: {packages}\nlock: {lock}\nsandbox: unsandboxed\nwarning: the Python worker runs unsandboxed; a sandboxed long-lived worker is deferred\n"
    ))
}

fn lock_status(root: &Path) -> String {
    let uv_lock = root.join(".vox").join("python").join("uv.lock");
    if !uv_lock.is_file() {
        return "absent".to_string();
    }
    let Ok(bytes) = std::fs::read(&uv_lock) else {
        return "absent".to_string();
    };
    let actual = blake3_hex(&bytes);
    match read_recorded_hash(&root.join("vox.lock")) {
        Some(recorded) if recorded == actual => format!("match {actual}"),
        _ => "mismatch".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blake3_hex_of_empty_is_the_vox_crypto_digest() {
        assert_eq!(
            blake3_hex(b""),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn sync_without_python_section_does_not_ask_uv() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Vox.toml"), "[package]\nname = \"a\"\n").unwrap();
        let err = sync_project(dir.path()).unwrap_err().to_string();
        assert!(err.contains(PY_ENV_MISSING), "{err}");
        assert!(!dir.path().join(".vox").exists());
    }

    #[test]
    fn doctor_without_python_section_names_py_env() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Vox.toml"), "[package]\nname = \"a\"\n").unwrap();
        let err = doctor_report(dir.path()).unwrap_err().to_string();
        assert!(err.contains("py-env:"), "{err}");
    }

    #[test]
    fn doctor_reports_unsandboxed_when_env_is_not_synced() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Vox.toml"),
            "[package]\nname = \"a\"\n\n[python]\nversion = \"3.12\"\npackages = [\"numpy>=2.1\"]\n",
        )
        .unwrap();
        let report = doctor_report(dir.path()).unwrap();
        assert!(report.contains("packages: numpy>=2.1"), "{report}");
        assert!(report.contains("lock: absent"), "{report}");
        assert!(report.contains("sandbox: unsandboxed"), "{report}");
        assert!(report.contains("unsandboxed"), "{report}");
        assert!(
            report.contains("python: missing") || report.contains("python: "),
            "{report}"
        );
    }

    #[test]
    fn recorded_hash_round_trips_through_vox_lock() {
        let dir = tempfile::tempdir().unwrap();
        let hash = blake3_hex(b"uv.lock");
        record_uv_lock_hash(dir.path(), &hash).unwrap();
        assert_eq!(
            read_recorded_hash(&dir.path().join("vox.lock")).as_deref(),
            Some(hash.as_str())
        );
        let again = "ab".repeat(32);
        record_uv_lock_hash(dir.path(), &again).unwrap();
        assert_eq!(
            read_recorded_hash(&dir.path().join("vox.lock")).as_deref(),
            Some(again.as_str())
        );
    }

    #[test]
    fn sync_writes_env_and_doctor_matches_the_lock_when_uv_is_present() {
        if which::which("uv").is_err() {
            eprintln!("skip: uv absent");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Vox.toml"),
            "[package]\nname = \"a\"\n\n[python]\nversion = \"3.12\"\n",
        )
        .unwrap();
        let hash = sync_project(dir.path()).unwrap();
        assert_eq!(hash.len(), 64);
        assert!(dir.path().join(".vox/python/pyproject.toml").is_file());
        assert!(dir.path().join(".vox/python/uv.lock").is_file());
        let report = doctor_report(dir.path()).unwrap();
        assert!(report.contains(&format!("lock: match {hash}")), "{report}");
        assert!(report.contains("sandbox: unsandboxed"), "{report}");
        assert!(report.contains("warning:"), "{report}");
        let py = venv_python(dir.path());
        assert!(py.is_some(), "uv sync should create a venv");
        assert!(report.contains("python: 3."), "{report}");
    }
}
