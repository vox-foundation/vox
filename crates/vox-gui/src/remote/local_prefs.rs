//! Client-side `gui.*` preferences for remote mode (plan 07 D1): they live in
//! `<app_config_dir>/axis-preferences.json` on the Axis machine and never reach the server.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::Manager;

static PREFS_LOCK: Mutex<()> = Mutex::new(());

// vox:defactored-from vox-orchestrator-mcp 2026-10-08
fn is_gui_preference_key(key: &str) -> bool {
    key.starts_with("gui.") && key.len() <= 128
}

fn check_key(key: &str) -> Result<(), String> {
    if is_gui_preference_key(key) {
        Ok(())
    } else {
        Err(format!("key '{key}' is not a GUI preference"))
    }
}

pub(crate) struct AxisLocalPrefs {
    path: PathBuf,
}

impl AxisLocalPrefs {
    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) fn for_app<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<Self, String> {
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        Ok(Self::at(dir.join("axis-preferences.json")))
    }

    fn load(&self) -> Result<BTreeMap<String, String>, String> {
        match std::fs::read_to_string(&self.path) {
            Ok(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub(crate) fn get(&self, key: &str) -> Result<Option<String>, String> {
        check_key(key)?;
        let _guard = PREFS_LOCK.lock().map_err(|e| e.to_string())?;
        Ok(self.load()?.remove(key))
    }

    pub(crate) fn set(&self, key: &str, value: &str) -> Result<(), String> {
        check_key(key)?;
        let _guard = PREFS_LOCK.lock().map_err(|e| e.to_string())?;
        let mut prefs = self.load()?;
        prefs.insert(key.to_string(), value.to_string());
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let body = serde_json::to_string_pretty(&prefs).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, body).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_then_get_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let prefs = AxisLocalPrefs::at(dir.path().join("cfg").join("axis-preferences.json"));
        prefs.set("gui.theme", "dark").unwrap();
        prefs.set("gui.sign", "on").unwrap();
        assert_eq!(prefs.get("gui.theme").unwrap().as_deref(), Some("dark"));
        assert_eq!(prefs.get("gui.sign").unwrap().as_deref(), Some("on"));
        assert!(
            !dir.path()
                .join("cfg")
                .join("axis-preferences.json.tmp")
                .exists()
        );
    }

    #[test]
    fn missing_file_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        let prefs = AxisLocalPrefs::at(dir.path().join("axis-preferences.json"));
        assert_eq!(prefs.get("gui.theme").unwrap(), None);
    }

    #[test]
    fn non_gui_keys_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("axis-preferences.json");
        let prefs = AxisLocalPrefs::at(path.clone());
        assert!(prefs.set("attention_enabled", "true").is_err());
        assert!(prefs.get("socrates_gate_enforced").is_err());
        assert!(!path.exists());
    }
}
