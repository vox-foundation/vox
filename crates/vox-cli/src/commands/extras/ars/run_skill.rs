use anyhow::{Context, Result};
use std::sync::Arc;
use vox_ars::hooks::HookRegistry;
use vox_ars::runtime::ArsRuntime;

use super::registry::make_registry;

pub async fn run(id: &str, input_json: Option<&str>, workflow: bool) -> Result<()> {
    let registry = make_registry().await;
    let db = vox_db::Codex::connect_default()
        .await
        .context("Database connection required for ARS run")?;
    let db = Arc::new(db);
    let hooks = Arc::new(HookRegistry::new());
    let runtime = ArsRuntime::new(db.clone(), hooks);

    let input: serde_json::Value = if let Some(j) = input_json {
        serde_json::from_str(j).context("Invalid input JSON")?
    } else {
        serde_json::json!({})
    };

    if workflow {
        println!(
            "Running ARS workflow remains a Wave 2 target (needs workflow engine integration)."
        );
        return Ok(());
    }

    let skill_manifest = registry
        .get(id)
        .context(format!("Skill '{}' not found in registry", id))?;

    let skill = vox_ars::domain::ArsSkill {
        id: skill_manifest.id.clone(),
        namespace: "local".into(),
        name: skill_manifest.name.clone(),
        version: skill_manifest.version.clone(),
        content_hash: skill_manifest.hash.clone().unwrap_or_default(),
        description: Some(skill_manifest.description.clone()),
        author: Some(skill_manifest.author.clone()),
        metadata: serde_json::json!({}),
        kind: vox_ars::manifest::SkillKind::Document,
        body: None,
        resource_limits: vox_ars::manifest::ResourceLimits::default(),
    };

    println!("🚀 Executing skill: {} v{}", skill.id, skill.version);
    let run_id = runtime.create_run(None, Some(&skill.id), input.clone(), None)?;

    let result = runtime.execute_skill(&run_id, &skill, input).await?;

    println!("\nResult ({}):", result["status"]);
    println!("{}", serde_json::to_string_pretty(&result).unwrap());

    if result["status"] == "success" {
        println!("\n✓ Execution run {} succeeded", run_id);
    } else {
        println!("\n✗ Execution run {} failed", run_id);
    }

    db.shutdown_for_drop();
    Ok(())
}
