//! `contracts/gui/command-backends.v1.yaml` must classify every Axis command module exactly once.

use anyhow::{Context, Result, bail};
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const MANIFEST_PATH: &str = "contracts/gui/command-backends.v1.yaml";
const COMMANDS_DIR: &str = "crates/vox-gui/src/commands";
const SURFACE_REGISTRY_PATH: &str = "contracts/gui/surface-registry.v1.yaml";
const CLASSES: [&str; 4] = ["daemon", "db", "host", "shell"];

/// Every disagreement between the manifest and the source tree, sorted; empty = consistent.
pub fn problems(
    modules_on_disk: &BTreeSet<String>,
    manifest: &Value,
    view_keys: &BTreeSet<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    let rows = manifest
        .get("commands")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    if rows.is_empty() {
        out.push(format!("{MANIFEST_PATH}: `commands` is missing or empty"));
    }
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for row in &rows {
        let module = row
            .get("module")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if module.is_empty() {
            out.push(format!("{MANIFEST_PATH}: row without `module`"));
            continue;
        }
        *seen.entry(module.to_string()).or_default() += 1;
        let class = row.get("class").and_then(Value::as_str).unwrap_or_default();
        if !CLASSES.contains(&class) {
            out.push(format!(
                "{module}: class `{class}` is not one of daemon|db|host|shell"
            ));
        }
        if !modules_on_disk.contains(module) {
            out.push(format!(
                "{module}: listed in {MANIFEST_PATH} but {COMMANDS_DIR}/{module}.rs does not exist"
            ));
        }
        for surface in row
            .get("surfaces")
            .and_then(Value::as_sequence)
            .into_iter()
            .flatten()
        {
            let surface = surface.as_str().unwrap_or_default();
            if !view_keys.contains(surface) {
                out.push(format!(
                    "{module}: surface `{surface}` is not a view_key in {SURFACE_REGISTRY_PATH}"
                ));
            }
        }
    }
    for (module, count) in &seen {
        if *count > 1 {
            out.push(format!("{module}: classified {count} times"));
        }
    }
    for module in modules_on_disk {
        if !seen.contains_key(module) {
            out.push(format!(
                "{module}: {COMMANDS_DIR}/{module}.rs is not classified in {MANIFEST_PATH}"
            ));
        }
    }
    out.sort();
    out
}

pub fn run(repo_root: &Path) -> Result<()> {
    let modules = modules_on_disk(&repo_root.join(COMMANDS_DIR))?;
    let manifest = read_yaml(&repo_root.join(MANIFEST_PATH))?;
    let registry = read_yaml(&repo_root.join(SURFACE_REGISTRY_PATH))?;
    let found = problems(&modules, &manifest, &view_keys(&registry));
    if !found.is_empty() {
        bail!(
            "gui-command-backends: {} problem(s):\n  {}",
            found.len(),
            found.join("\n  ")
        );
    }
    tracing::info!(
        modules = modules.len(),
        "gui-command-backends: every Axis command module is classified"
    );
    Ok(())
}

fn read_yaml(path: &Path) -> Result<Value> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_yaml::from_str(&raw).with_context(|| format!("parse {}", path.display()))
}

fn modules_on_disk(dir: &Path) -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if stem != "mod" {
                out.insert(stem.to_string());
            }
        }
    }
    Ok(out)
}

fn view_keys(registry: &Value) -> BTreeSet<String> {
    registry
        .get("surfaces")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("view_key").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn manifest(yaml: &str) -> Value {
        serde_yaml::from_str(yaml).expect("fixture yaml")
    }

    #[test]
    fn consistent_manifest_has_no_problems() {
        let m = manifest(
            "commands:\n- { module: mcp, class: daemon, surfaces: [mesh] }\n- { module: pty, class: host, surfaces: [] }\n",
        );
        assert!(problems(&set(&["mcp", "pty"]), &m, &set(&["mesh"])).is_empty());
    }

    #[test]
    fn unclassified_module_is_reported() {
        let m = manifest("commands:\n- { module: mcp, class: daemon, surfaces: [] }\n");
        let found = problems(&set(&["mcp", "new_thing"]), &m, &set(&[]));
        assert_eq!(found.len(), 1);
        assert!(found[0].starts_with("new_thing:"), "{found:?}");
        assert!(found[0].contains("is not classified"), "{found:?}");
    }

    #[test]
    fn row_for_missing_module_is_reported() {
        let m = manifest("commands:\n- { module: gone, class: db, surfaces: [] }\n");
        let found = problems(&set(&[]), &m, &set(&[]));
        assert!(
            found
                .iter()
                .any(|p| p.starts_with("gone:") && p.contains("does not exist")),
            "{found:?}"
        );
    }

    #[test]
    fn bad_class_duplicate_and_unknown_surface_are_reported() {
        let m = manifest(
            "commands:\n- { module: a, class: remote, surfaces: [nope] }\n- { module: a, class: db, surfaces: [] }\n",
        );
        let found = problems(&set(&["a"]), &m, &set(&["mesh"]));
        assert!(
            found.iter().any(|p| p.contains("class `remote`")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|p| p.contains("classified 2 times")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|p| p.contains("surface `nope`")),
            "{found:?}"
        );
    }

    #[test]
    fn empty_manifest_is_reported() {
        let found = problems(&set(&[]), &manifest("schema_version: 1\n"), &set(&[]));
        assert!(
            found
                .iter()
                .any(|p| p.contains("`commands` is missing or empty")),
            "{found:?}"
        );
    }

    #[test]
    fn repository_manifest_is_consistent() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        run(&root)
            .expect("contracts/gui/command-backends.v1.yaml matches crates/vox-gui/src/commands");
    }
}
