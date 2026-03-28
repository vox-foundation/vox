//! Optional **embedded replica** smoke test (`VoxDb::open_embedded_replica` + `sync_for`).
//!
//! **Run:**
//! `VOX_DB_EMBEDDED_REPLICA_INTEGRATION=1` plus the same URL/token vars as `sync_remote_integration.rs`, then  
//! `cargo test -p vox-db --features replication sync_embedded_replica_smoke -- --nocapture`.

#[path = "common/mod.rs"]
mod common;

use tempfile::tempdir;
use vox_db::{ReadConsistency, VoxDb};

#[tokio::test]
async fn sync_embedded_replica_smoke() {
    let Some((url, token)) = common::remote_creds("VOX_DB_EMBEDDED_REPLICA_INTEGRATION") else {
        eprintln!(
            "skip: set VOX_DB_EMBEDDED_REPLICA_INTEGRATION=1 and VOX_DB_URL+VOX_DB_TOKEN (or Turso aliases)"
        );
        return;
    };
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("embedded_replica.db");
    let path_str = path.to_str().expect("utf8 path");
    let db = VoxDb::open_embedded_replica(path_str, &url, &token)
        .await
        .expect("open_embedded_replica");
    db.sync_for(ReadConsistency::ReplicaLatest)
        .await
        .expect("sync_for(ReplicaLatest)");
}
