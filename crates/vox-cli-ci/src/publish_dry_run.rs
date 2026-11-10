//! `vox ci publish-dry-run` — verify crates.io publishability of crates in `crates/_public.toml`.

use anyhow::{Context, Result, bail};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::process::Command;

use crate::helpers::cargo_bin;

#[derive(Debug, Clone)]
struct PublicCrate {
    name: String,
    depends_on: Vec<String>,
}

/// Run publish-dry-run checks against all crates declared in `crates/_public.toml`.
#[inline(never)]
pub fn run(root: &Path) -> Result<()> {
    let public_path = root.join("crates/_public.toml");
    if !public_path.exists() {
        bail!("Missing public crate manifest: {}", public_path.display());
    }

    let manifest_str = std::fs::read_to_string(&public_path)
        .with_context(|| format!("Failed to read {}", public_path.display()))?;

    let toml_val: toml::Value = toml::from_str(&manifest_str)
        .with_context(|| format!("Failed to parse TOML from {}", public_path.display()))?;

    let raw_crates = toml_val
        .get("crates")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("`crates` array missing in crates/_public.toml"))?;

    let mut crates = Vec::new();
    for entry in raw_crates {
        let name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Entry in `crates` missing `name`"))?
            .to_string();

        let publish = entry
            .get("publish")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let depends_on = entry
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if publish {
            crates.push(PublicCrate { name, depends_on });
        }
    }

    let ordered_crates = topological_sort_crates(&crates)?;
    let cargo = cargo_bin();

    println!(
        "Checking publish readiness for {} crate(s)...",
        ordered_crates.len()
    );

    for krate in &ordered_crates {
        println!("  Verifying package: {}", krate.name);

        // 1. Check workspace-hack dependency in Cargo.toml
        let crate_manifest_path = root.join("crates").join(&krate.name).join("Cargo.toml");
        if !crate_manifest_path.exists() {
            bail!(
                "Cargo.toml not found for crate '{}' at {}",
                krate.name,
                crate_manifest_path.display()
            );
        }

        let manifest_content = std::fs::read_to_string(&crate_manifest_path)
            .with_context(|| format!("Failed to read {}", crate_manifest_path.display()))?;

        verify_no_workspace_hack(&krate.name, &manifest_content)?;

        // 2. Run cargo package --list --allow-dirty -p <crate>
        let output = Command::new(&cargo)
            .args(["package", "--list", "--allow-dirty", "-p", &krate.name])
            .current_dir(root)
            .output()
            .with_context(|| {
                format!(
                    "Failed to execute cargo package --list for '{}'",
                    krate.name
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "cargo package --list failed for '{}':\n{}",
                krate.name,
                stderr
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let files: Vec<&str> = stdout
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        // 3. Verify presence of Cargo.toml and README.md
        let has_cargo_toml = files
            .iter()
            .any(|f| *f == "Cargo.toml" || *f == "Cargo.toml.orig");
        if !has_cargo_toml {
            bail!(
                "Package '{}' missing Cargo.toml in packaged files",
                krate.name
            );
        }

        let has_readme = files
            .iter()
            .any(|f| *f == "README.md" || f.ends_with("/README.md"));
        if !has_readme {
            bail!(
                "Package '{}' missing README.md in packaged files",
                krate.name
            );
        }

        // 4. Verify zero crate-escaping path errors
        for file in &files {
            if file.starts_with('/')
                || file.starts_with('\\')
                || file.starts_with("..")
                || file.contains("/../")
                || file.contains("\\..\\")
            {
                bail!(
                    "Package '{}' has crate-escaping path in package list: {}",
                    krate.name,
                    file
                );
            }
        }

        println!(
            "  ✓ {} (Cargo.toml + README.md present, 0 path escapes, 0 workspace-hack deps)",
            krate.name
        );
    }

    println!(
        "All {} publishable crates passed dry-run verification.",
        ordered_crates.len()
    );
    Ok(())
}

#[inline(never)]
fn verify_no_workspace_hack(crate_name: &str, cargo_toml_str: &str) -> Result<()> {
    let toml_val: toml::Value = toml::from_str(cargo_toml_str)
        .with_context(|| format!("Failed to parse Cargo.toml for crate '{}'", crate_name))?;

    let dep_sections = ["dependencies", "dev-dependencies", "build-dependencies"];

    for section in dep_sections {
        if let Some(deps) = toml_val.get(section).and_then(|v| v.as_table()) {
            if deps.contains_key("workspace-hack") {
                bail!(
                    "Crate '{}' has forbidden `workspace-hack` dependency in [{}]",
                    crate_name,
                    section
                );
            }
        }
    }

    if let Some(targets) = toml_val.get("target").and_then(|v| v.as_table()) {
        for (target_cfg, target_val) in targets {
            for section in dep_sections {
                if let Some(deps) = target_val.get(section).and_then(|v| v.as_table()) {
                    if deps.contains_key("workspace-hack") {
                        bail!(
                            "Crate '{}' has forbidden `workspace-hack` dependency in [target.{}.{}]",
                            crate_name,
                            target_cfg,
                            section
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

#[inline(never)]
fn topological_sort_crates(crates: &[PublicCrate]) -> Result<Vec<PublicCrate>> {
    let crate_map: HashMap<&str, &PublicCrate> =
        crates.iter().map(|c| (c.name.as_str(), c)).collect();

    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for c in crates {
        in_degree.entry(&c.name).or_insert(0);
        for dep in &c.depends_on {
            if crate_map.contains_key(dep.as_str()) {
                *in_degree.entry(&c.name).or_insert(0) += 1;
                dependents.entry(dep.as_str()).or_default().push(&c.name);
            }
        }
    }

    // Preserve original manifest order for crates with same dependency level
    let mut queue: VecDeque<&str> = crates
        .iter()
        .map(|c| c.name.as_str())
        .filter(|name| in_degree.get(name).copied().unwrap_or(0) == 0)
        .collect();

    let mut sorted = Vec::new();
    while let Some(node) = queue.pop_front() {
        if let Some(&c) = crate_map.get(node) {
            sorted.push(c.clone());
        }
        if let Some(children) = dependents.get(node) {
            for &child in children {
                let entry = in_degree.get_mut(child).unwrap();
                *entry -= 1;
                if *entry == 0 {
                    queue.push_back(child);
                }
            }
        }
    }

    if sorted.len() != crates.len() {
        bail!("Cycle detected among publishable crates in crates/_public.toml");
    }

    Ok(sorted)
}
