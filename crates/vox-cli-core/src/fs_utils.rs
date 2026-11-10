//! Shared filesystem utilities for CLI tools.

use std::path::{Path, PathBuf};
use tracing::warn;

/// Open `url` in the system default browser.
pub async fn open_browser(url: &str) {
    let url = url.trim();
    if url.is_empty() {
        return;
    }

    let url_owned = url.to_string();
    let res = tokio::task::spawn_blocking(move || open_browser_sync(&url_owned)).await;

    match res {
        Ok(Ok(())) => {}
        Ok(Err(e)) => warn!("open_browser: {e}"),
        Err(e) => warn!("open_browser join: {e}"),
    }
}

fn open_browser_sync(url: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", "start", "", url]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        cmd.spawn()?;
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

/// Strip Windows `\\?\` / `\\?\UNC\` prefixes from paths.
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

/// Shared Cargo `target/` dir for a generated project under the Vox project root.
pub fn run_target_dir_for_workspace(start: Option<&std::path::Path>) -> PathBuf {
    let start = start
        .map(std::path::Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let root = vox_config::paths::find_repo_root(&start).unwrap_or_else(|| {
        #[cfg(feature = "db")]
        {
            vox_repository::discover_repository_or_fallback(&start).root
        }
        #[cfg(not(feature = "db"))]
        {
            start.clone()
        }
    });
    root.join("target")
}

pub fn dir_size_bytes(path: &Path) -> std::io::Result<u64> {
    let meta = path.metadata()?;
    if meta.is_file() {
        return Ok(meta.len());
    }
    if !meta.is_dir() {
        return Ok(0);
    }
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        total += dir_size_bytes(&entry.path()).unwrap_or(0);
    }
    Ok(total)
}

/// Trim `~/.vox/script-cache*` by entry count and total size (oldest first).
pub fn gc_script_cache(max_entries: usize, max_size_mb: u64) -> anyhow::Result<()> {
    let max_bytes = max_size_mb.saturating_mul(1024 * 1024);
    for wasi in [false, true] {
        let root = vox_config::script_cache_dir(wasi);
        if !root.is_dir() {
            continue;
        }
        let mut entries: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
        for e in std::fs::read_dir(&root)? {
            let e = e?;
            let p = e.path();
            if p.is_dir() {
                let mt = e
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                entries.push((p, mt));
            }
        }
        entries.sort_by_key(|a| a.1);
        loop {
            let count = entries.len();
            let total: u64 = entries
                .iter()
                .map(|(p, _)| dir_size_bytes(p).unwrap_or(0))
                .sum();
            if count <= max_entries && total <= max_bytes {
                break;
            }
            if entries.is_empty() {
                break;
            }
            let protect_recent = vox_config::timeouts::D_120S;
            let Some(idx) = entries.iter().position(|(_path, modified)| {
                modified
                    .elapsed()
                    .map(|e| e >= protect_recent)
                    .unwrap_or(false)
            }) else {
                break;
            };
            let (old, _) = entries.remove(idx);
            let _ = std::fs::remove_dir_all(&old);
        }
    }
    Ok(())
}

/// User home directory from `vox_config::paths::user_home_dir`.
pub fn user_home_dir() -> PathBuf {
    vox_config::paths::user_home_dir()
}

pub const SERVICE_SURFACE_MARKERS: &[&str] = &[
    "@page", "routes", "table", "server", "query", "mutation", "actor", "workflow", "activity",
];

pub fn has_service_surface(head: &str) -> bool {
    let h = format!("\n{head}");
    SERVICE_SURFACE_MARKERS.iter().any(|s| h.contains(s))
}

pub fn is_script_shaped(head: &str) -> bool {
    head.contains("fn main(") && !has_service_surface(head)
}
