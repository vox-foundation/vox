//! `vox plugin info <id>` — show manifest + install path + ABI status.

use super::list::{installed_version, plugins_root};
use anyhow::{Context, Result};

pub fn run(id: &str) -> Result<()> {
    let catalog = vox_plugin_catalog::all_plugins();
    let entry = catalog
        .iter()
        .find(|p| p.id == id)
        .with_context(|| format!("Plugin '{}' not found in catalog", id))?;

    println!("Plugin: {}", entry.id);
    println!("  Description : {}", entry.description);
    println!("  Payload kind: {:?}", entry.payload_kind);
    if let Some(ref ep) = entry.extension_points {
        println!("  Extension points: {}", ep.join(", "));
    }
    if let Some(ref tools) = entry.exposes_tools {
        println!("  Exposes tools   : {}", tools.join(", "));
    }
    if let Some(ref tag) = entry.requires_tag {
        println!("  Requires tag    : {}", tag);
    }
    if !entry.bundled_in.is_empty() {
        println!("  Bundled in      : {}", entry.bundled_in.join(", "));
    }
    println!("  Default source  : {}", entry.default_source);

    let root = plugins_root();
    match installed_version(&root, id) {
        Some(version) => {
            let install_dir = root.join(id).join(&version);
            println!("\nStatus: installed ({})", version);
            println!("  Install dir: {}", install_dir.display());

            // Try to read Plugin.toml from install dir.
            let plugin_toml = install_dir.join("Plugin.toml");
            if plugin_toml.exists() {
                println!("  Plugin.toml  : {}", plugin_toml.display());
            }

            // For code plugins, check dylib presence.
            let triple = vox_plugin_host::current_target_triple_key();
            let dylib_name = format!(
                "{}.{}",
                id,
                if cfg!(windows) {
                    "dll"
                } else if cfg!(target_os = "macos") {
                    "dylib"
                } else {
                    "so"
                }
            );
            let dylib = install_dir.join(&dylib_name);
            if dylib.exists() {
                println!("  Native lib   : {} ({})", dylib.display(), triple);
                println!("  ABI check    : (run `vox plugin doctor` to verify ABI)");
            }
        }
        None => {
            println!("\nStatus: not installed");
            println!("  Install with: vox plugin install {}", id);
        }
    }

    Ok(())
}
