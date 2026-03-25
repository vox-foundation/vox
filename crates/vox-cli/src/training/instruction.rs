//! Instruction pair templates and name extraction.

/// Instruction templates keyed by construct type.
/// Each entry is a list of template strings where `{name}` is replaced
/// with the primary identifier extracted from the code.
pub fn instruction_templates(construct: &str) -> &[&str] {
    match construct {
        "function" => &[
            "Write a Vox function called {name}",
            "Implement the {name} function using Vox syntax",
        ],
        "component" => &[
            "Write a Vox UI component called {name}",
            "Create a {name} component in Vox using JSX syntax",
        ],
        "actor" => &[
            "Write a Vox actor called {name} with state management",
            "Create a {name} actor in Vox using the actor model",
        ],
        "workflow" => &[
            "Write a Vox durable workflow called {name}",
            "Create a {name} workflow in Vox with retry policies",
        ],
        "activity" => &[
            "Write a Vox activity called {name}",
            "Create a retryable {name} activity in Vox",
        ],
        "table" => &[
            "Define a Vox database table called {name}",
            "Create a {name} table using @table in Vox",
        ],
        "query" => &[
            "Write a Vox database query called {name}",
            "Create a read-only query {name} in Vox",
        ],
        "mutation" => &[
            "Write a Vox database mutation called {name}",
            "Create a {name} mutation in Vox for data modification",
        ],
        "action" => &["Write a Vox server action called {name}"],
        "type" => &[
            "Define a Vox tagged union type called {name}",
            "Create a {name} ADT in Vox with typed variants",
        ],
        "test" => &[
            "Write a Vox test for {name}",
            "Create unit tests in Vox using @test and assert",
        ],
        "mcp_tool" => &[
            "Write a Vox MCP tool called {name}",
            "Create an MCP-compatible tool in Vox for AI assistants",
        ],
        "mcp_resource" => &["Write a Vox MCP resource for {name}"],
        "http_route" => &[
            "Write an HTTP route in Vox",
            "Create an HTTP endpoint in Vox",
        ],
        "routes" => &["Define client-side routes in Vox"],
        "server_fn" => &["Write a Vox server function called {name}"],
        "skill" => &["Write a Vox skill called {name}"],
        "agent_def" => &["Define a Vox AI agent called {name}"],
        "trait" => &["Define a Vox trait called {name}"],
        _ => &["Write Vox code demonstrating {name}"],
    }
}

/// Extract the primary name from a Vox source string.
pub fn extract_name_from_source(code: &str) -> String {
    // Try keywords that precede a name: fn, actor, type, workflow, etc.
    let keywords = [
        "fn ",
        "actor ",
        "type ",
        "workflow ",
        "activity ",
        "trait ",
        "agent ",
        "skill ",
        "hook ",
        "layout ",
    ];
    for line in code.lines() {
        let trimmed = line.trim();
        for kw in &keywords {
            if let Some(rest) = trimmed.strip_prefix(kw) {
                // Also check after decorators like "@component fn Name"
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    return name;
                }
            }
            // Check for decorator-prefixed: "@component fn Name"
            if trimmed.starts_with('@') {
                if let Some(idx) = trimmed.find(kw) {
                    let rest = &trimmed[idx + kw.len()..];
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        return name;
                    }
                }
            }
        }
    }
    "example".to_string()
}
