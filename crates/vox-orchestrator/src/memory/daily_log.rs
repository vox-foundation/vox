use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use super::error::MemoryError;
use super::time::timestamp_hms;

/// An append-only daily log file (`memory/YYYY-MM-DD.md`).
///
/// Each call to [`DailyLog::append`] writes a timestamped bullet to disk immediately.
/// Survives restarts — the file is opened in append mode every time.
pub struct DailyLog {
    path: PathBuf,
}

impl DailyLog {
    /// Open (or create) the log for the given date string (`YYYY-MM-DD`).
    pub fn open(log_dir: &Path, date_str: &str) -> Result<Self, MemoryError> {
        fs::create_dir_all(log_dir).map_err(MemoryError::Io)?;
        let path = log_dir.join(format!("{date_str}.md"));
        // Create the file with a heading if it doesn't exist yet
        if !path.exists() {
            let heading = format!("# Daily Log — {date_str}\n\n");
            fs::write(&path, heading.as_bytes()).map_err(MemoryError::Io)?;
        }
        Ok(Self { path })
    }

    /// Append a timestamped entry to this log.
    pub fn append(&self, entry: &str) -> Result<(), MemoryError> {
        let mut f = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(MemoryError::Io)?;
        writeln!(f, "- `{}` {}", timestamp_hms(), entry).map_err(MemoryError::Io)?;
        Ok(())
    }

    /// Read full contents of this log.
    pub fn read(&self) -> Result<String, MemoryError> {
        if self.path.exists() {
            crate::bounded_fs::read_utf8_path_capped(&self.path).map_err(|e| {
                MemoryError::Io(std::io::Error::other(e.to_string()))
            })
        } else {
            Ok(String::new())
        }
    }

    /// True if the backing file exists and is non-empty.
    pub fn exists(&self) -> bool {
        self.path.exists() && self.path.metadata().map(|m| m.len() > 0).unwrap_or(false)
    }

    /// Path to the backing file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
