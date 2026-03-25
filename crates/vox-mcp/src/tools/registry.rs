//! RMCP tool descriptors built from the static MCP registry.

use super::input_schemas;
use super::TOOL_REGISTRY;

/// Convert the static [`TOOL_REGISTRY`] table into RMCP [`rmcp::model::Tool`] descriptors.
pub fn tool_registry() -> Vec<rmcp::model::Tool> {
    TOOL_REGISTRY
        .iter()
        .map(|(n, d)| rmcp::model::Tool {
            name: std::borrow::Cow::Owned(n.to_string()),
            description: Some(std::borrow::Cow::Owned(d.to_string())),
            input_schema: std::sync::Arc::new(input_schemas::tool_input_schema(n)),
            output_schema: None,
            meta: None,
            annotations: None,
            execution: None,
            icons: None,
            title: None,
        })
        .collect()
}
