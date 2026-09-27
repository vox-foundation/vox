use std::fs::{self, File};
use std::path::Path;
use std::time::{Duration, SystemTime};
use tempfile::tempdir;
use vox_db::research_doc_io::{FileLock, atomic_write_secure};

#[test]
fn test_atomic_write_overwrites_content() {
    let dir = tempdir().expect("tempdir");
    let test_file = dir.path().join("test-doc.md");
    atomic_write_secure(&test_file, b"# Hello World").expect("atomic write");
    assert_eq!(fs::read_to_string(&test_file).unwrap(), "# Hello World");
    atomic_write_secure(&test_file, b"# Second").expect("atomic overwrite");
    assert_eq!(fs::read_to_string(&test_file).unwrap(), "# Second");
}

#[test]
fn test_file_lock_lifecycle_and_timeout() {
    let dir = tempdir().expect("tempdir");
    let target = dir.path().join("my-doc.md");

    // 1. Acquire first lock
    let lock1 = FileLock::acquire(&target, Duration::from_millis(500)).expect("acquire lock1");
    let expected_lock_path = dir.path().join("my-doc.md.lock");
    assert_eq!(lock1.lock_path(), expected_lock_path.as_path());
    assert!(expected_lock_path.exists());

    // 2. Attempt to acquire second lock while lock1 is held -> must timeout
    let lock2_res = FileLock::acquire(&target, Duration::from_millis(60));
    assert!(lock2_res.is_err());
    assert_eq!(lock2_res.unwrap_err().kind(), std::io::ErrorKind::TimedOut);

    // 3. Drop lock1 and verify lock file is removed
    drop(lock1);
    assert!(!expected_lock_path.exists());

    // 4. Now acquire succeeds
    let lock3 = FileLock::acquire(&target, Duration::from_millis(500)).expect("acquire lock3");
    assert!(expected_lock_path.exists());
    drop(lock3);
    assert!(!expected_lock_path.exists());
}

#[test]
fn test_file_lock_stale_recovery() {
    let dir = tempdir().expect("tempdir");
    let target = dir.path().join("stale-doc.md");
    let lock_path = dir.path().join("stale-doc.md.lock");

    // Manually write stale lock
    fs::write(&lock_path, "token:stale_token_from_dead_proc\n").expect("write lock");

    // Backdate modified time to 120 seconds ago
    let past = SystemTime::now() - Duration::from_secs(120);
    let times = std::fs::FileTimes::new().set_modified(past);
    let file = File::open(&lock_path).expect("open lock file");
    file.set_times(times).expect("set modified time");
    drop(file);

    // Acquire should detect stale lock (>60s), remove it, and acquire cleanly
    let lock = FileLock::acquire(&target, Duration::from_millis(500)).expect("acquire stale lock");
    assert!(lock_path.exists());
    let content = fs::read_to_string(&lock_path).unwrap();
    assert!(!content.contains("stale_token_from_dead_proc"));
    drop(lock);
    assert!(!lock_path.exists());
}

#[test]
fn test_atomic_write_secure_empty_parent() {
    let dir = tempdir().expect("tempdir");
    let cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");

    let res = atomic_write_secure(Path::new("relative-file.txt"), b"relative content");
    let read_back = fs::read_to_string("relative-file.txt");

    // Restore cwd
    std::env::set_current_dir(cwd).expect("restore cwd");

    res.expect("atomic write relative");
    assert_eq!(read_back.unwrap(), "relative content");
}
