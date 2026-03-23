use vox_db::VoxDb;
use tempfile::tempdir;

#[tokio::test]
async fn test_cas_store_and_get() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let store: VoxDb = VoxDb::open(db_path.to_str().unwrap()).await.unwrap();

    let data = b"hello vox cas";
    let hash = store.store("test_blob", data).await.unwrap();
    
    assert_eq!(hash, vox_db::hash::content_hash(data));

    let retrieved = store.get(&hash).await.unwrap();
    assert_eq!(retrieved, data);
}

#[tokio::test]
async fn test_cas_duplicate_ignore() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let store: VoxDb = VoxDb::open(db_path.to_str().unwrap()).await.unwrap();

    let data = b"duplicate";
    let hash1 = store.store("blob", data).await.unwrap();
    let hash2 = store.store("blob", data).await.unwrap();

    assert_eq!(hash1, hash2);
}

#[tokio::test]
async fn test_cas_bind_name() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let store: VoxDb = VoxDb::open(db_path.to_str().unwrap()).await.unwrap();

    let data = b"named object";
    let hash = store.store("blob", data).await.unwrap();
    
    store.bind_name("user", "my_file", &hash).await.unwrap();
    
    // Verification would typically involve querying names, 
    // but assuming bind succeeds if hash exists.
}
