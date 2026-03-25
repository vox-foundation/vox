/// List Codex-bound MCP invocable names (namespace `invocable` in `names`).
pub async fn capability_list() -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let mut rows = db
        .connection()
        .query(
            "SELECT name, hash FROM names WHERE namespace = 'invocable' ORDER BY name ASC",
            (),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut pairs: Vec<(String, String)> = Vec::new();
    while let Some(row) = rows.next().await? {
        let name: String = row.get(0).map_err(|e| anyhow::anyhow!("{e}"))?;
        let hash: String = row.get(1).map_err(|e| anyhow::anyhow!("{e}"))?;
        pairs.push((name, hash));
    }
    println!(
        "Codex invocable bindings (namespace `invocable`): {} entries",
        pairs.len()
    );
    println!("{:<48} hash (prefix)", "name");
    for (name, hash) in &pairs {
        let prefix: String = hash.chars().take(16).collect();
        let suffix = if hash.len() > 16 { "…" } else { "" };
        println!("{:<48} {}{}", name, prefix, suffix);
    }
    if pairs.is_empty() {
        println!("(none — run sync-invocables with an MCP invocables JSON array to populate)");
    }
    Ok(())
}

/// Ingest `mcp-invocables.json` (JSON array) into Codex CAS + `names`.
pub async fn sync_invocables(path: &std::path::Path) -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let mut engine = vox_db::InvocableSyncEngine::new(&db);
    let count = engine
        .sync_from_file(path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Synced {} invocable(s) from {}", count, path.display());
    Ok(())
}
