//! Cargo workspace member directory resolution and coarse multi-stack roots.

use std::path::{Path, PathBuf};

use crate::bounded_fs::read_utf8_file_capped;

fn read_package_json_name(root: &Path) -> Option<String> {
    let text = read_utf8_file_capped(&root.join("package.json"))?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("name")?.as_str().map(|s| s.to_string())
}

/// Slug for affinity group names: manifest `name` or directory name, with safe characters.
fn node_package_slug(root: &Path) -> String {
    let from_manifest = read_package_json_name(root);
    let raw = from_manifest.unwrap_or_else(|| {
        root.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "node".to_string())
    });
    raw.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn workspace_patterns_from_package_json(root: &Path) -> Vec<String> {
    let Some(text) = read_utf8_file_capped(&root.join("package.json")) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(ws) = v.get("workspaces") else {
        return Vec::new();
    };
    match ws {
        serde_json::Value::Array(a) => a
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.replace('\\', "/")))
            .collect(),
        serde_json::Value::Object(o) => o
            .get("packages")
            .and_then(|p| p.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.replace('\\', "/")))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn workspace_patterns_from_pnpm(root: &Path) -> Vec<String> {
    let p = root.join("pnpm-workspace.yaml");
    let Some(text) = read_utf8_file_capped(&p) else {
        return Vec::new();
    };
    let Ok(y) = serde_yaml::from_str::<serde_yaml::Value>(&text) else {
        return Vec::new();
    };
    y.get("packages")
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(|s| s.replace('\\', "/")))
                .collect()
        })
        .unwrap_or_default()
}

fn expand_node_workspace_pattern(root: &Path, pat: &str) -> Vec<PathBuf> {
    let pat = pat.replace('\\', "/");
    if pat.contains('*') {
        let base_name = pat
            .trim_end_matches("/**")
            .trim_end_matches("/*")
            .trim_end_matches('*');
        let base = root.join(base_name);
        if !base.is_dir() {
            return Vec::new();
        }
        let mut v = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&base) {
            for ent in rd.flatten() {
                let p = ent.path();
                if p.is_dir() && p.join("package.json").is_file() {
                    v.push(p);
                }
            }
        }
        v.sort();
        return v;
    }
    let single = root.join(pat);
    if single.join("package.json").is_file() {
        vec![single]
    } else {
        Vec::new()
    }
}

fn read_go_module_last_segment(root: &Path) -> Option<String> {
    let text = read_utf8_file_capped(&root.join("go.mod"))?;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("module ") {
            let m = rest.split_whitespace().next()?;
            return Some(m.rsplit('/').next().unwrap_or(m).to_string());
        }
    }
    None
}

/// Node package roots for orchestrator affinity: workspace members from `package.json` `workspaces`,
/// plus `pnpm-workspace.yaml` `packages`, each with a stable slug and package directory.
///
/// When no workspace patterns match, returns the repo root as a single package (if `package.json` exists).
pub fn node_workspace_packages(root: &Path) -> Vec<(String, PathBuf)> {
    if !root.join("package.json").is_file() {
        return Vec::new();
    }
    let mut patterns = workspace_patterns_from_package_json(root);
    patterns.extend(workspace_patterns_from_pnpm(root));
    patterns.sort();
    patterns.dedup();

    if patterns.is_empty() {
        let slug = node_package_slug(root);
        return vec![(slug, root.to_path_buf())];
    }

    let mut paths: Vec<PathBuf> = Vec::new();
    for pat in &patterns {
        paths.extend(expand_node_workspace_pattern(root, pat));
    }
    paths.sort();
    paths.dedup();

    if paths.is_empty() {
        let slug = node_package_slug(root);
        vec![(slug, root.to_path_buf())]
    } else {
        paths
            .into_iter()
            .map(|p| (node_package_slug(&p), p))
            .collect()
    }
}

/// Python project roots: the repo root when `pyproject.toml` or `setup.py` exists.
pub fn python_roots(root: &Path) -> Vec<(String, PathBuf)> {
    if !(root.join("pyproject.toml").is_file() || root.join("setup.py").is_file()) {
        return Vec::new();
    }
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "python".to_string());
    vec![(name, root.to_path_buf())]
}

/// Go module roots: the repo root when `go.mod` exists.
pub fn go_roots(root: &Path) -> Vec<(String, PathBuf)> {
    if !root.join("go.mod").is_file() {
        return Vec::new();
    }
    let name = read_go_module_last_segment(root).unwrap_or_else(|| "go".to_string());
    vec![(name, root.to_path_buf())]
}

/// Expand `[workspace].members` from the **root** `Cargo.toml` into directories that contain a `Cargo.toml`.
///
/// Handles simple `crates/*`-style patterns by scanning the parent directory for subcrates.
/// Literal member paths are joined to `root`.
pub fn cargo_workspace_member_dirs(root: &Path) -> Vec<PathBuf> {
    let manifest = root.join("Cargo.toml");
    if !manifest.is_file() {
        return Vec::new();
    }
    let Some(text) = read_utf8_file_capped(&manifest) else {
        return Vec::new();
    };
    let Ok(val) = toml::from_str::<toml::Value>(&text) else {
        return Vec::new();
    };
    let Some(members) = val
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
    else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for m in members {
        let Some(pat) = m.as_str() else {
            continue;
        };
        out.extend(expand_member_pattern(root, pat));
    }
    out.sort();
    out.dedup();
    out
}

fn expand_member_pattern(root: &Path, pat: &str) -> Vec<PathBuf> {
    let pat = pat.replace('\\', "/");
    if pat.contains('*') {
        let base_name = pat
            .trim_end_matches("/**")
            .trim_end_matches("/*")
            .trim_end_matches('*');
        let base = root.join(base_name);
        if base.is_dir() {
            let mut v = Vec::new();
            if let Ok(rd) = std::fs::read_dir(&base) {
                for ent in rd.flatten() {
                    let p = ent.path();
                    if p.join("Cargo.toml").is_file() {
                        v.push(p);
                    }
                }
            }
            return v;
        }
        return Vec::new();
    }
    let single = root.join(pat);
    if single.join("Cargo.toml").is_file() {
        vec![single]
    } else {
        Vec::new()
    }
}
