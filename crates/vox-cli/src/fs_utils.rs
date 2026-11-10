//! Shared filesystem utilities for Vox CLI commands.

use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::warn;

/// **PascalCase** `*.tsx` stem under `dir` (generated UI entry), preferring **`App`** when present, else first sorted match, else **`App`**.
pub fn find_component_name(dir: &Path) -> Result<String> {
    let mut candidates: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "tsx")
            && let Some(stem) = path.file_stem()
        {
            let name = stem.to_string_lossy().to_string();
            if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                candidates.push(name);
            }
        }
    }
    if candidates.iter().any(|n| n == "App") {
        return Ok("App".to_string());
    }
    candidates.sort();
    if let Some(n) = candidates.into_iter().next() {
        return Ok(n);
    }
    Ok("App".to_string())
}

/// Recursively copy a directory and all its contents.
pub fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let from_path = entry.path();
        let to_path = to.join(entry.file_name());
        if from_path.is_dir() {
            std::fs::create_dir_all(&to_path)?;
            copy_dir_recursive(&from_path, &to_path)?;
        } else {
            std::fs::copy(&from_path, &to_path)?;
        }
    }
    Ok(())
}

pub use vox_cli_core::fs_utils::run_target_dir_for_workspace;

/// Open `url` in the system default browser (best-effort; logs on failure).
pub async fn open_browser(url: &str) {
    let url = url.trim();
    if url.is_empty() {
        return;
    }

    let url_owned = url.to_string();
    let res = tokio::task::spawn_blocking(move || open_browser_sync(&url_owned)).await;

    match res {
        Ok(Ok(())) => {}
        Ok(Err(e)) => warn!(target: "vox_cli", "open_browser: {e}"),
        Err(e) => warn!(target: "vox_cli", "open_browser join: {e}"),
    }
}

/// Best-effort home directory (`HOME`, `USERPROFILE`, `HOMEDRIVE`+`HOMEPATH`; else `.`).
pub fn user_home_dir() -> PathBuf {
    vox_config::user_home_dir()
}

/// Strip Windows `\\?\` / `\\?\UNC\` prefixes from paths (e.g. [`std::fs::canonicalize`] output).
///
/// Verbatim paths break nested Cargo `path = "..."` dependencies when forwarded as `//?/C:/...`
/// after slash normalization.
#[cfg(windows)]
pub fn strip_windows_verbatim_path(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    let Some(rest) = s.strip_prefix(r"\\?\") else {
        return path;
    };
    let rest = rest.replace('/', "\\");
    if let Some(unc) = rest.strip_prefix("UNC\\") {
        PathBuf::from(format!(r"\\{}", unc))
    } else {
        PathBuf::from(rest)
    }
}

#[cfg(not(windows))]
pub fn strip_windows_verbatim_path(path: PathBuf) -> PathBuf {
    path
}

/// Path to the `vox-actor-runtime` crate for generated script projects (`VOX_RUNTIME_PATH` or repo layout).
pub fn resolve_vox_runtime_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("VOX_RUNTIME_PATH") {
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            return Some(strip_windows_verbatim_path(pb));
        }
    }
    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let ctx = vox_repository::discover_repository_or_fallback(&start);
    let candidate = ctx.root.join("crates").join("vox-actor-runtime");
    if candidate.is_dir() {
        return Some(strip_windows_verbatim_path(candidate));
    }
    None
}

pub use vox_cli_core::fs_utils::{dir_size_bytes, gc_script_cache};

fn open_browser_sync(url: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `vox new`-scaffolded project (`Vox.toml` at the root, no `.git` yet)
    /// must anchor `target/generated` at the project root, not at the source
    /// file's own directory — regression test for the `src/target/generated`
    /// bug where a missing `.git` caused the old git-only anchor to fall back
    /// to `start` itself.
    #[test]
    fn target_dir_anchors_to_vox_toml_root_without_git() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = std::fs::canonicalize(tmp.path()).unwrap();
        let src_dir = dir.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(dir.join("Vox.toml"), b"[project]\nname = \"demo\"\n").unwrap();

        let result = run_target_dir_for_workspace(Some(&src_dir));
        assert_eq!(
            result,
            dir.join("target"),
            "target dir must be under the Vox.toml project root, not src/"
        );
    }
}
