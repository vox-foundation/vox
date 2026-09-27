use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static TEMP_FILE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Atomically writes content to `dest_path` via a temporary file in the same directory,
/// followed by an fsync and an atomic rename with Windows-specific retry logic.
pub fn atomic_write_secure(dest_path: &Path, content: &[u8]) -> io::Result<()> {
    let parent = dest_path.parent().unwrap_or_else(|| Path::new("."));
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    fs::create_dir_all(parent)?;

    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let seq = TEMP_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
    let file_stem = dest_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let tmp_path = parent.join(format!(".{file_stem}.{pid}_{nanos}_{seq}.tmp"));

    {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        f.write_all(content)?;
        f.sync_all()?;
    }

    #[cfg(windows)]
    let mut attempts = 0;
    // Only Windows retries (sharing violations); elsewhere the first rename is final.
    #[cfg_attr(not(windows), allow(clippy::never_loop))]
    loop {
        match fs::rename(&tmp_path, dest_path) {
            Ok(_) => break,
            Err(e) => {
                #[cfg(windows)]
                {
                    attempts += 1;
                    let raw = e.raw_os_error().unwrap_or(0);
                    if (raw == 5 || raw == 32 || e.kind() == io::ErrorKind::PermissionDenied)
                        && attempts < 10
                    {
                        std::thread::sleep(Duration::from_millis(15 * attempts as u64));
                        continue;
                    }
                }
                let _ = fs::remove_file(&tmp_path);
                return Err(e);
            }
        }
    }

    #[cfg(unix)]
    {
        if let Ok(dir_file) = File::open(parent) {
            let _ = dir_file.sync_all();
        }
    }

    Ok(())
}

/// Advisory file lock using a `<target>.lock` sibling file with token validation on drop.
#[derive(Debug)]
pub struct FileLock {
    lock_path: std::path::PathBuf,
    token: String,
}

impl FileLock {
    /// Attempts to acquire an advisory file lock for `target` within the given `timeout`.
    /// Stale locks older than 60 seconds are automatically cleaned up.
    pub fn acquire(target: &Path, timeout: Duration) -> io::Result<Self> {
        let mut lock_name = target.file_name().unwrap_or_default().to_os_string();
        lock_name.push(".lock");
        let lock_path = target.with_file_name(lock_name);
        let start = std::time::Instant::now();
        let pid = std::process::id();

        while start.elapsed() < timeout {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let seq = TEMP_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
            let token = format!("{pid}:{nanos}:{seq}");

            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut f) => {
                    let _ = writeln!(f, "token:{token}");
                    return Ok(Self { lock_path, token });
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    if let Ok(meta) = fs::metadata(&lock_path)
                        && let Ok(elapsed) = meta
                            .modified()
                            .and_then(|m| m.elapsed().map_err(io::Error::other))
                        && elapsed > Duration::from_secs(60)
                    {
                        let _ = fs::remove_file(&lock_path);
                        continue;
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Lock acquisition timed out",
        ))
    }

    /// Accessor for the active lock path.
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.lock_path)
            && content.contains(&self.token)
        {
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_write_secure_in_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sub/test.txt");
        atomic_write_secure(&target, b"hello").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello");
    }

    #[test]
    fn test_file_lock_in_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("doc.md");
        let lock = FileLock::acquire(&target, Duration::from_millis(500)).unwrap();
        let lock_path = lock.lock_path().to_path_buf();
        assert!(lock_path.exists());
        drop(lock);
        assert!(!lock_path.exists());
    }
}
