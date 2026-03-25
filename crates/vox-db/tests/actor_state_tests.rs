use serde::{Deserialize, Serialize};
use vox_db::{DbConfig, VoxDb};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TestState {
    name: String,
    count: i32,
}

#[tokio::test]
async fn test_actor_state_crud() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();

    let state = TestState {
        name: "agent-1".to_string(),
        count: 42,
    };

    // Save
    db.save_actor_state("agent-1-key", &serde_json::to_string(&state).unwrap())
        .await
        .unwrap();

    // Load
    let loaded_str = db.load_actor_state("agent-1-key").await.unwrap().unwrap();
    let loaded: TestState = serde_json::from_str(&loaded_str).unwrap();
    assert_eq!(loaded, state);

    // Update
    let mut updated = state;
    updated.count = 43;
    db.save_actor_state("agent-1-key", &serde_json::to_string(&updated).unwrap())
        .await
        .unwrap();

    let loaded_str2 = db.load_actor_state("agent-1-key").await.unwrap().unwrap();
    let loaded2: TestState = serde_json::from_str(&loaded_str2).unwrap();
    assert_eq!(loaded2.count, 43);

    // Missing key
    let missing_str = db.load_actor_state("unknown-key").await.unwrap();
    assert!(missing_str.is_none());
}
