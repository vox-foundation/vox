use anyhow::Context;

use super::helpers::{extract_md_title, html_to_text_lossy, split_csv, summarize_text};

/// Fetch a URL and persist a normalized external research packet plus searchable document chunks.
#[allow(clippy::too_many_arguments)]
pub async fn research_ingest_url(
    vendor: &str,
    topic: &str,
    url: &str,
    title: Option<&str>,
    summary: Option<&str>,
    source_type: &str,
    area: Option<&str>,
    kb_id: Option<&str>,
    tags: Option<&str>,
    confidence: f64,
) -> anyhow::Result<()> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("failed to fetch {url}"))?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        anyhow::bail!("fetch failed for {url}: HTTP {status}");
    }

    let title = title
        .map(ToString::to_string)
        .unwrap_or_else(|| url.to_string());
    let plain_text = html_to_text_lossy(&body);
    let summary = summary
        .map(ToString::to_string)
        .unwrap_or_else(|| summarize_text(&plain_text, 320));
    let excerpt = summarize_text(&plain_text, 800);
    let packet = vox_db::ExternalResearchPacket {
        topic: topic.to_string(),
        vendor: vendor.to_string(),
        area: area.map(ToString::to_string),
        source_url: url.to_string(),
        source_type: source_type.to_string(),
        title,
        captured_at: chrono::Utc::now().to_rfc3339(),
        summary,
        raw_excerpt: excerpt,
        claims: vec![],
        tags: split_csv(tags),
        confidence,
        content_hash: String::new(),
        metadata: serde_json::json!({
            "http_status": status.as_u16(),
            "ingested_from": "vox codex research-ingest-url",
        }),
    };
    let kb_id = kb_id
        .map(ToString::to_string)
        .or_else(|| Some(format!("ecosystem/{vendor}")));
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<vox_db::ResearchIngestResult> {
        let mut req = vox_db::ResearchIngestRequest {
            packet,
            body: plain_text,
            kb_id,
            embeddings: vec![],
        };
        let db = vox_db::VoxDb::connect_default_sync().map_err(|e| anyhow::anyhow!("{e}"))?;
        let result = db
            .ingest_research_document(&mut req)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        db.shutdown_for_drop();
        Ok(result)
    })
    .await
    .map_err(|e| anyhow::anyhow!("research ingest task failed: {e}"))??;

    println!("Research source persisted");
    let doc_id = result
        .document_id
        .map(|d| d.to_string())
        .unwrap_or_else(|| "—".to_string());
    println!("  Packet ID   : {}", result.packet_id);
    println!("  Document ID : {doc_id}");
    println!("  Chunks      : {}", result.chunk_ids.len());
    println!("  KB ID       : {}", result.kb_id.clone().unwrap_or_default());
    println!("  Hash        : {}", result.content_hash);
    Ok(())
}

/// Ingest a local markdown file into Codex as an ecosystem research packet.
pub async fn research_ingest_file(
    vendor: &str,
    topic: &str,
    path: &std::path::Path,
    area: Option<&str>,
    kb_id: Option<&str>,
    tags: Option<&str>,
    confidence: f64,
) -> anyhow::Result<()> {
    let body = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let title = extract_md_title(&body);
    let summary = summarize_text(&body, 320);
    let excerpt = summarize_text(&body, 800);
    let source_url = format!(
        "file://{}",
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
    );
    let packet = vox_db::ExternalResearchPacket {
        topic: topic.to_string(),
        vendor: vendor.to_string(),
        area: area.map(ToString::to_string),
        source_url,
        source_type: "local_doc".to_string(),
        title,
        captured_at: chrono::Utc::now().to_rfc3339(),
        summary,
        raw_excerpt: excerpt,
        claims: vec![],
        tags: split_csv(tags),
        confidence,
        content_hash: String::new(),
        metadata: serde_json::json!({
            "ingested_from": "vox codex research-ingest-file",
            "file_path": path.display().to_string(),
        }),
    };
    let kb_id = kb_id
        .map(ToString::to_string)
        .or_else(|| Some(format!("ecosystem/{vendor}")));
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<vox_db::ResearchIngestResult> {
        let mut req = vox_db::ResearchIngestRequest {
            packet,
            body,
            kb_id,
            embeddings: vec![],
        };
        let db = vox_db::VoxDb::connect_default_sync().map_err(|e| anyhow::anyhow!("{e}"))?;
        let result = db
            .ingest_research_document(&mut req)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        db.shutdown_for_drop();
        Ok(result)
    })
    .await
    .map_err(|e| anyhow::anyhow!("research ingest file task failed: {e}"))??;

    println!("Research document persisted");
    let doc_id = result
        .document_id
        .map(|d| d.to_string())
        .unwrap_or_else(|| "—".to_string());
    println!("  Packet ID   : {}", result.packet_id);
    println!("  Document ID : {doc_id}");
    println!("  Chunks      : {}", result.chunk_ids.len());
    println!("  KB ID       : {}", result.kb_id.clone().unwrap_or_default());
    println!("  Hash        : {}", result.content_hash);
    Ok(())
}
