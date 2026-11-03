use std::path::Path;

/// An operator-chosen root is not proven curated; Library indexing is the curated path.
const MIRROR_TIER: vox_db::ContextTrustTier = vox_db::ContextTrustTier::Untrusted;

/// Mirror a markdown tree into Codex `search_documents` / chunk rows for hybrid RAG.
pub async fn mirror_search_corpus(root: &Path, source_uri_prefix: &str) -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let n = vox_search::ingest_markdown_tree(&db, root, source_uri_prefix, MIRROR_TIER).await?;
    println!("Ingested {n} markdown file(s); source_uri prefix: {source_uri_prefix:?}");
    Ok(())
}

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

#[cfg(test)]
mod tests {
    #[test]
    fn mirror_search_corpus_never_writes_a_curated_tier() {
        assert_eq!(super::MIRROR_TIER, vox_db::ContextTrustTier::Untrusted);
    }
}
