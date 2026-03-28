use std::sync::Arc;

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
    store
        .release_distributed_lock("my_lock", "node1", repo_id)
        .await
        .unwrap();

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
async fn distributed_lock_fence_increments_on_same_node_refresh() {
    let store: VoxDb = VoxDb::open_memory().await.unwrap();
    let repo = "repo-fence";
    let ttl = 120;
    let f1 = store
        .acquire_distributed_lock("L", "n1", "a1", ttl, repo)
        .await
        .unwrap()
        .unwrap();
    let f2 = store
        .acquire_distributed_lock("L", "n1", "a1", ttl, repo)
        .await
        .unwrap()
        .unwrap();
    assert!(
        f2 > f1,
        "refresh by same holder must advance fence (got {f1} then {f2})"
    );
}

#[tokio::test]
async fn distributed_lock_concurrent_acquires_single_holder() {
    let store = Arc::new(VoxDb::open_memory().await.unwrap());
    let repo = "repo-contend";
    let ttl = 300;
    let mut handles = Vec::new();
    for i in 0..16u32 {
        let db = store.clone();
        handles.push(tokio::spawn(async move {
            let node = format!("node-{i}");
            db.acquire_distributed_lock("contended", &node, "agent", ttl, repo)
                .await
        }));
    }
    let mut owners = 0i32;
    for h in handles {
        match h.await.unwrap() {
            Ok(Ok(_)) => owners += 1,
            Ok(Err(_)) => {}
            Err(e) => panic!("acquire db error: {e:?}"),
        }
    }
    assert_eq!(
        owners, 1,
        "exactly one concurrent acquirer may hold the lock"
    );
}

#[tokio::test]
async fn distributed_lock_many_distinct_keys_finishes_quickly() {
    let store: VoxDb = VoxDb::open_memory().await.unwrap();
    let repo = "repo-budget";
    let ttl = 600;
    let t0 = std::time::Instant::now();
    for i in 0..100u32 {
        store
            .acquire_distributed_lock(&format!("budget-lock-{i}"), "node", "agent", ttl, repo)
            .await
            .unwrap()
            .unwrap();
    }
    assert!(
        t0.elapsed().as_secs() < 15,
        "100 sequential acquires should stay within a modest budget"
    );
}

#[tokio::test]
async fn test_heartbeats_crud() {
    let store: VoxDb = VoxDb::open_memory().await.unwrap();

    let repo_id = "test-repo";
    let now_ms = 1_000_000;

    // 1. Upsert
    store
        .upsert_mesh_heartbeat("node1", "ag-1", "idle", now_ms, repo_id)
        .await
        .unwrap();

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
