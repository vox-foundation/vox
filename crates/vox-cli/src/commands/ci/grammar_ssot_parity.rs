use anyhow::{anyhow, Result};
use vox_grammar_export::ssot_markdown;

pub async fn run() -> Result<()> {
    let repo_root = super::repo_root();
    let ssot_path = repo_root.join("tree-sitter-vox").join("GRAMMAR_SSOT.md");

    if !ssot_path.exists() {
        return Err(anyhow!("GRAMMAR_SSOT.md not found at {}", ssot_path.display()));
    }

    let current_ssot = std::fs::read_to_string(&ssot_path)?;
    let expected_ssot = ssot_markdown::emit_ssot_markdown();

    if current_ssot.trim() != expected_ssot.trim() {
        eprintln!("Error: GRAMMAR_SSOT.md is out of sync with language_surface.rs.");
        eprintln!("Run `vox grammar --format ssot-markdown --output tree-sitter-vox/GRAMMAR_SSOT.md` to update.");
        return Err(anyhow::anyhow!("Grammar SSOT parity check failed"));
    }

    println!("GRAMMAR_SSOT.md is in sync with language_surface.rs.");
    Ok(())
}
