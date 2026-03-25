//! Cross-platform data directory resolution for Vox.
//!
//! Delegates to `vox_config` for a single source of truth. Re-exports for backward compatibility.

use std::path::PathBuf;

pub use vox_config::{
    APP_DIR_NAME, DEFAULT_DB_FILENAME, config_dir, data_dir, default_db_path, local_user_id,
    state_dir,
};

/// Separate SQLite file for training-run telemetry when the primary [`default_db_path`] uses a
/// legacy `schema_version` chain (see [`crate::VoxDb::connect_default_with_training_fallback`]).
#[must_use]
pub fn training_telemetry_db_path() -> Option<PathBuf> {
    default_db_path().map(|p| p.with_file_name("vox_training_telemetry.db"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_returns_some() {
        let dir = data_dir();
        assert!(dir.is_some(), "data_dir() should resolve on this platform");
        let path = dir.unwrap();
        assert!(
            path.to_str().unwrap().contains("vox"),
            "path should contain 'vox'"
        );
    }

    #[test]
    fn default_db_path_has_filename() {
        let path = default_db_path().expect("should resolve");
        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            DEFAULT_DB_FILENAME
        );
    }

    #[test]
    fn local_user_id_not_empty() {
        let id = local_user_id();
        assert!(!id.is_empty(), "local_user_id() should never be empty");
    }

    #[test]
    fn state_dir_creates_subdirectory() {
        let dir = state_dir().expect("should resolve");
        assert!(dir.ends_with("state"));
    }
}
