//! Implementation of `vox populi system-prompt-template`.
//!
//! Generates a comprehensive system prompt including grammar, constructs,
//! and project context for use in external IDEs.

use anyhow::Result;
use std::path::PathBuf;

/// Run the system-prompt-template subcommand.
pub async fn run(output: Option<PathBuf>, format: &str) -> Result<()> {
    let mut prompt = vox_corpus::training::generate_system_prompt();

    // Inject repository context from CWD (same discovery contract as MCP/orchestrator).
    append_repository_context(&mut prompt);

    // Wrap in IDE-specific formats
    let final_content = match format.to_lowercase().as_str() {
        "cursor" => {
            format!(
                "# Cursor Rules for Vox\n\
                 \n\
                 Apply these rules when working in this repository.\n\
                 \n\
                 {}\n\
                 \n\
                 ## IDE Specifics (Cursor)\n\
                 - Always check both .rs and .vox counterparts when modifying web features.\n\
                 - Use `cargo check -p <crate>` to verify changes.\n",
                prompt
            )
        }
        "claude" | "claude-code" => {
            format!(
                "# Claude Context for Vox (CLAUDE.md)\n\
                 \n\
                 {}\n\
                 \n\
                 ## IDE Specifics (Claude)\n\
                 - You are an agentic AI assistant.\n\
                 - Prefer reading VOX.md if it exists.\n\
                 - All new constructs must follow the v0.2 syntactic standard.\n",
                prompt
            )
        }
        "copilot" => {
            format!(
                "# GitHub Copilot Instructions for Vox\n\
                 \n\
                 {}\n",
                prompt
            )
        }
        "wind-pro" | "windsurf" => {
            format!(
                "# Windsurf Rules for Vox (.windsurfrules)\n\
                 \n\
                 {}\n",
                prompt
            )
        }
        _ => prompt, // "text" or unknown
    };

    if let Some(out_path) = output {
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&out_path, final_content)?;
        println!("  System prompt template written to {}", out_path.display());
    } else {
        println!("{}", final_content);
    }

    Ok(())
}

const REPO_CONTEXT_FILE_CAP: usize = 8192;

fn append_repository_context(prompt: &mut String) {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(_) => return,
    };
    let repo = vox_repository::discover_repository_or_fallback(&cwd);
    prompt.push_str("\n\n## Project Context\n");
    prompt.push_str(&format!("- Repository root: {}\n", repo.root.display()));
    if let Some(ref origin) = repo.origin_url {
        prompt.push_str(&format!("- Git origin: {origin}\n"));
    }
    prompt.push_str(&format!("- Repository ID: {}\n", repo.repository_id));
    if let Some(ref vt) = repo.vox_toml {
        prompt.push_str(&format!("- Vox.toml: {}\n", vt.display()));
    }
    prompt.push_str(&format!(
        "- Markers: vox_project={} cargo_ws={} node_ws={} python={} go={} git={}\n",
        repo.capabilities.vox_project,
        repo.capabilities.cargo_workspace,
        repo.capabilities.node_workspace,
        repo.capabilities.python_project,
        repo.capabilities.go_module,
        repo.capabilities.git
    ));

    let agents = repo.root.join("AGENTS.md");
    if agents.is_file() {
        if let Ok(text) = std::fs::read_to_string(&agents) {
            let take = text.len().min(REPO_CONTEXT_FILE_CAP);
            prompt.push_str("\n### AGENTS.md (excerpt)\n\n");
            prompt.push_str(&text[..take]);
            if text.len() > take {
                prompt.push_str("\n\n… (truncated)");
            }
        }
    }
}
