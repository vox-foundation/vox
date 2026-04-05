use anyhow::{Context, Result};
use vox_skills::ars_shim::context::{
    ArsContextBundle, ContextPolicy, RetrievalTier, assemble_bundle,
};

const CONTEXT_MEMORY_TYPES: &[&str] = &["session_turn", "message", "tool_call"];

#[cfg_attr(not(test), allow(dead_code))]
pub async fn context_assemble_bundle(
    tier: &str,
    policy_json: Option<&str>,
    agent_id: Option<&str>,
    codex_override: Option<&vox_db::Codex>,
) -> Result<ArsContextBundle> {
    let tier_parsed =
        RetrievalTier::parse(tier).ok_or_else(|| anyhow::anyhow!("Invalid tier: {}", tier))?;
    let mut policy: ContextPolicy = if let Some(p) = policy_json {
        serde_json::from_str(p).context("Invalid policy JSON")?
    } else {
        ContextPolicy {
            max_items: 10,
            ..Default::default()
        }
    };
    policy.tier = tier_parsed;

    let mut sources: Vec<serde_json::Value> = Vec::new();
    if let Some(db) = codex_override {
        let limit = policy.max_items as i64;
        if let Some(aid) = agent_id {
            if let Ok(entries) = db.recall_memory(aid, None, limit, None).await {
                for e in entries {
                    if let Ok(v) = serde_json::to_value(&e) {
                        sources.push(v);
                    }
                }
            }
        } else {
            let per_type = (limit / CONTEXT_MEMORY_TYPES.len() as i64).max(1);
            for memory_type in CONTEXT_MEMORY_TYPES {
                if let Ok(entries) = db
                    .recall_memory("", Some(memory_type), per_type, None)
                    .await
                {
                    for e in entries {
                        if let Ok(v) = serde_json::to_value(&e) {
                            sources.push(v);
                        }
                    }
                }
            }
        }
    } else if let Ok(db) = vox_db::Codex::connect_default().await {
        let limit = policy.max_items as i64;
        if let Some(aid) = agent_id {
            if let Ok(entries) = db.recall_memory(aid, None, limit, None).await {
                for e in entries {
                    if let Ok(v) = serde_json::to_value(&e) {
                        sources.push(v);
                    }
                }
            }
        } else {
            let per_type = (limit / CONTEXT_MEMORY_TYPES.len() as i64).max(1);
            for memory_type in CONTEXT_MEMORY_TYPES {
                if let Ok(entries) = db
                    .recall_memory("", Some(memory_type), per_type, None)
                    .await
                {
                    for e in entries {
                        if let Ok(v) = serde_json::to_value(&e) {
                            sources.push(v);
                        }
                    }
                }
            }
        }
        db.shutdown_for_drop();
    }

    Ok(assemble_bundle("cli-context", &policy, sources))
}

pub async fn context_assemble(
    tier: &str,
    policy_json: Option<&str>,
    agent_id: Option<&str>,
) -> Result<()> {
    let bundle = context_assemble_bundle(tier, policy_json, agent_id, None).await?;
    println!(
        "🔍 Assembling context bundle for tier: {:?} ({} sources)",
        bundle.tier,
        bundle.items.len()
    );
    println!("\nContext Bundle ({} items):", bundle.items.len());
    for (i, item) in bundle.items.iter().enumerate() {
        println!(
            "  - [{:?}] item {} (len: {})",
            bundle.tier,
            i,
            serde_json::to_string(item).map(|s| s.len()).unwrap_or(0)
        );
    }
    Ok(())
}
