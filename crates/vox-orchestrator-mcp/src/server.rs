//! RMCP [`ServerHandler`] for tool listing and `call_tool` dispatch.

use rmcp::model::{
    CallToolRequestParams, CallToolResult, GetPromptRequestParams, GetPromptResult, Implementation,
    InitializeRequestParams, InitializeResult, ListPromptsResult, ListResourceTemplatesResult,
    ListResourcesResult, ListToolsResult, PaginatedRequestParams, Prompt, PromptMessage,
    PromptMessageRole, RawResource, ReadResourceRequestParams, ReadResourceResult, Resource,
    ResourceContents, ServerCapabilities,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};

use crate::params::ToolResult;
use crate::server_state::ServerState;

const REM_DAEMON_UNREACHABLE: &str = "vox-orchestrator-d could not be reached or spawned. Check that the `vox-orchestrator-d` binary is installed (sibling of this executable, ~/.vox/bin, or PATH), then retry.";

const RESOURCE_LLMS_TXT_URI: &str = "resource://vox/llms.txt";
const RESOURCE_AGENTS_MD_URI: &str = "resource://vox/agents.md";

const LLMS_TXT_REL: &str = "docs/src/.well-known/llms.txt";
const AGENTS_MD_REL: &str = "AGENTS.md";

const PROMPT_AGENTS_POLICY: &str = "agents-policy";
const PROMPT_VOX_CHECK: &str = "vox-check";
const PROMPT_LLMS_DISCOVERY: &str = "llms-discovery";

fn static_vox_resources() -> Vec<Resource> {
    vec![
        Resource::new(
            RawResource::new(RESOURCE_LLMS_TXT_URI, "llms.txt")
                .with_title("Vox LLM discovery index")
                .with_description("Agent discovery index for the Vox repository")
                .with_mime_type("text/plain"),
            None,
        ),
        Resource::new(
            RawResource::new(RESOURCE_AGENTS_MD_URI, "agents.md")
                .with_title("AGENTS.md")
                .with_description("Cross-tool agent policy surface for the Vox repository")
                .with_mime_type("text/markdown"),
            None,
        ),
    ]
}

fn read_vox_resource(
    repo_root: &std::path::Path,
    uri: &str,
) -> Result<ReadResourceResult, ErrorData> {
    let (rel_path, mime_type) = match uri {
        RESOURCE_LLMS_TXT_URI => (LLMS_TXT_REL, "text/plain"),
        RESOURCE_AGENTS_MD_URI => (AGENTS_MD_REL, "text/markdown"),
        other => {
            return Err(ErrorData::invalid_params(
                format!("unknown resource uri: {other}"),
                None,
            ));
        }
    };

    let path = repo_root.join(rel_path);
    let text = std::fs::read_to_string(&path).map_err(|e| {
        ErrorData::internal_error(format!("failed to read {}: {e}", path.display()), None)
    })?;

    Ok(ReadResourceResult::new(vec![
        ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(mime_type.to_string()),
            text,
            meta: None,
        },
    ]))
}

fn static_vox_prompts() -> Vec<Prompt> {
    vec![
        Prompt::new(
            PROMPT_AGENTS_POLICY,
            Some("Load cross-tool agent policy from AGENTS.md"),
            None,
        )
        .with_title("Agents policy"),
        Prompt::new(
            PROMPT_VOX_CHECK,
            Some("Validate .vox source before edits using vox_check"),
            None,
        )
        .with_title("Vox check workflow"),
        Prompt::new(
            PROMPT_LLMS_DISCOVERY,
            Some("Repository discovery index from llms.txt"),
            None,
        )
        .with_title("LLM discovery index"),
    ]
}

fn get_vox_prompt(repo_root: &std::path::Path, name: &str) -> Result<GetPromptResult, ErrorData> {
    match name {
        PROMPT_AGENTS_POLICY => {
            let path = repo_root.join(AGENTS_MD_REL);
            let text = std::fs::read_to_string(&path).map_err(|e| {
                ErrorData::internal_error(format!("failed to read {}: {e}", path.display()), None)
            })?;
            Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                PromptMessageRole::User,
                format!("Follow this repository agent policy:\n\n{text}"),
            )]))
        }
        PROMPT_VOX_CHECK => Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            "Before committing .vox changes, call the `vox_check` MCP tool \
             (or `vox check --output-format json`) and fix all reported errors.",
        )])),
        PROMPT_LLMS_DISCOVERY => {
            let path = repo_root.join(LLMS_TXT_REL);
            let text = std::fs::read_to_string(&path).map_err(|e| {
                ErrorData::internal_error(format!("failed to read {}: {e}", path.display()), None)
            })?;
            Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                PromptMessageRole::User,
                format!("Use this discovery index:\n\n{text}"),
            )]))
        }
        other => Err(ErrorData::invalid_params(
            format!("unknown prompt: {other}"),
            None,
        )),
    }
}

/// RMCP [`ServerHandler`] implementation listing tools and dispatching `call_tool`.
///
/// T2.2: `state` still backs protocol-level concerns (tool-schema listing,
/// resources, prompts — all static/local data, no live orchestrator state
/// involved) so a client can discover this server's tools even before any
/// daemon is running. Actual tool *execution* (`call_tool`) is forwarded to
/// the single shared `vox-orchestrator-d` daemon via `daemon`, so a tool
/// call made through `vox mcp` lands in the same `ServerState` a GUI client
/// (or any other daemon client) sees — see `crate::daemon_route`.
pub struct VoxMcpServer {
    state: ServerState,
    daemon: std::sync::Arc<
        vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::OrchestratorDaemonEnsure,
    >,
}

impl VoxMcpServer {
    /// Wrap `state` for use with `rmcp` transport loops. `state` continues to
    /// back protocol-level concerns (tool listing/resources/prompts); tool
    /// execution routes through a freshly constructed daemon-ensure helper.
    pub fn new(state: ServerState) -> Self {
        let profile_id = state.active_profile.id_string();
        Self {
            state,
            daemon: std::sync::Arc::new(
                vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::OrchestratorDaemonEnsure::for_profile(
                    profile_id,
                ),
            ),
        }
    }
}

/// Local profile check for stdio `call_tool` (the daemon re-checks in `dispatch`).
pub fn profile_denial_result(state: &ServerState, name: &str) -> Option<CallToolResult> {
    let canonical = crate::canonical_tool_name(name);
    crate::profiles::guard_tool(state, crate::profiles::CallContext::LOCAL_WRITE, canonical)
        .err()
        .map(|denial| {
            let json =
                crate::profiles::denial_error_json(canonical, &denial, &state.active_profile);
            CallToolResult::error(vec![rmcp::model::Content::text(json.to_string())])
        })
}

impl ServerHandler for VoxMcpServer {
    async fn initialize(
        &self,
        params: InitializeRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        let tool_count = crate::registry::merged_tool_registry(&self.state).len();
        let vox_version = env!("CARGO_PKG_VERSION");
        let mut experimental: std::collections::BTreeMap<
            String,
            serde_json::Map<String, serde_json::Value>,
        > = std::collections::BTreeMap::new();
        let mut inner = serde_json::Map::new();
        inner.insert("messagepack".to_string(), serde_json::Value::Bool(true));
        inner.insert("inmem_transport".to_string(), serde_json::Value::Bool(true));
        experimental.insert("transport_capabilities".to_string(), inner);
        experimental.insert(
            "io.modelcontextprotocol/skills".to_string(),
            serde_json::Map::from_iter([(
                "version".to_string(),
                serde_json::Value::String("1".to_string()),
            )]),
        );
        // Skills may append tools after startup; `enable_tool_list_changed` tells
        // clients to refresh their tool list occasionally.
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_tool_list_changed()
            .enable_resources()
            .enable_prompts()
            .enable_experimental_with(experimental)
            .build();
        Ok(InitializeResult::new(capabilities)
            .with_protocol_version(params.protocol_version.clone())
            .with_server_info(Implementation::new("vox-mcp", vox_version))
            .with_instructions(format!(
                "vox-mcp v{} | tools: {} | protocol: {}",
                vox_version, tool_count, params.protocol_version,
            )))
    }

    async fn list_tools(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        if let Some(report) = crate::daemon_route::fetch_daemon_boot_report().await {
            self.state.set_boot_report(report);
        }
        let mut tool_list = crate::registry::merged_tool_registry(&self.state);

        let opt_in = workspace_tools_opt_in(
            std::env::var("VOX_MCP_WORKSPACE_TOOLS").ok().as_deref(),
            std::env::var("VOX_MCP_TIERS").ok().as_deref(),
        );
        warn_if_deprecated_tiers_env_set();
        retain_advertised_tools(&mut tool_list, opt_in != WorkspaceToolsOptIn::Off);

        // Auto-register tools from installed skills
        let skills = self.state.skill_registry.list(None);
        for skill in skills {
            for tool_name in &skill.tools {
                if !tool_list.iter().any(|t| t.name == *tool_name)
                    && crate::profiles::guard_tool(
                        &self.state,
                        crate::profiles::CallContext::LOCAL_WRITE,
                        tool_name,
                    )
                    .is_ok()
                {
                    let mut meta_map = serde_json::Map::new();
                    meta_map.insert(
                        "vox_source".to_string(),
                        serde_json::Value::String("skill".to_string()),
                    );
                    tool_list.push(
                        rmcp::model::Tool::new_with_raw(
                            std::borrow::Cow::Owned(tool_name.clone()),
                            Some(std::borrow::Cow::Owned(format!(
                                "Instructional macro tool from skill: {}",
                                skill.name
                            ))),
                            std::sync::Arc::new(serde_json::Map::new()),
                        )
                        .with_meta(rmcp::model::Meta(meta_map)),
                    );
                }
            }
        }

        Ok(ListToolsResult {
            meta: None,
            tools: tool_list,
            next_cursor: None,
        })
    }

    async fn list_resources(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let mut resources = static_vox_resources();
        for (uri, description) in
            crate::skills_resources::list_skill_resources(&self.state.skill_registry)
        {
            let mime_type = if uri.ends_with(".json") {
                "application/json"
            } else {
                "text/markdown"
            };
            resources.push(Resource::new(
                RawResource::new(uri.clone(), uri.clone())
                    .with_description(description)
                    .with_mime_type(mime_type),
                None,
            ));
        }
        for (uri, name) in crate::kb_resources::list_kb_doc_resources(&self.state.repository.root) {
            resources.push(Resource::new(
                RawResource::new(uri, name)
                    .with_description("Curated KB Library document")
                    .with_mime_type("text/markdown"),
                None,
            ));
        }
        for entry in &self.state.workspace_mcp.read().resources {
            resources.push(Resource::new(
                RawResource::new(entry.uri.clone(), entry.uri.clone())
                    .with_description(entry.description.clone())
                    .with_mime_type("text/plain"),
                None,
            ));
        }
        Ok(ListResourcesResult {
            meta: None,
            resources,
            next_cursor: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult {
            meta: None,
            resource_templates: crate::kb_resources::kb_resource_templates(),
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        params: ReadResourceRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        if params.uri.starts_with("skill://") {
            match crate::skills_resources::read_skill_resource(
                &self.state.skill_registry,
                &params.uri,
            ) {
                Ok((mime, text)) => {
                    return Ok(ReadResourceResult::new(vec![
                        ResourceContents::TextResourceContents {
                            uri: params.uri.clone(),
                            mime_type: Some(mime),
                            text,
                            meta: None,
                        },
                    ]));
                }
                Err(e) => {
                    return Err(ErrorData::invalid_params(e, None));
                }
            }
        }
        if params.uri.starts_with("kb://") {
            if let Err(e) = crate::kb_resources::parse_kb_uri(&params.uri) {
                return Err(ErrorData::invalid_params(e, None));
            }
            return match crate::kb_resources::read_kb_resource(&self.daemon, &params.uri).await {
                Ok((mime, text)) => Ok(ReadResourceResult::new(vec![
                    ResourceContents::TextResourceContents {
                        uri: params.uri.clone(),
                        mime_type: Some(mime),
                        text,
                        meta: None,
                    },
                ])),
                Err(e) => Err(ErrorData::internal_error(e, None)),
            };
        }
        if let Some(entry) = self.state.workspace_mcp.read().resource_by_uri(&params.uri) {
            let root = self
                .state
                .workspace_root
                .clone()
                .unwrap_or_else(|| self.state.repository.root.clone());
            match crate::workspace_mcp::dispatch_workspace_resource(
                &self.state.workspace_mcp.read(),
                &entry.uri,
                &root,
            ) {
                Ok(text) => {
                    return Ok(ReadResourceResult::new(vec![
                        ResourceContents::TextResourceContents {
                            uri: params.uri.clone(),
                            mime_type: Some("text/plain".into()),
                            text,
                            meta: None,
                        },
                    ]));
                }
                Err(e) => {
                    return Err(ErrorData::internal_error(e, None));
                }
            }
        }
        read_vox_resource(&self.state.repository.root, &params.uri)
    }

    async fn list_prompts(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Ok(ListPromptsResult {
            meta: None,
            prompts: static_vox_prompts(),
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        params: GetPromptRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, ErrorData> {
        get_vox_prompt(&self.state.repository.root, &params.name)
    }

    async fn call_tool(
        &self,
        params: CallToolRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        self.state.orchestrator.record_activity();
        let args = params
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
        let name_str = params.name.to_string();
        if let Some(denied) = profile_denial_result(&self.state, &name_str) {
            return Ok(denied);
        }
        let (result_json, is_error): (String, bool) =
            match crate::daemon_route::call_tool_via_daemon(&self.daemon, &name_str, args).await {
                Ok(json) => {
                    let is_err = tool_json_envelope_is_error(&json);
                    (json, is_err)
                }
                Err(e) => {
                    let msg = format!("{e}");
                    (
                        ToolResult::<serde_json::Value>::err_with_remediation(
                            msg,
                            REM_DAEMON_UNREACHABLE,
                        )
                        .to_json(),
                        true,
                    )
                }
            };

        let frames = vox_config::paths::browser_frames_cache_dir();
        let content =
            crate::tool_images::mcp_contents_for_tool_json(&name_str, &result_json, &frames);
        Ok(if is_error {
            CallToolResult::error(content)
        } else {
            CallToolResult::success(content)
        })
    }
}

/// Returns true when JSON looks like `ToolResult` with `success: false` (MCP `is_error` signal).
pub fn tool_json_envelope_is_error(json: &str) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return false;
    };
    v.get("success").and_then(|s| s.as_bool()) == Some(false)
        || crate::profiles::is_profile_denial_json(&v)
}

/// How the workspace-federated `@tool` opt-in was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceToolsOptIn {
    Off,
    On,
    OnViaDeprecatedTiers,
}

/// `VOX_MCP_WORKSPACE_TOOLS`, when set (even to empty), decides: `1`/`true` (trimmed,
/// case-insensitive) is on, anything else is off. When unset, the deprecated `VOX_MCP_TIERS`
/// alias opts in only for `workspace` or `all` (the values that revealed them before).
pub(crate) fn workspace_tools_opt_in(
    workspace_tools: Option<&str>,
    legacy_tiers: Option<&str>,
) -> WorkspaceToolsOptIn {
    if let Some(v) = workspace_tools {
        let v = v.trim();
        return if v == "1" || v.eq_ignore_ascii_case("true") {
            WorkspaceToolsOptIn::On
        } else {
            WorkspaceToolsOptIn::Off
        };
    }
    let legacy_on = legacy_tiers.is_some_and(|v| {
        v.split(',')
            .map(str::trim)
            .any(|t| matches!(t, "workspace" | "all"))
    });
    if legacy_on {
        WorkspaceToolsOptIn::OnViaDeprecatedTiers
    } else {
        WorkspaceToolsOptIn::Off
    }
}

/// Workspace-federated `@tool` functions are always dispatchable by name; they are advertised in
/// `tools/list` only when `include_workspace` is set.
pub(crate) fn retain_advertised_tools(tools: &mut Vec<rmcp::model::Tool>, include_workspace: bool) {
    if include_workspace {
        return;
    }
    tools.retain(|t| {
        t.meta
            .as_ref()
            .and_then(|m| m.0.get("vox_source"))
            .and_then(serde_json::Value::as_str)
            != Some("workspace")
    });
}

// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="capability-profiles" canonical="VOX_MCP_WORKSPACE_TOOLS"
fn warn_if_deprecated_tiers_env_set() {
    static WARNED: std::sync::Once = std::sync::Once::new();
    if std::env::var_os("VOX_MCP_TIERS").is_some() {
        WARNED.call_once(|| {
            tracing::warn!(
                "VOX_MCP_TIERS is deprecated; use VOX_MCP_WORKSPACE_TOOLS=1 to advertise workspace-federated tools"
            );
        });
    }
}

#[cfg(test)]
mod call_tool_tests {
    use super::tool_json_envelope_is_error;

    #[test]
    fn envelope_error_when_success_false() {
        assert!(tool_json_envelope_is_error(
            r#"{"success":false,"error":"nope"}"#
        ));
    }

    #[test]
    fn envelope_ok_when_success_true() {
        assert!(!tool_json_envelope_is_error(
            r#"{"success":true,"data":"x"}"#
        ));
    }

    #[test]
    fn envelope_ok_when_not_tool_result_shape() {
        assert!(!tool_json_envelope_is_error("not json"));
        assert!(!tool_json_envelope_is_error(r#"{"foo":1}"#));
    }

    #[test]
    fn envelope_error_for_profile_denial() {
        assert!(tool_json_envelope_is_error(
            r#"{"error":{"code":"tool_not_in_profile","message":"m"}}"#
        ));
        assert!(!tool_json_envelope_is_error(
            r#"{"error":{"code":"unrelated"}}"#
        ));
    }

    #[tokio::test]
    async fn profile_denial_result_is_error_for_out_of_profile_tool() {
        let state = crate::server_state::ServerState::new_test()
            .await
            .with_profile(crate::profiles::ActiveProfile::resolve(&["models"]).expect("models"));
        let r = super::profile_denial_result(&state, "vox_git_status").expect("denied");
        assert_eq!(r.is_error, Some(true));
        assert!(super::profile_denial_result(&state, "vox_list_models").is_none());
        let dev = crate::server_state::ServerState::new_test().await;
        assert!(super::profile_denial_result(&dev, "vox_git_status").is_none());
    }
}

#[cfg(test)]
mod prompt_tests {
    use super::*;

    #[test]
    fn static_prompt_catalog_lists_three() {
        assert_eq!(static_vox_prompts().len(), 3);
        let names: Vec<_> = static_vox_prompts().into_iter().map(|p| p.name).collect();
        assert!(names.contains(&PROMPT_AGENTS_POLICY.to_string()));
        assert!(names.contains(&PROMPT_VOX_CHECK.to_string()));
    }

    #[test]
    fn get_vox_check_prompt_has_message() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let result = get_vox_prompt(&root, PROMPT_VOX_CHECK).expect("vox-check prompt");
        assert!(!result.messages.is_empty());
    }

    #[test]
    fn get_unknown_prompt_is_invalid_params() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let err = get_vox_prompt(&root, "no-such-prompt").expect_err("unknown prompt");
        assert!(err.message.contains("unknown prompt"));
    }
}

#[cfg(test)]
mod list_filter_tests {
    use std::borrow::Cow;
    use std::sync::Arc;

    use rmcp::model::{Meta, Tool};

    use super::{WorkspaceToolsOptIn, retain_advertised_tools, workspace_tools_opt_in};

    fn tool(name: &'static str, source: Option<&str>) -> Tool {
        let mut meta = serde_json::Map::new();
        if let Some(s) = source {
            meta.insert("vox_source".into(), serde_json::Value::String(s.into()));
        }
        Tool::new_with_raw(
            Cow::Borrowed(name),
            Some(Cow::Borrowed("test tool")),
            Arc::new(serde_json::Map::new()),
        )
        .with_meta(Meta(meta))
    }

    fn sample() -> Vec<Tool> {
        vec![
            tool("vox_git_status", None),
            tool("read_file", Some("workspace")),
            tool("skill_macro", Some("skill")),
        ]
    }

    fn names(tools: &[Tool]) -> Vec<&str> {
        tools.iter().map(|t| t.name.as_ref()).collect()
    }

    #[test]
    fn default_hides_workspace_federated_and_keeps_catalog_and_skill_tools() {
        let mut tools = sample();
        retain_advertised_tools(&mut tools, false);
        assert_eq!(names(&tools), vec!["vox_git_status", "skill_macro"]);
    }

    #[test]
    fn opt_in_keeps_workspace_federated_tools() {
        let mut tools = sample();
        retain_advertised_tools(&mut tools, true);
        assert_eq!(
            names(&tools),
            vec!["vox_git_status", "read_file", "skill_macro"]
        );
    }

    #[test]
    fn opt_in_resolution() {
        use WorkspaceToolsOptIn::*;
        assert_eq!(workspace_tools_opt_in(None, None), Off);
        assert_eq!(workspace_tools_opt_in(Some("1"), None), On);
        assert_eq!(workspace_tools_opt_in(Some("true"), None), On);
        assert_eq!(workspace_tools_opt_in(Some("0"), None), Off);
        // Case-insensitive after trim; `yes` is deliberately not accepted.
        assert_eq!(workspace_tools_opt_in(Some("TRUE"), None), On);
        assert_eq!(workspace_tools_opt_in(Some("True"), None), On);
        assert_eq!(workspace_tools_opt_in(Some(" 1 "), None), On);
        assert_eq!(workspace_tools_opt_in(Some("yes"), None), Off);
        // Set-but-empty is "set and off": it masks the deprecated alias.
        assert_eq!(workspace_tools_opt_in(Some(""), None), Off);
        assert_eq!(workspace_tools_opt_in(Some(""), Some("all")), Off);
        // The new variable wins over the deprecated alias when set.
        assert_eq!(workspace_tools_opt_in(Some("0"), Some("all")), Off);
        assert_eq!(workspace_tools_opt_in(None, Some("core")), Off);
        // Not widened: these never revealed workspace tools and still do not.
        assert_eq!(workspace_tools_opt_in(None, Some("dev")), Off);
        assert_eq!(workspace_tools_opt_in(None, Some("advanced")), Off);
        for legacy in ["workspace", "all", "core,workspace", " all "] {
            assert_eq!(
                workspace_tools_opt_in(None, Some(legacy)),
                OnViaDeprecatedTiers,
                "{legacy}"
            );
        }
    }
}
