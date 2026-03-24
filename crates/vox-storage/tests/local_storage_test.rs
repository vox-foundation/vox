#![allow(missing_docs)]
//! Smoke tests for `vox-storage` `LocalObjectStorage`.


use tempfile::tempdir;
use vox_storage::{LocalObjectStorage, ObjectStorage};

fn temp_store() -> (tempfile::TempDir, LocalObjectStorage) {
    let dir = tempdir().expect("create temp dir");
    let store = LocalObjectStorage::new(dir.path());
    (dir, store)
}

#[tokio::test]
async fn put_then_get_returns_same_bytes() {
    let (_dir, store) = temp_store();
    store.put("test/hello.txt", b"world").await.unwrap();
    let out = store.get("test/hello.txt").await.unwrap();
    assert_eq!(out, Some(b"world".to_vec()));
}

#[tokio::test]
async fn get_missing_key_returns_none() {
    let (_dir, store) = temp_store();
    let out = store.get("not/here.txt").await.unwrap();
    assert!(out.is_none(), "missing key should return None");
}

#[tokio::test]
async fn delete_removes_object() {
    let (_dir, store) = temp_store();
    store.put("to_delete.bin", b"bye").await.unwrap();
    store.delete("to_delete.bin").await.unwrap();
    let out = store.get("to_delete.bin").await.unwrap();
    assert!(out.is_none(), "deleted key should return None");
}

#[tokio::test]
async fn delete_missing_key_is_noop() {
    let (_dir, store) = temp_store();
    // Should not error on a key that was never stored.
    store.delete("does_not_exist.txt").await.unwrap();
}

#[tokio::test]
async fn list_prefix_returns_matching_keys() {
    let (_dir, store) = temp_store();
    store.put("audio/clip1.wav", b"a").await.unwrap();
    store.put("audio/clip2.wav", b"b").await.unwrap();
    store.put("video/movie.mp4", b"c").await.unwrap();

    let audio_keys = store.list_prefix("audio").await.unwrap();
    assert_eq!(audio_keys.len(), 2, "should list two audio keys");
    for k in &audio_keys {
        assert!(k.starts_with("audio/"), "key '{k}' should start with 'audio/'");
    }
}

#[tokio::test]
async fn list_prefix_empty_string_returns_all_keys() {
    let (_dir, store) = temp_store();
    store.put("a/x.txt", b"1").await.unwrap();
    store.put("b/y.txt", b"2").await.unwrap();

    let all = store.list_prefix("").await.unwrap();
    assert!(all.len() >= 2, "empty prefix should return all keys");
}

#[tokio::test]
async fn put_overwrites_existing_value() {
    let (_dir, store) = temp_store();
    store.put("file.txt", b"first").await.unwrap();
    store.put("file.txt", b"second").await.unwrap();
    let out = store.get("file.txt").await.unwrap();
    assert_eq!(out, Some(b"second".to_vec()), "put should overwrite");
}
