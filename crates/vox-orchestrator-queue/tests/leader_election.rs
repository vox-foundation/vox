//! P0-T2: lock-leader election with heartbeat (vox-db backed).

use vox_db::{DbConfig, VoxDb};
use vox_orchestrator_queue::locks::leader::{LeaderRole, LockLeaderElection};

#[tokio::test]
async fn first_caller_becomes_leader() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
    let elect = LockLeaderElection::new(db, "node-A", "repo-1");
    let role = elect.try_become_leader().await.expect("claim");
    assert!(matches!(role, LeaderRole::Leader { .. }));
}

#[tokio::test]
async fn second_caller_becomes_follower_when_leader_alive() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
    let a = LockLeaderElection::new(db.clone(), "node-A", "repo-1");
    let b = LockLeaderElection::new(db, "node-B", "repo-1");

    let role_a = a.try_become_leader().await.expect("claim A");
    assert!(matches!(role_a, LeaderRole::Leader { .. }));

    let role_b = b.try_become_leader().await.expect("claim B");
    match role_b {
        LeaderRole::Follower { leader_node_id } => {
            assert_eq!(leader_node_id, "node-A")
        }
        LeaderRole::Leader { .. } => panic!("expected follower, got leader"),
    }
}

#[tokio::test]
async fn heartbeat_keeps_leadership_alive() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
    let elect = LockLeaderElection::with_ttl_ms(db, "node-A", "repo-1", 500);
    let _role = elect.try_become_leader().await.expect("claim");

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(elect.heartbeat().await.expect("heartbeat"), "still leader");
}

#[tokio::test]
async fn expired_lease_can_be_taken_over() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
    // Very short TTL so the lease expires quickly.
    let a = LockLeaderElection::with_ttl_ms(db.clone(), "node-A", "repo-1", 5);
    a.try_become_leader().await.expect("claim A");

    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    let b = LockLeaderElection::new(db, "node-B", "repo-1");
    let role = b.try_become_leader().await.expect("claim B");
    assert!(
        matches!(role, LeaderRole::Leader { .. }),
        "node-B should have taken over after A's lease expired"
    );
}
