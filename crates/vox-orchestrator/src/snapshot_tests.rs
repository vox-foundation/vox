use super::*;
use std::io::Write;
use std::path::Path;

fn agent() -> AgentId {
    AgentId(1)
}

#[test]
fn snapshot_id_display() {
    assert_eq!(SnapshotId(42).to_string(), "S-000042");
}

#[test]
fn take_and_list_snapshots() {
    let mut store = SnapshotStore::new(10);
    let id = store.take_snapshot(agent(), &[], "test snap");
    assert_eq!(store.count(), 1);
    assert!(store.get(id).is_some());
}

#[test]
fn diff_detects_added_and_removed() {
    let a = Snapshot {
        id: SnapshotId(1),
        agent_id: agent(),
        timestamp_ms: 0,
        description: String::new(),
        files: HashMap::from([(
            PathBuf::from("a.rs"),
            FileEntry {
                path: PathBuf::from("a.rs"),
                content_hash: "aaa".into(),
                size_bytes: 10,
            },
        )]),
    };
    let b = Snapshot {
        id: SnapshotId(2),
        agent_id: agent(),
        timestamp_ms: 1,
        description: String::new(),
        files: HashMap::from([(
            PathBuf::from("b.rs"),
            FileEntry {
                path: PathBuf::from("b.rs"),
                content_hash: "bbb".into(),
                size_bytes: 20,
            },
        )]),
    };

    let diffs = SnapshotStore::diff(&a, &b);
    assert_eq!(diffs.len(), 2);
    assert!(
        diffs
            .iter()
            .any(|d| d.path == Path::new("b.rs") && matches!(d.kind, FileDiffKind::Added))
    );
    assert!(
        diffs
            .iter()
            .any(|d| d.path == Path::new("a.rs") && matches!(d.kind, FileDiffKind::Removed))
    );
}

#[test]
fn diff_detects_modified() {
    let make = |hash: &str| Snapshot {
        id: SnapshotId(1),
        agent_id: agent(),
        timestamp_ms: 0,
        description: String::new(),
        files: HashMap::from([(
            PathBuf::from("x.rs"),
            FileEntry {
                path: PathBuf::from("x.rs"),
                content_hash: hash.into(),
                size_bytes: 5,
            },
        )]),
    };

    let diffs = SnapshotStore::diff(&make("old"), &make("new"));
    assert_eq!(diffs.len(), 1);
    assert!(matches!(diffs[0].kind, FileDiffKind::Modified));
}

#[test]
fn eviction_respects_max() {
    let mut store = SnapshotStore::new(3);
    for _ in 0..5 {
        store.take_snapshot(agent(), &[], "snap");
    }
    assert_eq!(store.count(), 3);
}

#[test]
fn hash_file_works_on_real_file() {
    let dir = std::env::temp_dir().join("vox_snap_test");
    std::fs::create_dir_all(&dir).ok();
    let file = dir.join("test.txt");
    let mut f = std::fs::File::create(&file).expect("create temp");
    write!(f, "hello world").expect("write");
    drop(f);

    let result = SnapshotStore::hash_file(&file);
    assert!(result.is_some());
    let (hash, size) = result.expect("hash");
    assert_eq!(size, 11);
    assert!(!hash.is_empty());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cas_deduplication() {
    let mut store = SnapshotStore::new(10);
    let data = b"shared content".to_vec();

    let h1 = store.store_blob(data.clone());
    let h2 = store.store_blob(data.clone());
    assert_eq!(h1, h2);
    assert_eq!(store.blob_count(), 1);
    assert!(store.dedup_ratio() >= 2.0 - f64::EPSILON);
}

#[test]
fn cas_get_blob() {
    let mut store = SnapshotStore::new(10);
    let data = b"hello cas".to_vec();
    let hash = store.store_blob(data.clone());
    assert_eq!(store.get_blob(&hash), Some(data.as_slice()));
    assert_eq!(store.get_blob(&"nonexistent".to_string()), None);
}

#[test]
fn cas_compact_removes_unreferenced_blobs() {
    let mut store = SnapshotStore::new(10);
    store.store_blob(b"orphaned blob".to_vec());
    assert_eq!(store.blob_count(), 1);

    let freed = store.compact();
    assert_eq!(freed, 1);
    assert_eq!(store.blob_count(), 0);
}

#[test]
fn take_snapshot_in_memory_deduplicates() {
    let mut store = SnapshotStore::new(10);
    let shared = b"shared file content".to_vec();

    store.take_snapshot_in_memory(
        AgentId(1),
        vec![(PathBuf::from("a.rs"), shared.clone())],
        "snap 1",
    );
    store.take_snapshot_in_memory(
        AgentId(1),
        vec![(PathBuf::from("b.rs"), shared.clone())],
        "snap 2",
    );

    assert_eq!(store.blob_count(), 1);
    assert_eq!(store.count(), 2);
    assert!(store.dedup_ratio() >= 2.0 - f64::EPSILON);
}
