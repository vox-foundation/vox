/// List Codex-bound MCP invocable names (namespace `invocable` in `names`).
pub async fn capability_list() -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let pairs = db
        .list_names_in_namespace("invocable")
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
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
