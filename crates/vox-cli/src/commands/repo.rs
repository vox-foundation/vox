//! `vox repo` — repository status, layout detection, and agent scope info.
//!
//! Uses `vox-repository` to identify the logical repo root and stack markers.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// High-level repository info struct for JSON output.
#[derive(serde::Serialize)]
struct RepoDocs {
    root: PathBuf,
    repo_id: String,
    has_git: bool,
    stack_markers: Vec<String>,
    workspace_members: Vec<PathBuf>,
}

/// Run the `vox repo` command.
pub async fn run(json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo = vox_repository::Repository::discover(Some(&cwd))
        .with_context(|| "Failed to discover repository root from current directory")?;

    if json {
        let docs = RepoDocs {
            root: repo.root().to_path_buf(),
            repo_id: repo.repository_id().to_string(),
            has_git: repo.has_git(),
            stack_markers: repo.stack_markers().iter().map(|s| s.to_string()).collect(),
            workspace_members: repo.cargo_workspace_member_dirs().into_iter().cloned().collect(),
        };
        println!("{}", serde_json::to_string_pretty(&docs)?);
        return Ok(());
    }

    println!();
    println!("  \x1b[1;36m╔══════════════════════════════════════════╗\x1b[0m");
    println!("  \x1b[1;36m║           Vox Repository Status          ║\x1b[0m");
    println!("  \x1b[1;36m╚══════════════════════════════════════════╝\x1b[0m");
    println!();

    println!("    \x1b[1mRoot:\x1b[0m       {}", repo.root().display());
    println!("    \x1b[1mID:\x1b[0m         {}", repo.repository_id());
    println!("    \x1b[1mGit:\x1b[0m        {}", if repo.has_git() { "\x1b[32m✓\x1b[0m detected" } else { "\x1b[31m✗\x1b[0m not found" });

    let markers = repo.stack_markers();
    if !markers.is_empty() {
        print!("    \x1b[1mMarkers:\x1b[0m    ");
        for (i, m) in markers.iter().enumerate() {
            if i > 0 { print!(", "); }
            print!("\x1b[36m{}\x1b[0m", m);
        }
        println!();
    }

    let members = repo.cargo_workspace_member_dirs();
    if !members.is_empty() {
        println!("\n    \x1b[1mWorkspace Members ({}):\x1b[0m", members.len());
        for m in members.iter().take(10) {
            let rel = m.strip_prefix(repo.root()).unwrap_or(m);
            println!("      • \x1b[2m{}\x1b[0m", rel.display());
        }
        if members.len() > 10 {
            println!("      ... and {} more", members.len() - 10);
        }
    }

    println!();
    Ok(())
}
