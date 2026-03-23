use vox_db::VoxDb;

#[tokio::test]
async fn test_locks_crud() {
    let store: VoxDb = VoxDb::open_memory().await.unwrap();

    // 1. Schema is initialised by open_memory()

    let repo_id = "test-repo";
    let ttl_secs = 60;

    // 2. Lock Acquisition
    let fence = store
        .acquire_distributed_lock("my_lock", "node1", "ag-1", ttl_secs, repo_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fence, 1);

    // 3. Prevent Double Acquire from another node
    let err = store
        .acquire_distributed_lock("my_lock", "node2", "ag-2", ttl_secs, repo_id)
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(err, "node1");

    // 4. Successful Release
    store.release_distributed_lock("my_lock", "node1", repo_id).await.unwrap();
    
    // 5. Re-acquire by node 2
    let fence2 = store
        .acquire_distributed_lock("my_lock", "node2", "ag-2", ttl_secs, repo_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fence2, 1);

    // 6. Prune handles expires_at <= now
    let pruned = store.prune_stale_distributed_locks().await.unwrap();
    assert_eq!(pruned, 0); // Not expired yet
}

#[tokio::test]
async fn test_heartbeats_crud() {
    let store: VoxDb = VoxDb::open_memory().await.unwrap();

    let repo_id = "test-repo";
    let now_ms = 1_000_000;

    // 1. Upsert
    store.upsert_mesh_heartbeat("node1", "ag-1", "idle", now_ms, repo_id).await.unwrap();

    // 2. List Live Nodes (min_seen_ms)
    let live = store.list_live_nodes(900_000, repo_id).await.unwrap();
    assert_eq!(live.len(), 1);
    assert_eq!(live[0][0], "node1");
    assert_eq!(live[0][1], "ag-1");
    assert_eq!(live[0][2], "idle");

    // 3. Stale read (min_seen_ms ahead of last_seen)
    let live_stale = store.list_live_nodes(1_000_001, repo_id).await.unwrap();
    assert!(live_stale.is_empty());

    // 4. Evict dead
    let count = store.evict_dead_heartbeats(2_000_000).await.unwrap();
    assert!(count > 0, "Expected to evict the dead heartbeat");
}
