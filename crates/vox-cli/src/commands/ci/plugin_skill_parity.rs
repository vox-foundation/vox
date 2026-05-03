//! `vox ci plugin-skill-parity`
//!
//! Walks `crates/` for any `Plugin.toml` declaring a skill or composite
//! payload, asserts the referenced `skill-md` file exists and is non-empty,
//! and that `tools.exposes` is non-empty.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ManifestHead {
    plugin: PluginHead,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct PluginHead {
    #[allow(dead_code)]
    id: String,
    payload: PayloadHead,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
enum PayloadHead {
    Code {},
    Skill(SkillHead),
    Composite {
        #[serde(default)]
        skill: SkillHead,
    },
}

#[derive(Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct SkillHead {
    #[serde(default)]
    skill_md: String,
    #[serde(default)]
    tools: ToolsHead,
}

#[derive(Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct ToolsHead {
    #[serde(default)]
    exposes: Vec<String>,
}

pub fn run() -> Result<()> {
    let mut errors: Vec<String> = Vec::new();
    let mut checked = 0usize;

    let crates_root = Path::new("crates");
    if !crates_root.is_dir() {
        println!("✓ no crates/ dir; nothing to check");
        return Ok(());
    }

    for entry in walkdir::WalkDir::new(crates_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "Plugin.toml")
    {
        let path = entry.path();
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let head: ManifestHead = match toml::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("{}: parse error: {e}", path.display()));
                continue;
            }
        };
        let skill = match &head.plugin.payload {
            PayloadHead::Skill(s) => s.clone(),
            PayloadHead::Composite { skill } => skill.clone(),
            PayloadHead::Code {} => continue,
        };
        if skill.skill_md.is_empty() {
            errors.push(format!("{}: skill-md is empty", path.display()));
            continue;
        }
        let skill_md_path = path.parent().unwrap().join(&skill.skill_md);
        match std::fs::read_to_string(&skill_md_path) {
            Ok(body) if body.trim().is_empty() => {
                errors.push(format!(
                    "{}: skill-md '{}' is empty",
                    path.display(),
                    skill.skill_md
                ));
            }
            Ok(_) => {}
            Err(e) => {
                errors.push(format!(
                    "{}: skill-md '{}' not readable: {e}",
                    path.display(),
                    skill.skill_md,
                ));
                continue;
            }
        }
        if skill.tools.exposes.is_empty() {
            errors.push(format!("{}: tools.exposes is empty", path.display()));
        }
        checked += 1;
    }
    if errors.is_empty() {
        println!(
            "✓ plugin-skill-parity ok ({} skill-bearing manifests checked)",
            checked
        );
        Ok(())
    } else {
        for e in &errors {
            eprintln!("✗ {e}");
        }
        anyhow::bail!("plugin-skill-parity failed with {} error(s)", errors.len())
    }
}
