use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// A safety mechanism that backups files before they are modified and allows rolling them back.
///
/// This is used by TOESTUB SLO gates to ensure that failed refactors can be undone automatically
/// without leaving the working tree in a broken state.
#[derive(Debug, Default)]
pub struct WorkspaceGuard {
    base_dir: PathBuf,
    /// Maps original paths to their original content bytes.
    backups: HashMap<PathBuf, Vec<u8>>,
}

impl WorkspaceGuard {
    /// Create a new guard rooted at `base_dir`.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            backups: HashMap::new(),
        }
    }

    /// Backup a set of files before modification.
    ///
    /// Only files that exist are backed up. If a path is already backed up, it is not overwritten
    /// (preserving the original state before the *first* modification in this guard's lifecycle).
    pub fn checkout(&mut self, paths: &[PathBuf]) -> Result<()> {
        for path in paths {
            let full_path = self.base_dir.join(path);
            if !self.backups.contains_key(path) && full_path.exists() {
                let content = fs::read(&full_path)
                    .with_context(|| format!("Failed to backup {}", full_path.display()))?;
                self.backups.insert(path.clone(), content);
            }
        }
        Ok(())
    }

    /// Rollback all backed-up files to their original state.
    pub fn rollback(&self) -> Result<()> {
        for (path, content) in &self.backups {
            let full_path = self.base_dir.join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&full_path, content)
                .with_context(|| format!("Failed to restore {}", full_path.display()))?;
        }
        tracing::info!("WorkspaceGuard: Rolled back {} files.", self.backups.len());
        Ok(())
    }

    /// Clear the backups without restoring (committing the changes).
    pub fn commit(&mut self) {
        self.backups.clear();
    }

    /// Access the list of backed-up paths.
    pub fn backed_up_paths(&self) -> Vec<&PathBuf> {
        self.backups.keys().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_checkout_and_rollback() -> Result<()> {
        let dir = tempdir()?;
        let file_path = PathBuf::from("test.txt");
        let full_path = dir.path().join(&file_path);
        
        {
            let mut f = fs::File::create(&full_path)?;
            f.write_all(b"original")?;
        }

        let mut guard = WorkspaceGuard::new(dir.path());
        guard.checkout(&[file_path.clone()])?;

        // Modify file
        fs::write(&full_path, b"modified")?;
        assert_eq!(fs::read_to_string(&full_path)?, "modified");

        // Rollback
        guard.rollback()?;
        assert_eq!(fs::read_to_string(&full_path)?, "original");

        Ok(())
    }
}
