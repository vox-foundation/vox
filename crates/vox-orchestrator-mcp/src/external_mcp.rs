//! RMCP client wiring for outbound connections to third-party MCP servers.

use anyhow::{Context, Result, anyhow, bail};
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, Tool};
use rmcp::service::{RunningService, ServiceError};
use rmcp::{RoleClient, ServiceExt};

use crate::mcp_client::ToolDefinition;

/// Configuration for an external MCP server reachable via stdio subprocess or HTTP.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExternalMcpServer {
    /// Stable identifier used in caches and telemetry (`server_id`).
    pub name: String,
    /// Stdio transport: executable to spawn (mutually exclusive with [`Self::url`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Arguments passed to [`Self::command`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Streamable HTTP transport endpoint (mutually exclusive with [`Self::command`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl ExternalMcpServer {
    /// Validate transport fields: exactly one of stdio (`command`) or HTTP (`url`).
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("external MCP server name must be non-empty");
        }
        match (&self.command, &self.url) {
            (Some(cmd), None) if !cmd.trim().is_empty() => Ok(()),
            (None, Some(url)) if !url.trim().is_empty() => Ok(()),
            (Some(_), Some(_)) => bail!(
                "external MCP server '{}' must specify either command or url, not both",
                self.name
            ),
            _ => bail!(
                "external MCP server '{}' requires a non-empty command or url",
                self.name
            ),
        }
    }

    fn client_info(&self) -> ClientInfo {
        ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("vox-external-mcp-client", env!("CARGO_PKG_VERSION"))
                .with_title(format!("vox external MCP client ({})", self.name)),
        )
    }
}

fn tool_to_definition(tool: &Tool) -> ToolDefinition {
    ToolDefinition {
        name: tool.name.to_string(),
        description: tool.description.as_deref().unwrap_or_default().to_string(),
        input_schema: serde_json::Value::Object((*tool.input_schema).clone()),
    }
}

async fn connect(config: &ExternalMcpServer) -> Result<RunningService<RoleClient, ClientInfo>> {
    config.validate()?;

    let client = config.client_info();

    if let Some(command) = &config.command {
        use rmcp::transport::TokioChildProcess;
        use tokio::process::Command as TokioCommand;

        let mut cmd = TokioCommand::new(command);
        cmd.args(&config.args);
        let raw_search_env = std::env::var("ENABLE_TOOL_SEARCH").ok();
        cmd.env(
            "ENABLE_TOOL_SEARCH",
            sanitize_mcp_tool_search_env(raw_search_env.as_deref()),
        );
        let transport = TokioChildProcess::new(cmd)
            .with_context(|| format!("spawn external MCP server '{}'", config.name))?;
        client
            .serve(transport)
            .await
            .map_err(|e| anyhow!("initialize external MCP server '{}': {e}", config.name))
    } else {
        use rmcp::transport::StreamableHttpClientTransport;

        let url = config
            .url
            .as_deref()
            .expect("validate() ensures url is present when command is absent");
        let transport = StreamableHttpClientTransport::from_uri(url);
        client
            .serve(transport)
            .await
            .map_err(|e| anyhow!("initialize external MCP server '{}': {e}", config.name))
    }
}

/// Connect to `config`, list all tools, then tear down the session.
pub async fn connect_and_list_tools(config: &ExternalMcpServer) -> Result<Vec<ToolDefinition>> {
    let mut session = connect(config).await?;
    let tools = session
        .peer()
        .list_all_tools()
        .await
        .map_err(map_service_error)?;
    session.close().await.ok();
    Ok(tools.iter().map(tool_to_definition).collect())
}

fn map_service_error(err: ServiceError) -> anyhow::Error {
    anyhow!("external MCP tool listing failed: {err}")
}

/// Maximum in-loop discovery rounds for the 3-tool MCP discovery wrapper (`pi-t3chat` `MAX_WRAPPER_ROUNDS`).
pub const MAX_LAZY_MCP_WRAPPER_ROUNDS: usize = 6;

/// Override `ENABLE_TOOL_SEARCH="auto"` or `"auto:N"` (`0 < N < 100`) to `"true"` (`t3code` `#24654`)
/// so deferred MCP tool definitions never flip inline mid-session and invalidate the prompt cache prefix.
#[must_use]
pub fn sanitize_mcp_tool_search_env(raw_val: Option<&str>) -> &'static str {
    let Some(trimmed) = raw_val.map(str::trim) else {
        return "true";
    };
    if trimmed.eq_ignore_ascii_case("false") || trimmed.eq_ignore_ascii_case("auto:0") {
        return "false";
    }
    if trimmed.eq_ignore_ascii_case("auto") {
        return "true";
    }
    if let Some(rest) = trimmed.strip_prefix("auto:") {
        if let Ok(pct) = rest.trim().parse::<u32>() {
            if pct > 0 && pct < 100 {
                return "true";
            }
        }
    }
    "true"
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LazyMcpToolEntry {
    pub name: String,
    pub client_tool_name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LazyMcpGroup {
    pub name: String,
    pub alias: String,
    pub tools: Vec<LazyMcpToolEntry>,
}

/// Lazy 3-tool MCP discovery registry (`list_mcp_servers`, `list_mcp_tools`, `resolve_call_mcp`)
/// extracted from `vibheksoni/pi-t3chat/mcp.ts` (`VOX-HARNESS-T3-02`).
#[derive(Debug, Clone, Default)]
pub struct LazyMcpGroupRegistry {
    groups: std::collections::BTreeMap<String, LazyMcpGroup>,
}

fn normalize_mcp_key(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

impl LazyMcpGroupRegistry {
    /// Group `mcp__<server>_<tool>` definitions into on-demand server groups so full MCP schemas
    /// do not bloat the initial system prompt.
    #[must_use]
    pub fn from_prefixed_tools(tools: &[ToolDefinition]) -> Self {
        let mut raw_groups: std::collections::BTreeMap<String, Vec<LazyMcpToolEntry>> =
            std::collections::BTreeMap::new();

        for t in tools {
            let full = t.name.trim();
            let Some(rest) = full.strip_prefix("mcp__") else {
                continue;
            };
            let Some((server_part, tool_part)) = rest.split_once('_') else {
                continue;
            };
            if server_part.is_empty() || tool_part.is_empty() {
                continue;
            }
            let group_name = format!("mcp__{server_part}");
            raw_groups
                .entry(group_name)
                .or_default()
                .push(LazyMcpToolEntry {
                    name: tool_part.to_string(),
                    client_tool_name: full.to_string(),
                    description: t.description.clone(),
                    parameters: t.input_schema.clone(),
                });
        }

        let mut groups = std::collections::BTreeMap::new();
        for (idx, (group_name, entries)) in raw_groups.into_iter().enumerate() {
            let alias = format!("source_{:02}", idx + 1);
            groups.insert(
                group_name.clone(),
                LazyMcpGroup {
                    name: group_name,
                    alias,
                    tools: entries,
                },
            );
        }
        Self { groups }
    }

    #[must_use]
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Tier-1 wrapper (`vox_list_mcp_servers` / `list_mcps`): returns server groups, `source_NN` aliases,
    /// tool counts, and an 8-tool preview.
    #[must_use]
    pub fn list_mcp_servers(&self) -> serde_json::Value {
        let mcps: Vec<serde_json::Value> = self
            .groups
            .values()
            .map(|g| {
                let preview: Vec<&str> = g.tools.iter().take(8).map(|t| t.name.as_str()).collect();
                serde_json::json!({
                    "mcp_name": g.name,
                    "alias": g.alias,
                    "tool_count": g.tools.len(),
                    "tools_preview": preview,
                })
            })
            .collect();
        serde_json::json!({ "mcps": mcps })
    }

    fn resolve_group(&self, mcp_name: &str) -> Option<&LazyMcpGroup> {
        let trimmed = mcp_name.trim();
        if let Some(exact) = self.groups.get(trimmed) {
            return Some(exact);
        }
        let wanted = normalize_mcp_key(trimmed);
        for g in self.groups.values() {
            if normalize_mcp_key(&g.alias) == wanted
                || normalize_mcp_key(&g.name) == wanted
                || g.name
                    .strip_prefix("mcp__")
                    .is_some_and(|short| normalize_mcp_key(short) == wanted)
            {
                return Some(g);
            }
        }
        None
    }

    /// Tier-2 wrapper (`vox_list_mcp_tools` / `list_mcp_tools`): returns full tool schemas for one server group.
    #[must_use]
    pub fn list_mcp_tools(&self, mcp_name: &str) -> serde_json::Value {
        let Some(group) = self.resolve_group(mcp_name) else {
            let available: Vec<serde_json::Value> = self
                .groups
                .values()
                .map(|g| serde_json::json!({ "mcp_name": g.name, "alias": g.alias }))
                .collect();
            return serde_json::json!({
                "error": format!("MCP group '{mcp_name}' not found."),
                "available_mcps": available,
            });
        };
        let tools: Vec<serde_json::Value> = group
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "client_tool_name": t.client_tool_name,
                    "description": t.description,
                    "parameters": t.parameters,
                })
            })
            .collect();
        serde_json::json!({
            "mcp_name": group.name,
            "alias": group.alias,
            "tools": tools,
        })
    }

    /// Tier-3 wrapper (`vox_call_mcp_tool` / `call_mcp`): resolves `(mcp_name, tool_name)` to the
    /// canonical `client_tool_name` and validated JSON arguments.
    pub fn resolve_call_mcp(
        &self,
        mcp_name: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<(String, serde_json::Value)> {
        let Some(group) = self.resolve_group(mcp_name) else {
            let names: Vec<&str> = self.groups.keys().map(String::as_str).collect();
            bail!(
                "MCP group '{}' not found. Available: {}",
                mcp_name,
                names.join(", ")
            );
        };
        let wanted = normalize_mcp_key(tool_name);
        let found = group.tools.iter().find(|t| {
            normalize_mcp_key(&t.name) == wanted || normalize_mcp_key(&t.client_tool_name) == wanted
        });
        let Some(tool) = found else {
            let names: Vec<&str> = group.tools.iter().map(|t| t.name.as_str()).collect();
            bail!(
                "Tool '{}' not found in '{}'. Available: {}",
                tool_name,
                group.name,
                names.join(", ")
            );
        };
        let normalized_args = if arguments.is_object() {
            arguments
        } else {
            serde_json::json!({})
        };
        Ok((tool.client_tool_name.clone(), normalized_args))
    }
}

#[cfg(test)]
mod tests {
    use super::ExternalMcpServer;

    #[test]
    fn parse_stdio_config_from_json() {
        // /tmp is a test fixture arg — not portable runtime code.
        // vox-arch-check: allow abs-path
        let raw = r#"{ "name": "filesystem", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"] }"#;
        let cfg: ExternalMcpServer = serde_json::from_str(raw).expect("parse");
        assert_eq!(cfg.name, "filesystem");
        assert_eq!(cfg.command.as_deref(), Some("npx"));
        assert_eq!(cfg.args.len(), 3);
        assert!(cfg.url.is_none());
        cfg.validate().expect("valid stdio config");
    }

    #[test]
    fn parse_http_config_from_json() {
        let raw = r#"{
            "name": "remote",
            "url": "http://127.0.0.1:8000/mcp"
        }"#;
        let cfg: ExternalMcpServer = serde_json::from_str(raw).expect("parse");
        assert_eq!(cfg.name, "remote");
        assert!(cfg.command.is_none());
        assert_eq!(cfg.url.as_deref(), Some("http://127.0.0.1:8000/mcp"));
        cfg.validate().expect("valid http config");
    }

    #[test]
    fn reject_empty_name_and_dual_transport() {
        let bad_name = ExternalMcpServer {
            name: "  ".into(),
            command: Some("vox".into()),
            args: vec![],
            url: None,
        };
        assert!(bad_name.validate().is_err());

        let dual = ExternalMcpServer {
            name: "both".into(),
            command: Some("vox".into()),
            args: vec![],
            url: Some("http://localhost/mcp".into()),
        };
        assert!(dual.validate().is_err());
    }

    #[test]
    fn lazy_mcp_group_registry_supports_list_servers_list_tools_and_call_resolution() {
        use super::LazyMcpGroupRegistry;
        use crate::mcp_client::ToolDefinition;

        let tools = vec![
            ToolDefinition {
                name: "mcp__github_create_issue".into(),
                description: "Create a GitHub issue".into(),
                input_schema: serde_json::json!({"type": "object", "required": ["title"]}),
            },
            ToolDefinition {
                name: "mcp__github_list_prs".into(),
                description: "List GitHub PRs".into(),
                input_schema: serde_json::json!({"type": "object"}),
            },
            ToolDefinition {
                name: "mcp__postgres_query_sql".into(),
                description: "Run read-only SQL".into(),
                input_schema: serde_json::json!({"type": "object", "required": ["sql"]}),
            },
        ];

        let reg = LazyMcpGroupRegistry::from_prefixed_tools(&tools);
        assert_eq!(reg.group_count(), 2);

        let servers = reg.list_mcp_servers();
        let arr = servers["mcps"].as_array().expect("mcps array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["mcp_name"], "mcp__github");
        assert_eq!(arr[0]["alias"], "source_01");
        assert_eq!(arr[0]["tool_count"], 2);

        // Resolve via source_01 alias
        let gh_tools = reg.list_mcp_tools("source_01");
        assert_eq!(gh_tools["mcp_name"], "mcp__github");
        assert_eq!(gh_tools["tools"].as_array().unwrap().len(), 2);

        // Resolve call_mcp to original client_tool_name
        let (client_name, args) = reg
            .resolve_call_mcp(
                "github",
                "create_issue",
                serde_json::json!({"title": "Bug report"}),
            )
            .expect("resolve call_mcp");
        assert_eq!(client_name, "mcp__github_create_issue");
        assert_eq!(args["title"], "Bug report");

        assert_eq!(super::sanitize_mcp_tool_search_env(None), "true");
        assert_eq!(super::sanitize_mcp_tool_search_env(Some("auto")), "true");
        assert_eq!(super::sanitize_mcp_tool_search_env(Some("auto:50")), "true");
        assert_eq!(super::sanitize_mcp_tool_search_env(Some("false")), "false");
        assert_eq!(super::sanitize_mcp_tool_search_env(Some("auto:0")), "false");
    }
}
