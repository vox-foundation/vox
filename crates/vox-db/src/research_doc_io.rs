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
                    if let Ok(meta) = fs::metadata(&lock_path) {
                        if let Ok(elapsed) = meta.modified().and_then(|m| {
                            m.elapsed()
                                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
                        }) {
                            if elapsed > Duration::from_secs(60) {
                                let _ = fs::remove_file(&lock_path);
                                continue;
                            }
                        }
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
        if let Ok(content) = fs::read_to_string(&self.lock_path) {
            if content.contains(&self.token) {
                let _ = fs::remove_file(&self.lock_path);
            }
        }
    }
}

/// Updates `research-index.md` idempotently by replacing an existing link row
/// or inserting a new link row under the matching category section.
pub fn update_research_index_md(
    index_path: &Path,
    category: &str,
    filename: &str,
    title: &str,
    description: &str,
) -> io::Result<()> {
    let _guard = FileLock::acquire(index_path, Duration::from_secs(5))?;
    let content = fs::read_to_string(index_path)?;
    let clean_desc = description.trim_end_matches('.');
    let new_entry = format!("- [{title}]({filename}) — {clean_desc}.");

    // Case 1: Existing link update (in-place replacement)
    let link_target = format!("]({filename})");
    let mut replaced = false;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    for line in lines.iter_mut() {
        if line.contains(&link_target) && line.trim_start().starts_with('-') {
            *line = new_entry.clone();
            replaced = true;
            break;
        }
    }

    if !replaced {
        let normalized_cat = category.to_lowercase();
        let target_heading = if normalized_cat.contains("mesh") || normalized_cat.contains("populi")
        {
            "## Mesh Implementation Plans"
        } else if normalized_cat.contains("storage") || normalized_cat.contains("db") {
            "## Data Storage"
        } else if normalized_cat.contains("gui")
            || normalized_cat.contains("ui")
            || normalized_cat.contains("dashboard")
        {
            "## User Interface & Dashboard"
        } else if normalized_cat.contains("language") || normalized_cat.contains("compiler") {
            "## Language platform"
        } else if normalized_cat.contains("audit") || normalized_cat.contains("assessment") {
            "## Audits & Assessments"
        } else if normalized_cat.contains("competitive") || normalized_cat.contains("ecosystem") {
            "## Competitive & Ecosystem Research"
        } else {
            "## Strategic & Value Proposition"
        };

        let mut insert_idx = None;
        let mut in_section = false;

        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("## ") {
                let heading_text = line.trim_start_matches("## ").to_lowercase();
                if heading_text.contains(&target_heading.trim_start_matches("## ").to_lowercase()) {
                    in_section = true;
                } else if in_section {
                    let mut target_idx = i;
                    while target_idx > 0 && lines[target_idx - 1].trim().is_empty() {
                        target_idx -= 1;
                    }
                    insert_idx = Some(target_idx);
                    break;
                }
            }
        }

        if in_section && insert_idx.is_none() {
            insert_idx = Some(lines.len());
        }

        let idx = insert_idx.unwrap_or_else(|| lines.len());
        lines.insert(idx, new_entry);
    }

    let updated_content = lines.join("\n") + "\n";
    atomic_write_secure(index_path, updated_content.as_bytes())
}
