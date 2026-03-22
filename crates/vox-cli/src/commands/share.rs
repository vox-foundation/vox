//! `vox share` — share artifacts (workflows, skills, code) via the Vox marketplace.

use anyhow::{Context, Result};
use vox_pm::{ArtifactEntry, CodeStore};

async fn connect() -> Result<CodeStore> {
    vox_db::open_project_code_store()
        .await
        .context("Failed to open Arca CodeStore (see VOX_DB_URL/VOX_DB_TOKEN, VOX_DB_PATH, or project store)")
}

fn print_artifact(a: &ArtifactEntry) {
    println!(
        "  {} ({}) v{} by {} — ⬇{} ★{:.1} [{}]",
        a.name, a.artifact_type, a.version, a.author_id, a.downloads, a.avg_rating, a.status
    );
    if let Some(ref desc) = a.description {
        println!("    {}", desc);
    }
}

/// Run the `vox share publish` subcommand.
pub async fn publish(
    artifact_type: &str,
    name: &str,
    hash: &str,
    version: &str,
    description: Option<&str>,
    tags: Option<&str>,
) -> Result<()> {
    let store = connect().await?;
    let id = format!("{name}-{version}");
    store
        .publish_artifact(
            &id,
            artifact_type,
            name,
            description,
            "local-user",
            hash,
            version,
            tags,
            "public",
        )
        .await?;
    println!("✓ Published {name} v{version} as {artifact_type}");
    Ok(())
}

/// Run the `vox share search` subcommand.
pub async fn search(query: &str) -> Result<()> {
    let store = connect().await?;
    let results = store.search_artifacts(query).await?;
    if results.is_empty() {
        println!("No artifacts found for '{query}'");
    } else {
        println!("Found {} artifacts:", results.len());
        for a in &results {
            print_artifact(a);
        }
    }
    Ok(())
}

/// Run the `vox share list` subcommand.
pub async fn list(artifact_type: &str) -> Result<()> {
    let store = connect().await?;
    let results = store.list_artifacts(artifact_type).await?;
    if results.is_empty() {
        println!("No {artifact_type} artifacts found.");
    } else {
        println!("{} {} artifacts:", results.len(), artifact_type);
        for a in &results {
            print_artifact(a);
        }
    }
    Ok(())
}

/// Run the `vox share review` subcommand.
pub async fn review(artifact_id: &str, rating: i64, comment: Option<&str>) -> Result<()> {
    let store = connect().await?;
    store
        .submit_review(artifact_id, "local-user", "approved", comment, Some(rating))
        .await?;
    println!("✓ Reviewed {artifact_id} with rating {rating}/5");
    Ok(())
}
