//! `vox config` — user preference and global configuration management.
//!
//! Handles `~/.vox/preferences.json` and registry-specific settings.

use anyhow::{Context, Result};

/// `vox config` — manage global user preferences and configuration.
pub async fn run(
    registry: Option<&str>,
    name: std::option::Option<String>,
    set_value: Option<String>,
    reset: bool,
    json: bool,
) -> Result<()> {
    let registry = registry.unwrap_or("google");

    if reset {
        vox_db::preferences::reset_registry_preferences(registry)
            .await
            .context("Failed to reset preferences")?;
        println!("✓ Reset all preferences for: {}", registry);
        return Ok(());
    }

    if let Some(n) = name {
        if let Some(v) = set_value {
            // Set
            vox_db::preferences::set_registry_preference(registry, &n, &v)
                .await
                .context("Failed to set preference")?;
            println!("✓ Set \x1b[1;36m{}\x1b[0m = \x1b[32m'{}'\x1b[0m for: \x1b[1m{}\x1b[0m", n, v, registry);
        } else {
            // Get
            let val = vox_db::preferences::get_registry_preference(registry, &n)
                .await
                .context("Failed to get preference")?
                .unwrap_or_else(|| "none".to_string());

            if json {
                println!("{}", serde_json::json!({ "name": n, "value": val, "registry": registry }));
            } else {
                println!("  \x1b[1;36m{} \x1b[0m = \x1b[32m'{}'\x1b[0m (\x1b[2m{}\x1b[0m)", n, val, registry);
            }
        }
    } else {
        // List
        let prefs = vox_db::preferences::get_all_registry_preferences(registry)
            .await
            .context("Failed to list preferences")?;

        if json {
            println!("{}", serde_json::to_string_pretty(&prefs)?);
        } else {
            println!("\n  \x1b[1mPreferences for: \x1b[1;36m{}\x1b[0m", registry);
            if prefs.is_empty() {
                println!("    No preferences stored.");
            } else {
                for (k, v) in prefs {
                    println!("    \x1b[1m{:24}\x1b[0m \x1b[2m→\x1b[0m \x1b[32m'{}'\x1b[0m", k, v);
                }
            }
            println!();
        }
    }

    Ok(())
}
