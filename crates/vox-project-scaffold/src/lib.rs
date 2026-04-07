//! Shared Vox project scaffolding for **`vox init`** and MCP **`vox_project_init`**.

use anyhow::{Context, Result};
use serde::Serialize;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

const CHATBOT_TEMPLATE: &str = r#"# Vox Chatbot — OpenRouter-powered chat app
#
# Edit OPENROUTER_API_KEY in .env to enable live LLM responses.
# Run with: vox build src/main.vox -o dist && vox run src/main.vox

@table type Conversation:
    user_id: str
    started_at: str

@table type MessageTrace:
    conversation_id: str
    role: str
    content: str
    request_id: str

@query fn recent_messages(conversation_id: str) to list[MessageTrace]:
    ret []

@mutation fn log_message(conversation_id: str, role: str, content: str, request_id: str) to Result[bool]:
    ret Ok(true)

activity call_provider(prompt: str) to Result[str]:
    Ok("stub-response: " + prompt)

workflow chat_pipeline(prompt: str) to Result[str]:
    let response = call_provider(prompt) with { retries: 3, timeout: "30s", activity_id: "provider-call" }
    response

@server fn chat(prompt: str, request_id: str) to Result[str]:
    let response = chat_pipeline(prompt) with { retries: 2, timeout: "45s" }
    let _ = log_message("conv-default", "user", prompt, request_id)
    let _ = log_message("conv-default", "assistant", "ok", request_id)
    ret response

@component fn Chat() to Element:
    let (messages, set_messages) = use_state([])
    let (input, set_input) = use_state("")
    <div class="chat-root">
        <h1>"Chat"</h1>
        <div class="messages">
            for msg in messages:
                <div class="message">{msg.content}</div>
        </div>
        <input value={input} on_change={set_input} placeholder="Type a message..." />
        <button on_click={fn():
            let _ = chat(input, "req-1")
        }>"Send"</button>
    </div>

routes:
    "/" to Chat
"#;

const DASHBOARD_TEMPLATE: &str = r#"# Vox Dashboard — data table with route params
#
# Run with: vox build src/main.vox -o dist && vox run src/main.vox

@table type Item:
    name: str
    value: int
    created_at: str

@query fn list_items() to list[Item]:
    ret []

@mutation fn add_item(name: str, value: int) to Result[str]:
    ret Ok("Added: " + name)

@health fn check_db() to bool:
    ret true

@metric fn items_created(name: str) to str:
    ret "ok"

@component fn Dashboard() to Element:
    let (items, set_items) = use_state([])
    <div class="dashboard">
        <h1>"Dashboard"</h1>
        <table>
            for item in items:
                <tr>
                    <td>{item.name}</td>
                    <td>{item.value}</td>
                </tr>
        </table>
    </div>

@component fn ItemDetail() to Element:
    let id = use_param("id")
    <div>
        <h2>"Item: " + id</h2>
    </div>

routes:
    "/" to Dashboard
    "/items/:id" to ItemDetail
"#;

const API_TEMPLATE: &str = r#"# Vox API — server functions with health + metrics
#
# Run with: vox build src/main.vox -o dist && vox run src/main.vox

@table type Task:
    title: str
    done: bool
    created_at: str

@health fn health_check() to bool:
    ret true

@metric fn tasks_created() to str:
    ret "ok"

@server fn create_task(title: str) to Result[str]:
    ret Ok("Created: " + title)

@server fn list_tasks() to Result[list[Task]]:
    ret Ok([])

@server fn complete_task(id: str) to Result[bool]:
    ret Ok(true)
"#;

const DEFAULT_FULL_STACK: &str = "# My Vox App — a full-stack starter\n#\n# Run with: vox build src/main.vox -o dist && vox run src/main.vox\n\n@table type Note:\n    title: str\n    content: str\n    created_at: str\n\n@server fn add_note(title: str, content: str) to Result[str]:\n    ret Ok(\"Added: \" + title)\n\n@server fn list_notes() to Result[str]:\n    ret Ok(\"[]\")\n\n@component fn App() to Element:\n    let (notes, set_notes) = use_state([])\n    <div class=\"app\">\n        <h1>\"My Vox App\"</h1>\n        <p>\"Edit src/main.vox to get started\"</p>\n    </div>\n\nroutes:\n    \"/\" to App\n";

const AGENT_KIND: &str = "# A Vox AI agent\n\n@agent_def fn MyAgent(query: str) to str:\n    \"Response to: \" + query\n";

const WORKFLOW_KIND: &str = "# A Vox workflow\n\nactivity process_data(input: str) to Result[str]:\n    Ok(\"Processed: \" + input)\n\nworkflow my_workflow(input: str) to Result[str]:\n    let result = process_data(input) with { retries: 3, timeout: \"10s\" }\n    result\n";

const MOBILE_PWA_TEMPLATE: &str = r#"# Vox Mobile PWA App
import std.mobile

@component fn App() to Element:
    let (photo, set_photo) = use_state("")
    <div class="app">
        <h1>"Camera Test"</h1>
        <button on_click={fn():
            let result = mobile.take_photo()
            if result.is_ok():
                set_photo(result.unwrap())
        }>"Take Photo"</button>
        if photo != "":
            <img src={photo} alt="Captured" />
    </div>

routes:
    "/" to App
"#;

/// Summary of files and directories created under the scaffold root.
#[derive(Debug, Clone, Serialize)]
pub struct ScaffoldSummary {
    pub package_kind: String,
    pub project_name: String,
    pub created_relative_paths: Vec<String>,
    /// Template key when `template` was honored (known keys only).
    pub template_applied: Option<String>,
}

/// Resolve target directory strictly under `workspace_repo_root` (empty / missing `target_subdir` → workspace root).
pub fn resolve_scaffold_target_under_repo(
    workspace_repo_root: &Path,
    target_subdir: Option<&str>,
) -> Result<PathBuf> {
    match target_subdir.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(workspace_repo_root.to_path_buf()),
        Some(rel) => vox_repository::resolve_strict_repo_relative_path(workspace_repo_root, rel)
            .map_err(|e| anyhow::anyhow!("{e}")),
    }
}

fn main_vox_content(package_kind: &str, template: Option<&str>) -> Cow<'static, str> {
    if let Some(tmpl) = template {
        return match tmpl {
            "chatbot" => Cow::Borrowed(CHATBOT_TEMPLATE),
            "dashboard" => Cow::Borrowed(DASHBOARD_TEMPLATE),
            "api" => Cow::Borrowed(API_TEMPLATE),
            "mobile-pwa" => Cow::Borrowed(MOBILE_PWA_TEMPLATE),
            other => Cow::Owned(format!(
                "# My Vox App\n\n@component fn App() to Element:\n    <div><h1>\"My Vox App\"</h1></div>\n\nroutes:\n    \"/\" to App\n\n# Unknown template '{other}'; use chatbot, dashboard, api, or mobile-pwa.\n"
            )),
        };
    }
    match package_kind {
        "agent" => Cow::Borrowed(AGENT_KIND),
        "workflow" => Cow::Borrowed(WORKFLOW_KIND),
        "chatbot" => Cow::Borrowed(CHATBOT_TEMPLATE),
        _ => Cow::Borrowed(DEFAULT_FULL_STACK),
    }
}

/// Scaffold under an **absolute** directory (e.g. CLI current dir). Creates the directory tree if needed.
pub fn scaffold_vox_project_at(
    root: &Path,
    project_name: &str,
    package_kind: &str,
    template: Option<&str>,
) -> Result<ScaffoldSummary> {
    std::fs::create_dir_all(root)
        .with_context(|| format!("create scaffold root {}", root.display()))?;

    let mut created_relative_paths = Vec::new();

    if package_kind == "skill" {
        let skill_file_name = vox_repository::skill_markdown_filename(project_name);
        let target_path = root.join(&skill_file_name);
        if target_path.exists() {
            anyhow::bail!("{} already exists in this directory", skill_file_name);
        }
        let content = vox_repository::skill_markdown_for_project(project_name);
        std::fs::write(&target_path, content)
            .with_context(|| format!("Failed to write {}", skill_file_name))?;
        created_relative_paths.push(skill_file_name.replace('\\', "/"));
        return Ok(ScaffoldSummary {
            package_kind: package_kind.to_string(),
            project_name: project_name.to_string(),
            created_relative_paths,
            template_applied: None,
        });
    }

    let manifest_path = root.join("Vox.toml");
    if manifest_path.exists() {
        anyhow::bail!("Vox.toml already exists in this directory");
    }

    let manifest = vox_pm::VoxManifest::scaffold(project_name, package_kind);
    let toml_content = manifest
        .to_toml_string()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    std::fs::write(&manifest_path, &toml_content).with_context(|| "Failed to write Vox.toml")?;
    created_relative_paths.push("Vox.toml".to_string());

    let src_dir = root.join("src");
    if !src_dir.exists() {
        std::fs::create_dir_all(&src_dir).with_context(|| "Failed to create src/ directory")?;
    }

    let main_file = src_dir.join("main.vox");
    if !main_file.exists() {
        let main_content = main_vox_content(package_kind, template);
        std::fs::write(&main_file, main_content.as_ref())
            .with_context(|| "Failed to write src/main.vox")?;
        created_relative_paths.push("src/main.vox".to_string());
    }

    let modules_dir = root.join(".vox_modules");
    if !modules_dir.exists() {
        std::fs::create_dir_all(&modules_dir).with_context(|| "Failed to create .vox_modules/")?;
        created_relative_paths.push(".vox_modules".to_string());
    }

    let template_applied = template.map(str::to_string);

    Ok(ScaffoldSummary {
        package_kind: package_kind.to_string(),
        project_name: project_name.to_string(),
        created_relative_paths,
        template_applied,
    })
}

/// Scaffold under `workspace_repo_root` or a strict repo-relative subdirectory (for MCP).
pub fn scaffold_vox_project_under_repo(
    workspace_repo_root: &Path,
    target_subdir: Option<&str>,
    project_name: &str,
    package_kind: &str,
    template: Option<&str>,
) -> Result<ScaffoldSummary> {
    let root = resolve_scaffold_target_under_repo(workspace_repo_root, target_subdir)?;
    scaffold_vox_project_at(&root, project_name, package_kind, template)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn skill_scaffold_writes_file() {
        let d = TempDir::new().unwrap();
        let s = scaffold_vox_project_at(d.path(), "my-skill", "skill", None).unwrap();
        assert_eq!(s.created_relative_paths.len(), 1);
        assert!(d.path().join("my-skill.skill.md").is_file());
    }

    #[test]
    fn application_scaffold_creates_manifest_and_main() {
        let d = TempDir::new().unwrap();
        let s = scaffold_vox_project_at(d.path(), "app1", "application", None).unwrap();
        assert!(d.path().join("Vox.toml").is_file());
        assert!(d.path().join("src/main.vox").is_file());
        assert!(s.created_relative_paths.contains(&"Vox.toml".to_string()));
    }
}
