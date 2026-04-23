use anyhow::Result;

/// `vox init` — scaffold a new Vox project with a `Vox.toml` manifest.
pub async fn run(name: Option<&str>, kind: Option<&str>, template: Option<&str>) -> Result<()> {
    let project_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .as_deref()
            .unwrap_or("my-project")
            .to_string()
            .leak()
    });
    let package_kind = kind.unwrap_or("application");

    let cwd = std::env::current_dir()?;
    let scaffold_path = cwd.join(project_name);
    let summary = vox_project_scaffold::scaffold_vox_project_at(
        &scaffold_path,
        project_name,
        package_kind,
        template.as_deref(),
    )?;

    if let Some("mobile-pwa") = template.as_deref() {
        crate::templates::mobile_pwa::scaffold(project_name, &scaffold_path)?;
    }

    if summary.package_kind == "skill" {
        let skill_file_name = vox_repository::skill_markdown_filename(project_name);
        println!("✓ Initialized Vox skill `{}`", project_name);
        println!("  Created {}", skill_file_name);
        println!();
        println!("  Next steps:");
        println!("    1. Edit {}", skill_file_name);
        println!("    2. Install with: vox skill install {}", skill_file_name);
        return Ok(());
    }

    let template_note = if let Some(ref t) = template {
        format!(" (template: {t})")
    } else {
        String::new()
    };

    println!(
        "✓ Initialized Vox {} `{}`{}",
        summary.package_kind, summary.project_name, template_note
    );
    println!("  Created Vox.toml");
    println!("  Created src/main.vox");
    println!("  Created .vox_modules/");
    println!();
    match (package_kind, template.as_deref()) {
        (_, Some("chatbot")) | ("chatbot", _) => {
            println!("  Next steps:");
            println!("    1. Add OPENROUTER_API_KEY to .env");
            println!("    2. vox build src/main.vox -o dist");
            println!("    3. POST to http://localhost:3001/api/chat");
        }
        (_, Some("dashboard")) | (_, Some("web")) => {
            println!("  Next steps:");
            println!("    1. vox build src/main.vox -o dist");
            println!("    2. Open http://localhost:3000");
        }
        (_, Some("api")) => {
            println!("  Next steps:");
            println!("    1. vox build src/main.vox -o dist");
            println!("    2. curl http://localhost:3001/health");
        }
        ("application", _) => {
            println!("  Next steps:");
            println!("    1. vox build src/main.vox -o dist");
            println!("    2. vox run src/main.vox");
            println!("    3. Open http://localhost:3000");
        }
        ("workflow", _) => {
            println!("  Next steps:");
            println!("    1. vox build src/main.vox -o dist");
            println!("    2. Edit activities and workflow steps");
        }
        (_, Some("mobile-pwa")) => {
            println!("  Next steps:");
            println!("    1. pnpm install (or npm install)");
            println!("    2. vox build src/main.vox -o dist");
            println!("    3. npx cap add ios (or android)");
            println!("    4. npx cap sync");
        }
        _ => {
            println!("  Get started with: vox build src/main.vox");
        }
    }

    Ok(())
}
