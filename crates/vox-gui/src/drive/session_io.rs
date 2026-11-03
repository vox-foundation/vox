use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveSessionFile {
    pub schema_version: u32,
    pub pid: u32,
    pub port: u16,
    pub token_path: PathBuf,
    pub store_path: PathBuf,
    pub show: bool,
}

pub fn write_child_session(session: &DriveSessionFile) -> Result<(), String> {
    let path = std::env::var("VOX_GUI_DRIVE_SESSION_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_session_path());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(session).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn default_session_path() -> PathBuf {
    let home = std::env::var("VOX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".vox")
        });
    home.join("run/gui-drive.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_child_session_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gui-drive.json");
        unsafe { std::env::set_var("VOX_GUI_DRIVE_SESSION_PATH", &path) };
        write_child_session(&DriveSessionFile {
            schema_version: 1,
            pid: 9,
            port: 4321,
            token_path: PathBuf::from("/tmp/token"),
            store_path: PathBuf::from("/tmp/store.db"),
            show: false,
        })
        .expect("write");
        unsafe { std::env::remove_var("VOX_GUI_DRIVE_SESSION_PATH") };
        let parsed: DriveSessionFile =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(parsed.port, 4321);
        assert_eq!(parsed.pid, 9);
    }
}
