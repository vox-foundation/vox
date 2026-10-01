//! Knowledge base management commands for the Vox GUI.
//! Handles browsing, searching, node deletion, health metrics, manual text ingestion,
//! URL ingestion (via web scraping), and persisting research session findings.

use tauri::State;
use vox_db::KnowledgeNodeRecord;

use crate::commands::gui_db_pool::{GuiDbPool, map_db_err};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeNodeRow {
    pub id: String,
    pub label: String,
    pub snippet: String,
    pub node_type: Option<String>,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct KbHealthDto {
    pub node_count: usize,
    pub edge_count: usize,
    pub fts_available: bool,
    pub corpus_counts: std::collections::HashMap<String, usize>,
}

#[tauri::command]
pub async fn list_knowledge_nodes(
    pool: State<'_, GuiDbPool>,
    node_type: Option<String>,
    page: Option<usize>,
    limit: Option<usize>,
) -> Result<Vec<KnowledgeNodeRow>, String> {
    let db = pool.handle()?;
    let p = page.unwrap_or(0);
    let l = limit.unwrap_or(50);
    let records = db
        .list_knowledge_nodes(node_type.as_deref(), p, l)
        .await
        .map_err(map_db_err)?;

    Ok(records
        .into_iter()
        .map(|r: KnowledgeNodeRecord| {
            let snippet = if r.content.chars().count() > 200 {
                let first_200: String = r.content.chars().take(200).collect();
                format!("{first_200}...")
            } else {
                r.content
            };
            KnowledgeNodeRow {
                id: r.id,
                label: r.label,
                snippet,
                node_type: r.node_type,
                created_at: r.created_at,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn delete_knowledge_node(pool: State<'_, GuiDbPool>, id: String) -> Result<(), String> {
    let db = pool.handle()?;
    db.delete_knowledge_node(&id).await.map_err(map_db_err)
}

#[tauri::command]
pub async fn get_kb_health(pool: State<'_, GuiDbPool>) -> Result<KbHealthDto, String> {
    let db = pool.handle()?;
    let health = db.get_knowledge_health().await.map_err(map_db_err)?;
    Ok(KbHealthDto {
        node_count: health.node_count,
        edge_count: health.edge_count,
        fts_available: health.fts_available,
        corpus_counts: health.corpus_counts,
    })
}

#[tauri::command]
pub async fn ingest_text(
    pool: State<'_, GuiDbPool>,
    title: String,
    content: String,
    source_url: Option<String>,
) -> Result<String, String> {
    let db = pool.handle()?;
    if content.trim().is_empty() {
        return Err("Content cannot be empty".to_string());
    }
    let label = if title.trim().is_empty() {
        "Manual Ingest".to_string()
    } else {
        title.trim().to_string()
    };
    let content_hash = vox_crypto::hash_fast_hex(content.as_bytes());
    let node_id = format!("doc:{content_hash}");
    let meta = serde_json::json!({
        "title": label,
        "source_url": source_url,
        "ingest_type": "text",
    })
    .to_string();

    db.upsert_knowledge_node(
        &node_id,
        &label,
        &content,
        Some("document"),
        Some(&meta),
        None,
    )
    .await
    .map_err(map_db_err)?;

    Ok(node_id)
}

fn get_robots_url(target_url: &str) -> Option<(String, String)> {
    let remainder = target_url
        .strip_prefix("http://")
        .map(|r| ("http://", r))
        .or_else(|| target_url.strip_prefix("https://").map(|r| ("https://", r)))?;
    let (scheme, rest) = remainder;
    let (host, path) = match rest.split_once('/') {
        Some((h, p)) => (h, format!("/{}", p)),
        None => (rest, "/".to_string()),
    };
    if host.is_empty() {
        return None;
    }
    let robots_url = format!("{scheme}{host}/robots.txt");
    Some((robots_url, path))
}

async fn check_robots_allowed(client: &reqwest::Client, target_url: &str) -> bool {
    let Some((robots_url, target_path)) = get_robots_url(target_url) else {
        return true;
    };
    if let Ok(resp) = client.get(&robots_url).send().await {
        if resp.status().is_success() {
            if let Ok(text) = resp.text().await {
                let mut in_relevant_agent = false;
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    if let Some((k, v)) = trimmed.split_once(':') {
                        let key = k.trim().to_lowercase();
                        let val = v.trim();
                        if key == "user-agent" {
                            in_relevant_agent = val == "*" || val.to_lowercase().contains("vox");
                        } else if in_relevant_agent && key == "disallow" {
                            if val.is_empty() {
                                continue;
                            }
                            if val == "/" || target_path.starts_with(val) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
    }
    true
}

#[tauri::command]
pub async fn ingest_url(pool: State<'_, GuiDbPool>, url: String) -> Result<String, String> {
    let db = pool.handle()?;
    let clean_url = url.trim();
    if clean_url.is_empty() {
        return Err("URL cannot be empty".to_string());
    }

    let policy = vox_search::policy::SearchPolicy::from_env();
    let timeout_ms = policy.scraper_timeout_ms.max(1000);

    if policy.scraper_robots_txt_respect {
        let client = vox_http_client::client_builder()
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .user_agent("VoxResearchBot/1.0 (+https://vox.dev/research-bot)")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

        if !check_robots_allowed(&client, clean_url).await {
            return Err(format!("Scraping disallowed by robots.txt for {clean_url}"));
        }
    }

    let doc = vox_search::scraper::fetch_and_extract(clean_url, timeout_ms)
        .await
        .map_err(|e| format!("Failed to scrape URL {clean_url}: {e}"))?;

    let url_hash = vox_crypto::hash_fast_hex(clean_url.as_bytes());
    let node_id = format!("web:{url_hash}");
    let meta = serde_json::json!({
        "url": doc.url,
        "title": doc.title,
        "text_density": doc.text_density,
        "ingest_type": "url",
    })
    .to_string();

    db.upsert_knowledge_node(
        &node_id,
        &doc.title,
        &doc.markdown,
        Some("web_scrape"),
        Some(&meta),
        None,
    )
    .await
    .map_err(map_db_err)?;

    Ok(node_id)
}

#[derive(serde::Deserialize)]
struct MinimalArtifactCitation {
    #[serde(default)]
    url: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    snippet: String,
}

#[derive(serde::Deserialize)]
struct MinimalArtifactResult {
    #[serde(default)]
    citations: Vec<MinimalArtifactCitation>,
}

#[derive(serde::Deserialize)]
struct MinimalArtifact {
    #[serde(default)]
    result: Option<MinimalArtifactResult>,
}

#[tauri::command]
pub async fn save_research_session_to_kb(
    pool: State<'_, GuiDbPool>,
    session_id: i64,
) -> Result<usize, String> {
    let db = pool.handle()?;
    let session = db
        .get_research_session(session_id)
        .await
        .map_err(map_db_err)?
        .ok_or_else(|| format!("Research session {session_id} not found"))?;

    let artifact = db
        .get_research_artifact(session_id)
        .await
        .map_err(map_db_err)?
        .ok_or_else(|| format!("No artifact found for research session {session_id}"))?;

    let parsed: MinimalArtifact = serde_json::from_str(&artifact.artifact_json)
        .map_err(|e| format!("Failed to parse session artifact JSON: {e}"))?;

    let citations = parsed.result.map(|r| r.citations).unwrap_or_default();

    let sources: Vec<vox_orchestrator_mcp::chat_tools::chat::research_turn::Source> = citations
        .into_iter()
        .enumerate()
        .map(
            |(idx, c)| vox_orchestrator_mcp::chat_tools::chat::research_turn::Source {
                n: idx + 1,
                url: c.url,
                title: c.title,
                engine: "research_session".to_string(),
                snippet: c.snippet,
            },
        )
        .collect();

    let saved = vox_orchestrator_mcp::chat_tools::chat::research_turn::persist_research_findings_to_knowledgebase(
        Some(db.as_ref()),
        &session.query_text,
        &sources,
        artifact.report_markdown.as_deref(),
    )
    .await;

    Ok(saved)
}
