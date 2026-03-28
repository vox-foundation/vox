//! Optional remote sync smoke test for [`vox_db::VoxDb::sync_for`].
//!
//! Enables `ReadConsistency::ReplicaLatest` to exercise `sync()` / `pull` on a sync-backed client
//! without requiring credentials in default CI.
//!
//! **Run:** set `VOX_DB_SYNC_INTEGRATION=1` plus `VOX_DB_URL` / `VOX_DB_TOKEN` (or Turso alias vars), then
//! `cargo test -p vox-db sync_for_replica_latest_on_remote_smoke -- --nocapture`.
//! Without the gate, the test returns immediately (passes in CI).

#[path = "common/mod.rs"]
mod common;

use vox_db::{ReadConsistency, VoxDb};

#[tokio::test]
async fn sync_for_replica_latest_on_remote_smoke() {
    let Some((url, token)) = common::remote_creds("VOX_DB_SYNC_INTEGRATION") else {
        eprintln!(
            "skip: set VOX_DB_SYNC_INTEGRATION=1 and VOX_DB_URL+VOX_DB_TOKEN (or Turso aliases)"
        );
        return;
    };
    let db = VoxDb::open_remote(&url, &token).await.expect("open_remote");
    db.sync_for(ReadConsistency::ReplicaLatest)
        .await
        .expect("sync_for(ReplicaLatest)");
}
