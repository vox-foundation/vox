//! `vox bundle apply <id>` — install every plugin in a bundle.

use crate::commands::plugin::{install, list::installed_version, list::plugins_root};
use anyhow::{Context, Result};

/// Canonical default profile for bundle `id`, or `None` when the bundle declares none.
pub fn default_profile_for_bundle(id: &str) -> Result<Option<String>> {
    let ids = vox_plugin_catalog::bundle_profiles(id)
        .with_context(|| format!("resolving bundle '{}'", id))?;
    if ids.is_empty() {
        return Ok(None);
    }
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    Ok(Some(
        vox_mcp_registry::ActiveProfile::resolve(&ids)?.id_string(),
    ))
}

pub async fn run(id: &str, yes: bool) -> Result<()> {
    let plugins = vox_plugin_catalog::bundle_resolved(id)
        .with_context(|| format!("resolving bundle '{}'", id))?;

    println!("Bundle '{}' — {} plugin(s) to apply:", id, plugins.len());
    for p in &plugins {
        println!("  - {}", p.id);
    }
    println!();

    let root = plugins_root();
    let mut installed_count = 0usize;
    let mut skipped_count = 0usize;
    let mut failed: Vec<String> = Vec::new();

    for p in &plugins {
        if installed_version(&root, &p.id).is_some() {
            println!("  skip  {} (already installed)", p.id);
            skipped_count += 1;
            continue;
        }

        println!("  installing {} …", p.id);
        match install::run(Some(&p.id), None, None, yes, false, false).await {
            Ok(()) => {
                installed_count += 1;
            }
            Err(e) => {
                eprintln!("  ✗ failed to install {}: {}", p.id, e);
                failed.push(p.id.clone());
            }
        }
    }

    println!();
    println!(
        "Summary: {} installed, {} skipped, {} failed.",
        installed_count,
        skipped_count,
        failed.len()
    );

    if !failed.is_empty() {
        anyhow::bail!(
            "bundle apply partially failed — {} plugin(s) could not be installed: {}",
            failed.len(),
            failed.join(", ")
        );
    }

    if let Some(profile) = default_profile_for_bundle(id)? {
        vox_config::toml_config::set_user_config_value("VOX_PROFILE", &profile)
            .map_err(anyhow::Error::msg)?;
        println!(
            "Default capability profile set to `{profile}` (VOX_PROFILE in ~/.vox/config.toml)."
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::default_profile_for_bundle;

    #[test]
    fn server_bundle_default_profile_is_canonical() {
        assert_eq!(
            default_profile_for_bundle("vox-server")
                .expect("server")
                .as_deref(),
            Some("knowledge,mesh,orchestrator")
        );
    }

    #[test]
    fn bundle_without_profiles_writes_nothing() {
        assert_eq!(default_profile_for_bundle("vox-base").expect("base"), None);
    }
}
