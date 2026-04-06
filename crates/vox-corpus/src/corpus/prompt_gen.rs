//! Domain-specific prompt generators for synthetic training data Task 3.1.
//!
//! Provides a registry of templates and randomization helpers to generate
//! varied prompts for different training lanes.
//!
//! **SSOT:** Prompt variety is key to avoiding overfitting on specific phrasing.

use rand::seq::SliceRandom;

/// Domain category for prompt generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptDomain {
    Syntax,
    ActorModel,
    Workflows,
    ToolUse,
    Diagnostics,
}

/// A collection of templates for a specific domain.
struct DomainTemplates {
    /// Generic instruction starters (e.g. "Write me", "Show me", "Create").
    starters: &'static [&'static str],
    /// Core concepts / subjects (e.g. "an actor", "a durable workflow", "a script").
    subjects: &'static [&'static str],
    /// Constraints or specific tasks (e.g. "with state", "that handles retries", "with error handling").
    constraints: &'static [&'static str],
}

const SYNTAX: DomainTemplates = DomainTemplates {
    starters: &["Write me", "Show me", "Generate", "I need", "Create a"],
    subjects: &["Vox program", "function", "piece of Vox code", "syntax example", "script in Vox"],
    constraints: &[
        "that uses pattern matching",
        "with a custom type definition",
        "demonstrating nested loops",
        "with multiple return values",
        "using the optional operator",
    ],
};

const ACTOR: DomainTemplates = DomainTemplates {
    starters: &["Create", "Write", "Show me how to make", "Implement"],
    subjects: &["a Vox actor", "an actor model example", "a stateful actor", "an actor definition"],
    constraints: &[
        "that manages a counter",
        "with message handling for generic events",
        "that spawns child actors",
        "with isolated state variables",
        "implementing a pub/sub pattern",
    ],
};

const WORKFLOWS: DomainTemplates = DomainTemplates {
    starters: &["Generate", "I want", "Specify", "Show"],
    subjects: &["a durable workflow", "a linear workflow", "a Vox workflow", "an activity sequence"],
    constraints: &[
        "with persistent state across steps",
        "that uses the `match` construct",
        "with error recovery logic",
        "demonstrating long-running durability",
        "that calls multiple actors",
    ],
};

const TOOL_USE: DomainTemplates = DomainTemplates {
    starters: &["Show", "Write", "Simulate", "Create"],
    subjects: &["an MCP tool call", "a tool-enabled script", "an agentic tool trace", "a tool-use example"],
    constraints: &[
        "using `@mcp` annotations",
        "that interacts with a database action",
        "demonstrating tool result parsing",
        "with multi-turn tool interaction",
        "that uses the file-system skill",
    ],
};

/// Randomly generate a prompt for the given domain.
pub fn generate_prompt(domain: PromptDomain, rng: &mut rand::rngs::StdRng) -> String {
    let t = match domain {
        PromptDomain::Syntax => &SYNTAX,
        PromptDomain::ActorModel => &ACTOR,
        PromptDomain::Workflows => &WORKFLOWS,
        PromptDomain::ToolUse => &TOOL_USE,
        PromptDomain::Diagnostics => &SYNTAX, // fallback
    };

    let s = t.starters.choose(rng).unwrap_or(&"Write");
    let sub = t.subjects.choose(rng).unwrap_or(&"code");
    let c = t.constraints.choose(rng).unwrap_or(&"");

    format!("{} {} {}.", s, sub, c).trim().to_string()
}

/// Batch generate prompts for a target count.
pub fn batch_generate(domain: PromptDomain, count: usize, seed: u64) -> Vec<String> {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(generate_prompt(domain, &mut rng));
    }
    out
}
