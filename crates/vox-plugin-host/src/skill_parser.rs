//! SKILL.md format parser — extracts frontmatter + body from a SKILL.md file.
//!
//! Moved from `vox-skills::parser`. `vox-skills` re-exports `parse_skill_md` from here.
//!
//! Supports both the current AgentSkills-compliant frontmatter shape and the
//! legacy Vox-only shape for backwards compatibility.
//!
//! ## AgentSkills-compliant shape (current)
//! ```markdown
//! ---
//! name = "skill-compiler"
//! description = "Compiles Vox source files."
//!
//! [metadata]
//! "vox-id" = "vox.compiler"
//! "vox-version" = "0.1.0"
//! "vox-author" = "vox-team"
//! "vox-category" = "compiler"
//! "vox-tools" = ["vox_compile"]
//! "vox-tags" = ["compile"]
//! "vox-permissions" = ["read_files", "shell_exec"]
//! ---
//! ```
//!
//! ## Legacy Vox-only shape (still accepted)
//! ```markdown
//! ---
//! id = "vox.compiler"
//! name = "Vox Compiler"
//! version = "0.1.0"
//! author = "vox"
//! description = "Compiles Vox programs"
//! category = "compiler"
//! tools = ["vox_compile"]
//! ---
//! ```

use crate::skill_bundle::VoxSkillBundle;
use crate::skill_manifest::{SkillCategory, SkillManifest, SkillPermission};

/// Parse error returned by [`parse_skill_md`].
#[derive(Debug, thiserror::Error)]
pub enum ParseSkillError {
    /// SKILL.md must start with --- frontmatter.
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
    /// TOML parse failure (typically `SKILL.md` front matter).
    #[error("TOML error: {0}")]
    Toml(String),
}

/// Parse a full SKILL.md file into a `VoxSkillBundle`.
pub fn parse_skill_md(content: &str) -> Result<VoxSkillBundle, ParseSkillError> {
    // Split frontmatter from body
    let (frontmatter, _body) = split_frontmatter(content)?;

    // Parse frontmatter as TOML
    let raw: toml::Value =
        toml::from_str(&frontmatter).map_err(|e| ParseSkillError::Toml(e.to_string()))?;

    // Helper: look up a string field first in metadata.vox-<key>, then at top-level <key>.
    let meta = raw.get("metadata");
    let vox_str = |vox_key: &str, legacy_key: &str| -> Option<String> {
        meta.and_then(|m| m.get(vox_key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                raw.get(legacy_key)
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
    };
    let vox_arr = |vox_key: &str, legacy_key: &str| -> Vec<String> {
        meta.and_then(|m| m.get(vox_key))
            .and_then(|v| v.as_array())
            .or_else(|| raw.get(legacy_key).and_then(|v| v.as_array()))
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    };

    // --- AgentSkills required fields ---
    // `name` is the spec-required field at top level (e.g. "skill-compiler").
    // We keep it as the human-readable display name internally.
    let name = raw["name"]
        .as_str()
        .ok_or_else(|| ParseSkillError::InvalidManifest("missing name".into()))?
        .to_string();

    let description = raw
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // --- Vox-specific fields: metadata.vox-* with legacy top-level fallback ---

    // id: metadata."vox-id" -> top-level "id" -> derive from name
    let id = vox_str("vox-id", "id").unwrap_or_else(|| name.clone());

    let version = vox_str("vox-version", "version").unwrap_or_else(|| "0.1.0".to_string());

    let author = vox_str("vox-author", "author").unwrap_or_else(|| "unknown".to_string());

    let category_str = vox_str("vox-category", "category").unwrap_or_else(|| "custom".to_string());
    let category = parse_category(&category_str);

    let tools = vox_arr("vox-tools", "tools");
    let tags = vox_arr("vox-tags", "tags");
    let dependencies = vox_arr("vox-dependencies", "dependencies");

    let permissions: Vec<SkillPermission> = meta
        .and_then(|m| m.get("vox-permissions"))
        .and_then(|v| v.as_array())
        .or_else(|| raw.get("permissions").and_then(|v| v.as_array()))
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().and_then(parse_permission))
                .collect()
        })
        .unwrap_or_default();

    let homepage = raw
        .get("homepage")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut manifest = SkillManifest::new(id, name, version, author, description, category);
    manifest.tools = tools;
    manifest.tags = tags;
    manifest.dependencies = dependencies;
    manifest.permissions = permissions;
    manifest.homepage = homepage;

    Ok(VoxSkillBundle::new(manifest, content.to_string()))
}

fn split_frontmatter(content: &str) -> Result<(String, String), ParseSkillError> {
    let content = content.trim();
    if !content.starts_with("---") {
        return Err(ParseSkillError::InvalidManifest(
            "SKILL.md must start with --- frontmatter".into(),
        ));
    }
    // Skip the first "---"
    let after_first = &content[3..];
    // Find the closing "---"
    if let Some(end) = after_first.find("\n---") {
        let frontmatter = after_first[..end].trim().to_string();
        let body = after_first[end + 4..].trim().to_string();
        Ok((frontmatter, body))
    } else {
        Err(ParseSkillError::InvalidManifest(
            "SKILL.md frontmatter is not properly closed with ---".into(),
        ))
    }
}

fn parse_category(s: &str) -> SkillCategory {
    match s {
        "compiler" => SkillCategory::Compiler,
        "testing" => SkillCategory::Testing,
        "documentation" => SkillCategory::Documentation,
        "deployment" => SkillCategory::Deployment,
        "refactor" => SkillCategory::Refactor,
        "analysis" => SkillCategory::Analysis,
        "git" => SkillCategory::Git,
        "database" => SkillCategory::Database,
        "web_search" => SkillCategory::WebSearch,
        "communication" => SkillCategory::Communication,
        "security" => SkillCategory::Security,
        "monitoring" => SkillCategory::Monitoring,
        other => SkillCategory::Custom(other.to_string()),
    }
}

fn parse_permission(s: &str) -> Option<SkillPermission> {
    match s {
        "read_files" => Some(SkillPermission::ReadFiles),
        "write_files" => Some(SkillPermission::WriteFiles),
        "shell_exec" => Some(SkillPermission::ShellExec),
        "network" => Some(SkillPermission::Network),
        "db_read" => Some(SkillPermission::DbRead),
        "db_write" => Some(SkillPermission::DbWrite),
        "secrets" => Some(SkillPermission::Secrets),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_manifest::SkillCategory;

    /// Legacy TOML format (still accepted for backwards compatibility).
    const LEGACY_SKILL_MD: &str = r#"---
id = "vox.test-runner"
name = "Test Runner"
version = "1.0.0"
author = "vox-team"
description = "Runs the Vox test suite"
category = "testing"
tools = ["vox_run_tests", "vox_test_all"]
tags = ["testing", "ci"]
permissions = ["read_files", "shell_exec"]
---

# Test Runner Skill

Run all tests in the workspace using `cargo test`.
"#;

    /// AgentSkills-compliant format with metadata.vox-* fields.
    const AGENTSKILLS_SKILL_MD: &str = r#"---
name = "skill-test-runner"
description = "Runs the Vox test suite"

[metadata]
"vox-id" = "vox.test-runner"
"vox-version" = "1.0.0"
"vox-author" = "vox-team"
"vox-category" = "testing"
"vox-tools" = ["vox_run_tests", "vox_test_all"]
"vox-tags" = ["testing", "ci"]
"vox-permissions" = ["read_files", "shell_exec"]
---

# Test Runner Skill

Run all tests in the workspace using `cargo test`.
"#;

    #[test]
    fn parse_legacy_skill_md() {
        let bundle = parse_skill_md(LEGACY_SKILL_MD).expect("parse legacy");
        assert_eq!(bundle.manifest.id, "vox.test-runner");
        assert_eq!(bundle.manifest.version, "1.0.0");
        assert_eq!(bundle.manifest.category, SkillCategory::Testing);
        assert_eq!(bundle.manifest.tools, vec!["vox_run_tests", "vox_test_all"]);
        assert_eq!(bundle.manifest.tags, vec!["testing", "ci"]);
        assert!(
            bundle
                .manifest
                .permissions
                .contains(&SkillPermission::ReadFiles)
        );
        assert!(
            bundle
                .manifest
                .permissions
                .contains(&SkillPermission::ShellExec)
        );
    }

    #[test]
    fn parse_agentskills_skill_md() {
        let bundle = parse_skill_md(AGENTSKILLS_SKILL_MD).expect("parse agentskills");
        // vox-id from metadata block
        assert_eq!(bundle.manifest.id, "vox.test-runner");
        // name is the spec-level name (used as display name internally)
        assert_eq!(bundle.manifest.name, "skill-test-runner");
        assert_eq!(bundle.manifest.version, "1.0.0");
        assert_eq!(bundle.manifest.category, SkillCategory::Testing);
        assert_eq!(bundle.manifest.tools, vec!["vox_run_tests", "vox_test_all"]);
        assert_eq!(bundle.manifest.tags, vec!["testing", "ci"]);
        assert!(
            bundle
                .manifest
                .permissions
                .contains(&SkillPermission::ReadFiles)
        );
        assert!(
            bundle
                .manifest
                .permissions
                .contains(&SkillPermission::ShellExec)
        );
    }

    #[test]
    fn agentskills_id_derives_from_name_when_absent() {
        let md = r#"---
name = "skill-noop"
description = "No-op skill"
---

# Noop
"#;
        let bundle = parse_skill_md(md).expect("parse");
        assert_eq!(bundle.manifest.id, "skill-noop");
    }

    #[test]
    fn missing_frontmatter_is_error() {
        let result = parse_skill_md("# Just a heading\nNo frontmatter.");
        assert!(result.is_err());
    }
}
