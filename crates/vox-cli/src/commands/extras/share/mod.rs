//! `vox share` — share artifacts (workflows, skills, code) via the Vox marketplace.

use anyhow::Result;
use vox_db::VoxDb;

/// Get a VoxDb connection (workspace journey: `.vox/store.db` vs canonical per env).
async fn connect() -> Result<VoxDb> {
    crate::workspace_db::connect_cli_workspace_voxdb().await
}

/// Run the `vox share publish` subcommand.
pub async fn publish(
    _artifact_type: &str,
    name: &str,
    _hash: &str,
    version: &str,
    _description: Option<&str>,
    _tags: Option<&str>,
) -> Result<()> {
    let _store = connect().await?;
    println!("✓ Publish for {name} v{version} — artifact marketplace in progress.");
    Ok(())
}

/// Run the `vox share search` subcommand.
pub async fn search(query: &str) -> Result<()> {
    let store: VoxDb = connect().await?;
    let packages = store.search_packages(query, 100).await.unwrap_or_default();
    if packages.is_empty() {
        println!("No artifacts found for '{query}'");
    } else {
        println!("Found {} packages:", packages.len());
        for pkg in &packages {
            let d = pkg.description.as_deref().unwrap_or("");
            println!("  {} v{} — {d}", pkg.name, pkg.version);
        }
    }
    Ok(())
}

/// Run the `vox share list` subcommand.
pub async fn list(artifact_type: &str) -> Result<()> {
    let store: VoxDb = connect().await?;
    let packages = store.search_packages("", 500).await.unwrap_or_default();
    let filtered: Vec<_> = packages
        .into_iter()
        .filter(|_| artifact_type == "package" || artifact_type == "all")
        .collect();
    if filtered.is_empty() {
        println!("No {artifact_type} artifacts found.");
    } else {
        println!("{} {} artifacts:", filtered.len(), artifact_type);
        for pkg in &filtered {
            let d = pkg.description.as_deref().unwrap_or("");
            println!("  {} v{} — {d}", pkg.name, pkg.version);
        }
    }
    Ok(())
}

/// Run the `vox share review` subcommand.
pub async fn review(artifact_id: &str, rating: i64, _comment: Option<&str>) -> Result<()> {
    let _store = connect().await?;
    println!("✓ Review for {artifact_id} with rating {rating}/5 — marketplace in progress.");
    Ok(())
}
