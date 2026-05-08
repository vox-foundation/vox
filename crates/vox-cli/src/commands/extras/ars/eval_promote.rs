use anyhow::{Context, Result};

pub async fn eval_task(body: &str, input_json: Option<&str>) -> Result<()> {
    let input: serde_json::Value = if let Some(j) = input_json {
        serde_json::from_str(j).context("Invalid input JSON")?
    } else {
        serde_json::json!({})
    };

    println!("🚀 Evaluating ephemeral task in sandbox...");
    let limits = vox_openclaw_runtime::manifest::ResourceLimits::default();

    let result =
        vox_openclaw_runtime::executor::execute_vox_task(body, &input, &limits, None).await?;

    println!("\nResult:");
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    Ok(())
}

pub async fn promote_skill(session_id: &str, task_id: &str, name: &str) -> Result<()> {
    println!("Promoting ephemeral task {task_id} from session {session_id} to skill: {name}");

    let db = vox_db::Codex::connect_default()
        .await
        .context("Database connection required for ARS promote_skill")?;

    let skill_id = promoted_skill_id(task_id);
    let description = format!("Promoted ephemeral task {task_id} from session {session_id}");
    let mut manifest = vox_openclaw_runtime::SkillManifest::new(
        skill_id.clone(),
        name.to_string(),
        "0.1.0".to_string(),
        session_id.to_string(),
        description.clone(),
        vox_openclaw_runtime::SkillCategory::Custom("promoted".into()),
    );
    manifest.tags = vec!["promoted".into(), format!("task:{task_id}")];
    let manifest_json =
        serde_json::to_string(&manifest).context("serialize skill manifest for publish_skill")?;
    let body =
        format!("# {name}\n\n{description}\n\n- session: `{session_id}`\n- task: `{task_id}`\n",);
    let skill_md = format!(
        "---\nid = \"{}\"\nname = \"{}\"\nversion = \"0.1.0\"\nauthor = \"{}\"\ndescription = \"{}\"\ncategory = \"custom\"\ntags = [\"promoted\"]\n---\n\n{body}",
        toml_escape(&skill_id),
        toml_escape(name),
        toml_escape(session_id),
        toml_escape(&description),
    );

    db.publish_skill(&skill_id, "0.1.0", &manifest_json, &skill_md)
        .await
        .context("Failed to publish promoted skill to Codex skill_manifests")?;

    db.shutdown_for_drop();
    println!("✅ Successfully promoted task to durable skill (id: {skill_id}).");
    Ok(())
}

fn promoted_skill_id(task_id: &str) -> String {
    let slug: String = task_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("vox.promoted.{slug}")
}

fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
