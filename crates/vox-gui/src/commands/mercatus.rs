use std::{
    env, fs,
    path::{Path, PathBuf},
};
use tauri::command;

fn config_path() -> PathBuf {
    if let Ok(p) = env::var("PRICE_WATCH_CONFIG") {
        return PathBuf::from(p);
    }
    // Default: <user config dir>/storage-tier/price-watch/price-watch.config.json.
    // Falls back to the current directory if the OS config dir cannot be resolved
    // (dirs::config_dir() only returns None on exotic platforms).
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("storage-tier")
        .join("price-watch")
        .join("price-watch.config.json")
}

/// The first-run state: no config saved yet. Not an error — a fresh
/// install/worktree has nothing at `config_path()`, and reporting that as a
/// raw OS "file not found" (previously surfaced verbatim to the user) reads
/// as a broken app rather than an empty watchlist waiting to be filled in.
fn empty_config() -> serde_json::Value {
    serde_json::json!({ "_meta": { "sources": {} }, "watchlist": [] })
}

fn redacted_config_label(path: &Path) -> &str {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("price-watch.config.json")
}

fn load_mercatus_config_from_path(path: &Path) -> Result<serde_json::Value, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(empty_config());
        }
        Err(e) => return Err(format!("Cannot read {}: {e}", redacted_config_label(path))),
    };
    serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {e}"))
}

#[command]
pub fn mercatus_load_config() -> Result<serde_json::Value, String> {
    let path = config_path();
    load_mercatus_config_from_path(&path)
}

fn save_mercatus_config_to_path(
    config_path: &Path,
    config: &serde_json::Value,
) -> Result<(), String> {
    let text =
        serde_json::to_string_pretty(config).map_err(|e| format!("JSON serialize error: {e}"))?;
    if let Some(parent) = config_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create price-watch config directory: {e}"))?;
    }
    let label = redacted_config_label(config_path);
    let tmp = config_path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("Cannot write {label}: {e}")
    })?;
    fs::rename(&tmp, config_path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("Cannot write {label}: {e}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_config_returns_empty_watchlist_on_first_run_not_an_error() {
        // config_path() is env-var-overridable; point it at a file that
        // genuinely does not exist rather than touching the real user config dir.
        let dir =
            std::env::temp_dir().join(format!("vox-gui-mercatus-test-{}", std::process::id()));
        // SAFETY: test-only, single-threaded within this test's scope.
        unsafe {
            std::env::set_var("PRICE_WATCH_CONFIG", dir.join("does-not-exist.json"));
        }
        let result = mercatus_load_config();
        unsafe {
            std::env::remove_var("PRICE_WATCH_CONFIG");
        }
        assert_eq!(result, Ok(empty_config()));
    }

    #[test]
    fn save_config_creates_missing_parent_dirs_and_round_trips() {
        let dir = std::env::temp_dir().join(format!(
            "vox-gui-mercatus-save-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        let target = dir.join(".vox").join("price-watch").join("config.json");
        assert!(!dir.join(".vox").exists());

        let payload = serde_json::json!({
            "_meta": { "sources": { "openrouter": true } },
            "watchlist": [{ "model": "mens-qwen3-8b", "target_usd": 0.15 }]
        });
        save_mercatus_config_to_path(&target, &payload).expect("save config with missing .vox/");

        let raw = fs::read_to_string(&target).expect("read written config");
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
        assert_eq!(parsed, payload);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_save_leaves_no_tmp_file_and_redacts_host_paths_on_error() {
        let dir = std::env::temp_dir().join(format!(
            "vox-gui-mercatus-atomic-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        let target = dir
            .join("secret-home-segment")
            .join("price-watch.config.json");
        let tmp_sibling = target.with_extension("json.tmp");

        let first = serde_json::json!({ "_meta": { "sources": {} }, "watchlist": [{ "id": 1 }] });
        save_mercatus_config_to_path(&target, &first).expect("first atomic write");
        assert!(target.exists());
        assert!(
            !tmp_sibling.exists(),
            "atomic write must leave no .json.tmp file"
        );

        let second = serde_json::json!({ "_meta": { "sources": {} }, "watchlist": [{ "id": 2 }] });
        save_mercatus_config_to_path(&target, &second).expect("atomic overwrite");
        assert!(
            !tmp_sibling.exists(),
            "atomic overwrite must leave no .json.tmp file"
        );
        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&target).expect("read")).expect("json");
        assert_eq!(parsed, second);

        // Point load_mercatus_config_from_path at a directory so fs::read_to_string fails with
        // non-NotFound and verify the error string does NOT leak the host directory path.
        let dir_as_file = dir
            .join("secret-home-segment")
            .join("price-watch.config.json.dir");
        fs::create_dir_all(&dir_as_file).expect("mkdir");
        let err =
            load_mercatus_config_from_path(&dir_as_file).expect_err("reading directory must error");
        assert!(
            !err.contains("secret-home-segment"),
            "error must not leak host path: {err}"
        );
        assert!(err.starts_with("Cannot read price-watch.config.json.dir:"));

        let _ = fs::remove_dir_all(&dir);
    }
}

#[command]
pub fn mercatus_save_config(config: serde_json::Value) -> Result<(), String> {
    let config_path = config_path();
    save_mercatus_config_to_path(&config_path, &config)
}
