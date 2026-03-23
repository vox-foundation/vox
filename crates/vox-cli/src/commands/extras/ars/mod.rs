//! `vox skill` — manage Vox skills from the CLI.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use vox_ars::context::{ArsContextBundle, ContextPolicy, RetrievalTier, assemble_bundle};
use vox_ars::hooks::HookRegistry;
use vox_ars::runtime::ArsRuntime;

pub async fn list() -> Result<()> {
    let registry = make_registry().await;
    let skills = registry.list(None);

    if skills.is_empty() {
        println!("No skills installed.");
        println!("  Install from file: vox skill install <path/to/skill.skill.md>");
        return Ok(());
    }

    println!("Installed skills ({}):\n", skills.len());
    for skill in &skills {
        println!(
            "  {:30} {:10} [{:?}]  {}",
            skill.id, skill.version, skill.category, skill.description
        );
        if !skill.tools.is_empty() {
            println!("    tools: {}", skill.tools.join(", "));
        }
    }
    Ok(())
}

pub async fn install(path: &PathBuf) -> Result<()> {
    let registry = make_registry().await;

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read skill file: {}", path.display()))?;

    let bundle = vox_ars::parser::parse_skill_md(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse skill file: {e}"))?;

    let result = registry.install(&bundle).await?;
    if result.already_installed {
        println!(
            "✓ Skill '{}' (v{}) already installed",
            result.id, result.version
        );
    } else {
        println!(
            "✓ Installed skill '{}' v{} (hash: {})",
            result.id,
            result.version,
            &result.hash[..8.min(result.hash.len())]
        );
    }
    Ok(())
}

pub async fn uninstall(id: &str) -> Result<()> {
    let registry = make_registry().await;
    let result = registry.uninstall(id).await?;
    if result.was_installed {
        println!("✓ Uninstalled '{}'", id);
    } else {
        println!("  Skill '{}' was not installed", id);
    }
    Ok(())
}

pub async fn search(query: &str) -> Result<()> {
    let registry = make_registry().await;
    let hits = registry.search(query);
    if hits.is_empty() {
        println!("No skills matching '{}'", query);
    } else {
        println!("Skills matching '{}' ({}):\n", query, hits.len());
        for skill in &hits {
            println!(
                "  {:30} {}  ({})",
                skill.id, skill.version, skill.description
            );
        }
    }
    Ok(())
}

pub async fn info(id: &str) -> Result<()> {
    let registry = make_registry().await;
    match registry.get(id) {
        Some(skill) => {
            println!("Skill: {}", skill.id);
            println!("  Name:        {}", skill.name);
            println!("  Version:     {}", skill.version);
            println!("  Author:      {}", skill.author);
            println!("  Description: {}", skill.description);
            println!("  Category:    {:?}", skill.category);
            println!("  Tags:        {}", skill.tags.join(", "));
            println!("  Tools:       {}", skill.tools.join(", "));
        }
        None => println!("Skill '{}' is not installed", id),
    }
    Ok(())
}

pub async fn create(name: &str) -> Result<()> {
    let id = name.to_lowercase().replace(' ', "-");
    let filename = format!("{}.skill.md", id);
    let path = PathBuf::from(&filename);

    if path.exists() {
        anyhow::bail!("{} already exists", filename);
    }

    let template = format!(
        "---\nid = \"my-org.{id}\"\nname = \"{name}\"\nversion = \"0.1.0\"\nauthor = \"your-name\"\ndescription = \"A short description of what this skill does\"\ncategory = \"utilities\"\ntools = [\"example_tool\"]\ntags = [\"example\", \"custom\"]\npermissions = [\"read_files\"]\n---\n\n# {name}\n\nDescribe what this skill does and when to use it.\n\n## Tools\n\n### `example_tool`\n\nA brief description of what this tool does.\n",
        id = id,
        name = name,
    );

    std::fs::write(&path, &template)?;
    println!("✓ Created {}", filename);
    println!("  Edit the frontmatter and markdown body, then:");
    println!("  vox skill install {}", filename);
    Ok(())
}

pub async fn eval_task(body: &str, input_json: Option<&str>) -> Result<()> {
    let input: serde_json::Value = if let Some(j) = input_json {
        serde_json::from_str(j).context("Invalid input JSON")?
    } else {
        serde_json::json!({})
    };

    println!("🚀 Evaluating ephemeral task in sandbox...");
    // Limits will use defaults for now from CLI.
    let limits = vox_ars::manifest::ResourceLimits::default();

    let result = vox_ars::executor::execute_vox_task(body, &input, &limits, None).await?;

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
    let mut manifest = vox_ars::SkillManifest::new(
        skill_id.clone(),
        name.to_string(),
        "0.1.0".to_string(),
        session_id.to_string(),
        description.clone(),
        vox_ars::SkillCategory::Custom("promoted".into()),
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

    // Try to find skill in registry or hydrate
    let skill_manifest = registry
        .get(id)
        .context(format!("Skill '{}' not found in registry", id))?;

    // For now, we need the full ArsSkill domain object.
    // We can map from SkillManifest.
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

/// Memory types to query when no agent_id is set (session turns, messages, tool calls).
const CONTEXT_MEMORY_TYPES: &[&str] = &["session_turn", "message", "tool_call"];

/// Assembles a context bundle from tier, policy, and optional Codex (or default connection).
/// Returns the bundle for inspection; when `codex_override` is `None`, uses `Codex::connect_default()`.
#[cfg_attr(not(test), allow(dead_code))]
pub async fn context_assemble_bundle(
    tier: &str,
    policy_json: Option<&str>,
    agent_id: Option<&str>,
    codex_override: Option<&vox_db::Codex>,
) -> Result<ArsContextBundle, anyhow::Error> {
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
            if let Ok(entries) = db.recall_memory(aid, None, limit).await {
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
                    .recall_memory("", Some(memory_type), per_type)
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
            if let Ok(entries) = db.recall_memory(aid, None, limit).await {
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
                    .recall_memory("", Some(memory_type), per_type)
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

/// Assembles a context bundle and prints it to stdout. Uses default Codex connection.
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
async fn make_registry() -> Arc<vox_ars::SkillRegistry> {
    let registry = vox_skills::new_registry_arc();
    if let Ok(db) = vox_db::Codex::connect_default().await {
        let db_arc = Arc::new(db);
        registry.set_db(db_arc.clone());
        let _ = registry.hydrate_from_db().await;
    }
    // Auto-load all 5 built-in skills (embedded via include_str! at compile time)
    let _ = vox_ars::install_builtins(registry.as_ref()).await;
    registry
}

/// 9.4 — Skill auto-discovery: scan the crate graph for `.skill.md` files
/// and suggest installable skills not yet in the registry.
pub async fn discover() -> Result<()> {
    use owo_colors::OwoColorize;
    use std::collections::HashSet;

    let registry = make_registry().await;
    let installed: HashSet<String> = registry.list(None).into_iter().map(|s| s.id).collect();

    // Walk workspace for .skill.md files (up to 6 levels deep)
    let workspace_root = std::env::current_dir().unwrap_or_default();
    let mut found: Vec<(std::path::PathBuf, String)> = Vec::new();

    walk_for_skills(&workspace_root, 0, 6, &mut found);

    if found.is_empty() {
        println!("{}", "No .skill.md files found in the workspace.".dimmed());
        println!("  Create one with: {}", "vox skill create <name>".cyan());
        return Ok(());
    }

    let new_count = found
        .iter()
        .filter(|(_, id)| !installed.contains(id))
        .count();
    println!(
        "\n{} Found {} skill file(s) ({} not yet installed)\n",
        "🔍".bold(),
        found.len(),
        new_count
    );

    for (path, id) in &found {
        let is_installed = installed.contains(id);
        let rel = path.strip_prefix(&workspace_root).unwrap_or(path);
        if is_installed {
            println!(
                "  {} {:<32} [{}]",
                "✅".green(),
                id.dimmed(),
                rel.display().to_string().dimmed()
            );
        } else {
            println!(
                "  {} {:<32}  {} {}",
                "📦".yellow(),
                id.yellow(),
                rel.display().to_string().dimmed(),
                "← not installed".bright_yellow()
            );
            println!(
                "     {} vox skill install {}",
                "→".dimmed(),
                rel.display().to_string().cyan()
            );
        }
    }

    if new_count > 0 {
        println!(
            "\nInstall all: {}",
            "for f in $(find . -name '*.skill.md'); do vox skill install $f; done".cyan()
        );
    }

    Ok(())
}

fn walk_for_skills(
    dir: &std::path::Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<(std::path::PathBuf, String)>,
) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Skip hidden dirs, target/, node_modules/
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && (name.starts_with('.') || name == "target" || name == "node_modules")
        {
            continue;
        }
        if path.is_dir() {
            walk_for_skills(&path, depth + 1, max_depth, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if fname.ends_with(".skill.md") {
                // Quick parse: extract `id = "..."` from frontmatter
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let id = extract_skill_id(&content)
                        .unwrap_or_else(|| fname.trim_end_matches(".skill.md").to_string());
                    out.push((path, id));
                }
            }
        }
    }
}

fn extract_skill_id(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("id") {
            // Handle: id = "foo.bar" or id = 'foo.bar'
            if let Some(rest) = trimmed.strip_prefix("id").map(|s| s.trim())
                && let Some(rest) = rest.strip_prefix('=')
            {
                let val = rest.trim().trim_matches('"').trim_matches('\'').to_string();
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn context_assemble_invalid_tier_errors() {
        let r = context_assemble_bundle("invalid_tier", None, None, None).await;
        assert!(r.is_err(), "invalid tier should error");
    }

    #[tokio::test]
    async fn context_assemble_invalid_policy_json_errors() {
        let r = context_assemble_bundle("standard", Some("not valid json"), None, None).await;
        assert!(r.is_err(), "invalid policy JSON should error");
    }

    /// Use in-memory Codex to avoid connect_default/shutdown_for_drop. Multi-thread: Arca uses blocking.
    #[cfg(feature = "ars")]
    #[tokio::test(flavor = "multi_thread")]
    async fn context_assemble_valid_tier_succeeds() {
        use vox_db::{Codex, DbConfig};

        let db = Codex::connect(DbConfig::Memory)
            .await
            .expect("in-memory Codex");
        let r = context_assemble_bundle("standard", None, None, Some(&db)).await;
        assert!(r.is_ok(), "valid tier should succeed");
        assert!(r.unwrap().items.is_empty(), "no memories => empty bundle");
    }

    #[cfg(feature = "codex")]
    #[tokio::test(flavor = "multi_thread")]
    async fn context_assemble_with_agent_id_succeeds() {
        use vox_db::{Codex, CodexConfig};

        let db = Codex::connect(CodexConfig::Memory)
            .await
            .expect("in-memory Codex");
        let r = context_assemble_bundle("shallow", None, Some("test-agent"), Some(&db)).await;
        assert!(r.is_ok(), "valid tier with agent_id should succeed");
    }

    /// Integration test: in-memory Codex with one memory yields non-empty bundle.
    /// Multi-thread runtime: Arca::store_memory uses blocking.
    #[cfg(feature = "ars")]
    #[tokio::test(flavor = "multi_thread")]
    async fn context_assemble_with_memory_data_returns_non_empty_bundle() {
        use vox_db::MemoryParams;
        use vox_db::{Codex, DbConfig};

        let db = Codex::connect(DbConfig::Memory)
            .await
            .expect("in-memory Codex for test");
        let params = MemoryParams {
            agent_id: "test-agent",
            session_id: "test-session",
            memory_type: "session_turn",
            content: "User said hello",
            metadata: None,
            importance: 0.5,
            vcs_snapshot_id: None,
        };
        db.store_memory(params).await.expect("store one memory");
        let bundle = context_assemble_bundle("standard", None, None, Some(&db))
            .await
            .expect("assemble with test Codex");
        assert!(
            !bundle.items.is_empty(),
            "bundle should contain the stored memory, got {} items",
            bundle.items.len()
        );
    }
}
