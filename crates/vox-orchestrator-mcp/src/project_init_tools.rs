//! MCP `vox_project_init` — parity with `vox init` via `vox_project_scaffold`.

use crate::params::ToolResult;
use crate::server_state::ServerState;

const REM_PROJECT_INIT: &str = "Provide `project_name`, optional `package_kind` (default application), optional `template` (chatbot|dashboard|api), optional `target_subdir` (repo-relative, no `..`). Refuses if Vox.toml or skill file already exists.";

pub async fn project_init(state: &ServerState, args: serde_json::Value) -> String {
    let project_name = match args
        .get("project_name")
        .and_then(|v| v.as_str())
        .map(str::trim)
    {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return ToolResult::<serde_json::Value>::err_with_remediation(
                "Missing or empty project_name".to_string(),
                REM_PROJECT_INIT,
            )
            .to_json();
        }
    };
    let package_kind = args
        .get("package_kind")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("application");
    let template = args
        .get("template")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let target_subdir = args
        .get("target_subdir")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let harness = args
        .get("harness")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let read_only = args
        .get("read_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if read_only {
        let target_root = match vox_project_scaffold::resolve_scaffold_target_under_repo(
            &state.repository.root,
            target_subdir.as_deref(),
        ) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::<serde_json::Value>::err_with_remediation(
                    e.to_string(),
                    REM_PROJECT_INIT,
                )
                .to_json();
            }
        };
        return match vox_repository::onboard_repository_harness(&target_root, true) {
            Ok(summary) => ToolResult::ok(summary).to_json(),
            Err(e) => ToolResult::<serde_json::Value>::err_with_remediation(
                e.to_string(),
                REM_PROJECT_INIT,
            )
            .to_json(),
        };
    }

    match vox_project_scaffold::scaffold_vox_project_under_repo_with_options(
        &state.repository.root,
        target_subdir.as_deref(),
        &project_name,
        package_kind,
        template.as_deref(),
        harness,
    ) {
        Ok(summary) => {
            ToolResult::ok(serde_json::to_value(&summary).expect("ScaffoldSummary serializes"))
                .to_json()
        }
        Err(e) => {
            ToolResult::<serde_json::Value>::err_with_remediation(e.to_string(), REM_PROJECT_INIT)
                .to_json()
        }
    }
}
