//! Construct taxonomy and curriculum difficulty.

/// The full taxonomy of construct types for coverage reporting.
pub const TAXONOMY: &[&str] = &[
    "action",
    "activity",
    "actor",
    "agent",
    "agent_def",
    "collection",
    "component",
    "config",
    "const",
    "context",
    "environment",
    "error_boundary",
    "fixture",
    "function",
    "hook",
    "http_route",
    "import",
    "impl",
    "index",
    "keyframes",
    "layout",
    "loading",
    "mcp_resource",
    "mcp_tool",
    "message",
    "mock",
    "mutation",
    "not_found",
    "page",
    "provider",
    "query",
    "routes",
    "scheduled",
    "search_index",
    "server_fn",
    "skill",
    "table",
    "test",
    "theme",
    "trait",
    "type",
    "v0_component",
    "vector_index",
    "workflow",
];

/// Return a difficulty score (0-10) for a construct category.
/// Used for curriculum learning: sort training pairs simple→complex.
pub fn construct_difficulty(construct: &str) -> u8 {
    match construct {
        // Tier 0: basic building blocks
        "const" | "import" => 1,
        // Tier 1: simple declarations
        "type" | "function" | "test" | "fixture" | "mock" => 2,
        // Tier 2: UI and data
        "component" | "v0_component" | "table" | "index" | "page" => 3,
        "hook" | "provider" | "context" | "layout" | "loading" | "not_found" => 3,
        // Tier 3: medium complexity
        "query" | "mutation" | "action" | "server_fn" | "http_route" => 4,
        "routes" | "config" | "message" | "scheduled" | "keyframes" => 4,
        "theme" | "environment" | "error_boundary" => 4,
        // Tier 4: concurrency and data
        "actor" | "collection" | "vector_index" | "search_index" => 5,
        "trait" | "impl" => 5,
        // Tier 5: advanced patterns
        "activity" | "workflow" => 7,
        "agent" | "agent_def" | "skill" => 8,
        "mcp_tool" | "mcp_resource" => 8,
        // Documentation-sourced
        "documentation" => 3,
        _ => 5,
    }
}
