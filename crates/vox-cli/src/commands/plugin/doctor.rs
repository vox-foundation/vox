//! `vox plugin doctor` — walk installed plugins, check ABI, report manifest issues.

use super::list::{installed_version, plugins_root};
use anyhow::Result;
use vox_plugin_host::VOX_PLUGIN_ABI_VERSION;

pub fn run() -> Result<()> {
    let root = plugins_root();
    let catalog = vox_plugin_catalog::all_plugins();
    let mut issues = 0usize;
    let mut checked = 0usize;

    println!("Plugin install root: {}", root.display());
    println!("Host ABI version: {}", VOX_PLUGIN_ABI_VERSION);
    println!();

    for entry in catalog {
        let Some(version) = installed_version(&root, &entry.id) else {
            continue;
        };
        checked += 1;
        let install_dir = root.join(&entry.id).join(&version);

        // Check Plugin.toml presence.
        let plugin_toml = install_dir.join("Plugin.toml");
        if !plugin_toml.exists() {
            eprintln!(
                "✗ {}: Plugin.toml missing in {}",
                entry.id,
                install_dir.display()
            );
            issues += 1;
            continue;
        }

        // Parse Plugin.toml to check ABI for code plugins.
        let raw = match std::fs::read_to_string(&plugin_toml) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("✗ {}: cannot read Plugin.toml: {}", entry.id, e);
                issues += 1;
                continue;
            }
        };

        // Lightweight parse: just look for abi-version field.
        let abi_ok = check_abi_from_toml(&entry.id, &raw, &mut issues);

        // Check declared native-libs if any (advisory only — we report what's declared).
        if let Some(libs) = check_native_libs_from_toml(&raw) {
            for lib in &libs {
                // We don't resolve lib paths yet — just report them.
                println!(
                    "  {} requires native lib: {} (presence not verified)",
                    entry.id, lib
                );
            }
        }

        if abi_ok {
            println!("✓ {} v{} — ok", entry.id, version);
        }
    }

    println!();
    if issues == 0 {
        println!(
            "✓ doctor: {} installed plugin(s) checked, no issues found.",
            checked
        );
    } else {
        eprintln!(
            "✗ doctor: {} installed plugin(s) checked, {} issue(s) found.",
            checked, issues
        );
        anyhow::bail!("plugin doctor found {} issue(s)", issues);
    }
    Ok(())
}

/// Attempt a lightweight ABI version check from raw Plugin.toml content.
/// Returns `true` if ok or if not a code/composite plugin (skill-only is always fine).
fn check_abi_from_toml(id: &str, raw: &str, issues: &mut usize) -> bool {
    // Parse as a generic TOML value to extract abi-version.
    let Ok(val) = toml::from_str::<toml::Value>(raw) else {
        return true; // can't parse details, already checked file exists
    };

    let abi_version = val
        .get("plugin")
        .and_then(|p| p.get("payload"))
        .and_then(|pl| pl.get("abi-version"))
        .and_then(|v| v.as_integer());

    if let Some(abi) = abi_version {
        let host = VOX_PLUGIN_ABI_VERSION as i64;
        if abi != host {
            eprintln!(
                "✗ {}: ABI mismatch — plugin declares abi-version={}, host expects {}",
                id, abi, host
            );
            *issues += 1;
            return false;
        }
    }
    // Skill-only plugins have no abi-version; that's fine.
    true
}

/// Extract `requires.native-libs` array from Plugin.toml if present.
fn check_native_libs_from_toml(raw: &str) -> Option<Vec<String>> {
    let val = toml::from_str::<toml::Value>(raw).ok()?;
    let libs = val
        .get("plugin")
        .and_then(|p| p.get("requires"))
        .and_then(|r| r.get("native-libs"))
        .and_then(|v| v.as_array())?;
    let out: Vec<String> = libs
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    if out.is_empty() { None } else { Some(out) }
}
