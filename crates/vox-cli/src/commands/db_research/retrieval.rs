/// Show retrieval diagnostics (embeddings/graph/adaptive fusion state).
pub async fn retrieval_status() -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let diag = vox_db::retrieval_diagnostics(&db).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Retrieval diagnostics");
    println!("  Embeddings      : {}", diag.embeddings_count);
    println!("  KnowledgeNodes  : {}", diag.knowledge_nodes_count);
    println!("  KnowledgeEdges  : {}", diag.knowledge_edges_count);
    println!("  VectorWeight    : {}", diag.vector_weight);
    if let Some(ms) = diag.last_retrieval_latency_ms {
        println!("  LastLatencyMs   : {ms}");
    }
    println!("  ModeSplits      : {:?}", diag.retrieval_mode_splits);
    Ok(())
}
