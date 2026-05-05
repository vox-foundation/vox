use tempfile::tempdir;
use turso::params;
use vox_db::MemoryParams as SaveMemoryParams;
use vox_db::{RetrievalEvidenceSource, SearchBackend, VoxDb};

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

    store
        .upsert_knowledge_node("rust", "Rust", "Systems language", Some("lang"), None, None)
        .await
        .unwrap();
    store
        .upsert_knowledge_node("vox", "Vox", "AI language", Some("lang"), None, None)
        .await
        .unwrap();

    store
        .create_knowledge_edge("vox", "rust", "built_with", 1.0, None)
        .await
        .unwrap();

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

    store
        .store_embedding("test", "a", "model-v1", &vec_a, None, None)
        .await
        .unwrap();
    store
        .store_embedding("test", "b", "model-v1", &vec_b, None, None)
        .await
        .unwrap();
    store
        .store_embedding("test", "c", "model-v1", &vec_c, None, None)
        .await
        .unwrap();

    let query = vec![1.0, 0.0, 0.0];
    let results = store
        .search_similar_embeddings(&query, None, 2)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0.source_id, "a");
    assert_eq!(results[1].0.source_id, "b");
}

#[tokio::test]
async fn test_knowledge_search_falls_back_or_uses_fts() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let store: VoxDb = VoxDb::open(db_path.to_str().unwrap()).await.unwrap();

    store
        .upsert_knowledge_node(
            "fts-node-1",
            "Rust vectors",
            "Embeddings and retrieval in vox",
            Some("concept"),
            None,
            None,
        )
        .await
        .unwrap();

    let hits = store.query_knowledge_nodes("Rust", 10).await.unwrap();
    assert!(!hits.is_empty());
    assert!(hits.iter().any(|(id, _, _)| id == "fts-node-1"));

    let cap = store.sqlite_capabilities_snapshot().await.unwrap();
    if cap.fts5_reported {
        let rows = store
            .query_all(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='knowledge_nodes_fts'",
                (),
            )
            .await
            .unwrap();
        assert!(
            !rows.is_empty(),
            "fts5 reported but knowledge_nodes_fts table missing"
        );
    }
}

#[tokio::test]
async fn test_search_document_chunks_falls_back_or_uses_fts() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("chunks_fts.db");
    let store: VoxDb = VoxDb::open(db_path.to_str().unwrap()).await.unwrap();

    store
        .connection()
        .execute(
            "INSERT INTO search_documents (source_uri, title) VALUES (?1, ?2)",
            params!["test://chunk-path", "RAG doc title"],
        )
        .await
        .unwrap();
    let doc_id = store.connection().last_insert_rowid();

    store
        .connection()
        .execute(
            "INSERT INTO search_document_chunks (document_id, chunk_index, body_text) VALUES (?1, 0, ?2)",
            params![doc_id, "hybrid retrieval alpha token for vox chunks"],
        )
        .await
        .unwrap();

    let hits = store
        .query_search_document_chunks("alpha", 10)
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].1, doc_id);
    assert!(hits[0].2.contains("alpha"));
    assert_eq!(hits[0].3, "RAG doc title");

    let cap = store.sqlite_capabilities_snapshot().await.unwrap();
    if cap.fts5_reported {
        let rows = store
            .query_all(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='search_document_chunks_fts'",
                (),
            )
            .await
            .unwrap();
        assert!(
            !rows.is_empty(),
            "fts5 reported but search_document_chunks_fts table missing"
        );
    }
}

#[tokio::test]
async fn test_search_document_chunks_hybrid_uses_vector_hits() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("chunks_hybrid.db");
    let store: VoxDb = VoxDb::open(db_path.to_str().unwrap()).await.unwrap();

    store
        .connection()
        .execute(
            "INSERT INTO search_documents (source_uri, title) VALUES (?1, ?2)",
            params!["test://chunk-hybrid", "Hybrid doc"],
        )
        .await
        .unwrap();
    let doc_id = store.connection().last_insert_rowid();

    store
        .replace_search_document_chunks_with_refs(
            doc_id,
            &[String::from("vector grounded chunk")],
            &[None],
        )
        .await
        .unwrap();
    let rows = store
        .query_all(
            "SELECT id FROM search_document_chunks WHERE document_id = ?1 AND chunk_index = 0",
            params![doc_id],
        )
        .await
        .unwrap();
    let chunk_id: i64 = rows[0].get(0).unwrap();
    store
        .store_embedding(
            "search_document_chunk",
            &chunk_id.to_string(),
            "test-model",
            &[1.0, 0.0, 0.0],
            Some("vector grounded chunk"),
            None,
        )
        .await
        .unwrap();

    let (hits, diagnostics) = store
        .query_search_document_chunks_hybrid("grounded", Some(&[1.0, 0.0, 0.0]), 5, 0.60)
        .await
        .unwrap();
    assert!(!hits.is_empty());
    assert_eq!(hits[0].chunk_id, chunk_id.to_string());
    assert_eq!(
        hits[0].evidence_source,
        RetrievalEvidenceSource::Hybrid,
        "lexical + vector chunk ids must align for fused Hybrid provenance"
    );
    assert!(diagnostics.backends_used.contains(&SearchBackend::ChunkFts));
    assert!(
        diagnostics
            .backends_used
            .contains(&SearchBackend::ChunkVector)
    );
}

#[tokio::test]
async fn test_search_document_chunks_hybrid_chunk_vector_weight_affects_score() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("chunks_hybrid_weight.db");
    let store: VoxDb = VoxDb::open(db_path.to_str().unwrap()).await.unwrap();

    store
        .connection()
        .execute(
            "INSERT INTO search_documents (source_uri, title) VALUES (?1, ?2)",
            params!["test://chunk-hybrid-weight", "Weight doc"],
        )
        .await
        .unwrap();
    let doc_id = store.connection().last_insert_rowid();

    store
        .replace_search_document_chunks_with_refs(
            doc_id,
            &[String::from("alpha grounded retrieval overlap")],
            &[None],
        )
        .await
        .unwrap();
    let rows = store
        .query_all(
            "SELECT id FROM search_document_chunks WHERE document_id = ?1 AND chunk_index = 0",
            params![doc_id],
        )
        .await
        .unwrap();
    let chunk_db_id: i64 = rows[0].get(0).unwrap();
    store
        .store_embedding(
            "search_document_chunk",
            &chunk_db_id.to_string(),
            "test-model",
            &[1.0, 0.0, 0.0],
            Some("alpha grounded retrieval overlap"),
            None,
        )
        .await
        .unwrap();

    let qv = &[1.0_f32, 0.0, 0.0];
    let (low_vec_weight, _) = store
        .query_search_document_chunks_hybrid("grounded", Some(qv), 5, 0.05)
        .await
        .unwrap();
    let (high_vec_weight, _) = store
        .query_search_document_chunks_hybrid("grounded", Some(qv), 5, 0.95)
        .await
        .unwrap();
    assert!(!low_vec_weight.is_empty() && !high_vec_weight.is_empty());
    assert!(
        (low_vec_weight[0].score - high_vec_weight[0].score).abs() > f32::EPSILON,
        "fusion weight should change the fused score when both lexical and vector legs hit"
    );
}
