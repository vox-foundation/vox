use anyhow::Result;
use std::fs;

pub async fn run(name: Option<&str>) -> Result<()> {
    run_with_options(name, false).await
}

pub async fn run_with_options(name: Option<&str>, read_only: bool) -> Result<()> {
    let project_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .as_deref()
            .unwrap_or("my-project")
            .to_string()
            .leak()
    });

    let cwd = std::env::current_dir()?;
    let repo_dir = cwd;

    let harness = vox_repository::onboard_repository_harness(&repo_dir, read_only)?;
    if read_only {
        println!(
            "✓ Initialized read-only external cache for '{}' (zero writes inside target repo)",
            project_name
        );
        println!("  repository_id:      {}", harness.repository_id);
        println!("  catalog_path:       {}", harness.catalog_path);
        println!("  global_library_dir: {}", harness.global_library_dir);
        for dir in &harness.cache_dirs {
            println!("  cache_dir:          {dir}");
        }
        return Ok(());
    }

    for created in &harness.created_paths {
        println!("Created {created}");
    }

    // Derive other ignore files from `.voxignore` SSOT.
    vox_cli_ci::sync_ignore_files::run(&repo_dir, false)?;

    // Preserve .vox/agents/, .vox/handoffs/, and .github/copilot-instructions.md harness structure.
    let vox_dir = repo_dir.join(".vox");
    let agents_dir = vox_dir.join("agents");
    if !agents_dir.exists() {
        fs::create_dir_all(&agents_dir)?;
        println!("Created .vox/agents/ directory");
    }

    let github_dir = repo_dir.join(".github");
    if !github_dir.exists() {
        fs::create_dir_all(&github_dir)?;
    }
    let copilot_instructions = github_dir.join("copilot-instructions.md");
    if !copilot_instructions.exists() {
        let copilot_content =
            "# Copilot Instructions\n\nSee AGENTS.md for primary cross-tool agent instructions.\n";
        fs::write(&copilot_instructions, copilot_content)?;
        println!("Created .github/copilot-instructions.md");
    } else {
        println!(".github/copilot-instructions.md already exists, skipping");
    }

    let handoffs_dir = vox_dir.join("handoffs");
    if !handoffs_dir.exists() {
        fs::create_dir_all(&handoffs_dir)?;
        println!("Created .vox/handoffs/ directory");
    }

    println!(
        "✓ Initialized Vox repository harness for '{}' (id: {})",
        project_name, harness.repository_id
    );

    Ok(())
}
