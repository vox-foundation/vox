use vox_db::{VoxDb, DbConfig, ResearchIngestRequest, ExternalResearchPacket, CapabilityMapRecord};
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn test_research_ingestion_and_listing() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    
    let packet = ExternalResearchPacket {
        topic: "rust_concurrency".to_string(),
        vendor: "rust_lang".to_string(),
        area: Some("threading".to_string()),
        source_url: "https://doc.rust-lang.org".to_string(),
        source_type: "doc".to_string(),
        title: "Rust Threading Guide".to_string(),
        captured_at: "2026-03-23T00:00:00Z".to_string(),
        summary: "Guide to threading in Rust".to_string(),
        raw_excerpt: "... Rayon ... Tokio ...".to_string(),
        claims: vec![json!({"claim": "safe"})],
        tags: vec!["safe".to_string(), "native".to_string()],
        confidence: 0.9,
        content_hash: "".to_string(),
        metadata: json!({}),
    };
    
    let mut req = ResearchIngestRequest {
        packet,
        body: "Rust provides safe concurrency via ownership...".to_string(),
        kb_id: None,
        embeddings: vec![],
    };
    
    let result = db.ingest_research_document_async(&mut req).await.unwrap();
    assert!(result.packet_id > 0);
    assert_eq!(result.chunk_ids.len(), 1);
    
    let packets = db.list_research_packets(Some("rust_lang"), None, 10).unwrap();
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].topic, "rust_concurrency");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_capability_map_crud() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    
    let record = CapabilityMapRecord {
        topic: "agent_orchestration".to_string(),
        vendor: "competitor_x".to_string(),
        area: "orchestration".to_string(),
        openclaw_capability: "dist_task".to_string(),
        vox_evidence: "Vox has native mesh support...".to_string(),
        status: "active".to_string(),
        advantage_direction: "vox".to_string(),
        recommended_action: "implement more tests".to_string(),
        linked_paths: vec!["crates/vox-orchestrator".to_string()],
        metadata: json!({}),
    };
    
    let id = db.store_capability_map_record(&record).unwrap();
    assert!(id > 0);
    
    let records = db.list_capability_map_records(Some("competitor_x"), None, 10).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].topic, "agent_orchestration");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_retrieval_diagnostics() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    
    let diag = vox_db::retrieval_diagnostics(&db).unwrap();
    // Fresh DB should have 0/0/0
    assert_eq!(diag.knowledge_nodes_count, 0);
    
    // Ingest something
    let packet = ExternalResearchPacket {
        topic: "test".to_string(),
        vendor: "test".to_string(),
        area: None,
        source_url: "url".to_string(),
        source_type: "web".to_string(),
        title: "title".to_string(),
        captured_at: "now".to_string(),
        summary: "sum".to_string(),
        raw_excerpt: "raw".to_string(),
        claims: vec![],
        tags: vec![],
        confidence: 0.5,
        content_hash: "".to_string(),
        metadata: json!({}),
    };
    let mut req = ResearchIngestRequest {
        packet,
        body: "content".to_string(),
        kb_id: None,
        embeddings: vec![],
    };
    db.ingest_research_document_async(&mut req).await.unwrap();
    
    let diag2 = vox_db::retrieval_diagnostics(&db).unwrap();
    assert_eq!(diag2.knowledge_nodes_count, 1);
}
