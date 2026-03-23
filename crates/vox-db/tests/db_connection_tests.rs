use vox_db::{VoxDb, DbConfig};
use tempfile::tempdir;

#[tokio::test]
async fn test_db_memory_smoke() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    assert_eq!(db.schema_version().await.unwrap(), 1);
    
    let hash = db.store("test", b"hello").await.unwrap();
    assert!(!hash.is_empty());
}

#[tokio::test]
async fn test_db_local_file_persistence() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("vox.db");
    let path_str = db_path.to_str().unwrap().to_string();
    let hash;

    {
        let db = VoxDb::connect(DbConfig::Local { path: path_str.clone() }).await.unwrap();
        hash = db.store("perm", b"data").await.unwrap();
    }

    // Reopen and check if it still works
    let db = VoxDb::connect(DbConfig::Local { path: path_str }).await.unwrap();
    let obj = db.get(&hash).await.unwrap();
    assert_eq!(obj, b"data");
}

#[tokio::test]
async fn test_db_circuit_breaker() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    let breaker = db.breaker();
    assert_eq!(breaker.state(), vox_db::CircuitState::Closed);
    
    // We can't easily trigger a real failure in memory without mock, 
    // but we can check if it exists and is closed.
}

#[tokio::test]
async fn test_db_transaction_success() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    
    db.transaction(async {
        db.save_memory(vox_db::MemoryParams {
            agent_id: "tx_agent",
            session_id: "sess_1",
            memory_type: "observation",
            content: "tx_data",
            metadata: None,
            importance: 1.0,
            vcs_snapshot_id: None,
        }).await?;
        Ok(())
    }).await.unwrap();

    let recalled = db.recall_memory("tx_agent", None, 10, None).await.unwrap();
    assert_eq!(recalled.len(), 1);
}
