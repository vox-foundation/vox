use std::fs::{self, File};
use std::path::Path;
use std::time::{Duration, SystemTime};
use tempfile::tempdir;
use vox_db::research_doc_io::{FileLock, atomic_write_secure, update_research_index_md};

#[test]
fn test_atomic_write_and_idempotent_index_update() {
    let dir = tempdir().expect("tempdir");
    let test_file = dir.path().join("test-doc.md");
    atomic_write_secure(&test_file, b"# Hello World").expect("atomic write");
    assert_eq!(fs::read_to_string(&test_file).unwrap(), "# Hello World");

    let index_file = dir.path().join("research-index.md");
    let initial_index = "---\ntitle: \"Research Index\"\n---\n\n## Strategic & Value Proposition\n\n- [Old Doc](old-doc.md) — Existing description.\n\n## Data Storage\n";
    fs::write(&index_file, initial_index).unwrap();

    // 1. Insert new entry
    update_research_index_md(
        &index_file,
        "Strategic & Value Proposition",
        "new-doc.md",
        "New Doc Title",
        "A brand new architecture document.",
    )
    .expect("update index");

    let content = fs::read_to_string(&index_file).unwrap();
    assert!(content.contains("- [New Doc Title](new-doc.md) — A brand new architecture document."));

    // 2. Update existing entry idempotently (no duplicate rows)
    update_research_index_md(
        &index_file,
        "Strategic & Value Proposition",
        "new-doc.md",
        "New Doc Title Updated",
        "An updated description.",
    )
    .expect("update index again");

    let updated = fs::read_to_string(&index_file).unwrap();
    assert_eq!(
        updated.matches("new-doc.md").count(),
        1,
        "Must not duplicate links"
    );
    assert!(updated.contains("- [New Doc Title Updated](new-doc.md) — An updated description."));
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

#[test]
fn test_update_research_index_md_all_categories() {
    let dir = tempdir().expect("tempdir");
    let index_file = dir.path().join("research-index.md");
    let initial_index = r#"---
title: "Research Index"
---

## Strategic & Value Proposition

- [Existing Strategic](strat.md) — Strategic desc.

## Mesh Implementation Plans

- [Existing Mesh](mesh.md) — Mesh desc.

## Data Storage

- [Existing Storage](store.md) — Storage desc.

## Audits & Assessments

- [Existing Audit](audit.md) — Audit desc.
"#;
    fs::write(&index_file, initial_index).unwrap();

    update_research_index_md(
        &index_file,
        "Audits & Assessments",
        "new-audit.md",
        "New Audit",
        "Detailed audit findings.",
    )
    .expect("update audit");

    let content = fs::read_to_string(&index_file).unwrap();
    assert!(content.contains("- [New Audit](new-audit.md) — Detailed audit findings."));

    // Check it's under Audits & Assessments section
    let audit_idx = content.find("## Audits & Assessments").unwrap();
    let entry_idx = content.find("new-audit.md").unwrap();
    assert!(entry_idx > audit_idx);
}
