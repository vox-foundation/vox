use super::helpers::split_csv;

/// List normalized external research packets stored in Codex.
pub async fn research_list(
    vendor: Option<&str>,
    topic: Option<&str>,
    limit: i64,
) -> anyhow::Result<()> {
    let vendor = vendor.map(ToString::to_string);
    let topic = topic.map(ToString::to_string);
    let rows = tokio::task::spawn_blocking(
        move || -> anyhow::Result<Vec<vox_db::ExternalResearchPacket>> {
            let db = vox_db::VoxDb::connect_default_sync().map_err(|e| anyhow::anyhow!("{e}"))?;
            let rows = db
                .list_research_packets(vendor.as_deref(), topic.as_deref(), limit)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            db.shutdown_for_drop();
            Ok(rows)
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("research list task failed: {e}"))??;
    if rows.is_empty() {
        println!("(no research packets)");
        return Ok(());
    }
    println!("{:<16} {:<24} {:<40} Conf", "Vendor", "Topic", "Title");
    for row in rows {
        println!(
            "{:<16} {:<24} {:<40} {:.2}",
            row.vendor, row.topic, row.title, row.confidence
        );
    }
    Ok(())
}

/// Persist one competitor capability-map row into Codex.
#[allow(clippy::too_many_arguments)]
pub async fn research_map_add(
    vendor: &str,
    topic: &str,
    area: &str,
    openclaw_capability: &str,
    vox_evidence: &str,
    status: &str,
    advantage_direction: &str,
    recommended_action: &str,
    linked_paths: Option<&str>,
) -> anyhow::Result<()> {
    let rec = vox_db::CapabilityMapRecord {
        topic: topic.to_string(),
        vendor: vendor.to_string(),
        area: area.to_string(),
        openclaw_capability: openclaw_capability.to_string(),
        vox_evidence: vox_evidence.to_string(),
        status: status.to_string(),
        advantage_direction: advantage_direction.to_string(),
        recommended_action: recommended_action.to_string(),
        linked_paths: split_csv(linked_paths),
        metadata: serde_json::json!({
            "created_from": "vox codex research-map-add",
        }),
    };
    let id = tokio::task::spawn_blocking(move || -> anyhow::Result<i64> {
        let db = vox_db::VoxDb::connect_default_sync().map_err(|e| anyhow::anyhow!("{e}"))?;
        let id = db
            .store_capability_map_record(&rec)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        db.shutdown_for_drop();
        Ok(id)
    })
    .await
    .map_err(|e| anyhow::anyhow!("research map add task failed: {e}"))??;
    println!("Capability map row persisted: {id}");
    Ok(())
}

/// List capability-map rows stored in Codex.
pub async fn research_map_list(
    vendor: Option<&str>,
    topic: Option<&str>,
    limit: i64,
) -> anyhow::Result<()> {
    let vendor = vendor.map(ToString::to_string);
    let topic = topic.map(ToString::to_string);
    let rows = tokio::task::spawn_blocking(
        move || -> anyhow::Result<Vec<vox_db::CapabilityMapRecord>> {
            let db = vox_db::VoxDb::connect_default_sync().map_err(|e| anyhow::anyhow!("{e}"))?;
            let rows = db
                .list_capability_map_records(vendor.as_deref(), topic.as_deref(), limit)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            db.shutdown_for_drop();
            Ok(rows)
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("research map list task failed: {e}"))??;
    if rows.is_empty() {
        println!("(no capability-map rows)");
        return Ok(());
    }
    for row in rows {
        println!("- [{}] {} / {}", row.vendor, row.topic, row.area);
        println!("  capability : {}", row.openclaw_capability);
        println!("  status     : {}", row.status);
        println!("  direction  : {}", row.advantage_direction);
        println!("  action     : {}", row.recommended_action);
        println!("  evidence   : {}", row.vox_evidence);
        if !row.linked_paths.is_empty() {
            println!("  links      : {}", row.linked_paths.join(", "));
        }
    }
    Ok(())
}
