//! discover() walks an install root for Plugin.toml manifests, parses each,
//! and populates a Registry. For skill payloads (and the skill side of
//! composite payloads), the SKILL.md body is eagerly read and registered.
//! For code payloads, the dylib path is recorded but NOT loaded — actual
//! dlopen happens lazily via Loader::load().

use crate::errors::LoadError;
use crate::registry::{PluginEntry, Registry};
use crate::telemetry;
use std::path::Path;
use vox_plugin_api::manifest::{PluginManifest, PluginPayload, SkillPayload};
use vox_plugin_api::skill::{LoadedSkill, SkillManifest};

pub fn discover(root: &Path) -> Result<Registry, LoadError> {
    let registry = Registry::new();
    if !root.is_dir() {
        return Ok(registry);
    }
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "Plugin.toml")
    {
        let path = entry.path();
        let raw = std::fs::read_to_string(path).map_err(|source| LoadError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let manifest: PluginManifest =
            toml::from_str(&raw).map_err(|source| LoadError::ManifestParse {
                path: path.to_path_buf(),
                source,
            })?;
        let install_dir = path.parent().unwrap().to_path_buf();

        // Skill side: eagerly parse and register if present.
        let skill_to_register: Option<&SkillPayload> = match &manifest.plugin.payload {
            PluginPayload::Skill(s) => Some(s),
            PluginPayload::Composite(c) => Some(&c.skill),
            PluginPayload::Code(_) => None,
        };

        if let Some(skill_payload) = skill_to_register {
            let skill_md_path = install_dir.join(&skill_payload.skill_md);
            let body = std::fs::read_to_string(&skill_md_path).unwrap_or_default();
            let exposed_tools = skill_payload.tools.exposes.clone();
            registry.skills.install(LoadedSkill {
                plugin_id: manifest.plugin.id.clone(),
                format_version: skill_payload.format_version,
                manifest: SkillManifest {
                    id: manifest.plugin.id.clone(),
                    name: manifest.plugin.name.clone(),
                    version: manifest.plugin.version.clone(),
                    description: manifest.plugin.description.clone(),
                    tools: exposed_tools.clone(),
                },
                body,
                exposed_tools,
            });
        }

        let payload_kind = match &manifest.plugin.payload {
            PluginPayload::Code(_) => "code",
            PluginPayload::Skill(_) => "skill",
            PluginPayload::Composite(_) => "composite",
        };
        let abi_or_format = match &manifest.plugin.payload {
            PluginPayload::Code(c) => c.abi_version,
            PluginPayload::Skill(s) => s.format_version,
            PluginPayload::Composite(c) => c.code.abi_version,
        };
        telemetry::discovered(
            &manifest.plugin.id,
            &manifest.plugin.version,
            payload_kind,
            abi_or_format,
        );

        registry.record(PluginEntry {
            id: manifest.plugin.id.clone(),
            version: manifest.plugin.version.clone(),
            install_dir,
            payload: manifest.plugin.payload,
        });
    }
    Ok(registry)
}
