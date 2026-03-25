//! `vox snippet` — save, search, and manage code snippets.

use anyhow::{Context, Result};
use vox_db::VoxDb;
use vox_db::store::SaveSnippetParams;

async fn connect() -> Result<VoxDb> {
    vox_db::open_project_db()
        .await
        .context("Failed to open Arca VoxDb (see VOX_DB_URL/VOX_DB_TOKEN, VOX_DB_PATH, or project store)")
}

fn language_from_path(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("vox") => "vox",
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") => "python",
        Some("md") => "markdown",
        _ => "text",
    }
}

/// Save a code snippet from a file.
pub async fn save(
    file: &std::path::Path,
    title: &str,
    description: Option<&str>,
    tags: Option<&str>,
) -> Result<()> {
    let store: VoxDb = connect().await?;
    let code = std::fs::read_to_string(file)?;
    let lang = language_from_path(file);
    let id = store
        .save_snippet(SaveSnippetParams {
            language: lang,
            title,
            code: &code,
            description,
            tags,
            author_id: None,
            source_ref: Some(&file.display().to_string()),
            embedding_ref: None,
        })
        .await
        .context("save_snippet")?;
    println!("Saved snippet id={id} title={title:?} language={lang}");
    Ok(())
}

/// Search code snippets.
pub async fn search(query: &str) -> Result<()> {
    let store: VoxDb = connect().await?;
    let rows = store
        .search_snippets(query, None)
        .await
        .context("search_snippets")?;
    if rows.is_empty() {
        println!("No snippets found for '{query}'.");
        return Ok(());
    }
    for row in rows {
        println!(
            "[{}] {} ({}) — {}",
            row.id,
            row.title,
            row.language,
            row.description.as_deref().unwrap_or("")
        );
    }
    Ok(())
}

/// Export snippets as JSON (for RLHF/RAG pipelines).
pub async fn export(limit: i64) -> Result<()> {
    let store: VoxDb = connect().await?;
    let rows = store
        .search_snippets("%", None)
        .await
        .context("search_snippets for export")?;
    let cap = limit.max(0) as usize;
    let trimmed: Vec<serde_json::Value> = rows
        .into_iter()
        .take(cap)
        .map(|row| {
            serde_json::json!({
                "id": row.id,
                "language": row.language,
                "title": row.title,
                "code": row.code,
                "description": row.description,
                "tags": row.tags,
            })
        })
        .collect();
    println!("{}", serde_json::json!({ "snippets": trimmed }));
    Ok(())
}
