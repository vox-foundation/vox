//! `vox plugin remove <id>` — delete the install dir for a plugin.

use super::list::plugins_root;
use anyhow::{Context, Result};

pub fn run(id: &str) -> Result<()> {
    let root = plugins_root();
    let id_dir = root.join(id);

    if !id_dir.exists() {
        eprintln!(
            "Plugin '{}' does not appear to be installed (no directory at {}).",
            id,
            id_dir.display()
        );
        return Ok(());
    }

    std::fs::remove_dir_all(&id_dir)
        .with_context(|| format!("removing {}", id_dir.display()))?;

    println!("✓ Removed plugin '{}' ({})", id, id_dir.display());
    Ok(())
}
