//! Skill markdown scaffold for `vox init --kind skill` (CLI); reusable from MCP or other surfaces.

/// Filename for a skill at the workspace root: `{name}.skill.md`.
pub fn skill_markdown_filename(project_name: &str) -> String {
    format!("{project_name}.skill.md")
}

/// Front matter + body for a new Vox skill file.
pub fn skill_markdown_for_project(project_name: &str) -> String {
    format!(
        r#"---
id = "vox.{name}"
name = "{name}"
version = "0.1.0"
author = "your-name"
description = "A new Vox skill"
category = "custom:misc"
tools = ["vox_my_new_tool"]
tags = ["custom", "vox"]
permissions = []
---

# {name} Skill

Provide instructions for the Vox LLM on how to use this skill here.

## Tools

- `vox_my_new_tool` — What this tool does.

## Instructions

1. Use `vox_my_new_tool` when you need to...
2. Next steps...
"#,
        name = project_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_filename_and_body_reference_name() {
        let body = skill_markdown_for_project("my-skill");
        assert!(body.contains("id = \"vox.my-skill\""));
        assert_eq!(skill_markdown_filename("my-skill"), "my-skill.skill.md");
    }
}
