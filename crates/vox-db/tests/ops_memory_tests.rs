use vox_db::VoxDb;
use vox_db::MemoryParams as SaveMemoryParams;
use tempfile::tempdir;

#[tokio::test]
async fn test_memory_save_and_recall() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let store: VoxDb = VoxDb::open(db_path.to_str().unwrap()).await.unwrap();

    let agent_id = "mem_agent";
    let session_id = "sess_mem";
    
    let params = SaveMemoryParams {
        agent_id,
        session_id,
        memory_type: "observation",
        content: "The user likes coffee.",
        metadata: Some("{}"),
        importance: 0.8,
        vcs_snapshot_id: None,
    };
    
    store.save_memory(params).await.unwrap();
    
    let recalled = store.recall_memory(agent_id, None, 10, None).await.unwrap();
    assert_eq!(recalled.len(), 1);
    assert_eq!(recalled[0].content, "The user likes coffee.");
}

#[tokio::test]
async fn test_knowledge_graph_traversal() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let store: VoxDb = VoxDb::open(db_path.to_str().unwrap()).await.unwrap();

    store.upsert_knowledge_node("rust", "Rust", "Systems language", Some("lang"), None, None).await.unwrap();
    store.upsert_knowledge_node("vox", "Vox", "AI language", Some("lang"), None, None).await.unwrap();
    
    store.create_knowledge_edge("vox", "rust", "built_with", 1.0, None).await.unwrap();
    
    let neighbors = store.get_knowledge_neighbors("vox").await.unwrap();
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].0, "rust");
    assert_eq!(neighbors[0].2, "built_with");
}

#[tokio::test]
async fn test_embedding_similarity() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let store: VoxDb = VoxDb::open(db_path.to_str().unwrap()).await.unwrap();

    let vec_a = vec![1.0, 0.0, 0.0];
    let vec_b = vec![0.9, 0.1, 0.0];
    let vec_c = vec![0.0, 1.0, 0.0];

    store.store_embedding("test", "a", "model-v1", &vec_a, None, None).await.unwrap();
    store.store_embedding("test", "b", "model-v1", &vec_b, None, None).await.unwrap();
    store.store_embedding("test", "c", "model-v1", &vec_c, None, None).await.unwrap();

    let query = vec![1.0, 0.0, 0.0];
    let results = store.search_similar_embeddings(&query, None, 2).await.unwrap();
    
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0.source_id, "a");
    assert_eq!(results[1].0.source_id, "b");
}
